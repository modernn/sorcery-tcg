import { readFile } from 'node:fs/promises';
import { basename, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  canonicalArtifactSchema,
  formatArtifactSchema,
  normalizedCardSnapshotSchema,
  type FormatDefinition,
  type Hash,
  type NormalizedCard,
} from '../authority/schemas.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  observeGame,
  stepGame,
  verifyGameReplay,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameElement,
  type GameLegalAction,
  type GameManifest,
  type GameSeat,
  type GameSession,
} from '../engine/game.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const DEFAULT_SCENARIO = resolve(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'scenarios',
  'vanilla-constructed.json',
);
type ScenarioConfig = Readonly<{
  airborneSeed: number;
  airSeed: number;
  avatar: Readonly<{ drawSpell: boolean; stableId: string }>;
  cannotDefendMinionStableId: string;
  chargeMinionStableId: string;
  deathriteMinionStableId: string;
  earthProviderMinionStableId: string;
  earthFirstStrikeSeed: number;
  earthRangedSeed: number;
  earthSeed: number;
  earthWardSeed: number;
  fireSeed: number;
  firstStrikeMinionStableId: string;
  firstStrikeTargetMinionStableId: string;
  genesisMinionStableId: string;
  ghostTownSiteStableId: string;
  healingMinionStableId: string;
  lethalMinionStableId: string;
  lumberingMinionStableId: string;
  manaMinionStableId: string;
  monstrousLionStableId: string;
  movementMinionStableId: string;
  movementTwoSeed: number;
  providerMinionStableId: string;
  rangedMinionStableId: string;
  revisionId: string;
  roamingMinionStableId: string;
  roamingSeed: number;
  seed: number;
  sedgeCrabsSeed: number;
  slyFoxSeed: number;
  stealthSeed: number;
  waterSeed: number;
  wardMinionStableId: string;
}>;

type DeckList = Readonly<{
  atlas: readonly Readonly<{ copies: number; name: string }>[];
  avatar: string;
  spellbook: readonly Readonly<{ copies: number; name: string }>[];
}>;

export type PrivateGameCheck = Readonly<{
  acceptedActionCount: number;
  airArcLightning: Readonly<{
    acceptedActionCount: number;
    arcLightning: string;
    deck: DeckList;
    farSameRegionUnitUnavailable: boolean;
    manaPaid: number;
    nearbyTargetAvailable: boolean;
    replayVerified: boolean;
    snowLeopard: string;
    snowLeopardDied: boolean;
    spellEnteredCemetery: boolean;
  }>;
  airLightningBolt: Readonly<{
    acceptedActionCount: number;
    avatarUnchanged: boolean;
    deck: DeckList;
    lethalDamageRecorded: boolean;
    lightningBolt: string;
    manaPaid: number;
    occupiedLocationTargeted: boolean;
    randomSelectionRecorded: boolean;
    replayVerified: boolean;
    snowLeopard: string;
    snowLeopardDied: boolean;
    spellEnteredCemetery: boolean;
  }>;
  airRainOfArrows: Readonly<{
    acceptedActionCount: number;
    avatarsPreserved: boolean;
    causalEventsVerified: boolean;
    cemeteriesOtherwisePreserved: boolean;
    deck: DeckList;
    gameRemainedActive: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    noTargetChoice: boolean;
    rainOfArrows: string;
    replayVerified: boolean;
    sitesPreserved: boolean;
    snowLeopard: string;
    spellEnteredCemetery: boolean;
    surfaceLeopardsDamagedAndSurvived: boolean;
  }>;
  airSpellcasterFreeze: Readonly<{
    acceptedActionCount: number;
    apprenticeWizard: string;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactCasterRelativeAction: boolean;
    freeze: string;
    genesisDrewSpell: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    replayVerified: boolean;
    seravaDisabled: boolean;
    seravaTownsfolk: string;
    spellEnteredCemetery: boolean;
    wizardCastWhileSummoningSick: boolean;
    wizardStatePreserved: boolean;
  }>;
  airTeleport: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactAllySitePair: boolean;
    manaPaid: number;
    noPathTeleport: boolean;
    replayVerified: boolean;
    siteUnchanged: boolean;
    snowLeopard: string;
    spellEnteredCemetery: boolean;
    teleportedToOpponentSiteSurface: boolean;
    teleport: string;
    unitStatePreserved: boolean;
  }>;
  airGenesisSpell: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    drewSpell: boolean;
    genesisMinion: string;
    handSizePreserved: boolean;
    hiddenFromOpponent: boolean;
    replayVerified: boolean;
    seed: number;
  }>;
  airLeyline: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    firstHengeDrewNothing: boolean;
    genesisDrewOne: boolean;
    henge: string;
    hiddenFromOpponent: boolean;
    replayVerified: boolean;
    seed: number;
  }>;
  airborne: Readonly<{
    acceptedActionCount: number;
    airborneCanAttackGround: boolean;
    airborneMinion: string;
    deck: DeckList;
    diagonalMove: boolean;
    groundCannotAttackAirborne: boolean;
    groundCannotIntercept: boolean;
    groundMinion: string;
    replayVerified: boolean;
    seed: number;
  }>;
  airMovement: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    movementMinion: string;
    replayVerified: boolean;
    seed: number;
    twoStepDefend: boolean;
    twoStepMoveAndAttack: boolean;
  }>;
  airMovementTwo: Readonly<{
    acceptedActionCount: number;
    attackAvailableAfterThreeSteps: boolean;
    deck: DeckList;
    movementMinion: string;
    replayVerified: boolean;
    repeatedStepUnavailable: boolean;
    returningPathAvailable: boolean;
    seed: number;
    threeStepAirbornePath: boolean;
  }>;
  airSummoning: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    ordinaryRestricted: boolean;
    replayVerified: boolean;
    roamingMinion: string;
    seed: number;
    summonedAtEnemySite: boolean;
  }>;
  airVoidwalk: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    forsaken: string;
    forsakenInnerSurfaceUnavailable: boolean;
    forsakenInnerVoidUnavailable: boolean;
    forsakenOuterVoidAvailable: boolean;
    nonVoidSurfaceAvailable: boolean;
    nonVoidVoidUnavailable: boolean;
    replayVerified: boolean;
    seed: number;
    siteTargetAvailableAfterExit: boolean;
    subsurfaceExitUnavailable: boolean;
    summonedInVoid: boolean;
    surfaceExitAvailable: boolean;
    surfaceSummonAvailable: boolean;
    targetWasVoid: boolean;
    voidMoveAvailable: boolean;
    voidSummonAvailable: boolean;
    voidwalkMinion: string;
  }>;
  airZap: Readonly<{
    acceptedActionCount: number;
    damageDealt: number;
    deck: DeckList;
    manaPaid: number;
    replayVerified: boolean;
    snowLeopard: string;
    snowLeopardSurvived: boolean;
    spellEnteredCemetery: boolean;
    spellLeftHand: boolean;
    zap: string;
  }>;
  authorityHash: Hash;
  avatarSpellDrawn: boolean;
  charge: Readonly<{ activatedOnSummon: boolean; minion: string }>;
  classification: 'private-local_actual-cards_unranked-partial-rules';
  combat: Readonly<{
    northMinion: string;
    northMinionDied: boolean;
    southMinion: string;
    southMinionDied: boolean;
  }>;
  decks: Readonly<Record<GameSeat, DeckList>>;
  finalStateHash: Hash;
  formatStableId: string;
  genesis: Readonly<{ minion: string; siteDrawn: boolean }>;
  earthBurrowing: Readonly<{
    acceptedActionCount: number;
    burrowingMinion: string;
    deck: DeckList;
    movedUnderground: boolean;
    nonBurrowingSurfaceAvailable: boolean;
    nonBurrowingUndergroundUnavailable: boolean;
    replayVerified: boolean;
    seed: number;
    siteTargetAvailableAfterSurfacing: boolean;
    siteTargetUnavailableUnderground: boolean;
    surfaceSummonAvailable: boolean;
    surfaced: boolean;
    targetIsLandSite: boolean;
    undergroundSummonAvailable: boolean;
  }>;
  earthOverpower: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    currentPowerIncreasedByTwo: boolean;
    deck: DeckList;
    elthamTownsfolk: string;
    exactOwnAllyChoices: boolean;
    expiredBeforeTurnEnded: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    overpower: string;
    printedPowerRestored: boolean;
    replayVerified: boolean;
    spellEnteredCemetery: boolean;
    unitStatePreservedOnGrant: boolean;
  }>;
  earthBury: Readonly<{
    acceptedActionCount: number;
    boskTroll: string;
    buriedBeforeDeath: boolean;
    bury: string;
    causalEventsVerified: boolean;
    deck: DeckList;
    deathNotBanishmentAndGameActive: boolean;
    exactlyOneBuryTarget: boolean;
    manaPaid: number;
    replayVerified: boolean;
    spellEnteredCemetery: boolean;
    targetEnteredCemetery: boolean;
    targetLeftRealm: boolean;
  }>;
  earthRescue: Readonly<{
    acceptedActionCount: number;
    boskTroll: string;
    buryStayedNorthCemetery: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    hiddenFromNorthAfterReturn: boolean;
    manaPaid: number;
    onlyOwnCemeteryMinionChoice: boolean;
    replayVerified: boolean;
    rescue: string;
    rescueEnteredSouthCemetery: boolean;
    returnedToSouthHand: boolean;
  }>;
  earthShallowGrave: Readonly<{
    acceptedActionCount: number;
    affinityProvided: boolean;
    avatarTapped: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    discardedInDeckOrder: boolean;
    gameRemainedActive: boolean;
    hiddenBeforeDiscard: boolean;
    manaProvided: boolean;
    publicAfterDiscard: boolean;
    replayVerified: boolean;
    shallowGrave: string;
    siteEstablished: boolean;
    spellHandUnchanged: boolean;
    spellbookReducedByTwo: boolean;
  }>;
  earthSinkhole: Readonly<{
    acceptedActionCount: number;
    avatarRemainedOnSurface: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactActivationAvailable: boolean;
    noAffinityOrControlContribution: boolean;
    replayVerified: boolean;
    sinkhole: string;
    sourceAndTargetEnteredCemetery: boolean;
    twoNeutralRubbleSites: boolean;
  }>;
  earthDivineHealing: Readonly<{
    acceptedActionCount: number;
    actualLifeGained: number;
    deck: DeckList;
    divineHealing: string;
    exactlyOneTargetlessCast: boolean;
    lifeCappedAtMaximum: boolean;
    lifeWasDamagedAboveDeathsDoor: boolean;
    manaPaid: number;
    replayVerified: boolean;
    spellEnteredCemetery: boolean;
  }>;
  earthEntombed: Readonly<{
    acceptedActionCount: number;
    boskTroll: string;
    boskTrollSurfaceAvailable: boolean;
    boskTrollUndergroundUnavailable: boolean;
    deck: DeckList;
    entombed: string;
    entombedSurfaceUnavailable: boolean;
    entombedUndergroundAvailable: boolean;
    replayVerified: boolean;
    seed: number;
    summonedUnderground: boolean;
  }>;
  earthImmobile: Readonly<{
    acceptedActionCount: number;
    comparatorMinion: string;
    deck: DeckList;
    dragChoicePairAvailable: boolean;
    dragOnlyAcceptedActionCount: number;
    dragOnlyEventsVerified: boolean;
    dragOnlyReplayVerified: boolean;
    dragOnlyStateVerified: boolean;
    fightAcceptedActionCount: number;
    fightEventsVerified: boolean;
    fightReplayVerified: boolean;
    fightStateVerified: boolean;
    localDefendAvailable: boolean;
    nearbySitePresent: boolean;
    positiveStepMoveUnavailable: boolean;
    pudgeButcher: string;
    replayVerified: boolean;
    sameLocationAttackAvailable: boolean;
  }>;
  earthForwardMovement: Readonly<{
    acceptedActionCount: number;
    backwardPathUnavailable: boolean;
    deck: DeckList;
    forwardPathAvailable: boolean;
    phalanx: string;
    replayVerified: boolean;
    seed: number;
    sidewaysPathUnavailable: boolean;
    siteTargetAvailable: boolean;
  }>;
  earthSecretTunnel: Readonly<{
    acceptedActionCount: number;
    avatarDirectUnavailable: boolean;
    avatarPhysicalAvailable: boolean;
    caveTrolls: string;
    deck: DeckList;
    directOpponentUnavailable: boolean;
    directTunnelMoveAvailable: boolean;
    movedUnderground: boolean;
    physicalMoveAvailable: boolean;
    replayVerified: boolean;
    secretTunnel: string;
    seed: number;
  }>;
  earthRamp: Readonly<{
    acceptedActionCount: number;
    affinityAdded: boolean;
    deck: DeckList;
    deathriteMinion: string;
    deathriteSiteDrawnBeforeCemetery: boolean;
    genesisSiteDrawn: boolean;
    ghostTown: string;
    ghostTownBonusMana: number;
    ghostTownUnusedManaExpired: boolean;
    manaGained: number;
    manaMinion: string;
    manaUnavailableWhileSick: boolean;
    movingDefendUnavailable: boolean;
    payoffCanMoveAndAttack: boolean;
    rampPaidFive: boolean;
    rampPayoffMinion: string;
    replayVerified: boolean;
    seed: number;
  }>;
  earthFirstStrike: Readonly<{
    acceptedActionCount: number;
    attackerSurvivedUndamaged: boolean;
    deck: DeckList;
    firstStrikeMinion: string;
    replayVerified: boolean;
    seed: number;
    targetDiedBeforeReturn: boolean;
    targetMinion: string;
  }>;
  earthRanged: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    rangedMinion: string;
    rangedOneStep: boolean;
    rangedShooterStayedSafe: boolean;
    rangedTargetDied: boolean;
    replayVerified: boolean;
    seed: number;
  }>;
  earthWard: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    rangedMinion: string;
    replayVerified: boolean;
    seed: number;
    wardBroke: boolean;
    wardMinion: string;
    wardPreventedDamage: boolean;
    wardTargetDiedAfterSecondShot: boolean;
    wardTargetSurvived: boolean;
  }>;
  fireCharge: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    charge: string;
    deck: DeckList;
    exactNonTargetAllyChoice: boolean;
    expiredAtEndOfTurn: boolean;
    manaPaid: number;
    moveAvailableAfterCharge: boolean;
    moveUnavailableBeforeCharge: boolean;
    raalDromedary: string;
    replayVerified: boolean;
    spellEnteredCemetery: boolean;
    temporaryChargeRecorded: boolean;
    unitStatePreservedOnGrant: boolean;
  }>;
  fireGenesisLifeLoss: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    cemeteriesUnchanged: boolean;
    deck: DeckList;
    lesserBloodDemon: string;
    lifeAfter: number;
    lifeLost: number;
    noDamageDeathOrTerminalEvents: boolean;
    noRandomDraws: boolean;
    otherStatePreserved: boolean;
    replayVerified: boolean;
    summonedAtC3: boolean;
  }>;
  fireIgnited: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    chargeActionAvailableImmediately: boolean;
    deck: DeckList;
    ignited: string;
    manaPaid: number;
    mandatoryDeathAndCemetery: boolean;
    noDeathriteDamageTerminalOrRandomEffects: boolean;
    otherStatePreserved: boolean;
    replayVerified: boolean;
    summonedStateVerified: boolean;
  }>;
  fireMinorExplosion: Readonly<{
    acceptedActionCount: number;
    avatarTookThreeDamage: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactLocationTargetAvailable: boolean;
    manaPaid: number;
    minorExplosion: string;
    noRandomDraws: boolean;
    raalDromedary: string;
    replayVerified: boolean;
    simultaneousDamageVerified: boolean;
    spellEnteredCemetery: boolean;
    targetWithinTwoSteps: boolean;
    twoMinionsDied: boolean;
    twoMinionsEnteredCemetery: boolean;
  }>;
  fireResponse: Readonly<{
    acceptedActionCount: number;
    chargeMoveAndAttack: boolean;
    deck: DeckList;
    defendUnavailable: boolean;
    interceptUnavailable: boolean;
    lumberingGiant: string;
    monstrousLion: string;
    replayVerified: boolean;
    seed: number;
    siteTargetUnavailable: boolean;
    unitTargetAvailable: boolean;
  }>;
  lethal: Readonly<{ minion: string; tougherMinionKilled: boolean }>;
  provider: Readonly<{ affinityAdded: boolean; minion: string }>;
  replayVerified: boolean;
  revisionId: string;
  seed: number;
  stealth: Readonly<{
    acceptedActionCount: number;
    attackSkippedDefend: boolean;
    deck: DeckList;
    enteredStealthed: boolean;
    groundCouldNotAttack: boolean;
    groundMinion: string;
    groundMinionDied: boolean;
    replayVerified: boolean;
    seed: number;
    stealthLostAfterAttack: boolean;
    stealthMinion: string;
  }>;
  waterEdgeConnection: Readonly<{
    acceptedActionCount: number;
    avatarWrapUnavailable: boolean;
    deck: DeckList;
    polarBears: string;
    replayVerified: boolean;
    seed: number;
    siteTargetAvailable: boolean;
    wrapMoveAvailable: boolean;
  }>;
  waterFreeze: Readonly<{
    acceptedActionCount: number;
    actionAvailableBefore: boolean;
    actionReturnedOnNextTurn: boolean;
    actionUnavailableWhileDisabled: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    disabledThroughOpponentTurn: boolean;
    disabledStateRecorded: boolean;
    expiredAtCasterStart: boolean;
    freeze: string;
    manaPaid: number;
    replayVerified: boolean;
    seravaTownsfolk: string;
    spellEnteredCemetery: boolean;
    unitStatePreserved: boolean;
  }>;
  waterLure: Readonly<{
    acceptedActionCount: number;
    allyUnchanged: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactNonTargetChoices: boolean;
    lure: string;
    manaPaid: number;
    noCombatDamageOrTap: boolean;
    noRandomDraws: boolean;
    replayVerified: boolean;
    seravaTownsfolk: string;
    spellEnteredCemetery: boolean;
    targetCemeteriesUnchanged: boolean;
    uniqueStepResolved: boolean;
  }>;
  waterPirateShip: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    disabledAtLand: boolean;
    enabledAtWater: boolean;
    exactMoveAvailable: boolean;
    ghostTown: string;
    ghostTownManaUsed: boolean;
    movementEventVerified: boolean;
    noCombatDamageDeathOrRandomness: boolean;
    noSubsequentUnitActions: boolean;
    pirateShip: string;
    replayVerified: boolean;
    sitesUnchanged: boolean;
    unitStatePreserved: boolean;
  }>;
  waterDrown: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    deck: DeckList;
    deathNotBanishmentAndGameActive: boolean;
    drown: string;
    exactTargetAvailable: boolean;
    ghostTownManaConsumed: boolean;
    manaPaid: number;
    replayVerified: boolean;
    seravaTownsfolk: string;
    spellEnteredCemetery: boolean;
    targetEnteredCemetery: boolean;
    targetLeftRealm: boolean;
    transitionBeforeDeath: boolean;
  }>;
  waterDrowned: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    drowned: string;
    drownedSurfaceUnavailable: boolean;
    drownedUnderwaterAvailable: boolean;
    replayVerified: boolean;
    seed: number;
    slyFox: string;
    slyFoxSurfaceAvailable: boolean;
    slyFoxUnderwaterUnavailable: boolean;
    summonedUnderwater: boolean;
  }>;
  waterLugbog: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    enemyLandUnavailable: boolean;
    enemyWaterAvailable: boolean;
    lugbogCat: string;
    replayVerified: boolean;
    slyFox: string;
    slyFoxControlledWaterAvailable: boolean;
    slyFoxEnemyWaterUnavailable: boolean;
    summonedToEnemyWater: boolean;
  }>;
  waterEndTurnStealth: Readonly<{
    acceptedActionCount: number;
    attackSiteAvailable: boolean;
    coLocatedReadyAttacker: boolean;
    deck: DeckList;
    gainedStealthAtEndOfTurn: boolean;
    replayVerified: boolean;
    seed: number;
    slyFox: string;
    slyFoxAttackUnavailable: boolean;
    summonedUnstealthed: boolean;
  }>;
  waterSidewaysMovement: Readonly<{
    acceptedActionCount: number;
    backwardPathUnavailable: boolean;
    deck: DeckList;
    forwardPathUnavailable: boolean;
    replayVerified: boolean;
    sedgeCrabs: string;
    seed: number;
    sidewaysPathAvailable: boolean;
  }>;
  waterSubmerge: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    nonSubmergeSurfaceAvailable: boolean;
    nonSubmergeUnderwaterUnavailable: boolean;
    replayVerified: boolean;
    seed: number;
    submergeMinion: string;
    summonedUnderwater: boolean;
    surfaceSummonAvailable: boolean;
    targetIsWaterSite: boolean;
    underwaterSummonAvailable: boolean;
  }>;
  waterHealing: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    healed: number;
    healedBeforeCemetery: boolean;
    healingMinionDied: boolean;
    healingMinion: string;
    opponentMinionDied: boolean;
    replayVerified: boolean;
    seed: number;
  }>;
}>;

function isJsonRecord(value: unknown): value is Readonly<Record<string, JsonValue>> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function scenarioConfig(value: JsonValue): ScenarioConfig {
  if (!isJsonRecord(value)) {
    throw new Error('private game scenario must be an object');
  }
  const avatar = value.avatar;
  if (!isJsonRecord(avatar)
    || !Number.isSafeInteger(value.airborneSeed)
    || typeof value.airborneSeed !== 'number'
    || value.airborneSeed < 0
    || !Number.isSafeInteger(value.airSeed)
    || typeof value.airSeed !== 'number'
    || value.airSeed < 0
    || typeof avatar.stableId !== 'string'
    || typeof avatar.drawSpell !== 'boolean'
    || typeof value.cannotDefendMinionStableId !== 'string'
    || typeof value.chargeMinionStableId !== 'string'
    || typeof value.deathriteMinionStableId !== 'string'
    || typeof value.earthProviderMinionStableId !== 'string'
    || !Number.isSafeInteger(value.earthFirstStrikeSeed)
    || typeof value.earthFirstStrikeSeed !== 'number'
    || value.earthFirstStrikeSeed < 0
    || !Number.isSafeInteger(value.earthRangedSeed)
    || typeof value.earthRangedSeed !== 'number'
    || value.earthRangedSeed < 0
    || !Number.isSafeInteger(value.earthSeed)
    || typeof value.earthSeed !== 'number'
    || value.earthSeed < 0
    || !Number.isSafeInteger(value.earthWardSeed)
    || typeof value.earthWardSeed !== 'number'
    || value.earthWardSeed < 0
    || !Number.isSafeInteger(value.fireSeed)
    || typeof value.fireSeed !== 'number'
    || value.fireSeed < 0
    || typeof value.firstStrikeMinionStableId !== 'string'
    || typeof value.firstStrikeTargetMinionStableId !== 'string'
    || typeof value.genesisMinionStableId !== 'string'
    || typeof value.ghostTownSiteStableId !== 'string'
    || typeof value.healingMinionStableId !== 'string'
    || typeof value.lethalMinionStableId !== 'string'
    || typeof value.lumberingMinionStableId !== 'string'
    || typeof value.manaMinionStableId !== 'string'
    || typeof value.monstrousLionStableId !== 'string'
    || typeof value.movementMinionStableId !== 'string'
    || !Number.isSafeInteger(value.movementTwoSeed)
    || typeof value.movementTwoSeed !== 'number'
    || value.movementTwoSeed < 0
    || typeof value.providerMinionStableId !== 'string'
    || typeof value.rangedMinionStableId !== 'string'
    || typeof value.revisionId !== 'string'
    || typeof value.roamingMinionStableId !== 'string'
    || !Number.isSafeInteger(value.roamingSeed)
    || typeof value.roamingSeed !== 'number'
    || value.roamingSeed < 0
    || !Number.isSafeInteger(value.seed)
    || typeof value.seed !== 'number'
    || value.seed < 0
    || !Number.isSafeInteger(value.sedgeCrabsSeed)
    || typeof value.sedgeCrabsSeed !== 'number'
    || value.sedgeCrabsSeed < 0
    || !Number.isSafeInteger(value.slyFoxSeed)
    || typeof value.slyFoxSeed !== 'number'
    || value.slyFoxSeed < 0
    || !Number.isSafeInteger(value.stealthSeed)
    || typeof value.stealthSeed !== 'number'
    || value.stealthSeed < 0
    || !Number.isSafeInteger(value.waterSeed)
    || typeof value.waterSeed !== 'number'
    || value.waterSeed < 0
    || typeof value.wardMinionStableId !== 'string'
    || Object.keys(value).sort().join(',') !== 'airSeed,airborneSeed,avatar,cannotDefendMinionStableId,chargeMinionStableId,deathriteMinionStableId,earthFirstStrikeSeed,earthProviderMinionStableId,earthRangedSeed,earthSeed,earthWardSeed,fireSeed,firstStrikeMinionStableId,firstStrikeTargetMinionStableId,genesisMinionStableId,ghostTownSiteStableId,healingMinionStableId,lethalMinionStableId,lumberingMinionStableId,manaMinionStableId,monstrousLionStableId,movementMinionStableId,movementTwoSeed,providerMinionStableId,rangedMinionStableId,revisionId,roamingMinionStableId,roamingSeed,sedgeCrabsSeed,seed,slyFoxSeed,stealthSeed,wardMinionStableId,waterSeed'
    || Object.keys(avatar).sort().join(',') !== 'drawSpell,stableId') {
    throw new Error('private game scenario has an unsupported shape');
  }
  return {
    airborneSeed: value.airborneSeed,
    airSeed: value.airSeed,
    avatar: { drawSpell: avatar.drawSpell, stableId: avatar.stableId },
    cannotDefendMinionStableId: value.cannotDefendMinionStableId,
    chargeMinionStableId: value.chargeMinionStableId,
    deathriteMinionStableId: value.deathriteMinionStableId,
    earthFirstStrikeSeed: value.earthFirstStrikeSeed,
    earthProviderMinionStableId: value.earthProviderMinionStableId,
    earthRangedSeed: value.earthRangedSeed,
    earthSeed: value.earthSeed,
    earthWardSeed: value.earthWardSeed,
    fireSeed: value.fireSeed,
    firstStrikeMinionStableId: value.firstStrikeMinionStableId,
    firstStrikeTargetMinionStableId: value.firstStrikeTargetMinionStableId,
    genesisMinionStableId: value.genesisMinionStableId,
    ghostTownSiteStableId: value.ghostTownSiteStableId,
    healingMinionStableId: value.healingMinionStableId,
    lethalMinionStableId: value.lethalMinionStableId,
    lumberingMinionStableId: value.lumberingMinionStableId,
    manaMinionStableId: value.manaMinionStableId,
    monstrousLionStableId: value.monstrousLionStableId,
    movementMinionStableId: value.movementMinionStableId,
    movementTwoSeed: value.movementTwoSeed,
    providerMinionStableId: value.providerMinionStableId,
    rangedMinionStableId: value.rangedMinionStableId,
    revisionId: value.revisionId,
    roamingMinionStableId: value.roamingMinionStableId,
    roamingSeed: value.roamingSeed,
    seed: value.seed,
    sedgeCrabsSeed: value.sedgeCrabsSeed,
    slyFoxSeed: value.slyFoxSeed,
    stealthSeed: value.stealthSeed,
    waterSeed: value.waterSeed,
    wardMinionStableId: value.wardMinionStableId,
  };
}

async function readPrivateInputs(path: string): Promise<Readonly<{
  airborneMinion: NormalizedCard;
  airborneTargetMinion: NormalizedCard;
  arcLightning: NormalizedCard;
  authorityHash: Hash;
  bury: NormalizedCard;
  burrowingMinion: NormalizedCard;
  cards: readonly NormalizedCard[];
  cannotDefendMinion: NormalizedCard;
  chargeMagic: NormalizedCard;
  chargeMinion: NormalizedCard;
  config: ScenarioConfig;
  deathriteMinion: NormalizedCard;
  dalceanPhalanx: NormalizedCard;
  divineHealing: NormalizedCard;
  drown: NormalizedCard;
  drowned: NormalizedCard;
  earthProviderMinion: NormalizedCard;
  elthamTownsfolk: NormalizedCard;
  entombed: NormalizedCard;
  format: FormatDefinition;
  formatStableId: string;
  firstStrikeMinion: NormalizedCard;
  firstStrikeTargetMinion: NormalizedCard;
  forsaken: NormalizedCard;
  freeze: NormalizedCard;
  genesisSpellMinion: NormalizedCard;
  genesisMinion: NormalizedCard;
  ghostTownSite: NormalizedCard;
  healingMinion: NormalizedCard;
  lethalMinion: NormalizedCard;
  leylineHenge: NormalizedCard;
  lesserBloodDemon: NormalizedCard;
  ignited: NormalizedCard;
  lightningBolt: NormalizedCard;
  lugbogCat: NormalizedCard;
  lure: NormalizedCard;
  lumberingMinion: NormalizedCard;
  manaMinion: NormalizedCard;
  minorExplosion: NormalizedCard;
  monstrousLion: NormalizedCard;
  movementMinion: NormalizedCard;
  movementTwoMinion: NormalizedCard;
  overpower: NormalizedCard;
  providerMinion: NormalizedCard;
  raalDromedary: NormalizedCard;
  rangedMinion: NormalizedCard;
  rescue: NormalizedCard;
  roamingMinion: NormalizedCard;
  secretTunnel: NormalizedCard;
  sedgeCrabs: NormalizedCard;
  seravaTownsfolk: NormalizedCard;
  shallowGrave: NormalizedCard;
  sinkhole: NormalizedCard;
  slyFox: NormalizedCard;
  stealthMinion: NormalizedCard;
  stealthTargetMinion: NormalizedCard;
  submergeMinion: NormalizedCard;
  teleport: NormalizedCard;
  voidwalkMinion: NormalizedCard;
  wardMinion: NormalizedCard;
  polarBears: NormalizedCard;
  pirateShip: NormalizedCard;
  pudgeButcher: NormalizedCard;
  rainOfArrows: NormalizedCard;
  zap: NormalizedCard;
}>> {
  const config = scenarioConfig(parseJsonWithDuplicateKeyCheck(await readFile(path, 'utf8')));
  const revisionRoot = resolve(REPOSITORY_ROOT, '.local', 'authority', 'revisions', config.revisionId);
  if (basename(revisionRoot) !== config.revisionId) throw new Error('private revision ID must be one path segment');

  const artifact = canonicalArtifactSchema.parse(parseJsonWithDuplicateKeyCheck(
    await readFile(resolve(revisionRoot, 'cards.normalized.json'), 'utf8'),
  ));
  if (artifact.identity.artifactKind !== 'card-snapshot'
    || identityHash(artifact.identity as unknown as JsonValue) !== artifact.contentHash) {
    throw new Error('private normalized card artifact identity is invalid');
  }
  const snapshot = normalizedCardSnapshotSchema.parse(artifact.identity.payload);
  const shallowGrave = snapshot.cards.find(({ name }) => name === 'Shallow Grave');
  if (!shallowGrave
    || shallowGrave.cardType !== 'site'
    || shallowGrave.rulesText.trim() !== 'Genesis → Discard your top two spells.'
    || shallowGrave.manaCost !== null
    || shallowGrave.attack !== null
    || shallowGrave.defense !== null
    || shallowGrave.life !== null
    || shallowGrave.elements.length !== 1
    || shallowGrave.elements[0] !== 'earth'
    || shallowGrave.thresholds.air !== 0
    || shallowGrave.thresholds.earth !== 1
    || shallowGrave.thresholds.fire !== 0
    || shallowGrave.thresholds.water !== 0
    || shallowGrave.rarity !== 'exceptional') {
    throw new Error('private site discard Genesis no longer matches its supported facts');
  }
  const sinkhole = snapshot.cards.find(({ name }) => name === 'Sinkhole');
  if (!sinkhole
    || sinkhole.cardType !== 'site'
    || sinkhole.rulesText.trim() !== 'Sacrifice Sinkhole → Destroy a nearby site.'
    || sinkhole.manaCost !== null
    || sinkhole.attack !== null
    || sinkhole.defense !== null
    || sinkhole.life !== null
    || sinkhole.elements.length !== 0
    || sinkhole.thresholds.air !== 0
    || sinkhole.thresholds.earth !== 0
    || sinkhole.thresholds.fire !== 0
    || sinkhole.thresholds.water !== 0
    || sinkhole.rarity !== 'elite') {
    throw new Error('private sacrifice-to-destroy site no longer matches its supported facts');
  }
  const bury = snapshot.cards.find(({ name }) => name === 'Bury');
  if (!bury
    || bury.cardType !== 'magic'
    || bury.rulesText.trim() !== 'Burrow target minion or artifact, if able.'
    || bury.manaCost !== 3
    || bury.attack !== null
    || bury.defense !== null
    || bury.life !== null
    || bury.elements.length !== 1
    || bury.elements[0] !== 'earth'
    || bury.thresholds.air !== 0
    || bury.thresholds.earth !== 1
    || bury.thresholds.fire !== 0
    || bury.thresholds.water !== 0
    || bury.rarity !== 'ordinary') {
    throw new Error('private forced-burrow Magic no longer matches its supported facts');
  }
  const rescue = snapshot.cards.find(({ name }) => name === 'Rescue');
  if (!rescue
    || rescue.cardType !== 'magic'
    || rescue.rulesText.trim() !== 'Return a minion from your cemetery to your hand.'
    || rescue.manaCost !== 3
    || rescue.attack !== null
    || rescue.defense !== null
    || rescue.life !== null
    || rescue.elements.length !== 1
    || rescue.elements[0] !== 'earth'
    || rescue.thresholds.air !== 0
    || rescue.thresholds.earth !== 2
    || rescue.thresholds.fire !== 0
    || rescue.thresholds.water !== 0
    || rescue.rarity !== 'ordinary') {
    throw new Error('private cemetery-to-hand Rescue no longer matches its supported facts');
  }
  const divineHealing = snapshot.cards.find(({ name }) => name === 'Divine Healing');
  if (!divineHealing
    || divineHealing.cardType !== 'magic'
    || divineHealing.rulesText.trim() !== 'You gain 7 life.'
    || divineHealing.manaCost !== 1
    || divineHealing.attack !== null
    || divineHealing.defense !== null
    || divineHealing.life !== null
    || divineHealing.elements.length !== 1
    || divineHealing.elements[0] !== 'earth'
    || divineHealing.thresholds.air !== 0
    || divineHealing.thresholds.earth !== 3
    || divineHealing.thresholds.fire !== 0
    || divineHealing.thresholds.water !== 0
    || divineHealing.rarity !== 'exceptional') {
    throw new Error('private controller-healing Magic no longer matches its supported facts');
  }
  const arcLightning = snapshot.cards.find(({ name }) => name === 'Arc Lightning');
  if (!arcLightning
    || arcLightning.cardType !== 'magic'
    || arcLightning.rulesText.trim() !== 'Deal 4 damage to target nearby unit.'
    || arcLightning.manaCost !== 4
    || arcLightning.attack !== null
    || arcLightning.defense !== null
    || arcLightning.life !== null
    || arcLightning.elements.length !== 1
    || arcLightning.elements[0] !== 'air'
    || arcLightning.thresholds.air !== 2
    || arcLightning.thresholds.earth !== 0
    || arcLightning.thresholds.fire !== 0
    || arcLightning.thresholds.water !== 0
    || arcLightning.rarity !== 'ordinary') {
    throw new Error('private nearby target-unit damage Magic no longer matches its supported facts');
  }
  const lightningBolt = snapshot.cards.find(({ name }) => name === 'Lightning Bolt');
  if (!lightningBolt
    || lightningBolt.cardType !== 'magic'
    || lightningBolt.rulesText.trim() !== 'Deal 3 damage to a random unit at target location.'
    || lightningBolt.manaCost !== 2
    || lightningBolt.attack !== null
    || lightningBolt.defense !== null
    || lightningBolt.life !== null
    || lightningBolt.elements.length !== 1
    || lightningBolt.elements[0] !== 'air'
    || lightningBolt.thresholds.air !== 1
    || lightningBolt.thresholds.earth !== 0
    || lightningBolt.thresholds.fire !== 0
    || lightningBolt.thresholds.water !== 0
    || lightningBolt.rarity !== 'ordinary') {
    throw new Error('private random location-damage Magic no longer matches its supported facts');
  }
  const rainOfArrows = snapshot.cards.find(({ name }) => name === 'Rain of Arrows');
  if (!rainOfArrows
    || rainOfArrows.cardType !== 'magic'
    || rainOfArrows.rulesText.trim() !== 'Deal 1 damage to each aboveground minion.'
    || rainOfArrows.manaCost !== 2
    || rainOfArrows.attack !== null
    || rainOfArrows.defense !== null
    || rainOfArrows.life !== null
    || rainOfArrows.elements.length !== 1
    || rainOfArrows.elements[0] !== 'air'
    || rainOfArrows.thresholds.air !== 1
    || rainOfArrows.thresholds.earth !== 0
    || rainOfArrows.thresholds.fire !== 0
    || rainOfArrows.thresholds.water !== 0
    || rainOfArrows.rarity !== 'ordinary') {
    throw new Error('private aboveground-minion damage Magic no longer matches its supported facts');
  }
  const minorExplosion = snapshot.cards.find(({ name }) => name === 'Minor Explosion');
  if (!minorExplosion
    || minorExplosion.cardType !== 'magic'
    || minorExplosion.rulesText.trim()
      !== 'Deal 3 damage to each unit at target location up to two steps away.'
    || minorExplosion.manaCost !== 3
    || minorExplosion.attack !== null
    || minorExplosion.defense !== null
    || minorExplosion.life !== null
    || minorExplosion.elements.length !== 1
    || minorExplosion.elements[0] !== 'fire'
    || minorExplosion.thresholds.air !== 0
    || minorExplosion.thresholds.earth !== 0
    || minorExplosion.thresholds.fire !== 2
    || minorExplosion.thresholds.water !== 0
    || minorExplosion.rarity !== 'ordinary') {
    throw new Error('private location-wide damage Magic no longer matches its supported facts');
  }
  const chargeMagic = snapshot.cards.find(({ name }) => name === 'Charge');
  if (!chargeMagic
    || chargeMagic.cardType !== 'magic'
    || chargeMagic.rulesText.trim() !== 'Give an ally Charge this turn.'
    || chargeMagic.manaCost !== 1
    || chargeMagic.attack !== null
    || chargeMagic.defense !== null
    || chargeMagic.life !== null
    || chargeMagic.elements.length !== 1
    || chargeMagic.elements[0] !== 'fire'
    || chargeMagic.thresholds.air !== 0
    || chargeMagic.thresholds.earth !== 0
    || chargeMagic.thresholds.fire !== 1
    || chargeMagic.thresholds.water !== 0
    || chargeMagic.rarity !== 'ordinary') {
    throw new Error('private temporary Charge Magic no longer matches its supported facts');
  }
  const overpower = snapshot.cards.find(({ name }) => name === 'Overpower');
  if (!overpower
    || overpower.cardType !== 'magic'
    || overpower.rulesText.trim() !== 'Give an ally +2 power this turn.'
    || overpower.manaCost !== 1
    || overpower.attack !== null
    || overpower.defense !== null
    || overpower.life !== null
    || overpower.elements.length !== 1
    || overpower.elements[0] !== 'earth'
    || overpower.thresholds.air !== 0
    || overpower.thresholds.earth !== 1
    || overpower.thresholds.fire !== 0
    || overpower.thresholds.water !== 0
    || overpower.rarity !== 'ordinary') {
    throw new Error('private temporary power Magic no longer matches its supported facts');
  }
  const elthamTownsfolk = snapshot.cards.find(({ name }) => name === 'Eltham Townsfolk');
  if (!elthamTownsfolk
    || elthamTownsfolk.cardType !== 'minion'
    || elthamTownsfolk.rulesText.trim() !== ''
    || elthamTownsfolk.manaCost !== 1
    || elthamTownsfolk.attack !== 2
    || elthamTownsfolk.defense !== 2
    || elthamTownsfolk.life !== null
    || elthamTownsfolk.elements.length !== 1
    || elthamTownsfolk.elements[0] !== 'earth'
    || elthamTownsfolk.thresholds.air !== 0
    || elthamTownsfolk.thresholds.earth !== 1
    || elthamTownsfolk.thresholds.fire !== 0
    || elthamTownsfolk.thresholds.water !== 0
    || elthamTownsfolk.rarity !== 'ordinary') {
    throw new Error('private temporary power helper minion no longer matches its supported facts');
  }
  const lesserBloodDemon = snapshot.cards.find(({ name }) => name === 'Lesser Blood Demon');
  if (!lesserBloodDemon
    || lesserBloodDemon.cardType !== 'minion'
    || lesserBloodDemon.rulesText.trim() !== 'Genesis → Lose 2 life.'
    || lesserBloodDemon.manaCost !== 2
    || lesserBloodDemon.attack !== 3
    || lesserBloodDemon.defense !== 3
    || lesserBloodDemon.life !== null
    || lesserBloodDemon.elements.length !== 1
    || lesserBloodDemon.elements[0] !== 'fire'
    || lesserBloodDemon.thresholds.air !== 0
    || lesserBloodDemon.thresholds.earth !== 0
    || lesserBloodDemon.thresholds.fire !== 1
    || lesserBloodDemon.thresholds.water !== 0
    || lesserBloodDemon.rarity !== 'ordinary') {
    throw new Error('private Genesis life-loss minion no longer matches its supported facts');
  }
  const ignited = snapshot.cards.find(({ name }) => name === 'Ignited');
  if (!ignited
    || ignited.cardType !== 'minion'
    || ignited.rulesText.trim() !== 'Charge\n\nDies at the end of your turn.'
    || ignited.manaCost !== 2
    || ignited.attack !== 3
    || ignited.defense !== 3
    || ignited.life !== null
    || ignited.elements.length !== 1
    || ignited.elements[0] !== 'fire'
    || ignited.thresholds.air !== 0
    || ignited.thresholds.earth !== 0
    || ignited.thresholds.fire !== 1
    || ignited.thresholds.water !== 0
    || ignited.rarity !== 'ordinary') {
    throw new Error('private printed-Charge end-turn-death minion no longer matches its supported facts');
  }
  const freeze = snapshot.cards.find(({ name }) => name === 'Freeze');
  if (!freeze
    || freeze.cardType !== 'magic'
    || freeze.rulesText.trim() !== 'Disable target nearby minion until your next turn.'
    || freeze.manaCost !== 1
    || freeze.attack !== null
    || freeze.defense !== null
    || freeze.life !== null
    || freeze.elements.length !== 1
    || freeze.elements[0] !== 'water'
    || freeze.thresholds.air !== 0
    || freeze.thresholds.earth !== 0
    || freeze.thresholds.fire !== 0
    || freeze.thresholds.water !== 1
    || freeze.rarity !== 'ordinary') {
    throw new Error('private timed-disable Magic no longer matches its supported facts');
  }
  const lure = snapshot.cards.find(({ name }) => name === 'Lure');
  if (!lure
    || lure.cardType !== 'magic'
    || lure.rulesText.trim()
      !== 'An ally tempts an enemy minion at a nearby site into taking a step closer.'
    || lure.manaCost !== 1
    || lure.attack !== null
    || lure.defense !== null
    || lure.life !== null
    || lure.elements.length !== 1
    || lure.elements[0] !== 'water'
    || lure.thresholds.air !== 0
    || lure.thresholds.earth !== 0
    || lure.thresholds.fire !== 0
    || lure.thresholds.water !== 1
    || lure.rarity !== 'ordinary') {
    throw new Error('private non-target Lure Magic no longer matches its supported facts');
  }
  const pirateShip = snapshot.cards.find(({ name }) => name === 'Pirate Ship');
  if (!pirateShip
    || pirateShip.cardType !== 'minion'
    || pirateShip.rulesText.trim() !== 'Waterbound'
    || pirateShip.manaCost !== 4
    || pirateShip.attack !== 5
    || pirateShip.defense !== 5
    || pirateShip.life !== null
    || pirateShip.elements.length !== 1
    || pirateShip.elements[0] !== 'water'
    || pirateShip.thresholds.air !== 0
    || pirateShip.thresholds.earth !== 0
    || pirateShip.thresholds.fire !== 0
    || pirateShip.thresholds.water !== 1
    || pirateShip.rarity !== 'ordinary') {
    throw new Error('private Waterbound minion no longer matches its supported facts');
  }
  const seravaTownsfolk = snapshot.cards.find(({ name }) => name === 'Serava Townsfolk');
  if (!seravaTownsfolk
    || seravaTownsfolk.cardType !== 'minion'
    || seravaTownsfolk.rulesText.trim() !== ''
    || seravaTownsfolk.manaCost !== 1
    || seravaTownsfolk.attack !== 2
    || seravaTownsfolk.defense !== 2
    || seravaTownsfolk.life !== null
    || seravaTownsfolk.elements.length !== 1
    || seravaTownsfolk.elements[0] !== 'water'
    || seravaTownsfolk.thresholds.air !== 0
    || seravaTownsfolk.thresholds.earth !== 0
    || seravaTownsfolk.thresholds.fire !== 0
    || seravaTownsfolk.thresholds.water !== 1
    || seravaTownsfolk.rarity !== 'ordinary') {
    throw new Error('private timed-disable target minion no longer matches its supported facts');
  }
  const raalDromedary = snapshot.cards.find(({ name }) => name === 'Raal Dromedary');
  if (!raalDromedary
    || raalDromedary.cardType !== 'minion'
    || raalDromedary.rulesText.trim() !== ''
    || raalDromedary.manaCost !== 1
    || raalDromedary.attack !== 2
    || raalDromedary.defense !== 2
    || raalDromedary.life !== null
    || raalDromedary.elements.length !== 1
    || raalDromedary.elements[0] !== 'fire'
    || raalDromedary.thresholds.air !== 0
    || raalDromedary.thresholds.earth !== 0
    || raalDromedary.thresholds.fire !== 1
    || raalDromedary.thresholds.water !== 0
    || raalDromedary.rarity !== 'ordinary') {
    throw new Error('private location-wide damage target minion no longer matches its supported facts');
  }
  const teleport = snapshot.cards.find(({ name }) => name === 'Teleport');
  if (!teleport
    || teleport.cardType !== 'magic'
    || teleport.rulesText.trim() !== 'Teleport an ally to the surface of target site.'
    || teleport.manaCost !== 2
    || teleport.attack !== null
    || teleport.defense !== null
    || teleport.life !== null
    || teleport.elements.length !== 1
    || teleport.elements[0] !== 'air'
    || teleport.thresholds.air !== 2
    || teleport.thresholds.earth !== 0
    || teleport.thresholds.fire !== 0
    || teleport.thresholds.water !== 0
    || teleport.rarity !== 'ordinary') {
    throw new Error('private ally-to-site Teleport no longer matches its supported facts');
  }
  const airborneMinion = snapshot.cards.find(({ name }) => name === 'Plumed Pegasus');
  if (!airborneMinion
    || airborneMinion.cardType !== 'minion'
    || airborneMinion.rulesText.trim() !== 'Airborne'
    || airborneMinion.manaCost !== 3
    || airborneMinion.attack !== 3
    || airborneMinion.defense !== 3
    || airborneMinion.elements.length !== 1
    || airborneMinion.elements[0] !== 'air'
    || airborneMinion.thresholds.air !== 1
    || airborneMinion.thresholds.earth !== 0
    || airborneMinion.thresholds.fire !== 0
    || airborneMinion.thresholds.water !== 0
    || airborneMinion.rarity !== 'ordinary') {
    throw new Error('private Airborne minion no longer matches its supported facts');
  }
  const airborneTargetMinion = snapshot.cards.find(({ name }) => name === 'Ghoul');
  if (!airborneTargetMinion
    || airborneTargetMinion.cardType !== 'minion'
    || airborneTargetMinion.rulesText.trim() !== ''
    || airborneTargetMinion.manaCost !== 3
    || airborneTargetMinion.attack !== 3
    || airborneTargetMinion.defense !== 3
    || airborneTargetMinion.elements.length !== 1
    || airborneTargetMinion.elements[0] !== 'air'
    || airborneTargetMinion.thresholds.air !== 1
    || airborneTargetMinion.thresholds.earth !== 0
    || airborneTargetMinion.thresholds.fire !== 0
    || airborneTargetMinion.thresholds.water !== 0
    || airborneTargetMinion.rarity !== 'ordinary') {
    throw new Error('private Airborne target minion no longer matches its supported facts');
  }
  const stealthMinion = snapshot.cards.find(({ name }) => name === 'Band of Thieves');
  if (!stealthMinion
    || stealthMinion.cardType !== 'minion'
    || stealthMinion.rulesText.trim() !== 'Stealth'
    || stealthMinion.manaCost !== 3
    || stealthMinion.attack !== 3
    || stealthMinion.defense !== 3
    || stealthMinion.elements.length !== 1
    || stealthMinion.elements[0] !== 'air'
    || stealthMinion.thresholds.air !== 2
    || stealthMinion.thresholds.earth !== 0
    || stealthMinion.thresholds.fire !== 0
    || stealthMinion.thresholds.water !== 0
    || stealthMinion.rarity !== 'ordinary') {
    throw new Error('private Stealth minion no longer matches its supported facts');
  }
  const stealthTargetMinion = snapshot.cards.find(({ name }) => name === 'Snow Leopard');
  if (!stealthTargetMinion
    || stealthTargetMinion.cardType !== 'minion'
    || stealthTargetMinion.rulesText.trim() !== ''
    || stealthTargetMinion.manaCost !== 1
    || stealthTargetMinion.attack !== 2
    || stealthTargetMinion.defense !== 2
    || stealthTargetMinion.elements.length !== 1
    || stealthTargetMinion.elements[0] !== 'air'
    || stealthTargetMinion.thresholds.air !== 1
    || stealthTargetMinion.thresholds.earth !== 0
    || stealthTargetMinion.thresholds.fire !== 0
    || stealthTargetMinion.thresholds.water !== 0
    || stealthTargetMinion.rarity !== 'ordinary') {
    throw new Error('private Stealth target minion no longer matches its supported facts');
  }
  const slyFox = snapshot.cards.find(({ name }) => name === 'Sly Fox');
  if (!slyFox
    || slyFox.cardType !== 'minion'
    || slyFox.rulesText.trim() !== 'Gains Stealth at the end of your turn.'
    || slyFox.manaCost !== 1
    || slyFox.attack !== 1
    || slyFox.defense !== 1
    || slyFox.elements.length !== 1
    || slyFox.elements[0] !== 'water'
    || slyFox.thresholds.air !== 0
    || slyFox.thresholds.earth !== 0
    || slyFox.thresholds.fire !== 0
    || slyFox.thresholds.water !== 1
    || slyFox.rarity !== 'ordinary') {
    throw new Error('private end-turn Stealth minion no longer matches its supported facts');
  }
  const sedgeCrabs = snapshot.cards.find(({ name }) => name === 'Sedge Crabs');
  if (!sedgeCrabs
    || sedgeCrabs.cardType !== 'minion'
    || sedgeCrabs.rulesText.trim() !== 'Can only move themselves sideways.'
    || sedgeCrabs.manaCost !== 1
    || sedgeCrabs.attack !== 3
    || sedgeCrabs.defense !== 3
    || sedgeCrabs.elements.length !== 1
    || sedgeCrabs.elements[0] !== 'water'
    || sedgeCrabs.thresholds.air !== 0
    || sedgeCrabs.thresholds.earth !== 0
    || sedgeCrabs.thresholds.fire !== 0
    || sedgeCrabs.thresholds.water !== 1
    || sedgeCrabs.rarity !== 'ordinary') {
    throw new Error('private sideways-only minion no longer matches its supported facts');
  }
  const submergeMinion = snapshot.cards.find(({ name }) => name === 'Coral-Reef Kelpie');
  if (!submergeMinion
    || submergeMinion.cardType !== 'minion'
    || submergeMinion.rulesText.trim() !== 'Submerge'
    || submergeMinion.manaCost !== 3
    || submergeMinion.attack !== 3
    || submergeMinion.defense !== 3
    || submergeMinion.elements.length !== 1
    || submergeMinion.elements[0] !== 'water'
    || submergeMinion.thresholds.air !== 0
    || submergeMinion.thresholds.earth !== 0
    || submergeMinion.thresholds.fire !== 0
    || submergeMinion.thresholds.water !== 1
    || submergeMinion.rarity !== 'ordinary') {
    throw new Error('private Submerge minion no longer matches its supported facts');
  }
  const drowned = snapshot.cards.find(({ name }) => name === 'Drowned');
  if (!drowned
    || drowned.cardType !== 'minion'
    || drowned.rulesText.trim().replaceAll('\r\n', '\n')
      !== 'Submerge\nMust be cast submerged.'
    || drowned.manaCost !== 2
    || drowned.attack !== 3
    || drowned.defense !== 3
    || drowned.elements.length !== 1
    || drowned.elements[0] !== 'water'
    || drowned.thresholds.air !== 0
    || drowned.thresholds.earth !== 0
    || drowned.thresholds.fire !== 0
    || drowned.thresholds.water !== 1
    || drowned.rarity !== 'ordinary') {
    throw new Error('private submerged-only casting minion no longer matches its supported facts');
  }
  const drown = snapshot.cards.find(({ name }) => name === 'Drown');
  if (!drown
    || drown.cardType !== 'magic'
    || drown.rulesText.trim() !== 'Submerge target minion or artifact, if able.'
    || drown.manaCost !== 3
    || drown.attack !== null
    || drown.defense !== null
    || drown.life !== null
    || drown.elements.length !== 1
    || drown.elements[0] !== 'water'
    || drown.thresholds.air !== 0
    || drown.thresholds.earth !== 0
    || drown.thresholds.fire !== 0
    || drown.thresholds.water !== 1
    || drown.rarity !== 'ordinary') {
    throw new Error('private forced-submerge Magic no longer matches its supported facts');
  }
  const lugbogCat = snapshot.cards.find(({ name }) => name === 'Lugbog Cat');
  if (!lugbogCat
    || lugbogCat.cardType !== 'minion'
    || lugbogCat.rulesText.trim() !== 'Can and must be cast to any water site.'
    || lugbogCat.manaCost !== 3
    || lugbogCat.attack !== 4
    || lugbogCat.defense !== 4
    || lugbogCat.elements.length !== 1
    || lugbogCat.elements[0] !== 'water'
    || lugbogCat.thresholds.air !== 0
    || lugbogCat.thresholds.earth !== 0
    || lugbogCat.thresholds.fire !== 0
    || lugbogCat.thresholds.water !== 2
    || lugbogCat.rarity !== 'exceptional') {
    throw new Error('private water-site-only casting minion no longer matches its supported facts');
  }
  const burrowingMinion = snapshot.cards.find(({ name }) => name === 'Cave Trolls');
  if (!burrowingMinion
    || burrowingMinion.cardType !== 'minion'
    || burrowingMinion.rulesText.trim() !== 'Burrowing'
    || burrowingMinion.manaCost !== 3
    || burrowingMinion.attack !== 3
    || burrowingMinion.defense !== 3
    || burrowingMinion.elements.length !== 1
    || burrowingMinion.elements[0] !== 'earth'
    || burrowingMinion.thresholds.air !== 0
    || burrowingMinion.thresholds.earth !== 1
    || burrowingMinion.thresholds.fire !== 0
    || burrowingMinion.thresholds.water !== 0
    || burrowingMinion.rarity !== 'ordinary') {
    throw new Error('private Burrowing minion no longer matches its supported facts');
  }
  const entombed = snapshot.cards.find(({ name }) => name === 'Entombed');
  if (!entombed
    || entombed.cardType !== 'minion'
    || entombed.rulesText.trim().replaceAll('\r\n', '\n')
      !== 'Burrowing\n\nMust be cast burrowed.'
    || entombed.manaCost !== 2
    || entombed.attack !== 3
    || entombed.defense !== 3
    || entombed.elements.length !== 1
    || entombed.elements[0] !== 'earth'
    || entombed.thresholds.air !== 0
    || entombed.thresholds.earth !== 1
    || entombed.thresholds.fire !== 0
    || entombed.thresholds.water !== 0
    || entombed.rarity !== 'ordinary') {
    throw new Error('private burrowed-only casting minion no longer matches its supported facts');
  }
  const dalceanPhalanx = snapshot.cards.find(({ name }) => name === 'Dalcean Phalanx');
  if (!dalceanPhalanx
    || dalceanPhalanx.cardType !== 'minion'
    || dalceanPhalanx.rulesText.trim() !== 'Can only move themselves forward.'
    || dalceanPhalanx.manaCost !== 4
    || dalceanPhalanx.attack !== 5
    || dalceanPhalanx.defense !== 5
    || dalceanPhalanx.elements.length !== 1
    || dalceanPhalanx.elements[0] !== 'earth'
    || dalceanPhalanx.thresholds.air !== 0
    || dalceanPhalanx.thresholds.earth !== 2
    || dalceanPhalanx.thresholds.fire !== 0
    || dalceanPhalanx.thresholds.water !== 0
    || dalceanPhalanx.rarity !== 'exceptional') {
    throw new Error('private forward-only minion no longer matches its supported facts');
  }
  const secretTunnel = snapshot.cards.find(({ name }) => name === 'Secret Tunnel');
  if (!secretTunnel
    || secretTunnel.cardType !== 'site'
    || secretTunnel.rulesText.trim()
      !== 'Burrowed allies can move as if this were adjacent to your other sites.'
    || secretTunnel.manaCost !== null
    || secretTunnel.attack !== null
    || secretTunnel.defense !== null
    || secretTunnel.life !== null
    || secretTunnel.elements.length !== 1
    || secretTunnel.elements[0] !== 'earth'
    || secretTunnel.thresholds.air !== 0
    || secretTunnel.thresholds.earth !== 1
    || secretTunnel.thresholds.fire !== 0
    || secretTunnel.thresholds.water !== 0
    || secretTunnel.rarity !== 'exceptional') {
    throw new Error('private burrowed-allies connection site no longer matches its supported facts');
  }
  const voidwalkMinion = snapshot.cards.find(({ name }) => name === 'Spectral Stalker');
  if (!voidwalkMinion
    || voidwalkMinion.cardType !== 'minion'
    || voidwalkMinion.rulesText.trim() !== 'Voidwalk'
    || voidwalkMinion.manaCost !== 2
    || voidwalkMinion.attack !== 2
    || voidwalkMinion.defense !== 2
    || voidwalkMinion.elements.length !== 1
    || voidwalkMinion.elements[0] !== 'air'
    || voidwalkMinion.thresholds.air !== 1
    || voidwalkMinion.thresholds.earth !== 0
    || voidwalkMinion.thresholds.fire !== 0
    || voidwalkMinion.thresholds.water !== 0
    || voidwalkMinion.rarity !== 'ordinary') {
    throw new Error('private Voidwalk minion no longer matches its supported facts');
  }
  const forsaken = snapshot.cards.find(({ name }) => name === 'Forsaken');
  if (!forsaken
    || forsaken.cardType !== 'minion'
    || forsaken.rulesText.trim().replaceAll('\r\n', '\n')
      !== 'Voidwalk\n\nMust be cast to an outer column.'
    || forsaken.manaCost !== 2
    || forsaken.attack !== 3
    || forsaken.defense !== 3
    || forsaken.elements.length !== 1
    || forsaken.elements[0] !== 'air'
    || forsaken.thresholds.air !== 1
    || forsaken.thresholds.earth !== 0
    || forsaken.thresholds.fire !== 0
    || forsaken.thresholds.water !== 0
    || forsaken.rarity !== 'ordinary') {
    throw new Error('private outer-column casting minion no longer matches its supported facts');
  }
  const leylineHenge = snapshot.cards.find(({ name }) => name === 'Leyline Henge');
  if (!leylineHenge
    || leylineHenge.cardType !== 'site'
    || leylineHenge.rulesText.trim()
      !== 'Genesis → Draw a spell for each other adjacent Leyline Henge.'
    || leylineHenge.manaCost !== null
    || leylineHenge.attack !== null
    || leylineHenge.defense !== null
    || leylineHenge.life !== null
    || leylineHenge.elements.length !== 0
    || leylineHenge.thresholds.air !== 0
    || leylineHenge.thresholds.earth !== 0
    || leylineHenge.thresholds.fire !== 0
    || leylineHenge.thresholds.water !== 0
    || leylineHenge.rarity !== 'ordinary') {
    throw new Error('private adjacent Leyline Genesis site no longer matches its supported facts');
  }
  const genesisSpellMinion = snapshot.cards.find(({ name }) => name === 'Apprentice Wizard');
  if (!genesisSpellMinion
    || genesisSpellMinion.cardType !== 'minion'
    || genesisSpellMinion.rulesText.trim().replaceAll('\r\n', '\n') !== 'Spellcaster\nGenesis → Draw a spell.'
    || genesisSpellMinion.manaCost !== 3
    || genesisSpellMinion.attack !== 1
    || genesisSpellMinion.defense !== 1
    || genesisSpellMinion.elements.length !== 1
    || genesisSpellMinion.elements[0] !== 'air'
    || genesisSpellMinion.thresholds.air !== 1
    || genesisSpellMinion.thresholds.earth !== 0
    || genesisSpellMinion.thresholds.fire !== 0
    || genesisSpellMinion.thresholds.water !== 0
    || genesisSpellMinion.rarity !== 'ordinary') {
    throw new Error('private Genesis spell-draw minion no longer matches its supported facts');
  }
  const polarBears = snapshot.cards.find(({ name }) => name === 'Polar Bears');
  if (!polarBears
    || polarBears.cardType !== 'minion'
    || polarBears.rulesText.trim() !== 'Can move as if the top and bottom edges of the realm were connected.'
    || polarBears.manaCost !== 2
    || polarBears.attack !== 2
    || polarBears.defense !== 2
    || polarBears.elements.length !== 1
    || polarBears.elements[0] !== 'water'
    || polarBears.thresholds.air !== 0
    || polarBears.thresholds.earth !== 0
    || polarBears.thresholds.fire !== 0
    || polarBears.thresholds.water !== 1
    || polarBears.rarity !== 'ordinary') {
    throw new Error('private top/bottom connection minion no longer matches its supported facts');
  }
  const pudgeButcher = snapshot.cards.find(({ name }) => name === 'Pudge Butcher');
  if (!pudgeButcher
    || pudgeButcher.cardType !== 'minion'
    || pudgeButcher.rulesText.trim().replaceAll('\r\n', '\n')
      !== 'UPDATED: Immobile\n\nTap → Shoot a projectile. If it hits a unit, drag it to this location. Pudge may fight it when it arrives.'
    || pudgeButcher.manaCost !== 4
    || pudgeButcher.attack !== 5
    || pudgeButcher.defense !== 5
    || pudgeButcher.elements.length !== 1
    || pudgeButcher.elements[0] !== 'earth'
    || pudgeButcher.thresholds.air !== 0
    || pudgeButcher.thresholds.earth !== 2
    || pudgeButcher.thresholds.fire !== 0
    || pudgeButcher.thresholds.water !== 0
    || pudgeButcher.rarity !== 'exceptional') {
    throw new Error('private Immobile minion no longer matches its supported facts');
  }
  const zap = snapshot.cards.find(({ name }) => name === 'Zap!');
  if (!zap
    || zap.cardType !== 'magic'
    || zap.rulesText.trim() !== 'Deal 1 damage to target unit.'
    || zap.manaCost !== 1
    || zap.attack !== null
    || zap.defense !== null
    || zap.life !== null
    || zap.elements.length !== 1
    || zap.elements[0] !== 'air'
    || zap.thresholds.air !== 1
    || zap.thresholds.earth !== 0
    || zap.thresholds.fire !== 0
    || zap.thresholds.water !== 0
    || zap.rarity !== 'ordinary') {
    throw new Error('private target-unit damage Magic no longer matches its supported facts');
  }
  const cannotDefendMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.cannotDefendMinionStableId);
  if (!cannotDefendMinion
    || cannotDefendMinion.cardType !== 'minion'
    || cannotDefendMinion.rulesText.trim() !== "Can't move to defend."
    || cannotDefendMinion.attack === null
    || cannotDefendMinion.defense === null
    || cannotDefendMinion.manaCost === null
    || cannotDefendMinion.rarity === null) {
    throw new Error('private moving-Defend restriction minion no longer matches its supported facts');
  }
  const chargeMinion = snapshot.cards.find(({ stableId }) => stableId === config.chargeMinionStableId);
  if (!chargeMinion
    || chargeMinion.cardType !== 'minion'
    || chargeMinion.rulesText.trim() !== 'Charge'
    || chargeMinion.attack === null
    || chargeMinion.defense === null
    || chargeMinion.manaCost === null
    || chargeMinion.rarity === null) {
    throw new Error('private Charge minion no longer matches its supported facts');
  }
  const deathriteMinion = snapshot.cards.find(({ stableId }) => stableId === config.deathriteMinionStableId);
  if (!deathriteMinion
    || deathriteMinion.cardType !== 'minion'
    || deathriteMinion.rulesText.trim() !== 'Deathrite → Draw a site.'
    || deathriteMinion.attack === null
    || deathriteMinion.defense === null
    || deathriteMinion.manaCost === null
    || deathriteMinion.rarity === null) {
    throw new Error('private Deathrite minion no longer matches its supported facts');
  }
  const providerMinion = snapshot.cards.find(({ stableId }) => stableId === config.providerMinionStableId);
  if (!providerMinion
    || providerMinion.cardType !== 'minion'
    || providerMinion.rulesText.trim() !== 'Provides (F)'
    || providerMinion.attack === null
    || providerMinion.defense === null
    || providerMinion.manaCost === null
    || providerMinion.rarity === null) {
    throw new Error('private affinity provider no longer matches its supported facts');
  }
  const rangedMinion = snapshot.cards.find(({ stableId }) => stableId === config.rangedMinionStableId);
  if (!rangedMinion
    || rangedMinion.cardType !== 'minion'
    || rangedMinion.rulesText.trim() !== 'Ranged'
    || rangedMinion.attack === null
    || rangedMinion.defense === null
    || rangedMinion.manaCost === null
    || rangedMinion.rarity === null) {
    throw new Error('private Ranged minion no longer matches its supported facts');
  }
  const lethalMinion = snapshot.cards.find(({ stableId }) => stableId === config.lethalMinionStableId);
  if (!lethalMinion
    || lethalMinion.cardType !== 'minion'
    || lethalMinion.rulesText.trim() !== 'Lethal'
    || lethalMinion.attack === null
    || lethalMinion.defense === null
    || lethalMinion.manaCost === null
    || lethalMinion.rarity === null) {
    throw new Error('private Lethal minion no longer matches its supported facts');
  }
  const lumberingMinion = snapshot.cards.find(({ stableId }) => stableId === config.lumberingMinionStableId);
  if (!lumberingMinion
    || lumberingMinion.cardType !== 'minion'
    || lumberingMinion.rulesText.trim() !== "Can't defend or intercept."
    || lumberingMinion.attack === null
    || lumberingMinion.defense === null
    || lumberingMinion.manaCost === null
    || lumberingMinion.rarity === null) {
    throw new Error('private Defend-or-Intercept prohibition minion no longer matches its supported facts');
  }
  const monstrousLion = snapshot.cards.find(({ stableId }) => stableId === config.monstrousLionStableId);
  if (!monstrousLion
    || monstrousLion.cardType !== 'minion'
    || monstrousLion.rulesText.trim() !== "Charge, Can't attack sites"
    || monstrousLion.attack === null
    || monstrousLion.defense === null
    || monstrousLion.manaCost === null
    || monstrousLion.rarity === null) {
    throw new Error('private Charge and site-attack restriction minion no longer matches its supported facts');
  }
  const genesisMinion = snapshot.cards.find(({ stableId }) => stableId === config.genesisMinionStableId);
  if (!genesisMinion
    || genesisMinion.cardType !== 'minion'
    || genesisMinion.rulesText.trim() !== 'Genesis → Draw a site.'
    || genesisMinion.attack === null
    || genesisMinion.defense === null
    || genesisMinion.manaCost === null
    || genesisMinion.rarity === null) {
    throw new Error('private Genesis minion no longer matches its supported facts');
  }
  const ghostTownSite = snapshot.cards.find(({ stableId }) => stableId === config.ghostTownSiteStableId);
  if (!ghostTownSite
    || ghostTownSite.name !== 'Ghost Town'
    || ghostTownSite.cardType !== 'site'
    || ghostTownSite.rulesText.trim() !== 'Genesis → Gain (1) this turn.'
    || ghostTownSite.manaCost !== null
    || ghostTownSite.attack !== null
    || ghostTownSite.defense !== null
    || ghostTownSite.life !== null
    || ghostTownSite.elements.length !== 0
    || ghostTownSite.thresholds.air !== 0
    || ghostTownSite.thresholds.earth !== 0
    || ghostTownSite.thresholds.fire !== 0
    || ghostTownSite.thresholds.water !== 0
    || ghostTownSite.rarity !== 'exceptional') {
    throw new Error('private site Genesis mana card no longer matches its supported facts');
  }
  const healingMinion = snapshot.cards.find(({ stableId }) => stableId === config.healingMinionStableId);
  if (!healingMinion
    || healingMinion.cardType !== 'minion'
    || healingMinion.rulesText.trim() !== 'Deathrite → You heal 3.'
    || healingMinion.attack === null
    || healingMinion.defense === null
    || healingMinion.manaCost === null
    || healingMinion.rarity === null) {
    throw new Error('private Deathrite healing minion no longer matches its supported facts');
  }
  const earthProviderMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.earthProviderMinionStableId);
  if (!earthProviderMinion
    || earthProviderMinion.cardType !== 'minion'
    || earthProviderMinion.rulesText.trim() !== 'Provides (E)'
    || earthProviderMinion.attack === null
    || earthProviderMinion.defense === null
    || earthProviderMinion.manaCost === null
    || earthProviderMinion.rarity === null) {
    throw new Error('private Earth affinity provider no longer matches its supported facts');
  }
  const manaMinion = snapshot.cards.find(({ stableId }) => stableId === config.manaMinionStableId);
  if (!manaMinion
    || manaMinion.cardType !== 'minion'
    || manaMinion.rulesText.trim() !== 'Tap → Gain (2) this turn.'
    || manaMinion.attack === null
    || manaMinion.defense === null
    || manaMinion.manaCost === null
    || manaMinion.rarity === null) {
    throw new Error('private mana minion no longer matches its supported facts');
  }
  const movementMinion = snapshot.cards.find(({ stableId }) => stableId === config.movementMinionStableId);
  if (!movementMinion
    || movementMinion.cardType !== 'minion'
    || movementMinion.rulesText.trim() !== 'Movement +1'
    || movementMinion.attack === null
    || movementMinion.defense === null
    || movementMinion.manaCost === null
    || movementMinion.rarity === null) {
    throw new Error('private Movement +1 minion no longer matches its supported facts');
  }
  const movementTwoMinion = snapshot.cards.find(({ name }) => name === 'Cloud Spirit');
  if (!movementTwoMinion
    || movementTwoMinion.cardType !== 'minion'
    || movementTwoMinion.rulesText.trim() !== 'Airborne, Movement +2'
    || movementTwoMinion.manaCost !== 2
    || movementTwoMinion.attack !== 2
    || movementTwoMinion.defense !== 2
    || movementTwoMinion.elements.length !== 1
    || movementTwoMinion.elements[0] !== 'air'
    || movementTwoMinion.thresholds.air !== 2
    || movementTwoMinion.thresholds.earth !== 0
    || movementTwoMinion.thresholds.fire !== 0
    || movementTwoMinion.thresholds.water !== 0
    || movementTwoMinion.rarity !== 'ordinary') {
    throw new Error('private Movement +2 minion no longer matches its supported facts');
  }
  const roamingMinion = snapshot.cards.find(({ stableId }) => stableId === config.roamingMinionStableId);
  if (!roamingMinion
    || roamingMinion.cardType !== 'minion'
    || roamingMinion.rulesText.trim() !== 'May be summoned to any site.'
    || roamingMinion.attack === null
    || roamingMinion.defense === null
    || roamingMinion.manaCost === null
    || roamingMinion.rarity === null) {
    throw new Error('private unrestricted-site summon minion no longer matches its supported facts');
  }
  const wardMinion = snapshot.cards.find(({ stableId }) => stableId === config.wardMinionStableId);
  if (!wardMinion
    || wardMinion.name !== 'Holy Warrior'
    || wardMinion.cardType !== 'minion'
    || wardMinion.rulesText.trim() !== 'Ward'
    || wardMinion.manaCost !== 2
    || wardMinion.attack !== 2
    || wardMinion.defense !== 2
    || wardMinion.elements.length !== 1
    || wardMinion.elements[0] !== 'earth'
    || wardMinion.thresholds.earth !== 1
    || wardMinion.thresholds.air !== 0
    || wardMinion.thresholds.fire !== 0
    || wardMinion.thresholds.water !== 0
    || wardMinion.rarity !== 'ordinary') {
    throw new Error('private Ward minion no longer matches its supported facts');
  }
  const firstStrikeMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.firstStrikeMinionStableId);
  if (!firstStrikeMinion
    || firstStrikeMinion.name !== 'Albespine Pikemen'
    || firstStrikeMinion.cardType !== 'minion'
    || firstStrikeMinion.rulesText.trim() !== 'Strikes first while attacking.'
    || firstStrikeMinion.manaCost !== 3
    || firstStrikeMinion.attack !== 3
    || firstStrikeMinion.defense !== 3
    || firstStrikeMinion.elements.length !== 1
    || firstStrikeMinion.elements[0] !== 'earth'
    || firstStrikeMinion.thresholds.earth !== 2
    || firstStrikeMinion.thresholds.air !== 0
    || firstStrikeMinion.thresholds.fire !== 0
    || firstStrikeMinion.thresholds.water !== 0
    || firstStrikeMinion.rarity !== 'exceptional') {
    throw new Error('private attacking first-strike minion no longer matches its supported facts');
  }
  const firstStrikeTargetMinion = snapshot.cards
    .find(({ stableId }) => stableId === config.firstStrikeTargetMinionStableId);
  if (!firstStrikeTargetMinion
    || firstStrikeTargetMinion.name !== 'Bosk Troll'
    || firstStrikeTargetMinion.cardType !== 'minion'
    || firstStrikeTargetMinion.rulesText.trim() !== ''
    || firstStrikeTargetMinion.manaCost !== 2
    || firstStrikeTargetMinion.attack !== 3
    || firstStrikeTargetMinion.defense !== 3
    || firstStrikeTargetMinion.elements.length !== 1
    || firstStrikeTargetMinion.elements[0] !== 'earth'
    || firstStrikeTargetMinion.thresholds.earth !== 1
    || firstStrikeTargetMinion.thresholds.air !== 0
    || firstStrikeTargetMinion.thresholds.fire !== 0
    || firstStrikeTargetMinion.thresholds.water !== 0
    || firstStrikeTargetMinion.rarity !== 'ordinary') {
    throw new Error('private first-strike target minion no longer matches its supported facts');
  }

  const formatsValue = parseJsonWithDuplicateKeyCheck(await readFile(resolve(revisionRoot, 'formats.json'), 'utf8'));
  if (!isJsonRecord(formatsValue)
    || !Array.isArray(formatsValue.formats)) {
    throw new Error('private format artifact collection is invalid');
  }
  const formats = formatsValue.formats.map((value) => formatArtifactSchema.parse(value));
  formats.forEach((value) => {
    if (identityHash(value.identity as unknown as JsonValue) !== value.contentHash) {
      throw new Error('private format artifact identity is invalid');
    }
  });
  const selected = formats
    .filter(({ identity }) => identity.payload.name === 'Constructed')
    .sort((left, right) => right.identity.payload.effectiveDate.localeCompare(left.identity.payload.effectiveDate))[0];
  if (!selected) throw new Error('private authority has no Constructed format');
  return {
    airborneMinion,
    airborneTargetMinion,
    arcLightning,
    authorityHash: artifact.contentHash,
    bury,
    burrowingMinion,
    cards: snapshot.cards,
    cannotDefendMinion,
    chargeMagic,
    chargeMinion,
    config,
    dalceanPhalanx,
    deathriteMinion,
    divineHealing,
    drown,
    drowned,
    earthProviderMinion,
    elthamTownsfolk,
    entombed,
    format: selected.identity.payload,
    formatStableId: selected.identity.stableId,
    firstStrikeMinion,
    firstStrikeTargetMinion,
    forsaken,
    freeze,
    genesisSpellMinion,
    genesisMinion,
    ghostTownSite,
    healingMinion,
    lethalMinion,
    leylineHenge,
    lesserBloodDemon,
    ignited,
    lightningBolt,
    lugbogCat,
    lure,
    lumberingMinion,
    manaMinion,
    minorExplosion,
    monstrousLion,
    movementMinion,
    movementTwoMinion,
    overpower,
    polarBears,
    pirateShip,
    pudgeButcher,
    rainOfArrows,
    providerMinion,
    raalDromedary,
    rangedMinion,
    rescue,
    roamingMinion,
    secretTunnel,
    sedgeCrabs,
    seravaTownsfolk,
    shallowGrave,
    sinkhole,
    slyFox,
    stealthMinion,
    stealthTargetMinion,
    submergeMinion,
    teleport,
    voidwalkMinion,
    wardMinion,
    zap,
  };
}

function ordered<T extends { stableId: string }>(values: readonly T[], reverse: boolean): readonly T[] {
  return [...values].sort((left, right) => {
    const order = left.stableId < right.stableId ? -1 : left.stableId > right.stableId ? 1 : 0;
    return reverse ? -order : order;
  });
}

function fillZone(
  candidates: readonly NormalizedCard[],
  count: number,
  format: FormatDefinition,
  reverse: boolean,
): readonly string[] {
  const result: string[] = [];
  for (const card of ordered(candidates, reverse)) {
    if (!card.rarity) throw new Error('supported deck card lacks rarity');
    const copies = format.copyLimits[card.rarity];
    result.push(...Array.from({ length: Math.min(copies, count - result.length) }, () => card.stableId));
    if (result.length === count) return result;
  }
  throw new Error(`supported actual cards can fill only ${result.length} of ${count} required cards`);
}

function gameDefinition(
  card: NormalizedCard,
  drawSpell: boolean,
  charge = false,
  genesisDrawSite = false,
  lethal = false,
  provides?: GameElement,
  tapForMana?: number,
  deathriteDrawSite = false,
  cannotDefend = false,
  movementBonus: 0 | 1 | 2 = 0,
  deathriteHeal = 0,
  siteGenesisGainMana = 0,
  summonToAnySite = false,
  cannotDefendOrIntercept = false,
  cannotAttackSites = false,
  ranged = false,
  strikesFirstWhileAttacking = false,
  ward = false,
  airborne = false,
  stealth = false,
  gainsStealthAtEndOfTurn = false,
  movesOnlySideways = false,
  submerge = false,
  burrowing = false,
  voidwalk = false,
  genesisDrawSpell = false,
  connectsTopBottom = false,
  mustBeCastToOuterColumn = false,
  siteGenesisDrawSpellPerAdjacentSameCard = false,
  mustBeCastBurrowed = false,
  mustBeCastSubmerged = false,
  mustBeCastToWaterSite = false,
  movesOnlyForward = false,
  connectsBurrowedAllies = false,
  immobile = false,
  damageTargetUnit: 0 | 1 | 4 = 0,
  targetNearby = false,
  healController: 0 | 7 = 0,
  burrowTargetMinion = false,
  siteGenesisDiscardTopSpells: 0 | 2 = 0,
  shootsDragProjectile = false,
  damageRandomUnitAtLocation: 0 | 3 = 0,
  teleportAllyToTargetSite = false,
  returnMinionFromOwnCemetery = false,
  disableTargetNearbyMinionUntilNextTurn = false,
  sacrificeToDestroyNearbySite = false,
  submergeTargetMinion = false,
  damageEachUnitAtLocationWithinTwoSteps: 0 | 3 = 0,
  grantChargeToAllyThisTurn = false,
  lureEnemyMinionOneStepCloser = false,
  genesisLoseControllerLife: 0 | 2 = 0,
  waterbound = false,
  diesAtEndOfControllerTurn = false,
  damageEachAbovegroundMinion: 0 | 1 = 0,
  grantPowerToAllyThisTurn: 0 | 2 = 0,
  spellcaster = false,
): GameCardDefinition {
  if (card.cardType === 'avatar'
    && card.attack !== null
    && card.defense !== null
    && card.life !== null) {
    return {
      attack: card.attack,
      cardType: 'avatar',
      defense: card.defense,
      drawSpell,
      life: card.life,
    };
  }
  if (card.cardType === 'site') {
    return {
      cardType: 'site',
      ...(connectsBurrowedAllies ? { connectsBurrowedAllies: true } : {}),
      elements: card.elements,
      ...(siteGenesisDiscardTopSpells
        ? { genesisDiscardTopSpells: siteGenesisDiscardTopSpells }
        : {}),
      genesisDrawSpellPerAdjacentSameCard: siteGenesisDrawSpellPerAdjacentSameCard,
      ...(siteGenesisGainMana ? { genesisGainMana: siteGenesisGainMana } : {}),
      ...(sacrificeToDestroyNearbySite ? { sacrificeToDestroyNearbySite: true } : {}),
    };
  }
  const supportedMagicEffects = Number(damageTargetUnit !== 0)
    + Number(damageEachAbovegroundMinion !== 0)
    + Number(damageEachUnitAtLocationWithinTwoSteps !== 0)
    + Number(damageRandomUnitAtLocation !== 0)
    + Number(grantChargeToAllyThisTurn)
    + Number(grantPowerToAllyThisTurn !== 0)
    + Number(lureEnemyMinionOneStepCloser)
    + Number(teleportAllyToTargetSite)
    + Number(returnMinionFromOwnCemetery)
    + Number(disableTargetNearbyMinionUntilNextTurn)
    + Number(submergeTargetMinion)
    + Number(healController !== 0)
    + Number(burrowTargetMinion);
  if (card.cardType === 'magic'
    && card.manaCost !== null
    && supportedMagicEffects === 1) {
    return {
      ...(burrowTargetMinion ? { burrowTargetMinion: true } : {}),
      cardType: 'magic',
      ...(damageEachAbovegroundMinion !== 0 ? { damageEachAbovegroundMinion } : {}),
      ...(grantPowerToAllyThisTurn !== 0 ? { grantPowerToAllyThisTurn } : {}),
      ...(submergeTargetMinion ? { submergeTargetMinion: true } : {}),
      ...(damageTargetUnit !== 0
        ? { damageTargetUnit }
        : damageEachUnitAtLocationWithinTwoSteps !== 0
          ? { damageEachUnitAtLocationWithinTwoSteps }
          : damageRandomUnitAtLocation !== 0
            ? { damageRandomUnitAtLocation }
            : grantChargeToAllyThisTurn
              ? { grantChargeToAllyThisTurn: true }
              : lureEnemyMinionOneStepCloser
                ? { lureEnemyMinionOneStepCloser: true }
              : teleportAllyToTargetSite
            ? { teleportAllyToTargetSite: true }
            : returnMinionFromOwnCemetery
              ? { returnMinionFromOwnCemetery: true }
              : disableTargetNearbyMinionUntilNextTurn
                ? { disableTargetNearbyMinionUntilNextTurn: true }
        : healController !== 0 ? { healController } : {}),
      manaCost: card.manaCost,
      ...(targetNearby ? { targetNearby: true } : {}),
      thresholds: card.thresholds,
    };
  }
  if (card.cardType === 'minion'
    && card.attack !== null
    && card.defense !== null
    && card.manaCost !== null) {
    return {
      airborne,
      attack: card.attack,
      burrowing,
      cardType: 'minion',
      cannotAttackSites,
      cannotDefend,
      cannotDefendOrIntercept,
      charge,
      connectsTopBottom,
      deathriteDrawSite,
      ...(deathriteHeal ? { deathriteHeal } : {}),
      defense: card.defense,
      ...(diesAtEndOfControllerTurn ? { diesAtEndOfControllerTurn: true } : {}),
      genesisDrawSpell,
      genesisDrawSite,
      ...(genesisLoseControllerLife ? { genesisLoseControllerLife } : {}),
      gainsStealthAtEndOfTurn,
      immobile,
      lethal,
      manaCost: card.manaCost,
      movesOnlyForward,
      mustBeCastBurrowed,
      mustBeCastSubmerged,
      mustBeCastToOuterColumn,
      mustBeCastToWaterSite,
      ...(movementBonus ? { movementBonus } : {}),
      movesOnlySideways,
      ...(provides ? { provides } : {}),
      ranged,
      shootsDragProjectile,
      spellcaster,
      stealth,
      strikesFirstWhileAttacking,
      submerge,
      summonToAnySite,
      ...(tapForMana ? { tapForMana } : {}),
      thresholds: card.thresholds,
      voidwalk,
      waterbound,
      ward,
    };
  }
  throw new Error(`actual card ${card.stableId} lacks required supported facts`);
}

function buildManifest(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  seed: number,
  scenario: 'air' | 'air-arc-lightning' | 'air-genesis-spell' | 'air-leyline' | 'air-lightning-bolt' | 'air-rain-of-arrows' | 'air-spellcaster-freeze' | 'air-teleport' | 'air-voidwalk' | 'air-zap' | 'airborne' | 'combat' | 'earth' | 'earth-burrowing' | 'earth-bury' | 'earth-divine-healing' | 'earth-entombed' | 'earth-first-strike' | 'earth-forward' | 'earth-immobile' | 'earth-overpower' | 'earth-rescue' | 'earth-shallow-grave' | 'earth-sinkhole' | 'earth-tunnel' | 'earth-ward' | 'fire' | 'fire-charge' | 'fire-genesis-life-loss' | 'fire-ignited' | 'fire-minor-explosion' | 'movement-two' | 'stealth' | 'water' | 'water-drown' | 'water-drowned' | 'water-edge-connection' | 'water-freeze' | 'water-lugbog' | 'water-lure' | 'water-pirate-ship' | 'water-sideways' | 'water-stealth' | 'water-submerge' = 'combat',
): Readonly<{ manifest: GameManifest; names: ReadonlyMap<string, string> }> {
  const avatar = input.cards.find(({ stableId }) => stableId === input.config.avatar.stableId);
  if (!avatar || avatar.cardType !== 'avatar') throw new Error('private scenario Avatar is missing');
  const sites = input.cards.filter((card) =>
    card.cardType === 'site' && card.rulesText.trim() === '' && card.life === null);
  const minions = input.cards.filter((card) =>
    card.cardType === 'minion'
      && card.rulesText.trim() === ''
      && card.attack !== null
      && card.defense !== null
      && card.manaCost !== null);
  const deck = (reverse: boolean, includeCharge: boolean): GameDeckSpec => {
    const chargeCopies = includeCharge
      ? input.format.copyLimits[input.chargeMinion.rarity!]
      : 0;
    const providerCopies = includeCharge
      ? input.format.copyLimits[input.providerMinion.rarity!]
      : 0;
    const lethalCopies = includeCharge
      ? input.format.copyLimits[input.lethalMinion.rarity!]
      : 0;
    const genesisCopies = includeCharge
      ? input.format.copyLimits[input.genesisMinion.rarity!]
      : 0;
    return {
    atlas: fillZone(sites, input.format.atlasMinimum, input.format, reverse),
    avatar: avatar.stableId,
    spellbook: [
      ...Array.from({ length: chargeCopies }, () => input.chargeMinion.stableId),
      ...Array.from({ length: providerCopies }, () => input.providerMinion.stableId),
      ...Array.from({ length: lethalCopies }, () => input.lethalMinion.stableId),
      ...Array.from({ length: genesisCopies }, () => input.genesisMinion.stableId),
      ...fillZone(
        minions,
        input.format.spellbookMinimum
          - chargeCopies
          - providerCopies
          - lethalCopies
          - genesisCopies,
        input.format,
        reverse,
      ),
    ],
    };
  };
  const elementalDeck = (
    element: GameElement,
    featuredMinions: readonly NormalizedCard[],
    featuredSites: readonly NormalizedCard[] = [],
    featuredMagic: readonly NormalizedCard[] = [],
  ): GameDeckSpec => {
    const featuredSiteCards = featuredSites.flatMap((card) =>
      Array.from({ length: input.format.copyLimits[card.rarity!] }, () => card.stableId));
    const featuredSiteIds = new Set(featuredSites.map(({ stableId }) => stableId));
    const elementSites = sites.filter((card) =>
      card.rarity && card.elements.includes(element) && !featuredSiteIds.has(card.stableId));
    const elementSiteCount = Math.min(
      input.format.atlasMinimum - featuredSiteCards.length,
      elementSites.reduce((total, card) => total + input.format.copyLimits[card.rarity!], 0),
    );
    const elementSiteIds = new Set(elementSites.map(({ stableId }) => stableId));
    const featuredSpells = [...featuredMinions, ...featuredMagic];
    const featuredSpellCards = featuredSpells.flatMap((card) =>
      Array.from({ length: input.format.copyLimits[card.rarity!] }, () => card.stableId));
    const featuredSpellIds = new Set(featuredSpells.map(({ stableId }) => stableId));
    return {
      atlas: [
        ...featuredSiteCards,
        ...fillZone(elementSites, elementSiteCount, input.format, false),
        ...fillZone(
          sites.filter(({ stableId }) =>
            !featuredSiteIds.has(stableId) && !elementSiteIds.has(stableId)),
          input.format.atlasMinimum - featuredSiteCards.length - elementSiteCount,
          input.format,
          false,
        ),
      ],
      avatar: avatar.stableId,
      spellbook: [
        ...featuredSpellCards,
        ...fillZone(
          minions.filter(({ stableId }) => !featuredSpellIds.has(stableId)),
          input.format.spellbookMinimum - featuredSpellCards.length,
          input.format,
          false,
        ),
      ],
    };
  };
  const earthMinions = [
    input.earthProviderMinion,
    input.manaMinion,
    input.genesisMinion,
    input.deathriteMinion,
    input.cannotDefendMinion,
    input.rangedMinion,
    input.wardMinion,
    input.firstStrikeMinion,
    input.firstStrikeTargetMinion,
  ] as const;
  const earthDeck = elementalDeck('earth', earthMinions, [input.ghostTownSite]);
  const earthOverpowerDeck = elementalDeck(
    'earth',
    [...earthMinions, input.elthamTownsfolk],
    [],
    [input.overpower],
  );
  const earthBuryDeck = elementalDeck('earth', earthMinions, [], [input.bury]);
  const earthRescueDeck = elementalDeck('earth', earthMinions, [], [input.bury, input.rescue]);
  const earthDivineHealingDeck = elementalDeck('earth', earthMinions, [], [input.divineHealing]);
  const earthShallowGraveDeck = elementalDeck('earth', earthMinions, [input.shallowGrave]);
  const earthSinkholeDeck = elementalDeck('earth', earthMinions, [input.sinkhole]);
  const earthBurrowingDeck = elementalDeck('earth', [
    ...earthMinions,
    input.burrowingMinion,
  ], [input.ghostTownSite]);
  const earthEntombedDeck = elementalDeck('earth', [
    ...earthMinions,
    input.entombed,
  ], [input.ghostTownSite]);
  const earthForwardDeck = elementalDeck('earth', [
    ...earthMinions,
    input.dalceanPhalanx,
  ], [input.ghostTownSite]);
  const earthImmobileDeck = elementalDeck('earth', [
    ...earthMinions,
    input.pudgeButcher,
  ], [input.ghostTownSite]);
  const earthTunnelDeck = elementalDeck('earth', [
    ...earthMinions,
    input.burrowingMinion,
  ], [input.secretTunnel]);
  const airMinions = [
    input.movementMinion,
    input.roamingMinion,
    input.airborneMinion,
    input.airborneTargetMinion,
    input.stealthMinion,
    input.stealthTargetMinion,
    input.movementTwoMinion,
  ] as const;
  const airborneDeck = elementalDeck('air', airMinions);
  const airArcLightningDeck = elementalDeck('air', airMinions, [], [input.arcLightning]);
  const airLightningBoltDeck = elementalDeck('air', airMinions, [], [input.lightningBolt]);
  const airRainOfArrowsDeck = elementalDeck('air', airMinions, [], [input.rainOfArrows]);
  const airTeleportDeck = elementalDeck('air', airMinions, [], [input.teleport]);
  const airZapDeck = elementalDeck('air', airMinions, [], [input.zap]);
  const airGenesisSpellDeck = elementalDeck('air', [...airMinions, input.genesisSpellMinion]);
  const airLeylineDeck = elementalDeck('air', airMinions, [input.leylineHenge]);
  const airVoidwalkDeck = elementalDeck('air', [
    ...airMinions,
    input.voidwalkMinion,
    input.forsaken,
  ]);
  const spellcasterAirSite = ordered(sites.filter((card) =>
    card.rarity === 'ordinary' && card.elements.includes('air')), false)[0];
  const spellcasterWaterSite = ordered(sites.filter((card) =>
    card.rarity === 'ordinary'
      && card.elements.includes('water')
      && card.stableId !== spellcasterAirSite?.stableId), false)[0];
  if (!spellcasterAirSite || !spellcasterWaterSite) {
    throw new Error('private Spellcaster deck lacks supported Air and Water sites');
  }
  const spellcasterFixedSites = [input.ghostTownSite, spellcasterAirSite, spellcasterWaterSite];
  const spellcasterFixedSiteCards = spellcasterFixedSites.flatMap((card) =>
    Array.from({ length: input.format.copyLimits[card.rarity!] }, () => card.stableId));
  const spellcasterFixedSiteIds = new Set(spellcasterFixedSites.map(({ stableId }) => stableId));
  const airSpellcasterFreezeBase = elementalDeck(
    'air',
    [...airMinions, input.genesisSpellMinion, input.seravaTownsfolk],
    [],
    [input.freeze],
  );
  const airSpellcasterFreezeDeck: GameDeckSpec = {
    ...airSpellcasterFreezeBase,
    atlas: [
      ...spellcasterFixedSiteCards,
      ...fillZone(
        sites.filter((card) => card.rarity
          && !spellcasterFixedSiteIds.has(card.stableId)
          && (card.elements.includes('air') || card.elements.includes('water'))),
        input.format.atlasMinimum - spellcasterFixedSiteCards.length,
        input.format,
        false,
      ),
    ],
  };
  const fireMinorExplosionDeck = elementalDeck(
    'fire',
    [input.raalDromedary],
    [],
    [input.minorExplosion],
  );
  const fireChargeDeck = elementalDeck(
    'fire',
    [input.raalDromedary],
    [],
    [input.chargeMagic],
  );
  const fireGenesisLifeLossDeck = elementalDeck('fire', [input.lesserBloodDemon]);
  const fireIgnitedDeck = elementalDeck('fire', [input.ignited]);
  const waterDeck = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
    input.sedgeCrabs,
  ]);
  const waterFreezeDeck = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
    input.sedgeCrabs,
    input.seravaTownsfolk,
  ], [], [input.freeze]);
  const waterLureDeck = elementalDeck('water', [input.seravaTownsfolk], [], [input.lure]);
  const waterPirateShipDeck = elementalDeck(
    'water',
    [input.pirateShip],
    [input.ghostTownSite],
  );
  const waterDrownDeck = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
    input.sedgeCrabs,
    input.seravaTownsfolk,
  ], [input.ghostTownSite], [input.drown]);
  const waterSubmergeDeck = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
    input.sedgeCrabs,
    input.submergeMinion,
  ]);
  const waterDrownedDeck = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
    input.sedgeCrabs,
    input.drowned,
  ]);
  const waterEdgeConnectionDeck = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
    input.sedgeCrabs,
    input.polarBears,
  ]);
  const waterLugbogBase = elementalDeck('water', [
    input.healingMinion,
    input.slyFox,
    input.sedgeCrabs,
    input.lugbogCat,
  ]);
  const waterLugbogDeck: GameDeckSpec = waterLugbogBase;
  const decks = {
    north: scenario === 'air-leyline'
      ? airLeylineDeck
      : scenario === 'air-arc-lightning'
      ? airArcLightningDeck
      : scenario === 'air-lightning-bolt'
      ? airLightningBoltDeck
      : scenario === 'air-rain-of-arrows'
      ? airRainOfArrowsDeck
      : scenario === 'air-spellcaster-freeze'
      ? airSpellcasterFreezeDeck
      : scenario === 'air-teleport'
      ? airTeleportDeck
      : scenario === 'air-genesis-spell'
      ? airGenesisSpellDeck
      : scenario === 'air-voidwalk'
      ? airVoidwalkDeck
      : scenario === 'air-zap'
      ? airZapDeck
      : scenario === 'airborne' || scenario === 'movement-two' || scenario === 'stealth'
      ? airborneDeck
      : scenario === 'earth-entombed'
        ? earthEntombedDeck
      : scenario === 'earth-bury'
        ? earthBuryDeck
      : scenario === 'earth-rescue'
        ? earthRescueDeck
      : scenario === 'earth-divine-healing'
        ? earthDivineHealingDeck
      : scenario === 'earth-shallow-grave'
        ? earthShallowGraveDeck
      : scenario === 'earth-sinkhole'
        ? earthSinkholeDeck
      : scenario === 'earth-forward'
        ? earthForwardDeck
      : scenario === 'earth-immobile'
        ? earthImmobileDeck
      : scenario === 'earth-overpower'
        ? earthOverpowerDeck
      : scenario === 'earth-tunnel'
        ? earthTunnelDeck
      : scenario === 'earth-burrowing'
        ? earthBurrowingDeck
      : scenario === 'earth' || scenario === 'earth-first-strike' || scenario === 'earth-ward'
      ? earthDeck
      : scenario === 'air'
        ? elementalDeck('air', [input.movementMinion, input.roamingMinion])
        : scenario === 'fire'
          ? elementalDeck('fire', [input.lumberingMinion, input.monstrousLion])
        : scenario === 'fire-charge'
          ? fireChargeDeck
        : scenario === 'fire-genesis-life-loss'
          ? fireGenesisLifeLossDeck
        : scenario === 'fire-ignited'
          ? fireIgnitedDeck
        : scenario === 'fire-minor-explosion'
          ? fireMinorExplosionDeck
        : scenario === 'water-edge-connection'
          ? waterEdgeConnectionDeck
        : scenario === 'water-drowned'
          ? waterDrownedDeck
        : scenario === 'water-drown'
          ? waterDrownDeck
        : scenario === 'water-freeze'
          ? waterFreezeDeck
        : scenario === 'water-lure'
          ? waterLureDeck
        : scenario === 'water-pirate-ship'
          ? waterPirateShipDeck
        : scenario === 'water-lugbog'
          ? waterLugbogDeck
        : scenario === 'water-submerge'
          ? waterSubmergeDeck
        : scenario === 'water' || scenario === 'water-sideways' || scenario === 'water-stealth'
          ? waterDeck
          : deck(false, true),
    south: scenario === 'air-leyline'
      ? airLeylineDeck
      : scenario === 'air-arc-lightning'
      ? airArcLightningDeck
      : scenario === 'air-lightning-bolt'
      ? airLightningBoltDeck
      : scenario === 'air-rain-of-arrows'
      ? airRainOfArrowsDeck
      : scenario === 'air-spellcaster-freeze'
      ? airSpellcasterFreezeDeck
      : scenario === 'air-teleport'
      ? airTeleportDeck
      : scenario === 'air-genesis-spell'
      ? airGenesisSpellDeck
      : scenario === 'air-voidwalk'
      ? airVoidwalkDeck
      : scenario === 'air-zap'
      ? airZapDeck
      : scenario === 'airborne' || scenario === 'movement-two' || scenario === 'stealth'
      ? airborneDeck
      : scenario === 'water-edge-connection'
        ? waterEdgeConnectionDeck
      : scenario === 'water-drowned'
        ? waterDrownedDeck
      : scenario === 'water-drown'
        ? waterDrownDeck
      : scenario === 'water-freeze'
        ? waterFreezeDeck
      : scenario === 'water-lure'
        ? waterLureDeck
      : scenario === 'water-pirate-ship'
        ? waterPirateShipDeck
      : scenario === 'water-lugbog'
        ? waterLugbogDeck
      : scenario === 'earth-entombed'
        ? earthEntombedDeck
      : scenario === 'earth-bury'
        ? earthBuryDeck
      : scenario === 'earth-rescue'
        ? earthRescueDeck
      : scenario === 'earth-divine-healing'
        ? earthDivineHealingDeck
      : scenario === 'earth-shallow-grave'
        ? earthShallowGraveDeck
      : scenario === 'earth-sinkhole'
        ? earthSinkholeDeck
      : scenario === 'earth-forward'
        ? earthForwardDeck
      : scenario === 'earth-immobile'
        ? earthImmobileDeck
      : scenario === 'earth-overpower'
        ? earthOverpowerDeck
      : scenario === 'earth-tunnel'
        ? earthTunnelDeck
      : scenario === 'earth-burrowing'
        ? earthBurrowingDeck
      : scenario === 'earth-first-strike' || scenario === 'earth-ward' ? earthDeck : deck(true, false),
  };
  const referenced = new Set([
    decks.north.avatar,
    ...decks.north.atlas,
    ...decks.north.spellbook,
    ...decks.south.atlas,
    ...decks.south.spellbook,
  ]);
  const selectedCards = input.cards.filter(({ stableId }) => referenced.has(stableId));
  const definitions = Object.fromEntries(selectedCards.map((card) => [
    card.stableId,
    gameDefinition(
      card,
      card.stableId === avatar.stableId && input.config.avatar.drawSpell,
      card.stableId === input.chargeMinion.stableId
        || card.stableId === input.monstrousLion.stableId
        || card.stableId === input.ignited.stableId,
      card.stableId === input.genesisMinion.stableId,
      card.stableId === input.lethalMinion.stableId,
      card.stableId === input.providerMinion.stableId
        ? 'fire'
        : card.stableId === input.earthProviderMinion.stableId
          ? 'earth'
          : undefined,
      card.stableId === input.manaMinion.stableId ? 2 : undefined,
      card.stableId === input.deathriteMinion.stableId,
      card.stableId === input.cannotDefendMinion.stableId,
      card.stableId === input.movementMinion.stableId
        ? 1
        : card.stableId === input.movementTwoMinion.stableId ? 2 : 0,
      card.stableId === input.healingMinion.stableId ? 3 : 0,
      card.stableId === input.ghostTownSite.stableId ? 1 : 0,
      card.stableId === input.roamingMinion.stableId
        || card.stableId === input.lugbogCat.stableId,
      card.stableId === input.lumberingMinion.stableId,
      card.stableId === input.monstrousLion.stableId,
      card.stableId === input.rangedMinion.stableId,
      card.stableId === input.firstStrikeMinion.stableId,
      card.stableId === input.wardMinion.stableId,
      card.stableId === input.airborneMinion.stableId
        || card.stableId === input.movementTwoMinion.stableId,
      card.stableId === input.stealthMinion.stableId,
      card.stableId === input.slyFox.stableId,
      card.stableId === input.sedgeCrabs.stableId,
      card.stableId === input.submergeMinion.stableId
        || card.stableId === input.drowned.stableId,
      card.stableId === input.burrowingMinion.stableId
        || card.stableId === input.entombed.stableId,
      card.stableId === input.voidwalkMinion.stableId
        || card.stableId === input.forsaken.stableId,
      card.stableId === input.genesisSpellMinion.stableId,
      card.stableId === input.polarBears.stableId,
      card.stableId === input.forsaken.stableId,
      card.stableId === input.leylineHenge.stableId,
      card.stableId === input.entombed.stableId,
      card.stableId === input.drowned.stableId,
      card.stableId === input.lugbogCat.stableId,
      card.stableId === input.dalceanPhalanx.stableId,
      card.stableId === input.secretTunnel.stableId,
      card.stableId === input.pudgeButcher.stableId,
      card.stableId === input.arcLightning.stableId
        ? 4
        : card.stableId === input.zap.stableId ? 1 : 0,
      card.stableId === input.arcLightning.stableId,
      card.stableId === input.divineHealing.stableId ? 7 : 0,
      card.stableId === input.bury.stableId,
      card.stableId === input.shallowGrave.stableId ? 2 : 0,
      card.stableId === input.pudgeButcher.stableId,
      card.stableId === input.lightningBolt.stableId ? 3 : 0,
      card.stableId === input.teleport.stableId,
      card.stableId === input.rescue.stableId,
      card.stableId === input.freeze.stableId,
      card.stableId === input.sinkhole.stableId,
      card.stableId === input.drown.stableId,
      card.stableId === input.minorExplosion.stableId ? 3 : 0,
      card.stableId === input.chargeMagic.stableId,
      card.stableId === input.lure.stableId,
      card.stableId === input.lesserBloodDemon.stableId ? 2 : 0,
      card.stableId === input.pirateShip.stableId,
      card.stableId === input.ignited.stableId,
      card.stableId === input.rainOfArrows.stableId ? 1 : 0,
      card.stableId === input.overpower.stableId ? 2 : 0,
      card.stableId === input.genesisSpellMinion.stableId,
    ),
  ]));
  return {
    manifest: createGameManifest({
      authority: {
        contentHash: input.authorityHash,
        mode: 'private-local',
        revisionId: input.config.revisionId,
      },
      cards: definitions,
      decks,
      firstSeat: 'north',
      seed,
    }),
    names: new Map(selectedCards.map(({ name, stableId }) => [stableId, name])),
  };
}

function action(session: GameSession, predicate: (candidate: GameLegalAction) => boolean): GameLegalAction {
  const found = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!found) throw new Error(`actual-card scenario has no expected action in ${session.state.phase}`);
  return found;
}

function accept(session: GameSession, candidate: GameLegalAction): GameSession {
  const result = stepGame(session, candidate);
  if (!result.accepted) throw new Error(`actual-card scenario action rejected: ${result.reason.code}`);
  return result.session;
}

function openingPair(
  session: GameSession,
  seat: GameSeat,
  preferredCardId?: string,
  maximumMana = 1,
  excludedSiteInstanceId?: string,
): Readonly<{
  minionInstanceId: string;
  siteInstanceId: string;
}> | null {
  const player = session.state.players[seat];
  for (const site of player.hand.atlas) {
    if (site.instanceId === excludedSiteInstanceId) continue;
    const siteDefinition = session.state.cards[site.cardId];
    if (siteDefinition?.cardType !== 'site') continue;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    for (const minion of player.hand.spellbook) {
      if (preferredCardId && minion.cardId !== preferredCardId) continue;
      const definition = session.state.cards[minion.cardId];
      if (definition?.cardType !== 'minion' || definition.manaCost > maximumMana) continue;
      if ((['air', 'earth', 'fire', 'water'] as const)
        .every((element) => affinity[element] >= definition.thresholds[element])) {
        return { minionInstanceId: minion.instanceId, siteInstanceId: site.instanceId };
      }
    }
  }
  return null;
}

function availableMinionInstance(
  session: GameSession,
  seat: GameSeat,
  cardId: string,
  draws: number,
): string | undefined {
  const player = session.state.players[seat];
  return [...player.hand.spellbook, ...player.spellbook.slice(0, draws)]
    .find((card) => card.cardId === cardId)?.instanceId;
}

function openingSiteForMinion(
  session: GameSession,
  seat: GameSeat,
  cardId: string,
  excludedSiteInstanceId: string,
): string | undefined {
  const definition = session.state.cards[cardId];
  if (definition?.cardType !== 'minion') return undefined;
  return session.state.players[seat].hand.atlas.find((site) => {
    if (site.instanceId === excludedSiteInstanceId) return false;
    const siteDefinition = session.state.cards[site.cardId];
    if (siteDefinition?.cardType !== 'site') return false;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    return (['air', 'earth', 'fire', 'water'] as const)
      .every((element) => affinity[element] >= definition.thresholds[element]);
  })?.instanceId;
}

function earthOpponentOpening(session: GameSession): Readonly<{
  firstSiteInstanceId: string;
  minionInstanceId: string;
  secondSiteInstanceId: string;
}> | null {
  const player = session.state.players.south;
  for (const first of player.hand.atlas) {
    for (const second of player.hand.atlas) {
      if (first.instanceId === second.instanceId) continue;
      const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
      for (const site of [first, second]) {
        const definition = session.state.cards[site.cardId];
        if (definition?.cardType !== 'site') continue;
        definition.elements.forEach((element) => { affinity[element] += 1; });
      }
      const minion = [...player.hand.spellbook, ...player.spellbook.slice(0, 1)].find((card) => {
        const definition = session.state.cards[card.cardId];
        return definition?.cardType === 'minion'
          && definition.attack >= 2
          && definition.manaCost <= 2
          && (['air', 'earth', 'fire', 'water'] as const)
            .every((element) => affinity[element] >= definition.thresholds[element]);
      });
      if (minion) {
        return {
          firstSiteInstanceId: first.instanceId,
          minionInstanceId: minion.instanceId,
          secondSiteInstanceId: second.instanceId,
        };
      }
    }
  }
  return null;
}

function findOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  north: NonNullable<ReturnType<typeof openingPair>>;
  northChargeInstanceId: string;
  northChargeSiteInstanceId: string;
  northGenesisInstanceId: string;
  northProviderInstanceId: string;
  seed: number;
  session: GameSession;
  south: NonNullable<ReturnType<typeof openingPair>>;
}> {
  const seed = input.config.seed;
  const built = buildManifest(input, seed);
  const session = createGameSession(built.manifest);
  const north = openingPair(session, 'north', input.lethalMinion.stableId);
  const northChargeInstanceId = availableMinionInstance(
    session,
    'north',
    input.chargeMinion.stableId,
    1,
  );
  const northChargeSiteInstanceId = north && openingSiteForMinion(
    session,
    'north',
    input.chargeMinion.stableId,
    north.siteInstanceId,
  );
  const northProviderInstanceId = availableMinionInstance(
    session,
    'north',
    input.providerMinion.stableId,
    2,
  );
  const northGenesisInstanceId = availableMinionInstance(
    session,
    'north',
    input.genesisMinion.stableId,
    3,
  );
  const south = openingPair(session, 'south');
  if (north
    && northChargeInstanceId
    && northChargeSiteInstanceId
    && northGenesisInstanceId
    && northProviderInstanceId
    && northGenesisInstanceId !== northProviderInstanceId
    && south) {
    return {
      ...built,
      north,
      northChargeInstanceId,
      northChargeSiteInstanceId,
      northGenesisInstanceId,
      northProviderInstanceId,
      seed,
      session,
      south,
    };
  }
  throw new Error(`private scenario seed ${seed} no longer produces its supported opening`);
}

function findEarthOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  deathriteInstanceId: string;
  genesisInstanceId: string;
  ghostTownSiteInstanceId: string;
  manaInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  payoffInstanceId: string;
  providerInstanceId: string;
  seed: number;
  session: GameSession;
  southFirstSiteInstanceId: string;
  southMinionInstanceId: string;
  southSecondSiteInstanceId: string;
}> {
  const seed = input.config.earthSeed;
  const built = buildManifest(input, seed, 'earth');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('earth');
  });
  const ghostTownSiteInstanceId = session.state.players.north.hand.atlas
    .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
  const providerInstanceId = availableMinionInstance(
    session,
    'north',
    input.earthProviderMinion.stableId,
    1,
  );
  const manaInstanceId = availableMinionInstance(session, 'north', input.manaMinion.stableId, 2);
  const payoffInstanceId = availableMinionInstance(
    session,
    'north',
    input.cannotDefendMinion.stableId,
    3,
  );
  const genesisInstanceId = availableMinionInstance(session, 'north', input.genesisMinion.stableId, 4);
  const deathriteInstanceId = availableMinionInstance(
    session,
    'north',
    input.deathriteMinion.stableId,
    4,
  );
  const south = earthOpponentOpening(session);
  if (northSites.length >= 2
    && ghostTownSiteInstanceId
    && providerInstanceId
    && manaInstanceId
    && payoffInstanceId
    && genesisInstanceId
    && deathriteInstanceId
    && south) {
    return {
      ...built,
      deathriteInstanceId,
      genesisInstanceId,
      ghostTownSiteInstanceId,
      manaInstanceId,
      northSiteInstanceIds: [
        northSites[0]!.instanceId,
        northSites[1]!.instanceId,
        ghostTownSiteInstanceId,
      ],
      payoffInstanceId,
      providerInstanceId,
      seed,
      session,
      southFirstSiteInstanceId: south.firstSiteInstanceId,
      southMinionInstanceId: south.minionInstanceId,
      southSecondSiteInstanceId: south.secondSiteInstanceId,
    };
  }
  throw new Error(`private Earth scenario seed ${seed} no longer produces its supported opening`);
}

function findEarthDuelOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  mode: 'first-strike' | 'ranged' | 'ward' = 'ranged',
): Readonly<{
  attackerInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southFirstSiteInstanceId: string;
  southSecondSiteInstanceId: string;
  targetInstanceId: string;
}> {
  const seed = mode === 'ward'
    ? input.config.earthWardSeed
    : mode === 'first-strike'
      ? input.config.earthFirstStrikeSeed
      : input.config.earthRangedSeed;
  const built = buildManifest(
    input,
    seed,
    mode === 'ward' ? 'earth-ward' : mode === 'first-strike' ? 'earth-first-strike' : 'earth',
  );
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('earth');
  });
  const attackerCardId = mode === 'first-strike'
    ? input.firstStrikeMinion.stableId
    : input.rangedMinion.stableId;
  const attackerInstanceId = availableMinionInstance(
    session,
    'north',
    attackerCardId,
    2,
  );
  const south = session.state.players.south;
  const requiredTargetId = mode === 'ward'
    ? input.wardMinion.stableId
    : mode === 'first-strike'
      ? input.firstStrikeTargetMinion.stableId
      : undefined;
  for (const first of south.hand.atlas) {
    for (const second of south.hand.atlas) {
      if (first.instanceId === second.instanceId) continue;
      const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
      for (const site of [first, second]) {
        const definition = session.state.cards[site.cardId];
        if (definition?.cardType !== 'site') continue;
        definition.elements.forEach((element) => { affinity[element] += 1; });
      }
      const target = [...south.hand.spellbook, ...south.spellbook.slice(0, 2)].find((card) => {
        if (requiredTargetId && card.cardId !== requiredTargetId) return false;
        const definition = session.state.cards[card.cardId];
        return definition?.cardType === 'minion'
          && definition.defense <= 3
          && definition.manaCost <= 2
          && (['air', 'earth', 'fire', 'water'] as const)
            .every((element) => affinity[element] >= definition.thresholds[element]);
      });
      if (northSites.length >= 3 && attackerInstanceId && target) {
        return {
          ...built,
          attackerInstanceId,
          northSiteInstanceIds: [
            northSites[0]!.instanceId,
            northSites[1]!.instanceId,
            northSites[2]!.instanceId,
          ],
          seed,
          session,
          southFirstSiteInstanceId: first.instanceId,
          southSecondSiteInstanceId: second.instanceId,
          targetInstanceId: target.instanceId,
        };
      }
    }
  }
  throw new Error(`private Earth ${mode} scenario seed ${seed} no longer produces its supported opening`);
}

function findEarthOverpowerOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  elthamTownsfolkInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  overpowerInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-overpower');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    const elthamTownsfolkInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.elthamTownsfolk.stableId)?.instanceId;
    const overpowerInstanceId = availableMinionInstance(
      session,
      'north',
      input.overpower.stableId,
      1,
    );
    if (northSites.length >= 2
      && southSiteInstanceId
      && elthamTownsfolkInstanceId
      && overpowerInstanceId) {
      return {
        ...built,
        elthamTownsfolkInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        overpowerInstanceId,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private temporary power Magic no longer produces its supported opening');
}

function findEarthBurrowingOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  comparisonInstanceId: string;
  featuredInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 64; offset += 1) {
    const seed = input.config.earthRangedSeed + offset;
    const built = buildManifest(input, seed, 'earth-burrowing');
    const session = createGameSession(built.manifest);
    const landSites = (seat: GameSeat) => session.state.players[seat].hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && !definition.elements.includes('water');
    });
    const northLandSites = landSites('north');
    const earthSite = northLandSites.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
    const northSites = earthSite
      ? [earthSite, ...northLandSites.filter(({ instanceId }) => instanceId !== earthSite.instanceId)]
      : [];
    const southSites = landSites('south');
    const featuredInstanceId = availableMinionInstance(
      session,
      'north',
      input.burrowingMinion.stableId,
      2,
    );
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    northSites.slice(0, 3).forEach(({ cardId }) => {
      const definition = session.state.cards[cardId];
      if (definition?.cardType === 'site') {
        definition.elements.forEach((element) => { affinity[element] += 1; });
      }
    });
    const comparisonInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ].find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'minion'
        && definition.burrowing !== true
        && definition.manaCost <= 3
        && (['air', 'earth', 'fire', 'water'] as const)
          .every((element) => affinity[element] >= definition.thresholds[element]);
    })?.instanceId;
    if (northSites.length >= 3
      && southSites.length >= 2
      && featuredInstanceId
      && comparisonInstanceId) {
      return {
        ...built,
        comparisonInstanceId,
        featuredInstanceId,
        northSiteInstanceIds: [
          northSites[0]!.instanceId,
          northSites[1]!.instanceId,
          northSites[2]!.instanceId,
        ],
        seed,
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private Earth Burrowing scenario no longer produces its supported opening');
}

function findEarthEntombedOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  boskTrollInstanceId: string;
  entombedInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.earthSeed + offset;
    const built = buildManifest(input, seed, 'earth-entombed');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site'
        && definition.elements.includes('earth')
        && !definition.genesisGainMana;
    });
    const available = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const entombedInstanceId = available
      .find(({ cardId }) => cardId === input.entombed.stableId)?.instanceId;
    const boskTrollInstanceId = available
      .find(({ cardId }) => cardId === input.firstStrikeTargetMinion.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northSites.length >= 2
      && entombedInstanceId
      && boskTrollInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        boskTrollInstanceId,
        entombedInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        seed,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Earth Entombed scenario no longer produces its supported opening');
}

function findEarthForwardOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  ghostTownSiteInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  phalanxInstanceId: string;
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.earthSeed + offset;
    const built = buildManifest(input, seed, 'earth-forward');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return cardId !== input.ghostTownSite.stableId
        && definition?.cardType === 'site'
        && definition.elements.includes('earth');
    });
    const ghostTownSiteInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const phalanxInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ].find(({ cardId }) => cardId === input.dalceanPhalanx.stableId)?.instanceId;
    const southSites = session.state.players.south.hand.atlas;
    if (northSites.length >= 2
      && ghostTownSiteInstanceId
      && phalanxInstanceId
      && southSites.length >= 2) {
      return {
        ...built,
        ghostTownSiteInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        phalanxInstanceId,
        seed,
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private Earth forward-only scenario no longer produces its supported opening');
}

function findEarthImmobileOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  comparatorInstanceId: string;
  ghostTownSiteInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  pudgeInstanceId: string;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 2048; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-immobile');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return cardId !== input.ghostTownSite.stableId
        && definition?.cardType === 'site'
        && definition.elements.includes('earth');
    });
    const ghostTownSiteInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const pudgeInstanceId = availableMinionInstance(
      session,
      'north',
      input.pudgeButcher.stableId,
      2,
    );
    const comparatorInstanceId = availableMinionInstance(
      session,
      'south',
      input.firstStrikeTargetMinion.stableId,
      2,
    );
    const southSites = session.state.players.south.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
    if (northSites.length >= 2
      && ghostTownSiteInstanceId
      && pudgeInstanceId
      && comparatorInstanceId
      && southSites.length >= 2) {
      return {
        ...built,
        comparatorInstanceId,
        ghostTownSiteInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        pudgeInstanceId,
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private Earth Immobile scenario no longer produces its supported opening');
}

function findEarthDivineHealingOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  attackerInstanceId: string;
  divineHealingInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-divine-healing');
    const session = createGameSession(built.manifest);
    const earthSites = (seat: GameSeat) => session.state.players[seat].hand.atlas
      .filter(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('earth');
      });
    const northSites = earthSites('north');
    const southSites = earthSites('south');
    const divineHealingInstanceId = availableMinionInstance(
      session,
      'north',
      input.divineHealing.stableId,
      3,
    );
    const attackerInstanceId = availableMinionInstance(
      session,
      'south',
      input.firstStrikeTargetMinion.stableId,
      2,
    );
    if (northSites.length >= 3
      && southSites.length >= 2
      && divineHealingInstanceId
      && attackerInstanceId) {
      return {
        ...built,
        attackerInstanceId,
        divineHealingInstanceId,
        northSiteInstanceIds: [
          northSites[0]!.instanceId,
          northSites[1]!.instanceId,
          northSites[2]!.instanceId,
        ],
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private Earth controller-healing Magic scenario no longer produces its supported opening');
}

function findEarthBuryOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  boskTrollInstanceId: string;
  buryInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-bury');
    const session = createGameSession(built.manifest);
    const earthSites = (seat: GameSeat) => session.state.players[seat].hand.atlas
      .filter(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('earth');
      });
    const northSites = earthSites('north');
    const southSites = earthSites('south');
    const buryInstanceId = availableMinionInstance(session, 'north', input.bury.stableId, 2);
    const boskTrollInstanceId = availableMinionInstance(
      session,
      'south',
      input.firstStrikeTargetMinion.stableId,
      2,
    );
    if (northSites.length >= 3
      && southSites.length >= 2
      && buryInstanceId
      && boskTrollInstanceId) {
      return {
        ...built,
        boskTrollInstanceId,
        buryInstanceId,
        northSiteInstanceIds: [
          northSites[0]!.instanceId,
          northSites[1]!.instanceId,
          northSites[2]!.instanceId,
        ],
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private forced-burrow Magic scenario no longer produces its supported opening');
}

function findEarthRescueOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  boskTrollInstanceId: string;
  buryInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  rescueInstanceId: string;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string, string];
}> {
  // ponytail: bounded opening scan reuses Bury without adding private config.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-rescue');
    const session = createGameSession(built.manifest);
    const earthSites = (seat: GameSeat) => session.state.players[seat].hand.atlas
      .filter(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('earth');
      });
    const northSites = earthSites('north');
    const southSites = earthSites('south');
    const buryInstanceId = availableMinionInstance(session, 'north', input.bury.stableId, 2);
    const boskTrollInstanceId = availableMinionInstance(
      session,
      'south',
      input.firstStrikeTargetMinion.stableId,
      2,
    );
    const rescueInstanceId = availableMinionInstance(session, 'south', input.rescue.stableId, 3);
    if (northSites.length >= 3
      && southSites.length >= 3
      && buryInstanceId
      && boskTrollInstanceId
      && rescueInstanceId) {
      return {
        ...built,
        boskTrollInstanceId,
        buryInstanceId,
        northSiteInstanceIds: [
          northSites[0]!.instanceId,
          northSites[1]!.instanceId,
          northSites[2]!.instanceId,
        ],
        rescueInstanceId,
        session,
        southSiteInstanceIds: [
          southSites[0]!.instanceId,
          southSites[1]!.instanceId,
          southSites[2]!.instanceId,
        ],
      };
    }
  }
  throw new Error('private cemetery-to-hand Rescue scenario no longer produces its supported opening');
}

function findEarthShallowGraveOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  session: GameSession;
  shallowGraveInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-shallow-grave');
    const session = createGameSession(built.manifest);
    const shallowGraveInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.shallowGrave.stableId)?.instanceId;
    if (shallowGraveInstanceId && session.state.players.north.spellbook.length >= 2) {
      return { ...built, session, shallowGraveInstanceId };
    }
  }
  throw new Error('private site discard Genesis scenario no longer produces its supported opening');
}

function findEarthSinkholeOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  session: GameSession;
  sinkholeInstanceId: string;
  southSiteInstanceId: string;
  targetSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-sinkhole');
    const session = createGameSession(built.manifest);
    const sinkholeInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.sinkhole.stableId)?.instanceId;
    const targetSiteInstanceId = session.state.players.north.hand.atlas.find(({ cardId }) => {
      const card = input.cards.find(({ stableId }) => stableId === cardId);
      return card?.cardType === 'site'
        && card.rarity === 'ordinary'
        && card.rulesText.trim() === ''
        && card.elements.includes('earth');
    })?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId !== input.sinkhole.stableId)?.instanceId;
    if (sinkholeInstanceId && targetSiteInstanceId && southSiteInstanceId) {
      return {
        ...built,
        session,
        sinkholeInstanceId,
        southSiteInstanceId,
        targetSiteInstanceId,
      };
    }
  }
  throw new Error('private sacrifice-to-destroy site scenario no longer produces its supported opening');
}

function findEarthSecretTunnelOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  caveTrollsInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  secretTunnelInstanceId: string;
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.earthSeed + offset;
    const built = buildManifest(input, seed, 'earth-tunnel');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return cardId !== input.secretTunnel.stableId
        && definition?.cardType === 'site'
        && definition.elements.includes('earth');
    });
    const secretTunnelInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.secretTunnel.stableId)?.instanceId;
    const caveTrollsInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ].find(({ cardId }) => cardId === input.burrowingMinion.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && !definition.elements.includes('water');
    })?.instanceId;
    if (northSites.length >= 2
      && secretTunnelInstanceId
      && caveTrollsInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        caveTrollsInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        secretTunnelInstanceId,
        seed,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Secret Tunnel scenario no longer produces its supported opening');
}

function findAirOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  attackerInstanceId: string;
  manifest: GameManifest;
  movementInstanceId: string;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  const seed = input.config.airSeed;
  const built = buildManifest(input, seed, 'air');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('air');
  });
  const movementInstanceId = availableMinionInstance(
    session,
    'north',
    input.movementMinion.stableId,
    2,
  );
  const south = openingPair(session, 'south');
  const southCardId = south && session.state.players.south.hand.spellbook
    .find(({ instanceId }) => instanceId === south.minionInstanceId)?.cardId;
  const southDefinition = southCardId ? session.state.cards[southCardId] : undefined;
  if (northSites.length >= 3
    && movementInstanceId
    && south
    && southDefinition?.cardType === 'minion'
    && southDefinition.attack <= 2) {
    return {
      ...built,
      attackerInstanceId: south.minionInstanceId,
      movementInstanceId,
      northSiteInstanceIds: [
        northSites[0]!.instanceId,
        northSites[1]!.instanceId,
        northSites[2]!.instanceId,
      ],
      seed,
      session,
      southSiteInstanceId: south.siteInstanceId,
    };
  }
  throw new Error(`private Air scenario seed ${seed} no longer produces its supported opening`);
}

function findAirZapOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceId: string;
  session: GameSession;
  snowLeopardInstanceId: string;
  southSiteInstanceId: string;
  zapInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-zap');
    const session = createGameSession(built.manifest);
    const airSite = (seat: GameSeat): string | undefined =>
      session.state.players[seat].hand.atlas.find(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      })?.instanceId;
    const northSiteInstanceId = airSite('north');
    const southSiteInstanceId = airSite('south');
    const zapInstanceId = availableMinionInstance(session, 'north', input.zap.stableId, 1);
    const snowLeopardInstanceId = availableMinionInstance(
      session,
      'south',
      input.stealthTargetMinion.stableId,
      1,
    );
    if (northSiteInstanceId
      && southSiteInstanceId
      && zapInstanceId
      && snowLeopardInstanceId) {
      return {
        ...built,
        northSiteInstanceId,
        session,
        snowLeopardInstanceId,
        southSiteInstanceId,
        zapInstanceId,
      };
    }
  }
  throw new Error('private Air target-unit damage Magic scenario no longer produces its supported opening');
}

function findAirArcLightningOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  arcLightningInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string];
  session: GameSession;
  snowLeopardInstanceId: string;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-arc-lightning');
    const session = createGameSession(built.manifest);
    const northHandSites = session.state.players.north.hand.atlas;
    const northDrawnSite = session.state.players.north.atlas[0];
    const northSites = northDrawnSite ? [...northHandSites, northDrawnSite] : [];
    const northAirSources = northSites.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    }).length;
    const southFirstSite = session.state.players.south.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    });
    const southSecondSite = session.state.players.south.hand.atlas
      .find(({ instanceId }) => instanceId !== southFirstSite?.instanceId);
    const arcLightningInstanceId = availableMinionInstance(
      session,
      'north',
      input.arcLightning.stableId,
      2,
    );
    const snowLeopardInstanceId = availableMinionInstance(
      session,
      'south',
      input.stealthTargetMinion.stableId,
      1,
    );
    if (northSites.length === 4
      && northAirSources >= 2
      && southFirstSite
      && southSecondSite
      && arcLightningInstanceId
      && snowLeopardInstanceId) {
      return {
        ...built,
        arcLightningInstanceId,
        northSiteInstanceIds: [
          northSites[0]!.instanceId,
          northSites[1]!.instanceId,
          northSites[2]!.instanceId,
          northSites[3]!.instanceId,
        ],
        session,
        snowLeopardInstanceId,
        southSiteInstanceIds: [southFirstSite.instanceId, southSecondSite.instanceId],
      };
    }
  }
  throw new Error('private Air nearby Magic scenario no longer produces its supported opening');
}

function findAirLightningBoltOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  lightningBoltInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  snowLeopardInstanceId: string;
  southSiteInstanceId: string;
}> {
  // ponytail: the bounded scan locks both the opening and the actual seeded random outcome.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-lightning-bolt');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    });
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    const lightningBoltInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.lightningBolt.stableId)?.instanceId;
    const snowLeopardInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.stealthTargetMinion.stableId)?.instanceId;
    if (northSites.length < 2
      || !southSiteInstanceId
      || !lightningBoltInstanceId
      || !snowLeopardInstanceId) continue;

    let probe = keep(session);
    probe = keep(probe);
    const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
      probe = accept(probe, action(probe, predicate));
    };
    take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northSites[0]!.instanceId
      && descriptor.cell === 'C4');
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === snowLeopardInstanceId
      && descriptor.cell === 'C4');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === southSiteInstanceId
      && descriptor.cell === 'C1');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northSites[1]!.instanceId
      && descriptor.cell === 'C3');
    take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === lightningBoltInstanceId
      && descriptor.targetLocation?.cell === 'C4'
      && descriptor.targetLocation.region === 'surface');
    const randomDrawRecorded = probe.transcript.at(-1)?.randomDraws
      .some(({ purpose }) => purpose === 'magic_random_unit_at_location') === true;
    const leopardSelected = probe.state.realm.units
      .every(({ instanceId }) => instanceId !== snowLeopardInstanceId)
      && probe.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === snowLeopardInstanceId);
    if (randomDrawRecorded && leopardSelected) {
      return {
        ...built,
        lightningBoltInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        session,
        snowLeopardInstanceId,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private random location-damage Magic scenario no longer produces its supported opening');
}

function findAirRainOfArrowsOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  northSnowLeopardInstanceId: string;
  rainOfArrowsInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
  southSnowLeopardInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-rain-of-arrows');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    });
    const southSiteInstanceId = session.state.players.south.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    })?.instanceId;
    const northSnowLeopardInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.stealthTargetMinion.stableId)?.instanceId;
    const southSnowLeopardInstanceId = availableMinionInstance(
      session,
      'south',
      input.stealthTargetMinion.stableId,
      1,
    );
    const rainOfArrowsInstanceId = availableMinionInstance(
      session,
      'north',
      input.rainOfArrows.stableId,
      1,
    );
    if (northSites.length >= 2
      && northSnowLeopardInstanceId
      && rainOfArrowsInstanceId
      && southSiteInstanceId
      && southSnowLeopardInstanceId) {
      return {
        ...built,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        northSnowLeopardInstanceId,
        rainOfArrowsInstanceId,
        session,
        southSiteInstanceId,
        southSnowLeopardInstanceId,
      };
    }
  }
  throw new Error('private aboveground-minion damage Magic no longer produces its supported opening');
}

function findAirTeleportOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  snowLeopardInstanceId: string;
  southSiteInstanceId: string;
  teleportInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-teleport');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    });
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    const snowLeopardInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.stealthTargetMinion.stableId)?.instanceId;
    const teleportInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.teleport.stableId)?.instanceId;
    if (northSites.length >= 2
      && southSiteInstanceId
      && snowLeopardInstanceId
      && teleportInstanceId) {
      return {
        ...built,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        session,
        snowLeopardInstanceId,
        southSiteInstanceId,
        teleportInstanceId,
      };
    }
  }
  throw new Error('private ally-to-site Teleport scenario no longer produces its supported opening');
}

function findFireGenesisLifeLossOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  demonInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids adding another private seed field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-genesis-life-loss');
    const session = createGameSession(built.manifest);
    const northFireSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const demonInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ].find(({ cardId }) => cardId === input.lesserBloodDemon.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northFireSites.length >= 2 && demonInstanceId && southSiteInstanceId) {
      return {
        ...built,
        demonInstanceId,
        northSiteInstanceIds: [northFireSites[0]!.instanceId, northFireSites[1]!.instanceId],
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Genesis life-loss scenario no longer produces its supported opening');
}

function findFireIgnitedOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  ignitedInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids adding another private seed field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-ignited');
    const session = createGameSession(built.manifest);
    const northFireSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const ignitedInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ].find(({ cardId }) => cardId === input.ignited.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northFireSites.length >= 2 && ignitedInstanceId && southSiteInstanceId) {
      return {
        ...built,
        ignitedInstanceId,
        northSiteInstanceIds: [northFireSites[0]!.instanceId, northFireSites[1]!.instanceId],
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private printed-Charge end-turn-death scenario lacks its supported opening');
}

function findFireChargeOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  chargeInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  raalInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: a bounded opening scan avoids adding another private seed field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-charge');
    const session = createGameSession(built.manifest);
    const northFireSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    const accessibleSpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const chargeInstanceId = accessibleSpells
      .find(({ cardId }) => cardId === input.chargeMagic.stableId)?.instanceId;
    const raalInstanceId = accessibleSpells
      .find(({ cardId }) => cardId === input.raalDromedary.stableId)?.instanceId;
    if (northFireSites.length >= 2
      && southSiteInstanceId
      && chargeInstanceId
      && raalInstanceId) {
      return {
        ...built,
        chargeInstanceId,
        northSiteInstanceIds: [
          northFireSites[0]!.instanceId,
          northFireSites[1]!.instanceId,
        ],
        raalInstanceId,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private temporary Charge Magic scenario lacks its supported opening');
}

function findFireMinorExplosionOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  firstRaalInstanceId: string;
  manifest: GameManifest;
  minorExplosionInstanceId: string;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  secondRaalInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: a bounded opening scan avoids adding another private seed field.
  for (let offset = 1; offset <= 2048; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-minor-explosion');
    const session = createGameSession(built.manifest);
    const northHandSites = session.state.players.north.hand.atlas;
    const northFireSites = northHandSites.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const thirdNorthSite = northHandSites.find(({ instanceId }) =>
      instanceId !== northFireSites[0]?.instanceId
        && instanceId !== northFireSites[1]?.instanceId);
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    const initialRaals = session.state.players.north.hand.spellbook
      .filter(({ cardId }) => cardId === input.raalDromedary.stableId);
    const accessibleSpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const accessibleRaals = accessibleSpells
      .filter(({ cardId }) => cardId === input.raalDromedary.stableId);
    const firstRaalInstanceId = initialRaals[0]?.instanceId;
    const secondRaalInstanceId = accessibleRaals
      .find(({ instanceId }) => instanceId !== firstRaalInstanceId)?.instanceId;
    const minorExplosionInstanceId = accessibleSpells
      .find(({ cardId }) => cardId === input.minorExplosion.stableId)?.instanceId;
    if (northFireSites.length >= 2
      && thirdNorthSite
      && southSiteInstanceId
      && firstRaalInstanceId
      && secondRaalInstanceId
      && minorExplosionInstanceId) {
      return {
        ...built,
        firstRaalInstanceId,
        minorExplosionInstanceId,
        northSiteInstanceIds: [
          northFireSites[0]!.instanceId,
          northFireSites[1]!.instanceId,
          thirdNorthSite.instanceId,
        ],
        secondRaalInstanceId,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private location-wide damage Magic scenario lacks its supported opening');
}

function findAirborneOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  mode: 'airborne' | 'movement-two' = 'airborne',
): Readonly<{
  airborneInstanceId: string;
  groundInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string, string];
}> {
  const movementTwo = mode === 'movement-two';
  const seed = movementTwo ? input.config.movementTwoSeed : input.config.airborneSeed;
  const built = buildManifest(input, seed, mode);
  const session = createGameSession(built.manifest);
  const sites = (seat: GameSeat) => session.state.players[seat].hand.atlas;
  const northSites = sites('north');
    const southSites = sites('south');
    const northSiteInstanceIds = northSites.map(({ instanceId }) => instanceId);
    const southSiteInstanceIds = southSites.map(({ instanceId }) => instanceId);
    const airborneInstanceId = availableMinionInstance(
      session,
      'north',
      movementTwo ? input.movementTwoMinion.stableId : input.airborneMinion.stableId,
      2,
    );
    const groundInstanceId = availableMinionInstance(
      session,
      'south',
      input.airborneTargetMinion.stableId,
      3,
    );
    if (northSiteInstanceIds.length === 3
      && southSiteInstanceIds.length === 3
      && northSites.some(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      })
      && (!movementTwo || northSites.filter(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      }).length >= 2)
      && southSites.some(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      })
      && airborneInstanceId
      && groundInstanceId) {
      return {
        ...built,
        airborneInstanceId,
        groundInstanceId,
        northSiteInstanceIds: northSiteInstanceIds as [string, string, string],
        seed,
        session,
        southSiteInstanceIds: southSiteInstanceIds as [string, string, string],
      };
  }
  throw new Error(`private ${movementTwo ? 'Movement +2' : 'Airborne'} scenario seed ${seed} no longer produces its supported opening`);
}

function findAirVoidwalkOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  comparisonInstanceId: string;
  featuredInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
  restrictedInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.airborneSeed + offset;
    const built = buildManifest(input, seed, 'air-voidwalk');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    });
    const southSites = session.state.players.south.hand.atlas;
    const available = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const featuredInstanceId = available
      .find(({ cardId }) => cardId === input.voidwalkMinion.stableId)?.instanceId;
    const restrictedInstanceId = available
      .find(({ cardId }) => cardId === input.forsaken.stableId)?.instanceId;
    const comparisonInstanceId = available.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'minion'
        && definition.voidwalk !== true
        && definition.manaCost <= 2
        && definition.thresholds.air <= 2
        && definition.thresholds.earth === 0
        && definition.thresholds.fire === 0
        && definition.thresholds.water === 0;
    })?.instanceId;
    if (northSites.length >= 2
      && southSites.length >= 2
      && featuredInstanceId
      && restrictedInstanceId
      && comparisonInstanceId) {
      return {
        ...built,
        comparisonInstanceId,
        featuredInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        seed,
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
        restrictedInstanceId,
      };
    }
  }
  throw new Error('private Air Voidwalk scenario no longer produces its supported opening');
}

function findAirGenesisSpellOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  featuredInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 64; offset += 1) {
    const seed = input.config.airSeed + offset;
    const built = buildManifest(input, seed, 'air-genesis-spell');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas;
    const featuredInstanceId = availableMinionInstance(
      session,
      'north',
      input.genesisSpellMinion.stableId,
      2,
    );
    if (northSites.length === 3
      && northSites.some(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('air');
      })
      && featuredInstanceId
      && session.state.players.south.hand.atlas[0]) {
      return {
        ...built,
        featuredInstanceId,
        northSiteInstanceIds: northSites.map(({ instanceId }) => instanceId) as [string, string, string],
        seed,
        session,
        southSiteInstanceId: session.state.players.south.hand.atlas[0].instanceId,
      };
    }
  }
  throw new Error('private Air Genesis spell-draw scenario no longer produces its supported opening');
}

function findAirSpellcasterFreezeOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  apprenticeWizardInstanceId: string;
  freezeInstanceId: string;
  ghostTownInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northAirSiteInstanceId: string;
  northWaterSiteInstanceId: string;
  seravaInstanceId: string;
  session: GameSession;
  southWaterSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private config field.
  for (let offset = 1; offset <= 2_048; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-spellcaster-freeze');
    const session = createGameSession(built.manifest);
    const northAirSiteInstanceId = session.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    })?.instanceId;
    const northWaterSiteInstanceId = session.state.players.north.hand.atlas.find(({
      cardId,
      instanceId,
    }) => {
      const definition = session.state.cards[cardId];
      return instanceId !== northAirSiteInstanceId
        && definition?.cardType === 'site'
        && definition.elements.includes('water');
    })?.instanceId;
    const ghostTownInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const southWaterSiteInstanceId = session.state.players.south.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    })?.instanceId;
    const apprenticeWizardInstanceId = availableMinionInstance(
      session,
      'north',
      input.genesisSpellMinion.stableId,
      2,
    );
    const freezeInstanceId = availableMinionInstance(session, 'north', input.freeze.stableId, 3);
    const seravaInstanceId = availableMinionInstance(
      session,
      'south',
      input.seravaTownsfolk.stableId,
      1,
    );
    if (apprenticeWizardInstanceId
      && freezeInstanceId
      && ghostTownInstanceId
      && northAirSiteInstanceId
      && northWaterSiteInstanceId
      && seravaInstanceId
      && southWaterSiteInstanceId) {
      return {
        ...built,
        apprenticeWizardInstanceId,
        freezeInstanceId,
        ghostTownInstanceId,
        northAirSiteInstanceId,
        northWaterSiteInstanceId,
        seravaInstanceId,
        session,
        southWaterSiteInstanceId,
      };
    }
  }
  throw new Error('private Spellcaster Freeze scenario no longer produces its supported opening');
}

function findAirLeylineOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  hengeInstanceIds: readonly [string, string];
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.airSeed + offset;
    const built = buildManifest(input, seed, 'air-leyline');
    const session = createGameSession(built.manifest);
    const henges = session.state.players.north.hand.atlas
      .filter(({ cardId }) => cardId === input.leylineHenge.stableId);
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (henges.length >= 2 && southSiteInstanceId) {
      return {
        ...built,
        hengeInstanceIds: [henges[0]!.instanceId, henges[1]!.instanceId],
        seed,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Air Leyline Henge scenario no longer produces its supported opening');
}

function findStealthOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  groundInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string, string];
  stealthInstanceId: string;
}> {
  const seed = input.config.stealthSeed;
  const built = buildManifest(input, seed, 'stealth');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas;
  const southSites = session.state.players.south.hand.atlas;
  const airAffinity = (sites: typeof northSites): number => sites.reduce((total, { cardId }) => {
    const definition = session.state.cards[cardId];
    return total + (definition?.cardType === 'site' && definition.elements.includes('air') ? 1 : 0);
  }, 0);
  const stealthInstanceId = availableMinionInstance(
    session,
    'north',
    input.stealthMinion.stableId,
    2,
  );
  const groundInstanceId = availableMinionInstance(
    session,
    'south',
    input.stealthTargetMinion.stableId,
    2,
  );
  if (northSites.length === 3
    && southSites.length === 3
    && airAffinity(northSites) >= 2
    && airAffinity(southSites.slice(0, 2)) >= 1
    && stealthInstanceId
    && groundInstanceId) {
    return {
      ...built,
      groundInstanceId,
      northSiteInstanceIds: northSites.map(({ instanceId }) => instanceId) as [string, string, string],
      seed,
      session,
      southSiteInstanceIds: southSites.map(({ instanceId }) => instanceId) as [string, string, string],
      stealthInstanceId,
    };
  }
  throw new Error(`private Stealth scenario seed ${seed} no longer produces its supported opening`);
}

function findAirSummoningOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string, string];
  ordinaryInstanceId: string;
  roamingInstanceId: string;
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  const seed = input.config.roamingSeed;
  const built = buildManifest(input, seed, 'air');
  const session = createGameSession(built.manifest);
  const north = session.state.players.north;
  const sites = [...north.hand.atlas, ...north.atlas.slice(0, 2)];
  const spells = [...north.hand.spellbook, ...north.spellbook.slice(0, 2)];
  const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
  sites.forEach(({ cardId }) => {
    const definition = session.state.cards[cardId];
    if (definition?.cardType === 'site') {
      definition.elements.forEach((element) => { affinity[element] += 1; });
    }
  });
  const roamingInstanceId = spells
    .find(({ cardId }) => cardId === input.roamingMinion.stableId)?.instanceId;
  const ordinaryInstanceId = spells.find(({ cardId }) => {
    const definition = session.state.cards[cardId];
    return cardId !== input.movementMinion.stableId
      && cardId !== input.roamingMinion.stableId
      && definition?.cardType === 'minion'
      && definition.manaCost <= 5
      && (['air', 'earth', 'fire', 'water'] as const)
        .every((element) => affinity[element] >= definition.thresholds[element]);
  })?.instanceId;
  const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
  if (sites.length === 5
    && affinity.air >= 1
    && roamingInstanceId
    && ordinaryInstanceId
    && southSiteInstanceId) {
    return {
      ...built,
      northSiteInstanceIds: sites.map(({ instanceId }) => instanceId) as [string, string, string, string, string],
      ordinaryInstanceId,
      roamingInstanceId,
      seed,
      session,
      southSiteInstanceId,
    };
  }
  throw new Error(`private Air summoning seed ${input.config.roamingSeed} no longer produces its supported opening`);
}

function findFireOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  attackerInstanceId: string;
  lionInstanceId: string;
  lumberingInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  const seed = input.config.fireSeed;
  const built = buildManifest(input, seed, 'fire');
  const session = createGameSession(built.manifest);
  const northSites = [
    ...session.state.players.north.hand.atlas,
    ...session.state.players.north.atlas.slice(0, 1),
  ];
  const fireAffinity = northSites.reduce((total, { cardId }) => {
    const definition = session.state.cards[cardId];
    return total + (definition?.cardType === 'site' && definition.elements.includes('fire') ? 1 : 0);
  }, 0);
  const lumberingInstanceId = availableMinionInstance(
    session,
    'north',
    input.lumberingMinion.stableId,
    3,
  );
  const lionInstanceId = availableMinionInstance(
    session,
    'north',
    input.monstrousLion.stableId,
    2,
  );
  for (const first of session.state.players.south.hand.atlas) {
    const siteDefinition = session.state.cards[first.cardId];
    if (siteDefinition?.cardType !== 'site') continue;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    const attacker = session.state.players.south.hand.spellbook.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'minion'
        && definition.manaCost <= 1
        && (['air', 'earth', 'fire', 'water'] as const)
          .every((element) => affinity[element] >= definition.thresholds[element]);
    });
    const second = session.state.players.south.hand.atlas
      .find(({ instanceId }) => instanceId !== first.instanceId);
    if (northSites.length === 4
      && fireAffinity >= 2
      && lionInstanceId
      && lumberingInstanceId
      && attacker
      && second) {
      return {
        ...built,
        attackerInstanceId: attacker.instanceId,
        lionInstanceId,
        lumberingInstanceId,
        northSiteInstanceIds: northSites.map(({ instanceId }) => instanceId) as [string, string, string, string],
        seed,
        session,
        southSiteInstanceIds: [first.instanceId, second.instanceId],
      };
    }
  }
  throw new Error(`private Fire scenario seed ${input.config.fireSeed} no longer produces its supported opening`);
}

function findWaterOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  scenario: 'water' | 'water-sideways' | 'water-stealth' | 'water-submerge' = 'water',
): Readonly<{
  attackerInstanceId: string;
  comparisonInstanceId?: string;
  featuredInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string?];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  const endTurnStealth = scenario === 'water-stealth';
  const sideways = scenario === 'water-sideways';
  const submerge = scenario === 'water-submerge';
  const seed = endTurnStealth
    ? input.config.slyFoxSeed
    : sideways || submerge
      ? input.config.sedgeCrabsSeed + (submerge ? 3 : 0)
      : input.config.waterSeed;
  const built = buildManifest(input, seed, scenario);
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas.filter((site) => {
    const definition = session.state.cards[site.cardId];
    return definition?.cardType === 'site' && definition.elements.includes('water');
  });
  const featuredInstanceId = availableMinionInstance(
    session,
    'north',
    endTurnStealth
      ? input.slyFox.stableId
      : sideways
        ? input.sedgeCrabs.stableId
        : submerge ? input.submergeMinion.stableId : input.healingMinion.stableId,
    submerge ? 2 : 1,
  );
  const comparisonInstanceId = submerge
    ? [...session.state.players.north.hand.spellbook, ...session.state.players.north.spellbook.slice(0, 2)]
      .find(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'minion'
          && definition.submerge !== true
          && definition.manaCost <= 3
          && definition.thresholds.air === 0
          && definition.thresholds.earth === 0
          && definition.thresholds.fire === 0
          && definition.thresholds.water <= 3;
      })?.instanceId
    : undefined;
  const thirdNorthSite = submerge
    ? session.state.players.north.hand.atlas.find(({ instanceId }) =>
      instanceId !== northSites[0]?.instanceId && instanceId !== northSites[1]?.instanceId)
    : undefined;
  for (const first of session.state.players.south.hand.atlas) {
    const siteDefinition = session.state.cards[first.cardId];
    if (siteDefinition?.cardType !== 'site') continue;
    const affinity = { air: 0, earth: 0, fire: 0, water: 0 };
    siteDefinition.elements.forEach((element) => { affinity[element] += 1; });
    const attacker = session.state.players.south.hand.spellbook.find((card) => {
      const definition = session.state.cards[card.cardId];
      return definition?.cardType === 'minion'
        && definition.manaCost <= 1
        && definition.attack >= 2
        && definition.defense <= 2
        && (['air', 'earth', 'fire', 'water'] as const)
          .every((element) => affinity[element] >= definition.thresholds[element]);
    });
    const second = session.state.players.south.hand.atlas
      .find(({ instanceId }) => instanceId !== first.instanceId);
    if (northSites.length >= (sideways ? 3 : 2)
      && featuredInstanceId
      && (!submerge || comparisonInstanceId)
      && (!submerge || thirdNorthSite)
      && attacker
      && second) {
      const northSiteInstanceIds: [string, string, string?] = sideways
        ? [northSites[0]!.instanceId, northSites[1]!.instanceId, northSites[2]!.instanceId]
        : submerge
          ? [northSites[0]!.instanceId, northSites[1]!.instanceId, thirdNorthSite!.instanceId]
        : [northSites[0]!.instanceId, northSites[1]!.instanceId];
      return {
        ...built,
        attackerInstanceId: attacker.instanceId,
        ...(comparisonInstanceId ? { comparisonInstanceId } : {}),
        featuredInstanceId,
        northSiteInstanceIds,
        seed,
        session,
        southSiteInstanceIds: [first.instanceId, second.instanceId],
      };
    }
  }
  const scenarioName = endTurnStealth
    ? 'end-turn Stealth'
    : sideways ? 'sideways movement' : submerge ? 'Submerge' : 'healing';
  throw new Error(`private Water ${scenarioName} scenario no longer produces its supported opening`);
}

function findWaterDrownOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  drownInstanceId: string;
  ghostTownInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  seravaInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
  waterSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.waterSeed + offset, 'water-drown');
    const session = createGameSession(built.manifest);
    const waterSiteInstanceId = session.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    })?.instanceId;
    const ghostTownInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const drownInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.drown.stableId)?.instanceId;
    const seravaInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.seravaTownsfolk.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId !== input.ghostTownSite.stableId)?.instanceId;
    if (waterSiteInstanceId
      && ghostTownInstanceId
      && drownInstanceId
      && seravaInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        drownInstanceId,
        ghostTownInstanceId,
        seravaInstanceId,
        session,
        southSiteInstanceId,
        waterSiteInstanceId,
      };
    }
  }
  throw new Error('private forced-submerge Magic scenario no longer produces its supported opening');
}

function findWaterLureOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  lureInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSeravaInstanceId: string;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  southSeravaInstanceId: string;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded opening scan avoids adding another private seed field.
  for (let offset = 1; offset <= 2048; offset += 1) {
    const built = buildManifest(input, input.config.waterSeed + offset, 'water-lure');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const southSites = session.state.players.south.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const northAccessible = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ];
    const southAccessible = [
      ...session.state.players.south.hand.spellbook,
      ...session.state.players.south.spellbook.slice(0, 2),
    ];
    const lureInstanceId = northAccessible
      .find(({ cardId }) => cardId === input.lure.stableId)?.instanceId;
    const northSeravaInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ]
      .find(({ cardId }) => cardId === input.seravaTownsfolk.stableId)?.instanceId;
    const southSeravaInstanceId = southAccessible
      .find(({ cardId }) => cardId === input.seravaTownsfolk.stableId)?.instanceId;
    if (northSites.length >= 2
      && southSites.length >= 2
      && lureInstanceId
      && northSeravaInstanceId
      && southSeravaInstanceId) {
      return {
        ...built,
        lureInstanceId,
        northSeravaInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        session,
        southSeravaInstanceId,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private non-target Lure scenario no longer produces its supported opening');
}

function findWaterPirateShipOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  ghostTownInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northWaterSiteInstanceIds: readonly [string, string];
  pirateShipInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan keeps this private proof seed-free.
  for (let offset = 1; offset <= 2048; offset += 1) {
    const built = buildManifest(input, input.config.waterSeed + offset, 'water-pirate-ship');
    const session = createGameSession(built.manifest);
    const northWaterSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const ghostTownInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const pirateShipInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ].find(({ cardId }) => cardId === input.pirateShip.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northWaterSites.length >= 2
      && ghostTownInstanceId
      && pirateShipInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        ghostTownInstanceId,
        northWaterSiteInstanceIds: [
          northWaterSites[0]!.instanceId,
          northWaterSites[1]!.instanceId,
        ],
        pirateShipInstanceId,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Waterbound scenario no longer produces its supported opening');
}

function findWaterFreezeOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  freezeInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seravaInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.waterSeed + offset, 'water-freeze');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    const freezeInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.freeze.stableId)?.instanceId;
    const seravaInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.seravaTownsfolk.stableId)?.instanceId;
    if (northSites.length >= 2 && southSiteInstanceId && freezeInstanceId && seravaInstanceId) {
      return {
        ...built,
        freezeInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        seravaInstanceId,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private timed-disable Magic scenario no longer produces its supported opening');
}

function findWaterEdgeConnectionOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  featuredInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 64; offset += 1) {
    const seed = input.config.waterSeed + offset;
    const built = buildManifest(input, seed, 'water-edge-connection');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const featuredInstanceId = availableMinionInstance(
      session,
      'north',
      input.polarBears.stableId,
      1,
    );
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northSites.length >= 2 && featuredInstanceId && southSiteInstanceId) {
      return {
        ...built,
        featuredInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        seed,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Water top/bottom connection scenario no longer produces its supported opening');
}

function keep(session: GameSession): GameSession {
  return accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
}

function findWaterDrownedOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  drownedInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seed: number;
  session: GameSession;
  slyFoxInstanceId: string;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field; persist one only if this becomes slow.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.waterSeed + offset;
    const built = buildManifest(input, seed, 'water-drowned');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const available = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const drownedInstanceId = available
      .find(({ cardId }) => cardId === input.drowned.stableId)?.instanceId;
    const slyFoxInstanceId = available
      .find(({ cardId }) => cardId === input.slyFox.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northSites.length >= 2
      && drownedInstanceId
      && slyFoxInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        drownedInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        seed,
        session,
        slyFoxInstanceId,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Water Drowned scenario no longer produces its supported opening');
}

function findWaterLugbogOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  lugbogInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  session: GameSession;
  slyFoxInstanceId: string;
  southLandSiteInstanceId: string;
  southSecondDrawZone: 'atlas' | 'spellbook';
  southWaterSiteInstanceId: string;
}> {
  // ponytail: bounded scan keeps the ignored config stable for a small teaching scenario.
  for (let offset = 1; offset <= 128; offset += 1) {
    const built = buildManifest(input, input.config.slyFoxSeed + offset, 'water-lugbog');
    const session = createGameSession(built.manifest);
    const isWaterSite = (cardId: string): boolean => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    };
    const northWaterSites = session.state.players.north.hand.atlas.filter(({ cardId }) =>
      isWaterSite(cardId));
    const northThirdSite = session.state.players.north.hand.atlas.find(({ instanceId }) =>
      !northWaterSites.slice(0, 2).some((site) => site.instanceId === instanceId));
    const southLandSite = session.state.players.south.hand.atlas.find(({ cardId }) =>
      !isWaterSite(cardId));
    const southHandWaterSite = session.state.players.south.hand.atlas.find(({ cardId }) =>
      isWaterSite(cardId));
    const southDrawnWaterSite = !southHandWaterSite
      && session.state.players.south.atlas[0]
      && isWaterSite(session.state.players.south.atlas[0].cardId)
      ? session.state.players.south.atlas[0]
      : undefined;
    const southWaterSite = southHandWaterSite ?? southDrawnWaterSite;
    const available = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ];
    const lugbogInstanceId = available
      .find(({ cardId }) => cardId === input.lugbogCat.stableId)?.instanceId;
    const slyFoxInstanceId = available
      .find(({ cardId }) => cardId === input.slyFox.stableId)?.instanceId;
    if (northWaterSites.length >= 2
      && northThirdSite
      && southLandSite
      && southWaterSite
      && lugbogInstanceId
      && slyFoxInstanceId) {
      return {
        ...built,
        lugbogInstanceId,
        northSiteInstanceIds: [
          northWaterSites[0]!.instanceId,
          northWaterSites[1]!.instanceId,
          northThirdSite.instanceId,
        ],
        session,
        slyFoxInstanceId,
        southLandSiteInstanceId: southLandSite.instanceId,
        southSecondDrawZone: southHandWaterSite ? 'spellbook' : 'atlas',
        southWaterSiteInstanceId: southWaterSite.instanceId,
      };
    }
  }
  throw new Error('private Lugbog Cat scenario no longer produces its supported opening');
}

function deckList(deck: GameDeckSpec, names: ReadonlyMap<string, string>): DeckList {
  const summarize = (cards: readonly string[]): readonly Readonly<{ copies: number; name: string }>[] =>
    [...new Set(cards)].map((cardId) => ({
      copies: cards.filter((value) => value === cardId).length,
      name: names.get(cardId) ?? cardId,
    }));
  return {
    atlas: summarize(deck.atlas),
    avatar: names.get(deck.avatar) ?? deck.avatar,
    spellbook: summarize(deck.spellbook),
  };
}

function runEarthOverpower(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthOverpower'] {
  const opening = findEarthOverpowerOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.elthamTownsfolkInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const before = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  const observedBefore = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  if (!before || !observedBefore) throw new Error('private Overpower setup lacks Eltham Townsfolk');
  const allyActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.overpowerInstanceId);
  const selected = allyActions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.ally?.kind === 'minion'
    && descriptor.ally.instanceId === opening.elthamTownsfolkInstanceId
    && descriptor.ally.seat === 'north');
  if (!selected) throw new Error('private Overpower Townsfolk ally choice is unavailable');
  const avatarInstanceId = session.state.players.north.avatar.card.instanceId;
  const chosenAllyIds = allyActions.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.ally?.seat === 'north' ? [descriptor.ally.instanceId] : []).sort();
  const exactOwnAllyChoices = allyActions.length === 2
    && chosenAllyIds.join(',')
      === [avatarInstanceId, opening.elthamTownsfolkInstanceId].sort().join(',')
    && allyActions.every(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.ally !== undefined
      && descriptor.target === undefined
      && descriptor.targetLocation === undefined
      && descriptor.targetSiteInstanceId === undefined
      && descriptor.cemeteryMinionInstanceId === undefined
      && descriptor.temptedEnemy === undefined
      && descriptor.temptedDestination === undefined);
  const manaBefore = session.state.players.north.mana;
  session = accept(session, selected);
  const manaAfterCast = session.state.players.north.mana;

  const afterGrant = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  const observedAfterGrant = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  if (!afterGrant || !observedAfterGrant) throw new Error('private Overpower removed its ally');
  const castReceipt = session.transcript.at(-1);
  const castEvents = castReceipt?.events ?? [];
  const castPayload = castEvents[0] && isJsonRecord(castEvents[0].payload)
    ? castEvents[0].payload
    : undefined;
  const grantedPayload = castEvents[1] && isJsonRecord(castEvents[1].payload)
    ? castEvents[1].payload
    : undefined;
  const resolvedPayload = castEvents[2] && isJsonRecord(castEvents[2].payload)
    ? castEvents[2].payload
    : undefined;
  const currentPowerIncreasedByTwo = observedAfterGrant.attack === observedBefore.attack + 2
    && observedAfterGrant.defense === observedBefore.defense + 2;
  const unitStatePreservedOnGrant = afterGrant.temporaryPowerSources?.length === 1
    && afterGrant.temporaryPowerSources[0] === opening.overpowerInstanceId
    && afterGrant.cardId === before.cardId
    && afterGrant.controller === before.controller
    && afterGrant.damage === before.damage
    && afterGrant.location === before.location
    && afterGrant.owner === before.owner
    && afterGrant.region === before.region
    && afterGrant.stealthed === before.stealthed
    && afterGrant.summoningSickness === before.summoningSickness
    && afterGrant.tapped === before.tapped
    && afterGrant.warded === before.warded;

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  const afterExpiry = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  const observedAfterExpiry = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  const expiryReceipt = session.transcript.at(-1);
  const expiryEvents = expiryReceipt?.events ?? [];
  const expiryIndex = expiryEvents.findIndex(({ payload, type }) => type === 'power-expired'
    && isJsonRecord(payload)
    && payload.amount === 2
    && payload.instanceId === opening.elthamTownsfolkInstanceId
    && payload.seat === 'north'
    && payload.sourceInstanceId === opening.overpowerInstanceId);
  const turnEndedIndex = expiryEvents.findIndex(({ payload, type }) => type === 'turn-ended'
    && isJsonRecord(payload)
    && payload.seat === 'north');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: castEvents.map(({ type }) => type).join(',')
      === 'magic-cast,power-granted,magic-resolved'
      && castPayload?.allyInstanceId === opening.elthamTownsfolkInstanceId
      && castPayload.allySeat === 'north'
      && castPayload.instanceId === opening.overpowerInstanceId
      && castPayload.manaPaid === 1
      && castPayload.seat === 'north'
      && grantedPayload?.amount === 2
      && grantedPayload.instanceId === opening.elthamTownsfolkInstanceId
      && grantedPayload.seat === 'north'
      && grantedPayload.sourceInstanceId === opening.overpowerInstanceId
      && resolvedPayload?.instanceId === opening.overpowerInstanceId
      && expiryIndex >= 0
      && expiryIndex < turnEndedIndex,
    currentPowerIncreasedByTwo,
    deck: deckList(opening.manifest.decks.north, opening.names),
    elthamTownsfolk: input.elthamTownsfolk.name,
    exactOwnAllyChoices,
    expiredBeforeTurnEnded: expiryIndex >= 0 && expiryIndex < turnEndedIndex,
    manaPaid: manaBefore - manaAfterCast,
    noRandomDraws: castReceipt?.randomDraws.length === 0
      && expiryReceipt?.randomDraws.length === 0,
    overpower: input.overpower.name,
    printedPowerRestored: afterExpiry !== undefined
      && observedAfterExpiry !== undefined
      && afterExpiry.temporaryPowerSources === undefined
      && observedAfterExpiry.attack === input.elthamTownsfolk.attack
      && observedAfterExpiry.defense === input.elthamTownsfolk.defense
      && afterExpiry.damage === 0,
    replayVerified: verifyGameReplay(session),
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.overpowerInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.overpowerInstanceId),
    unitStatePreservedOnGrant,
  });
}

function runEarthBurrowing(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthBurrowing'] {
  const opening = findEarthBurrowingOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');

  const summons = legalGameActions(session.state, 'north');
  const matches = (cardInstanceId: string, region: 'surface' | 'underground'): boolean =>
    summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === cardInstanceId
      && descriptor.cell === 'C3'
      && (descriptor.region ?? 'surface') === region);
  const surfaceSummonAvailable = matches(opening.featuredInstanceId, 'surface');
  const undergroundSummonAvailable = matches(opening.featuredInstanceId, 'underground');
  const nonBurrowingSurfaceAvailable = matches(opening.comparisonInstanceId, 'surface');
  const nonBurrowingUndergroundUnavailable = !matches(opening.comparisonInstanceId, 'underground');
  const summonSite = session.state.realm.sites.C3;
  const summonSiteDefinition = summonSite && !('rubble' in summonSite)
    ? session.state.cards[summonSite.cardId]
    : undefined;
  const attackSite = session.state.realm.sites.C2;
  const attackSiteDefinition = attackSite && !('rubble' in attackSite)
    ? session.state.cards[attackSite.cardId]
    : undefined;
  const targetIsLandSite = summonSiteDefinition?.cardType === 'site'
    && !summonSiteDefinition.elements.includes('water')
    && attackSiteDefinition?.cardType === 'site'
    && !attackSiteDefinition.elements.includes('water');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.featuredInstanceId
    && descriptor.cell === 'C3'
    && descriptor.region === 'underground');

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.featuredInstanceId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C3/underground,C2/underground');
  const movedUnderground = session.state.realm.units.some(({ instanceId, location, region }) =>
    instanceId === opening.featuredInstanceId && location === 'C2' && region === 'underground');
  const siteTargetUnavailableUnderground = !legalGameActions(session.state, 'north')
    .some(({ descriptor }) => descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.featuredInstanceId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C2/underground,C2/surface');
  const surfaced = session.state.realm.units.some(({ instanceId, location, region }) =>
    instanceId === opening.featuredInstanceId && location === 'C2' && region === 'surface');
  const siteTargetAvailableAfterSurfacing = legalGameActions(session.state, 'north')
    .some(({ descriptor }) => descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    burrowingMinion: opening.names.get(input.burrowingMinion.stableId) ?? input.burrowingMinion.stableId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    movedUnderground,
    nonBurrowingSurfaceAvailable,
    nonBurrowingUndergroundUnavailable,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    siteTargetAvailableAfterSurfacing,
    siteTargetUnavailableUnderground,
    surfaceSummonAvailable,
    surfaced,
    targetIsLandSite,
    undergroundSummonAvailable,
  });
}

function runEarthEntombed(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthEntombed'] {
  const opening = findEarthEntombedOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const summons = legalGameActions(session.state, 'north');
  const matches = (cardInstanceId: string, region: 'surface' | 'underground'): boolean =>
    summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === cardInstanceId
      && descriptor.cell === 'C3'
      && (descriptor.region ?? 'surface') === region);
  const entombedSurfaceUnavailable = !matches(opening.entombedInstanceId, 'surface');
  const entombedUndergroundAvailable = matches(opening.entombedInstanceId, 'underground');
  const boskTrollSurfaceAvailable = matches(opening.boskTrollInstanceId, 'surface');
  const boskTrollUndergroundUnavailable = !matches(opening.boskTrollInstanceId, 'underground');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.entombedInstanceId
    && descriptor.cell === 'C3'
    && descriptor.region === 'underground');
  const summonedUnderground = session.state.realm.units.some(({ instanceId, location, region }) =>
    instanceId === opening.entombedInstanceId && location === 'C3' && region === 'underground');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    boskTroll:
      opening.names.get(input.firstStrikeTargetMinion.stableId)
        ?? input.firstStrikeTargetMinion.stableId,
    boskTrollSurfaceAvailable,
    boskTrollUndergroundUnavailable,
    deck: deckList(opening.manifest.decks.north, opening.names),
    entombed: opening.names.get(input.entombed.stableId) ?? input.entombed.stableId,
    entombedSurfaceUnavailable,
    entombedUndergroundAvailable,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    summonedUnderground,
  });
}

function runEarthForwardMovement(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthForwardMovement'] {
  const opening = findEarthForwardOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownSiteInstanceId
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.phalanxInstanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const moves = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.phalanxInstanceId);
  const hasPath = (cells: string): boolean => moves.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.path.map(({ cell }) => cell).join(',') === cells);
  const forwardPathAvailable = session.state.realm.sites.C2 !== undefined
    && hasPath('C3,C2');
  const backwardPathUnavailable = session.state.realm.sites.C4 !== undefined
    && !hasPath('C3,C4');
  const sidewaysPathUnavailable = session.state.realm.sites.B3 !== undefined
    && !hasPath('C3,B3');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.phalanxInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2');
  const siteTargetAvailable = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === session.state.realm.sites.C2?.instanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    backwardPathUnavailable,
    deck: deckList(opening.manifest.decks.north, opening.names),
    forwardPathAvailable,
    phalanx:
      opening.names.get(input.dalceanPhalanx.stableId) ?? input.dalceanPhalanx.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    sidewaysPathUnavailable,
    siteTargetAvailable,
  });
}

function runEarthImmobile(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthImmobile'] {
  const opening = findEarthImmobileOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.comparatorInstanceId
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownSiteInstanceId
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.pudgeInstanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  let dragCheckpoint = session;
  dragCheckpoint = accept(dragCheckpoint, action(dragCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  dragCheckpoint = accept(dragCheckpoint, action(dragCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const dragChoices = legalGameActions(dragCheckpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile'
      && descriptor.shooterInstanceId === opening.pudgeInstanceId
      && descriptor.direction === 'south'
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2'
      && descriptor.hit?.instanceId === opening.comparatorInstanceId);
  const noFightAction = dragChoices.find(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile' && !descriptor.fightOnArrival);
  const fightAction = dragChoices.find(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival);
  if (!noFightAction || !fightAction) {
    throw new Error('private Pudge drag scenario lacks both optional fight choices');
  }
  const dragChoicePairAvailable = dragChoices.length === 2;
  const dragOnlySession = accept(dragCheckpoint, noFightAction);
  const fightSession = accept(dragCheckpoint, fightAction);
  const dragOnlyEvents = dragOnlySession.transcript.at(-1)?.events ?? [];
  const fightEvents = fightSession.transcript.at(-1)?.events ?? [];
  const dragOnlyPudge = dragOnlySession.state.realm.units
    .find(({ instanceId }) => instanceId === opening.pudgeInstanceId);
  const dragOnlyBosk = dragOnlySession.state.realm.units
    .find(({ instanceId }) => instanceId === opening.comparatorInstanceId);
  const fightPudge = fightSession.state.realm.units
    .find(({ instanceId }) => instanceId === opening.pudgeInstanceId);
  const dragEventsAreSourceLinked = (events: typeof dragOnlyEvents): boolean => {
    const shot = events[0];
    const dragged = events[1];
    return shot?.type === 'projectile-shot'
      && isJsonRecord(shot.payload)
      && shot.payload.direction === 'south'
      && shot.payload.seat === 'north'
      && shot.payload.shooterInstanceId === opening.pudgeInstanceId
      && isJsonRecord(shot.payload.hit)
      && shot.payload.hit.instanceId === opening.comparatorInstanceId
      && dragged?.type === 'unit-dragged'
      && isJsonRecord(dragged.payload)
      && dragged.payload.seat === 'north'
      && dragged.payload.sourceInstanceId === opening.pudgeInstanceId
      && dragged.payload.targetInstanceId === opening.comparatorInstanceId
      && dragged.payload.steps === 1
      && isJsonRecord(dragged.payload.from)
      && dragged.payload.from.cell === 'C2'
      && isJsonRecord(dragged.payload.to)
      && dragged.payload.to.cell === 'C3';
  };

  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.comparatorInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const pudgeMoves = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.pudgeInstanceId);
  const nearbySitePresent = session.state.realm.sites.C4 !== undefined
    && session.state.realm.sites.B3 !== undefined;
  const positiveStepMoveUnavailable = nearbySitePresent
    && pudgeMoves.every(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.path.length === 1);
  const sameLocationAttackAvailable = session.state.realm.units.some(({ instanceId, location }) =>
    instanceId === opening.comparatorInstanceId && location === 'C3')
    && pudgeMoves.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.path.length === 1
        && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.comparatorInstanceId
    && descriptor.path.length === 1
    && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'site'
    && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
  const defend = action(session, ({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === opening.pudgeInstanceId
      && descriptor.path.length === 1
      && descriptor.to.cell === 'C3');
  const localDefendAvailable = defend.descriptor.kind === 'defend'
    && defend.descriptor.path.length === 1;
  session = accept(session, defend);
  take(({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    comparatorMinion:
      opening.names.get(input.firstStrikeTargetMinion.stableId)
        ?? input.firstStrikeTargetMinion.stableId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    dragChoicePairAvailable,
    dragOnlyAcceptedActionCount: dragOnlySession.transcript.length,
    dragOnlyEventsVerified: dragOnlyEvents.map(({ type }) => type).join(',')
      === 'projectile-shot,unit-dragged'
      && dragEventsAreSourceLinked(dragOnlyEvents),
    dragOnlyReplayVerified: verifyGameReplay(dragOnlySession),
    dragOnlyStateVerified: dragOnlyPudge?.location === 'C3'
      && dragOnlyPudge.tapped
      && dragOnlyPudge.damage === 0
      && dragOnlyBosk?.location === 'C3'
      && dragOnlyBosk.damage === 0
      && !dragOnlySession.state.players.south.cemetery
        .some(({ instanceId }) => instanceId === opening.comparatorInstanceId)
      && dragOnlySession.state.terminal.status === 'active',
    fightAcceptedActionCount: fightSession.transcript.length,
    fightEventsVerified: fightEvents.map(({ type }) => type).join(',')
      === 'projectile-shot,unit-dragged,fight-started,strike-damage-allocated,damage-dealt,damage-dealt,minion-died'
      && dragEventsAreSourceLinked(fightEvents),
    fightReplayVerified: verifyGameReplay(fightSession),
    fightStateVerified: fightPudge?.location === 'C3'
      && fightPudge.tapped
      && fightPudge.damage === 3
      && !fightSession.state.realm.units
        .some(({ instanceId }) => instanceId === opening.comparatorInstanceId)
      && fightSession.state.players.south.cemetery
        .some(({ instanceId }) => instanceId === opening.comparatorInstanceId)
      && fightSession.state.terminal.status === 'active',
    localDefendAvailable,
    nearbySitePresent,
    positiveStepMoveUnavailable,
    pudgeButcher:
      opening.names.get(input.pudgeButcher.stableId) ?? input.pudgeButcher.stableId,
    replayVerified: verifyGameReplay(session),
    sameLocationAttackAvailable,
  });
}

function runEarthBury(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthBury'] {
  const opening = findEarthBuryOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.boskTrollInstanceId
    && descriptor.cell === 'C2'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');

  const buryActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.buryInstanceId
      && descriptor.target !== undefined
      && descriptor.target.instanceId === opening.boskTrollInstanceId);
  const exactlyOneBuryTarget = buryActions.length === 1;
  const chosenBury = buryActions[0];
  if (!chosenBury) throw new Error('private forced-burrow Magic target is unavailable');
  const manaBefore = session.state.players.north.mana;
  session = accept(session, chosenBury);

  const finalEvents = session.transcript.at(-1)?.events ?? [];
  const exactEventOrder = finalEvents.map(({ type }) => type).join(',')
    === 'magic-cast,minion-burrowed,minion-died,magic-resolved';
  const castPayload = finalEvents[0] && isJsonRecord(finalEvents[0].payload)
    ? finalEvents[0].payload
    : undefined;
  const burrowPayload = finalEvents[1] && isJsonRecord(finalEvents[1].payload)
    ? finalEvents[1].payload
    : undefined;
  const deathPayload = finalEvents[2] && isJsonRecord(finalEvents[2].payload)
    ? finalEvents[2].payload
    : undefined;
  const resolvedPayload = finalEvents[3] && isJsonRecord(finalEvents[3].payload)
    ? finalEvents[3].payload
    : undefined;
  const eventIndex = (type: string, instanceId: string): number =>
    finalEvents.findIndex(({ payload, type: eventType }) =>
      eventType === type && isJsonRecord(payload) && payload.instanceId === instanceId);
  const castIndex = eventIndex('magic-cast', opening.buryInstanceId);
  const burrowIndex = finalEvents.findIndex(({ payload, type }) =>
    type === 'minion-burrowed'
      && isJsonRecord(payload)
      && payload.instanceId === opening.boskTrollInstanceId
      && payload.sourceInstanceId === opening.buryInstanceId
      && payload.cell === 'C2');
  const deathIndex = eventIndex('minion-died', opening.boskTrollInstanceId);
  const resolvedIndex = eventIndex('magic-resolved', opening.buryInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    boskTroll:
      opening.names.get(input.firstStrikeTargetMinion.stableId)
        ?? input.firstStrikeTargetMinion.stableId,
    buriedBeforeDeath: burrowIndex >= 0 && burrowIndex < deathIndex,
    bury: opening.names.get(input.bury.stableId) ?? input.bury.stableId,
    causalEventsVerified: exactEventOrder
      && castIndex === 0
      && burrowIndex === 1
      && deathIndex === burrowIndex + 1
      && resolvedIndex === 3
      && castPayload?.instanceId === opening.buryInstanceId
      && castPayload.manaPaid === 3
      && castPayload.seat === 'north'
      && castPayload.targetInstanceId === opening.boskTrollInstanceId
      && castPayload.targetSeat === 'south'
      && burrowPayload?.cell === 'C2'
      && burrowPayload.instanceId === opening.boskTrollInstanceId
      && burrowPayload.seat === 'south'
      && burrowPayload.sourceInstanceId === opening.buryInstanceId
      && deathPayload?.cardId === input.firstStrikeTargetMinion.stableId
      && deathPayload.instanceId === opening.boskTrollInstanceId
      && deathPayload.owner === 'south'
      && resolvedPayload?.instanceId === opening.buryInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    deathNotBanishmentAndGameActive: exactEventOrder
      && session.state.terminal.status === 'active',
    exactlyOneBuryTarget,
    manaPaid: manaBefore - session.state.players.north.mana,
    replayVerified: verifyGameReplay(session),
    spellEnteredCemetery: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.buryInstanceId)
      && !session.state.players.north.hand.spellbook
        .some(({ instanceId }) => instanceId === opening.buryInstanceId),
    targetEnteredCemetery: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.boskTrollInstanceId),
    targetLeftRealm: !session.state.realm.units
      .some(({ instanceId }) => instanceId === opening.boskTrollInstanceId),
  });
}

function runEarthRescue(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthRescue'] {
  const opening = findEarthRescueOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.boskTrollInstanceId
    && descriptor.cell === 'C2'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardInstanceId === opening.buryInstanceId
    && descriptor.target?.instanceId === opening.boskTrollInstanceId);

  const northViewAfterBury = observeGame(session.state, 'north');
  const southHandCountAfterBury = northViewAfterBury.players.south.hand.spellbook;
  const boskWasPublic = northViewAfterBury.players.south.cemetery
    .some(({ cardId, instanceId }) => cardId === input.firstStrikeTargetMinion.stableId
      && instanceId === opening.boskTrollInstanceId);
  const buryStayedNorthCemetery = session.state.players.north.cemetery
    .some(({ instanceId }) => instanceId === opening.buryInstanceId);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[2]
    && descriptor.cell === 'B2');

  const choices = legalGameActions(session.state, 'south').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.rescueInstanceId);
  const selected = choices.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cemeteryMinionInstanceId === opening.boskTrollInstanceId);
  if (!selected) throw new Error('private Rescue cemetery choice is unavailable');
  const manaBefore = session.state.players.south.mana;
  session = accept(session, selected);

  const northViewAfter = observeGame(session.state, 'north');
  const events = session.transcript.at(-1)?.events ?? [];
  const castPayload = events[0] && isJsonRecord(events[0].payload) ? events[0].payload : undefined;
  const returnedPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    boskTroll: input.firstStrikeTargetMinion.name,
    buryStayedNorthCemetery: buryStayedNorthCemetery
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.buryInstanceId),
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'magic-cast,minion-returned-to-hand,magic-resolved'
      && castPayload?.cemeteryMinionInstanceId === opening.boskTrollInstanceId
      && returnedPayload?.cardId === input.firstStrikeTargetMinion.stableId
      && returnedPayload.instanceId === opening.boskTrollInstanceId
      && returnedPayload.owner === 'south'
      && returnedPayload.seat === 'south'
      && returnedPayload.sourceInstanceId === opening.rescueInstanceId,
    deck: deckList(opening.manifest.decks.south, opening.names),
    hiddenFromNorthAfterReturn: boskWasPublic
      && typeof southHandCountAfterBury === 'number'
      && typeof northViewAfter.players.south.hand.spellbook === 'number'
      && northViewAfter.players.south.hand.spellbook === southHandCountAfterBury + 1
      && northViewAfter.players.south.cemetery
        .every(({ instanceId }) => instanceId !== opening.boskTrollInstanceId),
    manaPaid: manaBefore - session.state.players.south.mana,
    onlyOwnCemeteryMinionChoice: choices.length === 1
      && choices.every(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.cemeteryMinionInstanceId === opening.boskTrollInstanceId)
      && session.state.players.north.cemetery
        .every(({ instanceId }) => instanceId !== opening.boskTrollInstanceId),
    replayVerified: verifyGameReplay(session),
    rescue: input.rescue.name,
    rescueEnteredSouthCemetery: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.rescueInstanceId),
    returnedToSouthHand: session.state.players.south.cemetery
      .every(({ instanceId }) => instanceId !== opening.boskTrollInstanceId)
      && session.state.players.south.hand.spellbook
        .some(({ instanceId }) => instanceId === opening.boskTrollInstanceId),
  });
}

function runEarthShallowGrave(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthShallowGrave'] {
  const opening = findEarthShallowGraveOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const before = session.state.players.north;
  const topTwo = before.spellbook.slice(0, 2);
  if (topTwo.length !== 2) throw new Error('private site discard Genesis lacks two spells');
  const southViewBefore = canonicalJson(observeGame(session.state, 'south') as unknown as JsonValue);
  const hiddenBeforeDiscard = topTwo.every(({ cardId, instanceId }) =>
    !southViewBefore.includes(cardId) && !southViewBefore.includes(instanceId));
  const spellHandBefore = canonicalJson(before.hand.spellbook as unknown as JsonValue);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.shallowGraveInstanceId
      && descriptor.cell === 'C4'));

  const after = session.state.players.north;
  const discarded = after.cemetery.slice(-2);
  const southViewAfter = observeGame(session.state, 'south').players.north.cemetery.slice(-2);
  const events = session.transcript.at(-1)?.events ?? [];
  const discardEventsMatch = topTwo.every((card, index) => {
    const event = events[index + 1];
    return event?.type === 'spell-discarded'
      && isJsonRecord(event.payload)
      && event.payload.cardId === card.cardId
      && event.payload.instanceId === card.instanceId
      && event.payload.owner === 'north'
      && event.payload.seat === 'north'
      && event.payload.sourceInstanceId === opening.shallowGraveInstanceId;
  });
  const site = session.state.realm.sites.C4;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    affinityProvided: observeGame(session.state, 'north').players.north.affinity.earth === 1,
    avatarTapped: after.avatar.tapped,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'site-played,spell-discarded,spell-discarded'
      && isJsonRecord(events[0]?.payload)
      && events[0]?.payload.instanceId === opening.shallowGraveInstanceId
      && discardEventsMatch,
    deck: deckList(opening.manifest.decks.north, opening.names),
    discardedInDeckOrder: discarded.length === 2
      && topTwo.every((card, index) =>
        discarded[index]?.cardId === card.cardId
          && discarded[index]?.instanceId === card.instanceId),
    gameRemainedActive: session.state.phase === 'main' && session.state.terminal.status === 'active',
    hiddenBeforeDiscard,
    manaProvided: after.mana === 1,
    publicAfterDiscard: southViewAfter.length === 2
      && topTwo.every((card, index) =>
        southViewAfter[index]?.cardId === card.cardId
          && southViewAfter[index]?.instanceId === card.instanceId),
    replayVerified: verifyGameReplay(session),
    shallowGrave:
      opening.names.get(input.shallowGrave.stableId) ?? input.shallowGrave.stableId,
    siteEstablished: site?.instanceId === opening.shallowGraveInstanceId
      && site.controller === 'north',
    spellHandUnchanged:
      canonicalJson(after.hand.spellbook as unknown as JsonValue) === spellHandBefore,
    spellbookReducedByTwo: after.spellbook.length === before.spellbook.length - 2,
  });
}

function runEarthSinkhole(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthSinkhole'] {
  const opening = findEarthSinkholeOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.sinkholeInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.targetSiteInstanceId
    && descriptor.cell === 'C3');

  const sourceBefore = session.state.realm.sites.C4;
  const targetBefore = session.state.realm.sites.C3;
  if (!sourceBefore || !targetBefore || 'rubble' in sourceBefore || 'rubble' in targetBefore) {
    throw new Error('private sacrifice-to-destroy setup lacks its two real sites');
  }
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-site-destruction'
      && descriptor.sourceSiteInstanceId === opening.sinkholeInstanceId
      && descriptor.targetCell === 'C3'
      && descriptor.targetSiteInstanceId === opening.targetSiteInstanceId);
  const selected = choices[0];
  if (!selected) throw new Error('private sacrifice-to-destroy site action is unavailable');
  const exactActivationAvailable = choices.length === 1;
  session = accept(session, selected);

  const rubbleC3 = session.state.realm.sites.C3;
  const rubbleC4 = session.state.realm.sites.C4;
  const events = session.transcript.at(-1)?.events ?? [];
  const sacrificedPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const destroyedPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const rubbleC3Payload = events[2] && isJsonRecord(events[2].payload)
    ? events[2].payload
    : undefined;
  const rubbleC4Payload = events[3] && isJsonRecord(events[3].payload)
    ? events[3].payload
    : undefined;
  const affinity = observeGame(session.state, 'north').players.north.affinity;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarRemainedOnSurface: session.state.players.north.avatar.location === 'C4'
      && session.state.players.north.avatar.region === 'surface',
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'site-sacrificed,site-destroyed,rubble-created,rubble-created'
      && sacrificedPayload?.cell === 'C4'
      && sacrificedPayload.instanceId === opening.sinkholeInstanceId
      && sacrificedPayload.owner === 'north'
      && sacrificedPayload.sourceInstanceId === opening.sinkholeInstanceId
      && destroyedPayload?.cell === 'C3'
      && destroyedPayload.instanceId === opening.targetSiteInstanceId
      && destroyedPayload.owner === 'north'
      && destroyedPayload.sourceInstanceId === opening.sinkholeInstanceId
      && rubbleC3Payload?.cell === 'C3'
      && rubbleC3Payload.instanceId === rubbleC3?.instanceId
      && rubbleC3Payload.sourceInstanceId === opening.sinkholeInstanceId
      && rubbleC4Payload?.cell === 'C4'
      && rubbleC4Payload.instanceId === rubbleC4?.instanceId
      && rubbleC4Payload.sourceInstanceId === opening.sinkholeInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactActivationAvailable,
    noAffinityOrControlContribution: affinity.air === 0
      && affinity.earth === 0
      && affinity.fire === 0
      && affinity.water === 0
      && Object.values(session.state.realm.sites)
        .every(({ controller }) => controller !== 'north'),
    replayVerified: verifyGameReplay(session),
    sinkhole: input.sinkhole.name,
    sourceAndTargetEnteredCemetery: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.sinkholeInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.targetSiteInstanceId),
    twoNeutralRubbleSites: rubbleC3 !== undefined
      && 'rubble' in rubbleC3
      && rubbleC3.rubble === true
      && rubbleC3.controller === null
      && !('cardId' in rubbleC3)
      && rubbleC3.instanceId !== opening.targetSiteInstanceId
      && rubbleC4 !== undefined
      && 'rubble' in rubbleC4
      && rubbleC4.rubble === true
      && rubbleC4.controller === null
      && !('cardId' in rubbleC4)
      && rubbleC4.instanceId !== opening.sinkholeInstanceId,
  });
}

function runEarthDivineHealing(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthDivineHealing'] {
  const opening = findEarthDivineHealingOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.attackerInstanceId
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.attackerInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'site'
    && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
  take(({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const avatarDefinition = session.state.cards[session.state.players.north.avatar.card.cardId];
  if (avatarDefinition?.cardType !== 'avatar') {
    throw new Error('private controller-healing Magic scenario Avatar is unsupported');
  }
  const lifeBefore = session.state.players.north.avatar.life;
  const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.divineHealingInstanceId);
  const exactlyOneTargetlessCast = casts.length === 1
    && casts[0]?.descriptor.kind === 'cast-magic'
    && casts[0].descriptor.target === undefined;
  const manaBefore = session.state.players.north.mana;
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardInstanceId === opening.divineHealingInstanceId
    && descriptor.target === undefined);
  const lifeAfter = session.state.players.north.avatar.life;
  const actualLifeGained = lifeAfter - lifeBefore;
  const manaPaid = manaBefore - session.state.players.north.mana;
  const spellEnteredCemetery = session.state.players.north.hand.spellbook
    .every(({ instanceId }) => instanceId !== opening.divineHealingInstanceId)
    && session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.divineHealingInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    actualLifeGained,
    deck: deckList(opening.manifest.decks.north, opening.names),
    divineHealing:
      opening.names.get(input.divineHealing.stableId) ?? input.divineHealing.stableId,
    exactlyOneTargetlessCast,
    lifeCappedAtMaximum: lifeAfter === avatarDefinition.life && actualLifeGained < 7,
    lifeWasDamagedAboveDeathsDoor: lifeBefore > 0 && lifeBefore < avatarDefinition.life,
    manaPaid,
    replayVerified: verifyGameReplay(session),
    spellEnteredCemetery,
  });
}

function runEarthSecretTunnel(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthSecretTunnel'] {
  const opening = findEarthSecretTunnelOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.secretTunnelInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.caveTrollsInstanceId
    && descriptor.cell === 'C4'
    && descriptor.region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const moves = legalGameActions(session.state, 'north');
  const hasPath = (
    unitInstanceId: string,
    cells: string,
    region: 'surface' | 'underground',
  ): boolean => moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === cells
    && descriptor.path.every((step) => step.region === region));
  const physicalMoveAvailable = hasPath(opening.caveTrollsInstanceId, 'C4,C3', 'underground');
  const directTunnelMoveAvailable = hasPath(
    opening.caveTrollsInstanceId,
    'C4,C2',
    'underground',
  );
  const directOpponentUnavailable = !hasPath(
    opening.caveTrollsInstanceId,
    'C4,C1',
    'underground',
  );
  const avatarInstanceId = session.state.players.north.avatar.card.instanceId;
  const avatarPhysicalAvailable = hasPath(avatarInstanceId, 'C4,C3', 'surface');
  const avatarDirectUnavailable = !hasPath(avatarInstanceId, 'C4,C2', 'surface');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.caveTrollsInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C2'
    && descriptor.path.every((step) => step.region === 'underground'));
  const movedUnderground = session.state.realm.units.some(({ instanceId, location, region }) =>
    instanceId === opening.caveTrollsInstanceId && location === 'C2' && region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarDirectUnavailable,
    avatarPhysicalAvailable,
    caveTrolls:
      opening.names.get(input.burrowingMinion.stableId) ?? input.burrowingMinion.stableId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    directOpponentUnavailable,
    directTunnelMoveAvailable,
    movedUnderground,
    physicalMoveAvailable,
    replayVerified: verifyGameReplay(session),
    secretTunnel: opening.names.get(input.secretTunnel.stableId) ?? input.secretTunnel.stableId,
    seed: opening.seed,
  });
}

function runEarthRamp(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthRamp'] {
  const opening = findEarthOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southFirstSiteInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3'));
  const affinityBeforeProvider = observeGame(session.state, 'north').players.north.affinity.earth;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.providerInstanceId
      && descriptor.cell === 'C4'));
  const affinityAdded =
    observeGame(session.state, 'north').players.north.affinity.earth === affinityBeforeProvider + 1;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSecondSiteInstanceId
      && descriptor.cell === 'B1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.southMinionInstanceId
      && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const manaBeforeGhostTown = session.state.players.north.mana;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'C2'));
  const ghostTownBonusMana =
    session.state.players.north.mana - manaBeforeGhostTown - 1;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.manaInstanceId
      && descriptor.cell === 'C3'));
  const manaUnavailableWhileSick = !legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === opening.manaInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  const ghostTownUnusedManaExpired = session.state.players.north.mana === 0;

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.southMinionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const manaBefore = session.state.players.north.mana;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === opening.manaInstanceId));
  const manaGained = session.state.players.north.mana - manaBefore;
  const payoffManaBefore = session.state.players.north.mana;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.payoffInstanceId
      && descriptor.cell === 'C3'));
  const rampPaidFive = payoffManaBefore === 5 && session.state.players.north.mana === 0;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const payoffCanMoveAndAttack = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === opening.payoffInstanceId);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === opening.manaInstanceId));
  const beforeGenesis = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.genesisInstanceId
      && descriptor.cell === 'C4'));
  const afterGenesis = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.deathriteInstanceId
      && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.southMinionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.deathriteInstanceId));
  const movingDefendUnavailable = !legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === opening.payoffInstanceId);
  const beforeDeathrite = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  const afterDeathrite = session.state.players.north;
  const finalEvents = session.transcript.at(-1)?.events ?? [];
  const siteDrawIndex = finalEvents.findIndex(({ type }) => type === 'site-drawn');
  const cemeteryIndex = finalEvents.findIndex(({ type }) => type === 'minion-died');
  const deathriteSiteDrawnBeforeCemetery =
    afterDeathrite.atlas.length === beforeDeathrite.atlas.length - 1
    && afterDeathrite.hand.atlas.length === beforeDeathrite.hand.atlas.length + 1
    && afterDeathrite.cemetery.some(({ instanceId }) => instanceId === opening.deathriteInstanceId)
    && siteDrawIndex >= 0
    && siteDrawIndex < cemeteryIndex;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    affinityAdded,
    deck: deckList(opening.manifest.decks.north, opening.names),
    deathriteMinion:
      opening.names.get(input.deathriteMinion.stableId) ?? input.deathriteMinion.stableId,
    deathriteSiteDrawnBeforeCemetery,
    genesisSiteDrawn:
      afterGenesis.atlas.length === beforeGenesis.atlas.length - 1
      && afterGenesis.hand.atlas.length === beforeGenesis.hand.atlas.length + 1,
    ghostTown: opening.names.get(input.ghostTownSite.stableId) ?? input.ghostTownSite.stableId,
    ghostTownBonusMana,
    ghostTownUnusedManaExpired,
    manaGained,
    manaMinion: opening.names.get(input.manaMinion.stableId) ?? input.manaMinion.stableId,
    manaUnavailableWhileSick,
    movingDefendUnavailable,
    payoffCanMoveAndAttack,
    rampPaidFive,
    rampPayoffMinion:
      opening.names.get(input.cannotDefendMinion.stableId) ?? input.cannotDefendMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function stageEarthDuel(opening: ReturnType<typeof findEarthDuelOpening>): GameSession {
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southFirstSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSecondSiteInstanceId
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.targetInstanceId
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B4');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  return session;
}

function runEarthRanged(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthRanged'] {
  const opening = findEarthDuelOpening(input);
  let session = stageEarthDuel(opening);

  const shot = action(session, ({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === opening.attackerInstanceId
      && descriptor.direction === 'south'
      && descriptor.hit?.instanceId === opening.targetInstanceId);
  const rangedOneStep = shot.descriptor.kind === 'shoot-projectile'
    && shot.descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2';
  session = accept(session, shot);
  const shooter = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.attackerInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    rangedMinion: opening.names.get(input.rangedMinion.stableId) ?? input.rangedMinion.stableId,
    rangedOneStep,
    rangedShooterStayedSafe:
      shooter?.location === 'C3' && shooter.tapped && shooter.damage === 0,
    rangedTargetDied: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.targetInstanceId),
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function runEarthFirstStrike(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthFirstStrike'] {
  const opening = findEarthDuelOpening(input, 'first-strike');
  let session = stageEarthDuel(opening);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.from.cell === 'C3'
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.targetInstanceId);
  take(({ descriptor }) => descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
  const attacker = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.attackerInstanceId);
  const fightEvents = session.transcript.at(-1)?.events ?? [];
  const targetDied = session.state.players.south.cemetery
    .some(({ instanceId }) => instanceId === opening.targetInstanceId);
  const targetDiedBeforeReturn = targetDied && !fightEvents.some(({ payload, type }) =>
    type === 'damage-dealt'
      && isJsonRecord(payload)
      && payload.instanceId === opening.attackerInstanceId
      && payload.amount !== 0);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackerSurvivedUndamaged: attacker?.damage === 0 && attacker.location === 'C2' && attacker.tapped,
    deck: deckList(opening.manifest.decks.north, opening.names),
    firstStrikeMinion:
      opening.names.get(input.firstStrikeMinion.stableId) ?? input.firstStrikeMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    targetDiedBeforeReturn,
    targetMinion:
      opening.names.get(input.firstStrikeTargetMinion.stableId) ?? input.firstStrikeTargetMinion.stableId,
  });
}

function runEarthWard(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthWard'] {
  const opening = findEarthDuelOpening(input, 'ward');
  let session = stageEarthDuel(opening);
  const targetBefore = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.targetInstanceId);
  const firstShot = action(session, ({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === opening.attackerInstanceId
      && descriptor.direction === 'south'
      && descriptor.hit?.instanceId === opening.targetInstanceId);
  session = accept(session, firstShot);
  const firstShotEvents = session.transcript.at(-1)?.events ?? [];
  const targetAfter = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.targetInstanceId);
  const wardBroke = targetBefore?.warded === true
    && targetAfter?.warded === false
    && firstShotEvents.some(({ payload, type }) =>
      type === 'ward-broken'
        && isJsonRecord(payload)
        && payload.instanceId === opening.targetInstanceId);
  const wardPreventedDamage = firstShotEvents.some(({ payload, type }) =>
    type === 'damage-dealt'
      && isJsonRecord(payload)
      && payload.instanceId === opening.targetInstanceId
      && payload.amount === 0
      && payload.prevented === true);
  const wardTargetSurvived = targetAfter?.damage === 0
    && !session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.targetInstanceId);

  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === opening.attackerInstanceId
      && descriptor.hit?.instanceId === opening.targetInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    rangedMinion: opening.names.get(input.rangedMinion.stableId) ?? input.rangedMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    wardBroke,
    wardMinion: opening.names.get(input.wardMinion.stableId) ?? input.wardMinion.stableId,
    wardPreventedDamage,
    wardTargetDiedAfterSecondShot: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.targetInstanceId),
    wardTargetSurvived,
  });
}

function runAirMovement(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airMovement'] {
  const opening = findAirOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardInstanceId === opening.southSiteInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === opening.attackerInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.movementInstanceId
      && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === session.state.realm.sites.C2?.instanceId));
  const defend = action(session, ({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === opening.movementInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
  const twoStepDefend = defend.descriptor.kind === 'defend' && defend.descriptor.path.length === 3;
  session = accept(session, defend);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const move = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.movementInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4');
  const twoStepMoveAndAttack = move.descriptor.kind === 'move-and-attack'
    && move.descriptor.path.length === 3;
  session = accept(session, move);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    movementMinion:
      opening.names.get(input.movementMinion.stableId) ?? input.movementMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    twoStepDefend,
    twoStepMoveAndAttack,
  });
}

function runAirZap(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airZap'] {
  const opening = findAirZapOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.snowLeopardInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const manaBefore = session.state.players.north.mana;
  const targetBefore = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.snowLeopardInstanceId);
  const spellWasInHand = session.state.players.north.hand.spellbook
    .some(({ instanceId }) => instanceId === opening.zapInstanceId);
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardInstanceId === opening.zapInstanceId
    && descriptor.target !== undefined
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === opening.snowLeopardInstanceId);
  const targetAfter = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.snowLeopardInstanceId);
  const damageDealt = (targetAfter?.damage ?? 0) - (targetBefore?.damage ?? 0);
  const manaPaid = manaBefore - session.state.players.north.mana;
  const spellLeftHand = spellWasInHand
    && session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.zapInstanceId);
  const spellEnteredCemetery = session.state.players.north.cemetery
    .some(({ instanceId }) => instanceId === opening.zapInstanceId);
  const snowLeopardSurvived = targetAfter?.damage === 1
    && session.state.players.south.cemetery
      .every(({ instanceId }) => instanceId !== opening.snowLeopardInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    damageDealt,
    deck: deckList(opening.manifest.decks.north, opening.names),
    manaPaid,
    replayVerified: verifyGameReplay(session),
    snowLeopard:
      opening.names.get(input.stealthTargetMinion.stableId)
        ?? input.stealthTargetMinion.stableId,
    snowLeopardSurvived,
    spellEnteredCemetery,
    spellLeftHand,
    zap: opening.names.get(input.zap.stableId) ?? input.zap.stableId,
  });
}

function runAirArcLightning(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airArcLightning'] {
  const opening = findAirArcLightningOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.snowLeopardInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.snowLeopardInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.snowLeopardInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[3]
    && descriptor.cell === 'B4');
  const targets = legalGameActions(session.state, 'north');
  const nearbyTargetAvailable = targets.some(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.arcLightningInstanceId
      && descriptor.target !== undefined
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.snowLeopardInstanceId);
  const southAvatarInstanceId = session.state.players.south.avatar.card.instanceId;
  const farSameRegionUnitUnavailable = targets.every(({ descriptor }) =>
    descriptor.kind !== 'cast-magic'
      || descriptor.cardInstanceId !== opening.arcLightningInstanceId
      || descriptor.target === undefined
      || descriptor.target.instanceId !== southAvatarInstanceId);
  const manaBefore = session.state.players.north.mana;
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardInstanceId === opening.arcLightningInstanceId
    && descriptor.target !== undefined
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === opening.snowLeopardInstanceId);
  const manaPaid = manaBefore - session.state.players.north.mana;
  const snowLeopardDied = session.state.realm.units
    .every(({ instanceId }) => instanceId !== opening.snowLeopardInstanceId)
    && session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.snowLeopardInstanceId);
  const spellEnteredCemetery = session.state.players.north.hand.spellbook
    .every(({ instanceId }) => instanceId !== opening.arcLightningInstanceId)
    && session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.arcLightningInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    arcLightning:
      opening.names.get(input.arcLightning.stableId) ?? input.arcLightning.stableId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    farSameRegionUnitUnavailable,
    manaPaid,
    nearbyTargetAvailable,
    replayVerified: verifyGameReplay(session),
    snowLeopard:
      opening.names.get(input.stealthTargetMinion.stableId)
        ?? input.stealthTargetMinion.stableId,
    snowLeopardDied,
    spellEnteredCemetery,
  });
}

function runAirLightningBolt(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airLightningBolt'] {
  const opening = findAirLightningBoltOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.snowLeopardInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const avatarLifeBefore = session.state.players.north.avatar.life;
  const manaBefore = session.state.players.north.mana;
  const occupants = session.state.realm.units.filter(({ location, region }) =>
    location === 'C4' && region === 'surface');
  const locationActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.lightningBoltInstanceId
      && descriptor.targetLocation?.cell === 'C4'
      && descriptor.targetLocation.region === 'surface');
  const occupiedLocationTargeted = locationActions.length === 1
    && session.state.players.north.avatar.location === 'C4'
    && session.state.players.north.avatar.region === 'surface'
    && occupants.length === 1
    && occupants.some(({ instanceId }) => instanceId === opening.snowLeopardInstanceId);
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardInstanceId === opening.lightningBoltInstanceId
    && descriptor.targetLocation?.cell === 'C4'
    && descriptor.targetLocation.region === 'surface');

  const receipt = session.transcript.at(-1);
  const random = receipt?.randomDraws[0];
  const randomDomain = random && isJsonRecord(random.domain) ? random.domain : undefined;
  const allocationIndex = receipt?.events.findIndex(({ payload, type }) =>
    type === 'magic-damage-allocated'
      && isJsonRecord(payload)
      && payload.amount === 3
      && payload.targetInstanceId === opening.snowLeopardInstanceId) ?? -1;
  const damageIndex = receipt?.events.findIndex(({ payload, type }) =>
    type === 'damage-dealt'
      && isJsonRecord(payload)
      && payload.amount === 3
      && payload.instanceId === opening.snowLeopardInstanceId) ?? -1;
  const deathIndex = receipt?.events.findIndex(({ payload, type }) =>
    type === 'minion-died'
      && isJsonRecord(payload)
      && payload.instanceId === opening.snowLeopardInstanceId) ?? -1;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarUnchanged: session.state.players.north.avatar.life === avatarLifeBefore,
    deck: deckList(opening.manifest.decks.north, opening.names),
    lethalDamageRecorded: allocationIndex >= 0
      && allocationIndex < damageIndex
      && damageIndex < deathIndex,
    lightningBolt: input.lightningBolt.name,
    manaPaid: manaBefore - session.state.players.north.mana,
    occupiedLocationTargeted,
    randomSelectionRecorded: receipt?.randomDraws.length === 1
      && random?.purpose === 'magic_random_unit_at_location'
      && randomDomain?.accepted === true
      && randomDomain.exclusiveMaximum === 2
      && randomDomain.kind === 'unit_index_candidate',
    replayVerified: verifyGameReplay(session),
    snowLeopard: input.stealthTargetMinion.name,
    snowLeopardDied: session.state.realm.units
      .every(({ instanceId }) => instanceId !== opening.snowLeopardInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.snowLeopardInstanceId),
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.lightningBoltInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.lightningBoltInstanceId),
  });
}

function runAirRainOfArrows(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airRainOfArrows'] {
  const opening = findAirRainOfArrowsOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.northSnowLeopardInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.southSnowLeopardInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const northBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.northSnowLeopardInstanceId);
  const southBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southSnowLeopardInstanceId);
  if (!northBefore || !southBefore) {
    throw new Error('private Rain of Arrows setup lacks both surface Snow Leopards');
  }
  const avatarsBefore = canonicalJson({
    north: session.state.players.north.avatar,
    south: session.state.players.south.avatar,
  } as unknown as JsonValue);
  const sitesBefore = canonicalJson(session.state.realm.sites as unknown as JsonValue);
  const northCemeteryBefore = canonicalJson(
    session.state.players.north.cemetery as unknown as JsonValue,
  );
  const southCemeteryBefore = canonicalJson(
    session.state.players.south.cemetery as unknown as JsonValue,
  );
  const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.rainOfArrowsInstanceId);
  const selected = casts[0];
  if (!selected || selected.descriptor.kind !== 'cast-magic' || casts.length !== 1) {
    throw new Error('private targetless Rain of Arrows action is not exactly available');
  }
  const noTargetChoice = selected.descriptor.target === undefined
    && selected.descriptor.targetLocation === undefined
    && selected.descriptor.targetSiteInstanceId === undefined
    && selected.descriptor.ally === undefined
    && selected.descriptor.cemeteryMinionInstanceId === undefined
    && selected.descriptor.temptedEnemy === undefined
    && selected.descriptor.temptedDestination === undefined;
  const manaBefore = session.state.players.north.mana;
  session = accept(session, selected);

  const northAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.northSnowLeopardInstanceId);
  const southAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southSnowLeopardInstanceId);
  const receipt = session.transcript.at(-1);
  const events = receipt?.events ?? [];
  const castPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const resolvedEvent = events.at(-1);
  const resolvedPayload = resolvedEvent && isJsonRecord(resolvedEvent.payload)
    ? resolvedEvent.payload
    : undefined;
  const targetIds = [
    opening.northSnowLeopardInstanceId,
    opening.southSnowLeopardInstanceId,
  ].sort();
  const allocations = events.filter(({ type }) => type === 'magic-damage-allocated');
  const damageEvents = events.filter(({ type }) => type === 'damage-dealt');
  const northOtherCemetery = session.state.players.north.cemetery.filter(({ instanceId }) =>
    instanceId !== opening.rainOfArrowsInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarsPreserved: canonicalJson({
      north: session.state.players.north.avatar,
      south: session.state.players.south.avatar,
    } as unknown as JsonValue) === avatarsBefore,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'magic-cast,magic-damage-allocated,magic-damage-allocated,damage-dealt,damage-dealt,magic-resolved'
      && castPayload?.instanceId === opening.rainOfArrowsInstanceId
      && castPayload.manaPaid === 2
      && castPayload.seat === 'north'
      && allocations.map(({ payload }) => isJsonRecord(payload)
        ? String(payload.targetInstanceId)
        : '').join(',') === targetIds.join(',')
      && allocations.every(({ payload }) => isJsonRecord(payload)
        && payload.amount === 1
        && payload.sourceInstanceId === opening.rainOfArrowsInstanceId)
      && damageEvents.map(({ payload }) => isJsonRecord(payload)
        ? String(payload.instanceId)
        : '').join(',') === targetIds.join(',')
      && damageEvents.every(({ payload }) => isJsonRecord(payload) && payload.amount === 1)
      && resolvedEvent?.type === 'magic-resolved'
      && resolvedPayload?.instanceId === opening.rainOfArrowsInstanceId,
    cemeteriesOtherwisePreserved:
      canonicalJson(northOtherCemetery as unknown as JsonValue) === northCemeteryBefore
      && canonicalJson(session.state.players.south.cemetery as unknown as JsonValue)
        === southCemeteryBefore,
    deck: deckList(opening.manifest.decks.north, opening.names),
    gameRemainedActive: session.state.terminal.status === 'active',
    manaPaid: manaBefore - session.state.players.north.mana,
    noRandomDraws: receipt?.randomDraws.length === 0,
    noTargetChoice,
    rainOfArrows: input.rainOfArrows.name,
    replayVerified: verifyGameReplay(session),
    sitesPreserved: canonicalJson(session.state.realm.sites as unknown as JsonValue) === sitesBefore,
    snowLeopard: input.stealthTargetMinion.name,
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.rainOfArrowsInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.rainOfArrowsInstanceId),
    surfaceLeopardsDamagedAndSurvived: northBefore.damage === 0
      && southBefore.damage === 0
      && northAfter?.damage === 1
      && southAfter?.damage === 1
      && northAfter.cardId === northBefore.cardId
      && northAfter.controller === northBefore.controller
      && northAfter.location === 'C4'
      && northAfter.owner === northBefore.owner
      && northAfter.region === 'surface'
      && northAfter.tapped === northBefore.tapped
      && southAfter.cardId === southBefore.cardId
      && southAfter.controller === southBefore.controller
      && southAfter.location === 'C1'
      && southAfter.owner === southBefore.owner
      && southAfter.region === 'surface'
      && southAfter.tapped === southBefore.tapped
      && events.every(({ type }) => type !== 'minion-died'),
  });
}

function runAirTeleport(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airTeleport'] {
  const opening = findAirTeleportOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.snowLeopardInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const before = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.snowLeopardInstanceId);
  const sourceSiteBefore = session.state.realm.sites.C4;
  const targetSiteBefore = session.state.realm.sites.C1;
  if (!before || !sourceSiteBefore || !targetSiteBefore) {
    throw new Error('private Teleport setup lacks its unit or sites');
  }
  const pairs = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.teleportInstanceId
      && descriptor.ally?.kind === 'minion'
      && descriptor.ally.instanceId === opening.snowLeopardInstanceId
      && descriptor.targetLocation?.cell === 'C1'
      && descriptor.targetLocation.region === 'surface'
      && descriptor.targetSiteInstanceId === opening.southSiteInstanceId);
  const chosen = pairs[0];
  if (!chosen) throw new Error('private Teleport ally/site pair is unavailable');
  const manaBefore = session.state.players.north.mana;
  const noPathTeleport = !('path' in chosen.descriptor);
  session = accept(session, chosen);

  const after = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.snowLeopardInstanceId);
  const receipt = session.transcript.at(-1);
  const events = receipt?.events ?? [];
  const castPayload = events[0] && isJsonRecord(events[0].payload) ? events[0].payload : undefined;
  const teleportedPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const from = teleportedPayload && isJsonRecord(teleportedPayload.from)
    ? teleportedPayload.from
    : undefined;
  const to = teleportedPayload && isJsonRecord(teleportedPayload.to)
    ? teleportedPayload.to
    : undefined;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'magic-cast,unit-teleported,magic-resolved'
      && castPayload?.allyInstanceId === opening.snowLeopardInstanceId
      && castPayload.targetSiteInstanceId === opening.southSiteInstanceId
      && teleportedPayload?.seat === 'north'
      && teleportedPayload.sourceInstanceId === opening.teleportInstanceId
      && teleportedPayload.targetInstanceId === opening.snowLeopardInstanceId
      && teleportedPayload.targetSiteInstanceId === opening.southSiteInstanceId
      && from?.cell === 'C4'
      && from.region === 'surface'
      && to?.cell === 'C1'
      && to.region === 'surface',
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactAllySitePair: pairs.length === 1,
    manaPaid: manaBefore - session.state.players.north.mana,
    noPathTeleport,
    replayVerified: verifyGameReplay(session),
    siteUnchanged: canonicalJson(session.state.realm.sites.C4 as unknown as JsonValue)
      === canonicalJson(sourceSiteBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.sites.C1 as unknown as JsonValue)
        === canonicalJson(targetSiteBefore as unknown as JsonValue),
    snowLeopard: input.stealthTargetMinion.name,
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.teleportInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.teleportInstanceId),
    teleportedToOpponentSiteSurface: before.location === 'C4'
      && before.region === 'surface'
      && after?.location === 'C1'
      && after.region === 'surface'
      && targetSiteBefore.controller === 'south',
    teleport: input.teleport.name,
    unitStatePreserved: after !== undefined
      && after.controller === before.controller
      && after.damage === before.damage
      && after.owner === before.owner
      && after.stealthed === before.stealthed
      && after.summoningSickness === before.summoningSickness
      && after.tapped === before.tapped
      && after.warded === before.warded,
  });
}

function runAirborne(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airborne'] {
  const opening = findAirborneOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.airborneInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[2]
      && descriptor.cell === 'B2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.groundInstanceId
      && descriptor.cell === 'B2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const move = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.airborneInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,B2');
  const diagonalMove = move.descriptor.kind === 'move-and-attack'
    && move.descriptor.path.length === 2;
  session = accept(session, move);
  const airborneCanAttackGround = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.groundInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const groundCannotIntercept = session.state.phase === 'main';
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.groundInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'B2');
  const groundCannotAttackAirborne = legalGameActions(session.state, 'south').every(({ descriptor }) =>
    descriptor.kind !== 'declare-attack'
      || descriptor.target.kind !== 'minion'
      || descriptor.target.instanceId !== opening.airborneInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    airborneCanAttackGround,
    airborneMinion: opening.names.get(input.airborneMinion.stableId) ?? input.airborneMinion.stableId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    diagonalMove,
    groundCannotAttackAirborne,
    groundCannotIntercept,
    groundMinion:
      opening.names.get(input.airborneTargetMinion.stableId) ?? input.airborneTargetMinion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function runAirMovementTwo(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airMovementTwo'] {
  const opening = findAirborneOpening(input, 'movement-two');
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.airborneInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[2]
      && descriptor.cell === 'B2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.groundInstanceId
      && descriptor.cell === 'B2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const legalMoves = legalGameActions(session.state, 'north');
  const returningPathAvailable = legalMoves.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.airborneInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4,C3');
  const repeatedStepUnavailable = !legalMoves.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.airborneInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4,C3,C4');
  const move = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.airborneInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4,B3,B2');
  const threeStepAirbornePath = move.descriptor.kind === 'move-and-attack'
    && move.descriptor.path.length === 4;
  session = accept(session, move);
  const attackAvailableAfterThreeSteps = legalGameActions(session.state, 'north')
    .some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === opening.groundInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackAvailableAfterThreeSteps,
    deck: deckList(opening.manifest.decks.north, opening.names),
    movementMinion:
      opening.names.get(input.movementTwoMinion.stableId) ?? input.movementTwoMinion.stableId,
    replayVerified: verifyGameReplay(session),
    repeatedStepUnavailable,
    returningPathAvailable,
    seed: opening.seed,
    threeStepAirbornePath,
  });
}

function runAirVoidwalk(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airVoidwalk'] {
  const opening = findAirVoidwalkOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const targetWasVoid = session.state.realm.sites.B2 === undefined;
  const summons = legalGameActions(session.state, 'north');
  const matches = (cardInstanceId: string, cell: string, region: 'surface' | 'void'): boolean =>
    summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === cardInstanceId
      && descriptor.cell === cell
      && (descriptor.region ?? 'surface') === region);
  const surfaceSummonAvailable = matches(opening.featuredInstanceId, 'C3', 'surface');
  const voidSummonAvailable = matches(opening.featuredInstanceId, 'B2', 'void');
  const forsakenOuterVoidAvailable = matches(opening.restrictedInstanceId, 'A2', 'void');
  const forsakenInnerVoidUnavailable = !matches(opening.restrictedInstanceId, 'B2', 'void');
  const forsakenInnerSurfaceUnavailable = !matches(opening.restrictedInstanceId, 'C3', 'surface');
  const nonVoidSurfaceAvailable = matches(opening.comparisonInstanceId, 'C3', 'surface');
  const nonVoidVoidUnavailable = !matches(opening.comparisonInstanceId, 'B2', 'void');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.featuredInstanceId
    && descriptor.cell === 'B2'
    && descriptor.region === 'void');
  const summonedInVoid = session.state.realm.units.some(({ instanceId, location, region }) =>
    instanceId === opening.featuredInstanceId && location === 'B2' && region === 'void');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const moves = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === opening.featuredInstanceId);
  const hasPath = (path: string): boolean => moves.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',') === path);
  const voidMoveAvailable = hasPath('B2/void,B1/void');
  const surfaceExitAvailable = hasPath('B2/void,C2/surface');
  const subsurfaceExitUnavailable = !hasPath('B2/void,C2/underground')
    && !hasPath('B2/void,C2/underwater');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.featuredInstanceId
    && descriptor.to.cell === 'C2'
    && descriptor.to.region === 'surface');
  const siteTargetAvailableAfterExit = legalGameActions(session.state, 'north')
    .some(({ descriptor }) => descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    forsaken: opening.names.get(input.forsaken.stableId) ?? input.forsaken.stableId,
    forsakenInnerSurfaceUnavailable,
    forsakenInnerVoidUnavailable,
    forsakenOuterVoidAvailable,
    nonVoidSurfaceAvailable,
    nonVoidVoidUnavailable,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    siteTargetAvailableAfterExit,
    subsurfaceExitUnavailable,
    summonedInVoid,
    surfaceExitAvailable,
    surfaceSummonAvailable,
    targetWasVoid,
    voidMoveAvailable,
    voidSummonAvailable,
    voidwalkMinion: opening.names.get(input.voidwalkMinion.stableId) ?? input.voidwalkMinion.stableId,
  });
}

function runAirGenesisSpell(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airGenesisSpell'] {
  const opening = findAirGenesisSpellOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');

  const before = session.state.players.north;
  const drawn = before.spellbook[0];
  if (!drawn) throw new Error('private Air Genesis spell-draw scenario lacks a spell to draw');
  const summoned = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.featuredInstanceId
      && descriptor.cell === 'B3'));
  if (!summoned.accepted) throw new Error('private Air Genesis spell-draw summon was rejected');
  session = summoned.session;
  const after = session.state.players.north;
  const drewSpell = after.spellbook.length === before.spellbook.length - 1
    && after.hand.spellbook.some(({ instanceId }) => instanceId === drawn.instanceId)
    && summoned.receipt.events.map(({ type }) => type).join(',') === 'minion-summoned,spell-drawn';
  const opponentHand = observeGame(session.state, 'south').players.north.hand.spellbook;
  const hiddenFromOpponent = typeof opponentHand === 'number'
    && !canonicalJson(summoned.receipt.events as unknown as JsonValue).includes(drawn.instanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    drewSpell,
    genesisMinion:
      opening.names.get(input.genesisSpellMinion.stableId) ?? input.genesisSpellMinion.stableId,
    handSizePreserved: after.hand.spellbook.length === before.hand.spellbook.length,
    hiddenFromOpponent,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function runAirSpellcasterFreeze(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airSpellcasterFreeze'] {
  const opening = findAirSpellcasterFreezeOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northAirSiteInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southWaterSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.seravaInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northWaterSiteInstanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'C2');

  const beforeGenesis = session.state.players.north;
  const genesisCard = beforeGenesis.spellbook[0];
  if (!genesisCard) throw new Error('private Spellcaster scenario lacks its Genesis spell draw');
  const summon = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.apprenticeWizardInstanceId
      && descriptor.cell === 'C2'));
  if (!summon.accepted) throw new Error('private Spellcaster summon was rejected');
  session = summon.session;
  const afterGenesis = session.state.players.north;
  const genesisDrewSpell = summon.receipt.events.map(({ type }) => type).join(',')
      === 'minion-summoned,spell-drawn'
    && afterGenesis.spellbook.length === beforeGenesis.spellbook.length - 1
    && afterGenesis.hand.spellbook.length === beforeGenesis.hand.spellbook.length
    && afterGenesis.hand.spellbook.some(({ instanceId }) => instanceId === genesisCard.instanceId);

  const wizardBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.apprenticeWizardInstanceId);
  const seravaBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seravaInstanceId);
  if (!wizardBefore || !seravaBefore) {
    throw new Error('private Spellcaster Freeze setup lacks its Wizard or target');
  }
  const avatarInstanceId = session.state.players.north.avatar.card.instanceId;
  const targetActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.freezeInstanceId
      && descriptor.target?.kind === 'minion'
      && descriptor.target.instanceId === opening.seravaInstanceId
      && descriptor.target.seat === 'south');
  const selected = targetActions[0];
  if (!selected || selected.descriptor.kind !== 'cast-magic' || targetActions.length !== 1) {
    throw new Error('private caster-relative Freeze action is not exactly available');
  }
  const exactCasterRelativeAction = selected.descriptor.casterInstanceId
      === opening.apprenticeWizardInstanceId
    && targetActions.every(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.casterInstanceId !== avatarInstanceId)
    && wizardBefore.location === 'C2'
    && wizardBefore.region === 'surface'
    && seravaBefore.location === 'C1'
    && seravaBefore.region === 'surface'
    && session.state.players.north.avatar.location === 'C4'
    && session.state.players.north.avatar.region === 'surface';
  const wizardCastWhileSummoningSick = wizardBefore.summoningSickness
    && !wizardBefore.tapped;
  const manaBefore = session.state.players.north.mana;
  session = accept(session, selected);
  const manaAfter = session.state.players.north.mana;

  const wizardAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.apprenticeWizardInstanceId);
  const seravaAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seravaInstanceId);
  const observedSerava = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === opening.seravaInstanceId);
  const receipt = session.transcript.at(-1);
  const events = receipt?.events ?? [];
  const castPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const disabledPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const resolvedPayload = events[2] && isJsonRecord(events[2].payload)
    ? events[2].payload
    : undefined;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    apprenticeWizard: input.genesisSpellMinion.name,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'magic-cast,minion-disabled,magic-resolved'
      && castPayload?.casterInstanceId === opening.apprenticeWizardInstanceId
      && castPayload.instanceId === opening.freezeInstanceId
      && castPayload.manaPaid === 1
      && castPayload.seat === 'north'
      && castPayload.targetInstanceId === opening.seravaInstanceId
      && castPayload.targetSeat === 'south'
      && disabledPayload?.expiresAtSeat === 'north'
      && disabledPayload.instanceId === opening.seravaInstanceId
      && disabledPayload.seat === 'south'
      && disabledPayload.sourceInstanceId === opening.freezeInstanceId
      && disabledPayload.stealthRemoved === false
      && disabledPayload.wardRemoved === false
      && resolvedPayload?.instanceId === opening.freezeInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactCasterRelativeAction,
    freeze: input.freeze.name,
    genesisDrewSpell,
    manaPaid: manaBefore - manaAfter,
    noRandomDraws: summon.receipt.randomDraws.length === 0
      && receipt?.randomDraws.length === 0,
    replayVerified: verifyGameReplay(session),
    seravaDisabled: observedSerava?.disabled === true
      && seravaAfter?.disableEffects?.length === 1
      && seravaAfter.disableEffects[0]?.expiresAtSeat === 'north'
      && seravaAfter.disableEffects[0].sourceInstanceId === opening.freezeInstanceId,
    seravaTownsfolk: input.seravaTownsfolk.name,
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.freezeInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.freezeInstanceId),
    wizardCastWhileSummoningSick,
    wizardStatePreserved: wizardAfter !== undefined
      && wizardAfter.cardId === wizardBefore.cardId
      && wizardAfter.controller === wizardBefore.controller
      && wizardAfter.damage === 0
      && wizardAfter.location === 'C2'
      && wizardAfter.owner === wizardBefore.owner
      && wizardAfter.region === 'surface'
      && wizardAfter.stealthed === wizardBefore.stealthed
      && wizardAfter.summoningSickness
      && !wizardAfter.tapped
      && wizardAfter.warded === wizardBefore.warded,
  });
}

function runAirLeyline(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airLeyline'] {
  const opening = findAirLeylineOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const beforeFirst = session.state.players.north;
  const first = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.hengeInstanceIds[0]
      && descriptor.cell === 'C4'));
  if (!first.accepted) throw new Error('private first Leyline Henge play was rejected');
  session = first.session;
  const afterFirst = session.state.players.north;
  const firstHengeDrewNothing = afterFirst.spellbook.length === beforeFirst.spellbook.length
    && afterFirst.hand.spellbook.length === beforeFirst.hand.spellbook.length
    && first.receipt.events.map(({ type }) => type).join(',') === 'site-played';
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const before = session.state.players.north;
  const drawn = before.spellbook[0];
  if (!drawn) throw new Error('private Leyline Henge scenario lacks a spell to draw');
  const second = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.hengeInstanceIds[1]
      && descriptor.cell === 'C3'));
  if (!second.accepted) throw new Error('private second Leyline Henge play was rejected');
  session = second.session;
  const after = session.state.players.north;
  const drawEvent = second.receipt.events[1];
  const genesisDrewOne = after.spellbook.length === before.spellbook.length - 1
    && after.hand.spellbook.length === before.hand.spellbook.length + 1
    && after.hand.spellbook.some(({ instanceId }) => instanceId === drawn.instanceId)
    && second.receipt.events.map(({ type }) => type).join(',') === 'site-played,spell-drawn'
    && drawEvent?.type === 'spell-drawn'
    && isJsonRecord(drawEvent.payload)
    && drawEvent.payload.sourceInstanceId === opening.hengeInstanceIds[1];
  const opponentHand = observeGame(session.state, 'south').players.north.hand.spellbook;
  const hiddenFromOpponent = typeof opponentHand === 'number'
    && !canonicalJson(second.receipt.events as unknown as JsonValue).includes(drawn.instanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    firstHengeDrewNothing,
    genesisDrewOne,
    henge: opening.names.get(input.leylineHenge.stableId) ?? input.leylineHenge.stableId,
    hiddenFromOpponent,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

function runStealth(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['stealth'] {
  const opening = findStealthOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.groundInstanceId
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.stealthInstanceId
      && descriptor.cell === 'C3');
  const enteredStealthed = session.state.realm.units
    .some(({ instanceId, stealthed }) => instanceId === opening.stealthInstanceId && stealthed);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.groundInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3');
  const groundTargets = legalGameActions(session.state, 'south');
  const protectedUnit = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.stealthInstanceId);
  const groundCouldNotAttack = protectedUnit?.location === 'C3'
    && protectedUnit.stealthed
    && groundTargets.some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site')
    && groundTargets.every(({ descriptor }) =>
      descriptor.kind !== 'declare-attack'
        || descriptor.target.kind !== 'minion'
        || descriptor.target.instanceId !== opening.stealthInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.stealthInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.groundInstanceId);
  const finalEvents = session.transcript.at(-1)?.events ?? [];
  const attackSkippedDefend = session.state.phase === 'main'
    && session.state.decisionSeat === 'north'
    && finalEvents.some(({ type }) => type === 'attack-declared')
    && finalEvents.every(({ type }) => type !== 'defend-window-closed');
  const strikeIndex = finalEvents.findIndex(({ type }) => type === 'strike-damage-allocated');
  const stealthLostIndex = finalEvents.findIndex(({ type }) => type === 'stealth-lost');
  const deathIndex = finalEvents.findIndex(({ type }) => type === 'minion-died');
  const stealthLostAfterAttack = session.state.realm.units.some(({ instanceId, stealthed }) =>
    instanceId === opening.stealthInstanceId && !stealthed)
    && finalEvents.some(({ payload, type }) =>
      type === 'stealth-lost'
        && isJsonRecord(payload)
        && payload.instanceId === opening.stealthInstanceId)
    && strikeIndex >= 0
    && strikeIndex < stealthLostIndex
    && stealthLostIndex < deathIndex;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackSkippedDefend,
    deck: deckList(opening.manifest.decks.north, opening.names),
    enteredStealthed,
    groundCouldNotAttack,
    groundMinion:
      opening.names.get(input.stealthTargetMinion.stableId) ?? input.stealthTargetMinion.stableId,
    groundMinionDied: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.groundInstanceId),
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    stealthLostAfterAttack,
    stealthMinion: opening.names.get(input.stealthMinion.stableId) ?? input.stealthMinion.stableId,
  });
}

function runAirSummoning(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airSummoning'] {
  const opening = findAirSummoningOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  const cells = ['C4', 'C3', 'B4', 'D4', 'B3'] as const;

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  for (let index = 1; index < cells.length; index += 1) {
    take(({ descriptor }) =>
      descriptor.kind === 'draw'
        && descriptor.zone === (index < 3 ? 'spellbook' : 'atlas'));
    take(({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === opening.northSiteInstanceIds[index]
        && descriptor.cell === cells[index]);
    if (index === cells.length - 1) break;
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
  }

  const actions = legalGameActions(session.state, 'north');
  const ordinaryActions = actions.filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.ordinaryInstanceId);
  const ordinaryRestricted = ordinaryActions.some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4')
    && ordinaryActions.every(({ descriptor }) =>
      descriptor.kind !== 'summon-minion' || descriptor.cell !== 'C1');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.roamingInstanceId
      && descriptor.cell === 'C1');
  const roamingUnit = session.state.realm.units
    .find(({ instanceId }) => instanceId === opening.roamingInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    ordinaryRestricted,
    replayVerified: verifyGameReplay(session),
    roamingMinion: opening.names.get(input.roamingMinion.stableId) ?? input.roamingMinion.stableId,
    seed: opening.seed,
    summonedAtEnemySite:
      roamingUnit?.location === 'C1' && session.state.realm.sites.C1?.controller === 'south',
  });
}

function runFireGenesisLifeLoss(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireGenesisLifeLoss'] {
  const opening = findFireGenesisLifeLossOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const northBefore = session.state.players.north;
  const southBefore = session.state.players.south;
  const sitesBefore = session.state.realm.sites;
  const summoned = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.demonInstanceId
      && descriptor.cell === 'C3'
      && descriptor.region === undefined));
  if (!summoned.accepted) throw new Error('private Lesser Blood Demon summon was rejected');
  session = summoned.session;
  const northAfter = session.state.players.north;
  const demon = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.demonInstanceId);
  const events = summoned.receipt.events;
  const summonedPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const lifePayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'minion-summoned,avatar-life-lost'
      && summonedPayload?.instanceId === opening.demonInstanceId
      && lifePayload?.amount === 2
      && lifePayload.life === 18
      && lifePayload.seat === 'north'
      && lifePayload.sourceInstanceId === opening.demonInstanceId,
    cemeteriesUnchanged: northAfter.cemetery.length === northBefore.cemetery.length
      && session.state.players.south.cemetery.length === southBefore.cemetery.length
      && northAfter.cemetery.every(({ instanceId }) => instanceId !== opening.demonInstanceId),
    deck: deckList(opening.manifest.decks.north, opening.names),
    lesserBloodDemon: input.lesserBloodDemon.name,
    lifeAfter: northAfter.avatar.life,
    lifeLost: northBefore.avatar.life - northAfter.avatar.life,
    noDamageDeathOrTerminalEvents: events.every(({ type }) =>
      type !== 'damage-dealt'
        && type !== 'minion-died'
        && type !== 'death-blow'
        && type !== 'avatar-reached-deaths-door'
        && type !== 'game-ended')
      && session.state.terminal.status === 'active',
    noRandomDraws: summoned.receipt.randomDraws.length === 0,
    otherStatePreserved:
      canonicalJson(session.state.players.south as unknown as JsonValue)
        === canonicalJson(southBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.sites as unknown as JsonValue)
        === canonicalJson(sitesBefore as unknown as JsonValue)
      && canonicalJson(northAfter.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.spellbook as unknown as JsonValue)
      && canonicalJson(northAfter.hand.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.hand.atlas as unknown as JsonValue)
      && northAfter.avatar.card.instanceId === northBefore.avatar.card.instanceId
      && northAfter.avatar.location === northBefore.avatar.location
      && northAfter.avatar.region === northBefore.avatar.region
      && northAfter.avatar.tapped === northBefore.avatar.tapped
      && session.state.phase === 'main',
    replayVerified: verifyGameReplay(session),
    summonedAtC3: demon?.controller === 'north'
      && demon.location === 'C3'
      && demon.owner === 'north'
      && demon.region === 'surface',
  });
}

function runFireIgnited(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireIgnited'] {
  const opening = findFireIgnitedOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const manaBefore = session.state.players.north.mana;
  const northAvatarLifeBefore = session.state.players.north.avatar.life;
  const southAvatarLifeBefore = session.state.players.south.avatar.life;
  const summoned = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.ignitedInstanceId
      && descriptor.cell === 'C3'
      && descriptor.region === undefined));
  if (!summoned.accepted) throw new Error('private Ignited summon was rejected');
  session = summoned.session;
  const ignited = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.ignitedInstanceId);
  if (!ignited) throw new Error('private Ignited summon did not enter the realm');
  const summonedPayload = summoned.receipt.events[0]
    && isJsonRecord(summoned.receipt.events[0].payload)
    ? summoned.receipt.events[0].payload
    : undefined;
  const chargeActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.ignitedInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4');
  const sitesBeforeEnd = canonicalJson(session.state.realm.sites as unknown as JsonValue);
  const northCemeteryBefore = session.state.players.north.cemetery.length;
  const southCemeteryBefore = canonicalJson(
    session.state.players.south.cemetery as unknown as JsonValue,
  );
  const otherUnitIdsBefore = session.state.realm.units
    .filter(({ instanceId }) => instanceId !== opening.ignitedInstanceId)
    .map(({ instanceId }) => instanceId)
    .sort();
  const endActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'end-turn');
  const ended = endActions[0] && stepGame(session, endActions[0]);
  if (!ended || !ended.accepted) throw new Error('private Ignited end turn was rejected');
  session = ended.session;

  const endEvents = ended.receipt.events;
  const deathPayload = endEvents[0] && isJsonRecord(endEvents[0].payload)
    ? endEvents[0].payload
    : undefined;
  const turnEndedPayload = endEvents[1] && isJsonRecord(endEvents[1].payload)
    ? endEvents[1].payload
    : undefined;
  const turnStartedPayload = endEvents[2] && isJsonRecord(endEvents[2].payload)
    ? endEvents[2].payload
    : undefined;
  const cemeteryEntries = session.state.players.north.cemetery.filter(({ instanceId }) =>
    instanceId === opening.ignitedInstanceId);
  const otherUnitIdsAfter = session.state.realm.units.map(({ instanceId }) => instanceId).sort();

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: summoned.receipt.events.map(({ type }) => type).join(',')
      === 'minion-summoned'
      && summonedPayload?.cardId === input.ignited.stableId
      && summonedPayload.instanceId === opening.ignitedInstanceId
      && summonedPayload.cell === 'C3'
      && summonedPayload.manaPaid === 2
      && summonedPayload.seat === 'north'
      && endEvents.map(({ type }) => type).join(',')
        === 'minion-died,turn-ended,turn-started'
      && deathPayload?.cardId === input.ignited.stableId
      && deathPayload.instanceId === opening.ignitedInstanceId
      && deathPayload.owner === 'north'
      && turnEndedPayload?.seat === 'north'
      && turnStartedPayload?.seat === 'south',
    chargeActionAvailableImmediately: chargeActions.length === 1,
    deck: deckList(opening.manifest.decks.north, opening.names),
    ignited: input.ignited.name,
    manaPaid: manaBefore - summoned.session.state.players.north.mana,
    mandatoryDeathAndCemetery: endActions.length === 1
      && session.state.realm.units.every(({ instanceId }) =>
        instanceId !== opening.ignitedInstanceId)
      && cemeteryEntries.length === 1
      && cemeteryEntries[0]?.cardId === input.ignited.stableId
      && cemeteryEntries[0].owner === 'north'
      && cemeteryEntries[0].source === 'spellbook'
      && session.state.players.south.cemetery.every(({ instanceId }) =>
        instanceId !== opening.ignitedInstanceId)
      && session.state.terminal.status === 'active',
    noDeathriteDamageTerminalOrRandomEffects: endEvents.every(({ type }) =>
      type !== 'site-drawn'
        && type !== 'avatar-healed'
        && type !== 'damage-dealt'
        && type !== 'death-blow'
        && type !== 'game-ended')
      && session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    otherStatePreserved:
      canonicalJson(session.state.realm.sites as unknown as JsonValue) === sitesBeforeEnd
      && session.state.players.north.avatar.life === northAvatarLifeBefore
      && session.state.players.south.avatar.life === southAvatarLifeBefore
      && session.state.players.north.cemetery.length === northCemeteryBefore + 1
      && canonicalJson(session.state.players.south.cemetery as unknown as JsonValue)
        === southCemeteryBefore
      && canonicalJson(otherUnitIdsAfter as unknown as JsonValue)
        === canonicalJson(otherUnitIdsBefore as unknown as JsonValue),
    replayVerified: verifyGameReplay(session),
    summonedStateVerified: ignited.cardId === input.ignited.stableId
      && ignited.controller === 'north'
      && ignited.damage === 0
      && ignited.location === 'C3'
      && ignited.owner === 'north'
      && ignited.region === 'surface'
      && ignited.summoningSickness
      && !ignited.tapped
      && summoned.session.state.players.north.hand.spellbook.every(({ instanceId }) =>
        instanceId !== opening.ignitedInstanceId),
  });
}

function runFireCharge(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireCharge'] {
  const opening = findFireChargeOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.raalInstanceId
    && descriptor.cell === 'C3');

  const hasPositiveRaalMove = (candidate: GameSession): boolean =>
    legalGameActions(candidate.state, 'north').some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === opening.raalInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4');
  const before = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.raalInstanceId);
  if (!before) throw new Error('private temporary Charge setup lacks Raal Dromedary');
  const moveUnavailableBeforeCharge = legalGameActions(session.state, 'north')
    .every(({ descriptor }) =>
      descriptor.kind !== 'move-and-attack'
        || descriptor.unitInstanceId !== opening.raalInstanceId);
  const chargeActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.chargeInstanceId
      && descriptor.ally?.kind === 'minion'
      && descriptor.ally.instanceId === opening.raalInstanceId
      && descriptor.ally.seat === 'north'
      && descriptor.target === undefined
      && descriptor.targetLocation === undefined
      && descriptor.targetSiteInstanceId === undefined
      && descriptor.cemeteryMinionInstanceId === undefined);
  const chosenCharge = chargeActions[0];
  if (!chosenCharge || chargeActions.length !== 1) {
    throw new Error('private non-target Charge ally choice is not exactly available');
  }
  const manaBefore = session.state.players.north.mana;
  session = accept(session, chosenCharge);
  const after = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.raalInstanceId);
  if (!after) throw new Error('private temporary Charge removed its ally');
  const castEvents = session.transcript.at(-1)?.events ?? [];
  const castPayload = castEvents[0] && isJsonRecord(castEvents[0].payload)
    ? castEvents[0].payload
    : undefined;
  const grantedPayload = castEvents[1] && isJsonRecord(castEvents[1].payload)
    ? castEvents[1].payload
    : undefined;
  const resolvedPayload = castEvents[2] && isJsonRecord(castEvents[2].payload)
    ? castEvents[2].payload
    : undefined;
  const temporaryChargeRecorded = after.temporaryChargeSources?.length === 1
    && after.temporaryChargeSources[0] === opening.chargeInstanceId;
  const moveAvailableAfterCharge = hasPositiveRaalMove(session);
  const unitStatePreservedOnGrant = after.cardId === before.cardId
    && after.controller === before.controller
    && after.damage === before.damage
    && after.location === before.location
    && after.owner === before.owner
    && after.region === before.region
    && after.stealthed === before.stealthed
    && after.summoningSickness === before.summoningSickness
    && after.tapped === before.tapped
    && after.warded === before.warded;

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  const expired = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.raalInstanceId);
  const expiryEvents = session.transcript.at(-1)?.events ?? [];
  const expiryIndex = expiryEvents.findIndex(({ payload, type }) =>
    type === 'charge-expired'
      && isJsonRecord(payload)
      && payload.instanceId === opening.raalInstanceId
      && payload.seat === 'north'
      && payload.sourceInstanceId === opening.chargeInstanceId);
  const turnEndedIndex = expiryEvents.findIndex(({ payload, type }) =>
    type === 'turn-ended' && isJsonRecord(payload) && payload.seat === 'north');
  const expiredAtEndOfTurn = expired !== undefined
    && expired.temporaryChargeSources === undefined
    && expiryIndex >= 0
    && expiryIndex < turnEndedIndex;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: castEvents.map(({ type }) => type).join(',')
      === 'magic-cast,charge-granted,magic-resolved'
      && castPayload?.instanceId === opening.chargeInstanceId
      && castPayload.manaPaid === 1
      && castPayload.seat === 'north'
      && castPayload.allyInstanceId === opening.raalInstanceId
      && castPayload.allySeat === 'north'
      && grantedPayload?.instanceId === opening.raalInstanceId
      && grantedPayload.seat === 'north'
      && grantedPayload.sourceInstanceId === opening.chargeInstanceId
      && resolvedPayload?.instanceId === opening.chargeInstanceId
      && expiryIndex >= 0
      && expiryIndex < turnEndedIndex,
    charge: input.chargeMagic.name,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactNonTargetAllyChoice: chargeActions.length === 1,
    expiredAtEndOfTurn,
    manaPaid: manaBefore - session.state.players.north.mana,
    moveAvailableAfterCharge,
    moveUnavailableBeforeCharge,
    raalDromedary: input.raalDromedary.name,
    replayVerified: verifyGameReplay(session),
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.chargeInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.chargeInstanceId),
    temporaryChargeRecorded,
    unitStatePreservedOnGrant,
  });
}

function runFireMinorExplosion(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireMinorExplosion'] {
  const opening = findFireMinorExplosionOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.firstRaalInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.secondRaalInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'C2');

  const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.minorExplosionInstanceId);
  const targetCells = casts.flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.targetLocation?.region === 'surface'
      ? [descriptor.targetLocation.cell]
      : []).sort();
  const selected = casts.filter(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.targetLocation?.cell === 'C4'
    && descriptor.targetLocation.region === 'surface');
  if (selected.length !== 1 || !selected[0]) {
    throw new Error('private location-wide damage Magic target is unavailable');
  }
  const targetIds = [opening.firstRaalInstanceId, opening.secondRaalInstanceId].sort();
  const occupantsBefore = session.state.realm.units.filter(({ instanceId, location, region }) =>
    targetIds.includes(instanceId) && location === 'C4' && region === 'surface');
  const avatarLifeBefore = session.state.players.north.avatar.life;
  const manaBefore = session.state.players.north.mana;
  session = accept(session, selected[0]);

  const receipt = session.transcript.at(-1);
  const events = receipt?.events ?? [];
  const castPayload = events[0] && isJsonRecord(events[0].payload) ? events[0].payload : undefined;
  const castTarget = castPayload && isJsonRecord(castPayload.targetLocation)
    ? castPayload.targetLocation
    : undefined;
  const resolvedEvent = events.at(-1);
  const resolvedPayload = resolvedEvent && isJsonRecord(resolvedEvent.payload)
    ? resolvedEvent.payload
    : undefined;
  const allocations = events.filter(({ payload, type }) => type === 'magic-damage-allocated'
    && isJsonRecord(payload)
    && targetIds.includes(String(payload.targetInstanceId)));
  const damageEvents = events.filter(({ payload, type }) => type === 'damage-dealt'
    && isJsonRecord(payload)
    && targetIds.includes(String(payload.instanceId)));
  const deathEvents = events.filter(({ payload, type }) => type === 'minion-died'
    && isJsonRecord(payload)
    && targetIds.includes(String(payload.instanceId)));
  const lastAllocationIndex = Math.max(...allocations.map((event) => events.indexOf(event)));
  const firstDamageIndex = Math.min(...damageEvents.map((event) => events.indexOf(event)));
  const lastDamageIndex = Math.max(...damageEvents.map((event) => events.indexOf(event)));
  const firstDeathIndex = Math.min(...deathEvents.map((event) => events.indexOf(event)));

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarTookThreeDamage: avatarLifeBefore - session.state.players.north.avatar.life === 3,
    causalEventsVerified: events[0]?.type === 'magic-cast'
      && castPayload?.instanceId === opening.minorExplosionInstanceId
      && castPayload.manaPaid === 3
      && castPayload.seat === 'north'
      && castTarget?.cell === 'C4'
      && castTarget.region === 'surface'
      && allocations.every(({ payload }) => isJsonRecord(payload)
        && payload.amount === 3
        && payload.sourceInstanceId === opening.minorExplosionInstanceId)
      && damageEvents.every(({ payload }) => isJsonRecord(payload) && payload.amount === 3)
      && deathEvents.every(({ payload }) => isJsonRecord(payload)
        && payload.cardId === input.raalDromedary.stableId
        && payload.owner === 'north')
      && events.at(-1)?.type === 'magic-resolved'
      && resolvedPayload?.instanceId === opening.minorExplosionInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactLocationTargetAvailable: selected.length === 1 && occupantsBefore.length === 2,
    manaPaid: manaBefore - session.state.players.north.mana,
    minorExplosion: input.minorExplosion.name,
    noRandomDraws: receipt?.randomDraws.length === 0,
    raalDromedary: input.raalDromedary.name,
    replayVerified: verifyGameReplay(session),
    simultaneousDamageVerified: allocations.length === 2
      && damageEvents.length === 2
      && deathEvents.length === 2
      && lastAllocationIndex < firstDamageIndex
      && lastDamageIndex < firstDeathIndex,
    spellEnteredCemetery: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.minorExplosionInstanceId),
    targetWithinTwoSteps: targetCells.join(',') === 'C2,C3,C4',
    twoMinionsDied: session.state.realm.units
      .every(({ instanceId }) => !targetIds.includes(instanceId)),
    twoMinionsEnteredCemetery: targetIds.every((instanceId) =>
      session.state.players.north.cemetery.some((card) => card.instanceId === instanceId)),
  });
}

function runFireResponse(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireResponse'] {
  const opening = findFireOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[3]
      && descriptor.cell === 'D4');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.lionInstanceId
      && descriptor.cell === 'C3');
  const chargeMoveAndAttack = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.lionInstanceId
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.lionInstanceId
      && descriptor.to.cell === 'C2');
  const lionTargets = legalGameActions(session.state, 'north');
  const unitTargetAvailable = lionTargets.some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.attackerInstanceId);
  const siteTargetUnavailable = lionTargets.every(({ descriptor }) =>
    descriptor.kind !== 'declare-attack' || descriptor.target.kind !== 'site');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.lumberingInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const interceptUnavailable = legalGameActions(session.state, 'north').every(({ descriptor }) =>
    descriptor.kind !== 'intercept' || descriptor.unitInstanceId !== opening.lumberingInstanceId);
  if (session.state.phase === 'intercept') {
    take(({ descriptor }) => descriptor.kind === 'close-intercept');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
  const defendUnavailable = legalGameActions(session.state, 'north').every(({ descriptor }) =>
    descriptor.kind !== 'defend' || descriptor.unitInstanceId !== opening.lumberingInstanceId);
  take(({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    chargeMoveAndAttack,
    deck: deckList(opening.manifest.decks.north, opening.names),
    defendUnavailable,
    interceptUnavailable,
    lumberingGiant:
      opening.names.get(input.lumberingMinion.stableId) ?? input.lumberingMinion.stableId,
    monstrousLion:
      opening.names.get(input.monstrousLion.stableId) ?? input.monstrousLion.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    siteTargetUnavailable,
    unitTargetAvailable,
  });
}

function runWaterSidewaysMovement(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterSidewaysMovement'] {
  const opening = findWaterOpening(input, 'water-sideways');
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.featuredInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const crabActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.featuredInstanceId);
  const hasPath = (cells: string): boolean => crabActions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.path.map(({ cell }) => cell).join(',') === cells);
  const sidewaysPathAvailable = session.state.realm.sites.B3 !== undefined
    && hasPath('C3,B3');
  const forwardPathUnavailable = session.state.realm.sites.C2 !== undefined
    && !hasPath('C3,C2');
  const backwardPathUnavailable = session.state.realm.sites.C4 !== undefined
    && !hasPath('C3,C4');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.featuredInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,B3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    backwardPathUnavailable,
    deck: deckList(opening.manifest.decks.north, opening.names),
    forwardPathUnavailable,
    replayVerified: verifyGameReplay(session),
    sedgeCrabs: opening.names.get(input.sedgeCrabs.stableId) ?? input.sedgeCrabs.stableId,
    seed: opening.seed,
    sidewaysPathAvailable,
  });
}

function runWaterSubmerge(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterSubmerge'] {
  const opening = findWaterOpening(input, 'water-submerge');
  if (!opening.comparisonInstanceId || !opening.northSiteInstanceIds[2]) {
    throw new Error('private Water Submerge opening is incomplete');
  }
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');

  const summons = legalGameActions(session.state, 'north');
  const matches = (cardInstanceId: string, region: 'surface' | 'underwater'): boolean =>
    summons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === cardInstanceId
        && descriptor.cell === 'C3'
        && (descriptor.region ?? 'surface') === region);
  const surfaceSummonAvailable = matches(opening.featuredInstanceId, 'surface');
  const underwaterSummonAvailable = matches(opening.featuredInstanceId, 'underwater');
  const nonSubmergeSurfaceAvailable = matches(opening.comparisonInstanceId, 'surface');
  const nonSubmergeUnderwaterUnavailable = !matches(opening.comparisonInstanceId, 'underwater');
  const targetSite = session.state.realm.sites.C3;
  const targetDefinition = targetSite && !('rubble' in targetSite)
    ? session.state.cards[targetSite.cardId]
    : undefined;
  const targetIsWaterSite = targetDefinition?.cardType === 'site'
    && targetDefinition.elements.includes('water');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.featuredInstanceId
      && descriptor.cell === 'C3'
      && descriptor.region === 'underwater');
  const summonedUnderwater = session.state.realm.units.some(({ instanceId, location, region }) =>
    instanceId === opening.featuredInstanceId && location === 'C3' && region === 'underwater');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    nonSubmergeSurfaceAvailable,
    nonSubmergeUnderwaterUnavailable,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    submergeMinion:
      opening.names.get(input.submergeMinion.stableId) ?? input.submergeMinion.stableId,
    summonedUnderwater,
    surfaceSummonAvailable,
    targetIsWaterSite,
    underwaterSummonAvailable,
  });
}

function runWaterDrown(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterDrown'] {
  const opening = findWaterDrownOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.waterSiteInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.seravaInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const manaBeforeGhostTown = session.state.players.north.mana;
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'C3');
  const ghostTownEvents = session.transcript.at(-1)?.events ?? [];
  const ghostTownManaPayload = ghostTownEvents[1] && isJsonRecord(ghostTownEvents[1].payload)
    ? ghostTownEvents[1].payload
    : undefined;
  const manaBeforeCast = session.state.players.north.mana;
  const drownActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === opening.drownInstanceId);
  const selected = drownActions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'minion'
    && descriptor.target.instanceId === opening.seravaInstanceId);
  if (!selected) throw new Error('private forced-submerge Magic target is unavailable');
  const exactTargetAvailable = drownActions.length === 1;
  session = accept(session, selected);

  const events = session.transcript.at(-1)?.events ?? [];
  const castPayload = events[0] && isJsonRecord(events[0].payload) ? events[0].payload : undefined;
  const submergedPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const deathPayload = events[2] && isJsonRecord(events[2].payload) ? events[2].payload : undefined;
  const resolvedPayload = events[3] && isJsonRecord(events[3].payload)
    ? events[3].payload
    : undefined;
  const submergedIndex = events.findIndex(({ type }) => type === 'minion-submerged');
  const deathIndex = events.findIndex(({ type }) => type === 'minion-died');
  const exactEventOrder = events.map(({ type }) => type).join(',')
    === 'magic-cast,minion-submerged,minion-died,magic-resolved';

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: exactEventOrder
      && castPayload?.instanceId === opening.drownInstanceId
      && castPayload.manaPaid === 3
      && castPayload.seat === 'north'
      && castPayload.targetInstanceId === opening.seravaInstanceId
      && castPayload.targetSeat === 'north'
      && submergedPayload?.cell === 'C4'
      && submergedPayload.instanceId === opening.seravaInstanceId
      && submergedPayload.seat === 'north'
      && submergedPayload.sourceInstanceId === opening.drownInstanceId
      && deathPayload?.cardId === input.seravaTownsfolk.stableId
      && deathPayload.instanceId === opening.seravaInstanceId
      && deathPayload.owner === 'north'
      && resolvedPayload?.instanceId === opening.drownInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    deathNotBanishmentAndGameActive: exactEventOrder
      && session.state.terminal.status === 'active',
    drown: input.drown.name,
    exactTargetAvailable,
    ghostTownManaConsumed: manaBeforeGhostTown === 1
      && manaBeforeCast === 3
      && session.state.players.north.mana === 0
      && ghostTownEvents.map(({ type }) => type).join(',') === 'site-played,mana-gained'
      && ghostTownManaPayload?.amount === 1
      && ghostTownManaPayload.seat === 'north'
      && ghostTownManaPayload.sourceInstanceId === opening.ghostTownInstanceId,
    manaPaid: manaBeforeCast - session.state.players.north.mana,
    replayVerified: verifyGameReplay(session),
    seravaTownsfolk: input.seravaTownsfolk.name,
    spellEnteredCemetery: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.drownInstanceId)
      && session.state.players.north.hand.spellbook
        .every(({ instanceId }) => instanceId !== opening.drownInstanceId),
    targetEnteredCemetery: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.seravaInstanceId),
    targetLeftRealm: session.state.realm.units
      .every(({ instanceId }) => instanceId !== opening.seravaInstanceId),
    transitionBeforeDeath: submergedIndex >= 0 && deathIndex === submergedIndex + 1,
  });
}

function runWaterDrowned(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterDrowned'] {
  const opening = findWaterDrownedOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const summons = legalGameActions(session.state, 'north');
  const matches = (cardInstanceId: string, region: 'surface' | 'underwater'): boolean =>
    summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === cardInstanceId
      && descriptor.cell === 'C3'
      && (descriptor.region ?? 'surface') === region);
  const drownedSurfaceUnavailable = !matches(opening.drownedInstanceId, 'surface');
  const drownedUnderwaterAvailable = matches(opening.drownedInstanceId, 'underwater');
  const slyFoxSurfaceAvailable = matches(opening.slyFoxInstanceId, 'surface');
  const slyFoxUnderwaterUnavailable = !matches(opening.slyFoxInstanceId, 'underwater');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.drownedInstanceId
    && descriptor.cell === 'C3'
    && descriptor.region === 'underwater');
  const summonedUnderwater = session.state.realm.units.some(({ instanceId, location, region }) =>
    instanceId === opening.drownedInstanceId && location === 'C3' && region === 'underwater');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    drowned: opening.names.get(input.drowned.stableId) ?? input.drowned.stableId,
    drownedSurfaceUnavailable,
    drownedUnderwaterAvailable,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    slyFox: opening.names.get(input.slyFox.stableId) ?? input.slyFox.stableId,
    slyFoxSurfaceAvailable,
    slyFoxUnderwaterUnavailable,
    summonedUnderwater,
  });
}

function runWaterLugbog(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterLugbog'] {
  const opening = findWaterLugbogOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southLandSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw'
    && descriptor.zone === opening.southSecondDrawZone);
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southWaterSiteInstanceId
    && descriptor.cell === 'B1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'C2');

  const summons = legalGameActions(session.state, 'north');
  const matches = (cardInstanceId: string, cell: 'B1' | 'C1' | 'C3'): boolean =>
    summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === cardInstanceId
      && descriptor.cell === cell
      && descriptor.region === undefined);
  const enemyWaterAvailable = matches(opening.lugbogInstanceId, 'B1');
  const enemyLandUnavailable = !matches(opening.lugbogInstanceId, 'C1');
  const slyFoxControlledWaterAvailable = matches(opening.slyFoxInstanceId, 'C3');
  const slyFoxEnemyWaterUnavailable = !matches(opening.slyFoxInstanceId, 'B1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.lugbogInstanceId
    && descriptor.cell === 'B1'
    && descriptor.region === undefined);
  const summonedToEnemyWater = session.state.realm.units.some((unit) =>
    unit.instanceId === opening.lugbogInstanceId
      && unit.controller === 'north'
      && unit.location === 'B1'
      && unit.region === 'surface')
    && session.state.realm.sites.B1?.controller === 'south';

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    enemyLandUnavailable,
    enemyWaterAvailable,
    lugbogCat: opening.names.get(input.lugbogCat.stableId) ?? input.lugbogCat.stableId,
    replayVerified: verifyGameReplay(session),
    slyFox: opening.names.get(input.slyFox.stableId) ?? input.slyFox.stableId,
    slyFoxControlledWaterAvailable,
    slyFoxEnemyWaterUnavailable,
    summonedToEnemyWater,
  });
}

function runWaterEdgeConnection(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterEdgeConnection'] {
  const opening = findWaterEdgeConnectionOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.featuredInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const moves = legalGameActions(session.state, 'north');
  const wraps = (candidate: GameLegalAction): boolean => candidate.descriptor.kind === 'move-and-attack'
    && candidate.descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C1';
  const wrapMoveAvailable = moves.some((candidate) => wraps(candidate)
    && candidate.descriptor.kind === 'move-and-attack'
    && candidate.descriptor.unitInstanceId === opening.featuredInstanceId);
  const avatarWrapUnavailable = !moves.some((candidate) => wraps(candidate)
    && candidate.descriptor.kind === 'move-and-attack'
    && candidate.descriptor.unitInstanceId === session.state.players.north.avatar.card.instanceId);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.featuredInstanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C1');
  const siteTargetAvailable = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarWrapUnavailable,
    deck: deckList(opening.manifest.decks.north, opening.names),
    polarBears: opening.names.get(input.polarBears.stableId) ?? input.polarBears.stableId,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    siteTargetAvailable,
    wrapMoveAvailable,
  });
}

function runWaterEndTurnStealth(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterEndTurnStealth'] {
  const opening = findWaterOpening(input, 'water-stealth');
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.featuredInstanceId
      && descriptor.cell === 'C3');
  const summonedUnstealthed = session.state.realm.units.some(({ instanceId, stealthed }) =>
    instanceId === opening.featuredInstanceId && !stealthed);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  const northEndEvents = session.transcript.at(-1)?.events ?? [];
  const stealthGainedIndex = northEndEvents.findIndex(({ payload, type }) =>
    type === 'stealth-gained'
      && isJsonRecord(payload)
      && payload.instanceId === opening.featuredInstanceId);
  const turnEndedIndex = northEndEvents.findIndex(({ type }) => type === 'turn-ended');
  const gainedStealthAtEndOfTurn = session.state.realm.units.some(({ instanceId, stealthed }) =>
    instanceId === opening.featuredInstanceId && stealthed)
    && stealthGainedIndex >= 0
    && stealthGainedIndex < turnEndedIndex;

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const readyAttacker = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.attackerInstanceId);
  const attackerWasReady = readyAttacker?.location === 'C2'
    && !readyAttacker.tapped
    && !readyAttacker.summoningSickness;
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3');
  const attackTargets = legalGameActions(session.state, 'south');
  const protectedUnit = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.featuredInstanceId);
  const movedAttacker = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.attackerInstanceId);
  const coLocatedReadyAttacker = attackerWasReady === true
    && protectedUnit?.location === 'C3'
    && movedAttacker?.location === 'C3';
  const attackSiteAvailable = attackTargets.some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
  const slyFoxAttackUnavailable = protectedUnit?.stealthed === true
    && attackTargets.every(({ descriptor }) =>
      descriptor.kind !== 'declare-attack'
        || descriptor.target.kind !== 'minion'
        || descriptor.target.instanceId !== opening.featuredInstanceId);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    attackSiteAvailable,
    coLocatedReadyAttacker,
    deck: deckList(opening.manifest.decks.north, opening.names),
    gainedStealthAtEndOfTurn,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    slyFox: opening.names.get(input.slyFox.stableId) ?? input.slyFox.stableId,
    slyFoxAttackUnavailable,
    summonedUnstealthed,
  });
}

function runWaterLure(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterLure'] {
  const opening = findWaterLureOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.northSeravaInstanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.southSeravaInstanceId
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const allyBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.northSeravaInstanceId);
  const targetBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southSeravaInstanceId);
  if (!allyBefore || !targetBefore) {
    throw new Error('private non-target Lure setup lacks its two Serava Townsfolk');
  }
  const northCemeteryBefore = session.state.players.north.cemetery;
  const southCemeteryBefore = session.state.players.south.cemetery;
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.lureInstanceId
      && descriptor.ally?.kind === 'minion'
      && descriptor.ally.instanceId === opening.northSeravaInstanceId
      && descriptor.ally.seat === 'north'
      && descriptor.temptedEnemy?.kind === 'minion'
      && descriptor.temptedEnemy.instanceId === opening.southSeravaInstanceId
      && descriptor.temptedEnemy.seat === 'south'
      && descriptor.temptedDestination?.cell === 'C3'
      && descriptor.temptedDestination.region === 'surface'
      && descriptor.target === undefined
      && descriptor.targetLocation === undefined
      && descriptor.targetSiteInstanceId === undefined
      && descriptor.cemeteryMinionInstanceId === undefined);
  const chosen = choices[0];
  if (!chosen || choices.length !== 1) {
    throw new Error('private Lure ally, enemy, and unique closer step are not exactly available');
  }
  const manaBefore = session.state.players.north.mana;
  session = accept(session, chosen);

  const allyAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.northSeravaInstanceId);
  const targetAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southSeravaInstanceId);
  if (!allyAfter || !targetAfter) throw new Error('private Lure removed a Serava Townsfolk');
  const receipt = session.transcript.at(-1);
  const events = receipt?.events ?? [];
  const castPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const luredPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const resolvedPayload = events[2] && isJsonRecord(events[2].payload)
    ? events[2].payload
    : undefined;
  const luredFrom = luredPayload && isJsonRecord(luredPayload.from)
    ? luredPayload.from
    : undefined;
  const luredTo = luredPayload && isJsonRecord(luredPayload.to)
    ? luredPayload.to
    : undefined;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    allyUnchanged: allyAfter.cardId === allyBefore.cardId
      && allyAfter.controller === allyBefore.controller
      && allyAfter.damage === allyBefore.damage
      && allyAfter.location === allyBefore.location
      && allyAfter.owner === allyBefore.owner
      && allyAfter.region === allyBefore.region
      && allyAfter.stealthed === allyBefore.stealthed
      && allyAfter.summoningSickness === allyBefore.summoningSickness
      && allyAfter.tapped === allyBefore.tapped
      && allyAfter.warded === allyBefore.warded,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'magic-cast,unit-lured,magic-resolved'
      && castPayload?.instanceId === opening.lureInstanceId
      && castPayload.manaPaid === 1
      && castPayload.seat === 'north'
      && castPayload.allyInstanceId === opening.northSeravaInstanceId
      && castPayload.allySeat === 'north'
      && luredPayload?.allyInstanceId === opening.northSeravaInstanceId
      && luredPayload.seat === 'south'
      && luredPayload.sourceInstanceId === opening.lureInstanceId
      && luredPayload.targetInstanceId === opening.southSeravaInstanceId
      && luredFrom?.cell === 'C2'
      && luredFrom.region === 'surface'
      && luredTo?.cell === 'C3'
      && luredTo.region === 'surface'
      && resolvedPayload?.instanceId === opening.lureInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactNonTargetChoices: choices.length === 1,
    lure: input.lure.name,
    manaPaid: manaBefore - session.state.players.north.mana,
    noCombatDamageOrTap: targetAfter.damage === targetBefore.damage
      && targetAfter.tapped === targetBefore.tapped
      && session.state.phase === 'main'
      && session.state.pendingCombat === null,
    noRandomDraws: receipt?.randomDraws.length === 0,
    replayVerified: verifyGameReplay(session),
    seravaTownsfolk: input.seravaTownsfolk.name,
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.lureInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.lureInstanceId),
    targetCemeteriesUnchanged: session.state.players.north.cemetery.length
      === northCemeteryBefore.length + 1
      && session.state.players.south.cemetery.length === southCemeteryBefore.length
      && session.state.players.north.cemetery
        .every(({ instanceId }) => instanceId !== opening.northSeravaInstanceId)
      && session.state.players.south.cemetery
        .every(({ instanceId }) => instanceId !== opening.southSeravaInstanceId),
    uniqueStepResolved: targetBefore.location === 'C2'
      && targetBefore.region === 'surface'
      && targetAfter.location === 'C3'
      && targetAfter.region === 'surface'
      && targetAfter.cardId === targetBefore.cardId
      && targetAfter.controller === targetBefore.controller
      && targetAfter.owner === targetBefore.owner
      && targetAfter.stealthed === targetBefore.stealthed
      && targetAfter.summoningSickness === targetBefore.summoningSickness
      && targetAfter.warded === targetBefore.warded,
  });
}

function runWaterPirateShip(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterPirateShip'] {
  const opening = findWaterPirateShipOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northWaterSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northWaterSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const manaBeforeGhostTown = session.state.players.north.mana;
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'C2');
  const manaBeforeSummon = session.state.players.north.mana;
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.pirateShipInstanceId
    && descriptor.cell === 'C3'
    && descriptor.region === undefined);
  const ghostTownManaUsed = manaBeforeGhostTown === 2
    && manaBeforeSummon === 4
    && session.state.players.north.mana === 0;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const before = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.pirateShipInstanceId);
  const observedBefore = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === opening.pirateShipInstanceId);
  const waterSite = session.state.realm.sites.C3;
  const waterSiteDefinition = waterSite && 'cardId' in waterSite
    ? session.state.cards[waterSite.cardId]
    : undefined;
  const ghostTownSite = session.state.realm.sites.C2;
  const ghostTownDefinition = ghostTownSite && 'cardId' in ghostTownSite
    ? session.state.cards[ghostTownSite.cardId]
    : undefined;
  if (!before || !waterSite || !ghostTownSite) {
    throw new Error('private Waterbound setup lacks its minion or sites');
  }
  const moveChoices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.pirateShipInstanceId
      && descriptor.from.cell === 'C3'
      && descriptor.from.region === 'surface'
      && descriptor.to.cell === 'C2'
      && descriptor.to.region === 'surface'
      && descriptor.path.map(({ cell, region }) => `${cell}:${region}`).join(',')
        === 'C3:surface,C2:surface');
  const selectedMove = moveChoices[0];
  if (!selectedMove || moveChoices.length !== 1) {
    throw new Error('private Waterbound move is not exactly available');
  }
  const sitesBefore = canonicalJson(session.state.realm.sites as unknown as JsonValue);
  const northCemeteryBefore = canonicalJson(
    session.state.players.north.cemetery as unknown as JsonValue,
  );
  const southCemeteryBefore = canonicalJson(
    session.state.players.south.cemetery as unknown as JsonValue,
  );
  const moved = stepGame(session, selectedMove);
  if (!moved.accepted) throw new Error('private Waterbound move was rejected');
  session = moved.session;

  const after = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.pirateShipInstanceId);
  const observedAfter = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === opening.pirateShipInstanceId);
  const events = moved.receipt.events;
  const movementPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const movementFrom = movementPayload && isJsonRecord(movementPayload.from)
    ? movementPayload.from
    : undefined;
  const movementTo = movementPayload && isJsonRecord(movementPayload.to)
    ? movementPayload.to
    : undefined;
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const subsequentActions = legalGameActions(session.state, 'north');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    disabledAtLand: observedAfter?.disabled === true
      && ghostTownDefinition?.cardType === 'site'
      && !ghostTownDefinition.elements.includes('water'),
    enabledAtWater: observedBefore?.disabled === false
      && waterSiteDefinition?.cardType === 'site'
      && waterSiteDefinition.elements.includes('water'),
    exactMoveAvailable: moveChoices.length === 1,
    ghostTown: input.ghostTownSite.name,
    ghostTownManaUsed,
    movementEventVerified: events.map(({ type }) => type).join(',')
      === 'move-and-attack-activated'
      && movementPayload?.seat === 'north'
      && movementPayload.steps === 1
      && movementPayload.unitInstanceId === opening.pirateShipInstanceId
      && movementFrom?.cell === 'C3'
      && movementFrom.region === 'surface'
      && movementTo?.cell === 'C2'
      && movementTo.region === 'surface',
    noCombatDamageDeathOrRandomness: moved.receipt.randomDraws.length === 0
      && events.every(({ type }) => ![
        'attack-declared',
        'damage-dealt',
        'fight-started',
        'minion-died',
        'strike-damage-allocated',
      ].includes(type))
      && after?.damage === before.damage
      && northCemeteryBefore === canonicalJson(
        session.state.players.north.cemetery as unknown as JsonValue,
      )
      && southCemeteryBefore === canonicalJson(
        session.state.players.south.cemetery as unknown as JsonValue,
      )
      && session.state.terminal.status === 'active',
    noSubsequentUnitActions: !subsequentActions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === opening.pirateShipInstanceId),
    pirateShip: input.pirateShip.name,
    replayVerified: verifyGameReplay(session),
    sitesUnchanged: sitesBefore
      === canonicalJson(session.state.realm.sites as unknown as JsonValue),
    unitStatePreserved: after !== undefined
      && before.cardId === after.cardId
      && before.controller === after.controller
      && before.damage === after.damage
      && before.location === 'C3'
      && after.location === 'C2'
      && before.owner === after.owner
      && before.region === 'surface'
      && after.region === 'surface'
      && before.stealthed === after.stealthed
      && before.summoningSickness === false
      && after.summoningSickness === false
      && before.tapped === false
      && after.tapped === true
      && before.warded === after.warded,
  });
}

function runWaterFreeze(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterFreeze'] {
  const opening = findWaterFreezeOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.seravaInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');

  const hasSeravaMove = (candidate: GameSession): boolean =>
    legalGameActions(candidate.state, 'north').some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === opening.seravaInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
  const before = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seravaInstanceId);
  if (!before) throw new Error('private timed-disable setup lacks its ready minion');
  const actionAvailableBefore = hasSeravaMove(session);
  const freezeActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.freezeInstanceId
      && descriptor.target?.kind === 'minion'
      && descriptor.target.instanceId === opening.seravaInstanceId);
  const chosenFreeze = freezeActions[0];
  if (!chosenFreeze || freezeActions.length !== 1) {
    throw new Error('private timed-disable Magic target is not exactly available');
  }
  const manaBefore = session.state.players.north.mana;
  session = accept(session, chosenFreeze);
  const manaPaid = manaBefore - session.state.players.north.mana;

  const afterCast = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seravaInstanceId);
  const castEvents = session.transcript.at(-1)?.events ?? [];
  const castPayload = castEvents[0] && isJsonRecord(castEvents[0].payload)
    ? castEvents[0].payload
    : undefined;
  const disabledPayload = castEvents[1] && isJsonRecord(castEvents[1].payload)
    ? castEvents[1].payload
    : undefined;
  const resolvedPayload = castEvents[2] && isJsonRecord(castEvents[2].payload)
    ? castEvents[2].payload
    : undefined;
  const disabledStateRecorded = afterCast?.disableEffects?.length === 1
    && afterCast.disableEffects[0]?.expiresAtSeat === 'north'
    && afterCast.disableEffects[0].sourceInstanceId === opening.freezeInstanceId;
  const actionUnavailableWhileDisabled = !hasSeravaMove(session);
  const unitStatePreservedAfterCast = afterCast !== undefined
    && afterCast.cardId === before.cardId
    && afterCast.controller === before.controller
    && afterCast.damage === before.damage
    && afterCast.location === before.location
    && afterCast.owner === before.owner
    && afterCast.region === before.region
    && afterCast.stealthed === before.stealthed
    && afterCast.summoningSickness === before.summoningSickness
    && afterCast.tapped === before.tapped
    && afterCast.warded === before.warded;

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const duringOpponentTurn = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seravaInstanceId);
  const disabledThroughOpponentTurn = duringOpponentTurn?.disableEffects?.length === 1
    && duringOpponentTurn.disableEffects[0]?.expiresAtSeat === 'north'
    && duringOpponentTurn.disableEffects[0].sourceInstanceId === opening.freezeInstanceId;
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  const afterExpiry = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seravaInstanceId);
  const expiryEvents = session.transcript.at(-1)?.events ?? [];
  const turnEndedIndex = expiryEvents.findIndex(({ payload, type }) =>
    type === 'turn-ended' && isJsonRecord(payload) && payload.seat === 'south');
  const expiryIndex = expiryEvents.findIndex(({ payload, type }) =>
    type === 'minion-disable-expired'
      && isJsonRecord(payload)
      && payload.instanceId === opening.seravaInstanceId
      && payload.seat === 'north'
      && payload.sourceInstanceId === opening.freezeInstanceId);
  const turnStartedIndex = expiryEvents.findIndex(({ payload, type }) =>
    type === 'turn-started' && isJsonRecord(payload) && payload.seat === 'north');
  const expiredAtCasterStart = afterExpiry !== undefined
    && afterExpiry.disableEffects === undefined
    && turnEndedIndex >= 0
    && expiryIndex === turnEndedIndex + 1
    && turnStartedIndex === expiryIndex + 1;
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    actionAvailableBefore,
    actionReturnedOnNextTurn: hasSeravaMove(session),
    actionUnavailableWhileDisabled,
    causalEventsVerified: castEvents.map(({ type }) => type).join(',')
      === 'magic-cast,minion-disabled,magic-resolved'
      && castPayload?.instanceId === opening.freezeInstanceId
      && castPayload.manaPaid === 1
      && castPayload.seat === 'north'
      && castPayload.targetInstanceId === opening.seravaInstanceId
      && castPayload.targetSeat === 'north'
      && disabledPayload?.expiresAtSeat === 'north'
      && disabledPayload.instanceId === opening.seravaInstanceId
      && disabledPayload.seat === 'north'
      && disabledPayload.sourceInstanceId === opening.freezeInstanceId
      && disabledPayload.stealthRemoved === false
      && disabledPayload.wardRemoved === false
      && resolvedPayload?.instanceId === opening.freezeInstanceId
      && turnEndedIndex < expiryIndex
      && expiryIndex < turnStartedIndex,
    deck: deckList(opening.manifest.decks.north, opening.names),
    disabledStateRecorded,
    disabledThroughOpponentTurn,
    expiredAtCasterStart,
    freeze: input.freeze.name,
    manaPaid,
    replayVerified: verifyGameReplay(session),
    seravaTownsfolk: input.seravaTownsfolk.name,
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.freezeInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.freezeInstanceId),
    unitStatePreserved: unitStatePreservedAfterCast
      && afterExpiry !== undefined
      && afterExpiry.cardId === before.cardId
      && afterExpiry.controller === before.controller
      && afterExpiry.damage === before.damage
      && afterExpiry.location === before.location
      && afterExpiry.owner === before.owner
      && afterExpiry.region === before.region
      && afterExpiry.stealthed === before.stealthed
      && afterExpiry.summoningSickness === before.summoningSickness
      && afterExpiry.tapped === before.tapped
      && afterExpiry.warded === before.warded,
  });
}

function runWaterHealing(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterHealing'] {
  const opening = findWaterOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[0]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]);
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.attackerInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
      && descriptor.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.featuredInstanceId
      && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
      && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.attackerInstanceId
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  for (let strike = 0; strike < 2; strike += 1) {
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === opening.attackerInstanceId
        && descriptor.to.cell === 'C3');
    take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === session.state.realm.sites.C3?.instanceId);
    take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    if (strike === 0) take(({ descriptor }) => descriptor.kind === 'end-turn');
  }

  const lifeBeforeHealing = session.state.players.north.avatar.life;
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.featuredInstanceId
      && descriptor.to.cell === 'C3');
  take(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.attackerInstanceId);
  take(({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
  const finalEvents = session.transcript.at(-1)?.events ?? [];
  const healIndex = finalEvents.findIndex(({ type }) => type === 'avatar-healed');
  const cemeteryIndex = finalEvents.findIndex(({ payload, type }) =>
    type === 'minion-died'
      && isJsonRecord(payload)
      && payload.instanceId === opening.featuredInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    healed: session.state.players.north.avatar.life - lifeBeforeHealing,
    healedBeforeCemetery: healIndex >= 0 && healIndex < cemeteryIndex,
    healingMinionDied: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.featuredInstanceId),
    healingMinion:
      opening.names.get(input.healingMinion.stableId) ?? input.healingMinion.stableId,
    opponentMinionDied: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.attackerInstanceId),
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
  });
}

export async function runPrivateGameCheck(path = DEFAULT_SCENARIO): Promise<PrivateGameCheck> {
  const input = await readPrivateInputs(path);
  const airGenesisSpell = runAirGenesisSpell(input);
  const airSpellcasterFreeze = runAirSpellcasterFreeze(input);
  const airArcLightning = runAirArcLightning(input);
  const airLightningBolt = runAirLightningBolt(input);
  const airRainOfArrows = runAirRainOfArrows(input);
  const airTeleport = runAirTeleport(input);
  const airLeyline = runAirLeyline(input);
  const airborne = runAirborne(input);
  const airMovement = runAirMovement(input);
  const airMovementTwo = runAirMovementTwo(input);
  const airSummoning = runAirSummoning(input);
  const airVoidwalk = runAirVoidwalk(input);
  const airZap = runAirZap(input);
  const earthBurrowing = runEarthBurrowing(input);
  const earthOverpower = runEarthOverpower(input);
  const earthBury = runEarthBury(input);
  const earthRescue = runEarthRescue(input);
  const earthDivineHealing = runEarthDivineHealing(input);
  const earthShallowGrave = runEarthShallowGrave(input);
  const earthSinkhole = runEarthSinkhole(input);
  const earthEntombed = runEarthEntombed(input);
  const earthFirstStrike = runEarthFirstStrike(input);
  const earthForwardMovement = runEarthForwardMovement(input);
  const earthImmobile = runEarthImmobile(input);
  const earthRamp = runEarthRamp(input);
  const earthRanged = runEarthRanged(input);
  const earthSecretTunnel = runEarthSecretTunnel(input);
  const earthWard = runEarthWard(input);
  const fireCharge = runFireCharge(input);
  const fireGenesisLifeLoss = runFireGenesisLifeLoss(input);
  const fireIgnited = runFireIgnited(input);
  const fireMinorExplosion = runFireMinorExplosion(input);
  const fireResponse = runFireResponse(input);
  const stealth = runStealth(input);
  const waterDrowned = runWaterDrowned(input);
  const waterDrown = runWaterDrown(input);
  const waterEdgeConnection = runWaterEdgeConnection(input);
  const waterLugbog = runWaterLugbog(input);
  const waterLure = runWaterLure(input);
  const waterPirateShip = runWaterPirateShip(input);
  const waterEndTurnStealth = runWaterEndTurnStealth(input);
  const waterFreeze = runWaterFreeze(input);
  const waterHealing = runWaterHealing(input);
  const waterSidewaysMovement = runWaterSidewaysMovement(input);
  const waterSubmerge = runWaterSubmerge(input);
  const opening = findOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardInstanceId === opening.north.siteInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === opening.north.minionInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardInstanceId === opening.south.siteInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === opening.south.minionInstanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northChargeSiteInstanceId
      && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.northChargeInstanceId
      && descriptor.cell === 'C3'));
  const chargeActivatedOnSummon = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.northChargeInstanceId
      && descriptor.to.cell === 'C3');
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.northChargeInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.north.minionInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.south.minionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const affinityBeforeProvider = observeGame(session.state, 'north').players.north.affinity.fire;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.northProviderInstanceId
      && descriptor.cell === 'C3'));
  const providerAffinityAdded =
    observeGame(session.state, 'north').players.north.affinity.fire === affinityBeforeProvider + 1;
  const beforeAvatarDraw = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'draw-spell'));
  const afterAvatarDraw = session.state.players.north;
  const avatarSpellDrawn = afterAvatarDraw.avatar.tapped
    && afterAvatarDraw.spellbook.length === beforeAvatarDraw.spellbook.length - 1
    && afterAvatarDraw.hand.spellbook.length === beforeAvatarDraw.hand.spellbook.length + 1;
  if (!avatarSpellDrawn) throw new Error('actual Avatar draw-spell ability did not resolve');
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.north.minionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.south.minionInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  const beforeGenesis = session.state.players.north;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.northGenesisInstanceId));
  const afterGenesis = session.state.players.north;
  const genesisSiteDrawn =
    afterGenesis.atlas.length === beforeGenesis.atlas.length - 1
    && afterGenesis.hand.atlas.length === beforeGenesis.hand.atlas.length + 1;

  const northCardId = opening.session.state.players.north.hand.spellbook
    .find(({ instanceId }) => instanceId === opening.north.minionInstanceId)!.cardId;
  const southCardId = opening.session.state.players.south.hand.spellbook
    .find(({ instanceId }) => instanceId === opening.south.minionInstanceId)!.cardId;
  const northDefinition = opening.session.state.cards[northCardId];
  const southDefinition = opening.session.state.cards[southCardId];
  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    airGenesisSpell,
    airSpellcasterFreeze,
    airArcLightning,
    airLightningBolt,
    airRainOfArrows,
    airTeleport,
    airLeyline,
    airborne,
    airMovement,
    airMovementTwo,
    airSummoning,
    airVoidwalk,
    airZap,
    authorityHash: input.authorityHash,
    avatarSpellDrawn,
    charge: {
      activatedOnSummon: chargeActivatedOnSummon,
      minion: opening.names.get(input.chargeMinion.stableId) ?? input.chargeMinion.stableId,
    },
    classification: 'private-local_actual-cards_unranked-partial-rules',
    combat: {
      northMinion: opening.names.get(northCardId) ?? northCardId,
      northMinionDied: session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.north.minionInstanceId),
      southMinion: opening.names.get(southCardId) ?? southCardId,
      southMinionDied: session.state.players.south.cemetery
        .some(({ instanceId }) => instanceId === opening.south.minionInstanceId),
    },
    decks: {
      north: deckList(opening.manifest.decks.north, opening.names),
      south: deckList(opening.manifest.decks.south, opening.names),
    },
    earthBurrowing,
    earthOverpower,
    earthBury,
    earthRescue,
    earthDivineHealing,
    earthShallowGrave,
    earthSinkhole,
    earthEntombed,
    earthRamp,
    earthFirstStrike,
    earthForwardMovement,
    earthImmobile,
    earthRanged,
    earthSecretTunnel,
    earthWard,
    fireCharge,
    fireGenesisLifeLoss,
    fireIgnited,
    fireMinorExplosion,
    fireResponse,
    finalStateHash: hashGameState(session.state),
    formatStableId: input.formatStableId,
    genesis: {
      minion: opening.names.get(input.genesisMinion.stableId) ?? input.genesisMinion.stableId,
      siteDrawn: genesisSiteDrawn,
    },
    lethal: {
      minion: opening.names.get(input.lethalMinion.stableId) ?? input.lethalMinion.stableId,
      tougherMinionKilled:
        northDefinition?.cardType === 'minion'
        && northDefinition.lethal === true
        && southDefinition?.cardType === 'minion'
        && northDefinition.attack < southDefinition.defense
        && session.state.players.south.cemetery
          .some(({ instanceId }) => instanceId === opening.south.minionInstanceId),
    },
    provider: {
      affinityAdded: providerAffinityAdded,
      minion: opening.names.get(input.providerMinion.stableId) ?? input.providerMinion.stableId,
    },
    replayVerified: verifyGameReplay(session),
    revisionId: input.config.revisionId,
    seed: opening.seed,
    stealth,
    waterDrowned,
    waterDrown,
    waterEdgeConnection,
    waterLugbog,
    waterLure,
    waterPirateShip,
    waterEndTurnStealth,
    waterFreeze,
    waterHealing,
    waterSidewaysMovement,
    waterSubmerge,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await runPrivateGameCheck(process.argv[2]);
  process.stdout.write(`${canonicalJson(result as unknown as JsonValue)}\n`);
}
