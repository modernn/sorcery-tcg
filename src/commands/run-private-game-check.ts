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

type StarterCheck = Readonly<{
  acceptedActionCount: number;
  causalEventsVerified: boolean;
  deck: DeckList;
  manaPaid: number;
  minion: string;
  noRandomDraws: boolean;
  replayVerified: boolean;
  site: string;
  siteAndMinionStateVerified: boolean;
}>;

export type StarterScenario = 'air-starter' | 'earth-starter' | 'fire-starter' | 'water-starter';
type BetaLessonScenario = 'air-vs-earth-lesson' | 'earth-vs-air-lesson';

export type PrivateStarterPreset = Readonly<{
  cardNames: Readonly<Record<string, string>>;
  id: StarterScenario | BetaLessonScenario;
  label: string;
  manifest: GameManifest;
  usesOnlyOrdinaryOrExceptionalCards: boolean;
}>;

export type PrivateGameCheck = Readonly<{
  acceptedActionCount: number;
  airStarter: StarterCheck;
  earthStarter: StarterCheck;
  fireStarter: StarterCheck;
  waterRiver: Readonly<{
    acceptedActionCount: number;
    bottomedNextSpell: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactChoices: boolean;
    hiddenFromOpponent: boolean;
    keptNextSpell: boolean;
    legalLowRarityDeck: true;
    noRandomDraws: boolean;
    replayVerified: boolean;
    river: string;
    seed: number;
  }>;
  fireGranaryRats: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    deck: DeckList;
    fireAffinityBeforeSummon: boolean;
    granaryRats: string;
    manaPaid: number;
    noRandomDraws: boolean;
    replayVerified: boolean;
    seed: number;
    siteAndMinionStateVerified: boolean;
    siteThresholdSuppressed: boolean;
    wasteland: string;
  }>;
  fireHamlet: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactDestinationCosts: boolean;
    fireAffinityVerified: boolean;
    hamlet: string;
    manaPaid: number;
    noRandomDraws: boolean;
    raalDromedary: string;
    replayVerified: boolean;
    seed: number;
    siteAndMinionStateVerified: boolean;
    wasteland: string;
  }>;
  waterStarter: StarterCheck;
  airBladderblimp: Readonly<{
    acceptedActionCount: number;
    airborneAtC3: boolean;
    bladderblimp: string;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactNearbySiteCounts: boolean;
    gameRemainedActive: boolean;
    lifeLossOnly: boolean;
    lightningBolt: string;
    magicManaPaid: number;
    minionAndMagicEnteredCemetery: boolean;
    randomSelectionRecorded: boolean;
    replayVerified: boolean;
    summonManaPaid: number;
  }>;
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
    damageReductionVerified: boolean;
    deck: DeckList;
    gameRemainedActive: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    noTargetChoice: boolean;
    rainOfArrows: string;
    replayVerified: boolean;
    seed: number;
    shellycoat: string;
    sitesPreserved: boolean;
    snowLeopard: string;
    spellEnteredCemetery: boolean;
    surfaceMinionsComparedAndSurvived: boolean;
  }>;
  airStaticServant: Readonly<{
    acceptedActionCount: number;
    avatarAndLeopardDamaged: boolean;
    causalEventsVerified: boolean;
    cemeteriesUnchanged: boolean;
    deck: DeckList;
    gameRemainedActive: boolean;
    manaPaid: number;
    noTargetChoiceOrRandomness: boolean;
    otherStatePreserved: boolean;
    replayVerified: boolean;
    snowLeopard: string;
    staticServant: string;
    staticServantExcludedAndUndamaged: boolean;
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
  airVoidArtifact: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    noRandomDraws: boolean;
    relocationVerified: boolean;
    replayVerified: boolean;
    seed: number;
    spectralStalker: string;
    swordAndShield: string;
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
  airFireFatality: Readonly<{
    acceptedActionCount: number;
    airFireAffinity: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactWoundedTarget: boolean;
    fatality: string;
    fatalityDealtNoDamage: boolean;
    fatalityEnteredCemetery: boolean;
    healthyTargetUnavailable: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    replayVerified: boolean;
    snowLeopard: string;
    targetEnteredOwnerCemetery: boolean;
    targetLeftRealm: boolean;
    zap: string;
    zapDamageExactlyOne: boolean;
    zapEnteredCemetery: boolean;
    zapManaPaid: number;
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
  earthSwordAndShield: Readonly<{
    acceptedActionCount: number;
    artifactCastUncarried: boolean;
    artifactPickedUpAndCarried: boolean;
    boskTroll: string;
    causalEventsVerified: boolean;
    combatDamageAndSurvivalVerified: boolean;
    deck: DeckList;
    dropAcceptedActionCount: number;
    dropChoiceVerified: boolean;
    dropDeathAcceptedActionCount: number;
    dropDeathReplayVerified: boolean;
    dropDeathVerified: boolean;
    dropEventVerified: boolean;
    dropNoRandomDraws: boolean;
    dropReplayVerified: boolean;
    dropSecondUseUnavailable: boolean;
    dropSideEffectsAbsent: boolean;
    dropStateVerified: boolean;
    dropUnavailableAfterInteraction: boolean;
    elthamTownsfolk: string;
    exactPickupChoice: boolean;
    gameRemainedActive: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    pickupSideEffectsAbsent: boolean;
    replayVerified: boolean;
    seed: number;
    swordAndShield: string;
    swordFollowedBearer: boolean;
    swordRemainedCarried: boolean;
    swordStayedOutOfCemetery: boolean;
    unrelatedStatePreserved: boolean;
  }>;
  earthPoisonousDagger: Readonly<{
    acceptedActionCount: number;
    artifactCastAndCarried: boolean;
    boskTroll: string;
    causalEventsVerified: boolean;
    combatLethalVerified: boolean;
    daggerDroppedUncontrolled: boolean;
    deck: DeckList;
    elthamTownsfolk: string;
    exactBearerChoice: boolean;
    gameRemainedActive: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    poisonousDagger: string;
    replayVerified: boolean;
    stateAndCemeteriesVerified: boolean;
  }>;
  earthHuntersLodge: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    deck: DeckList;
    enemyStealthRemoved: boolean;
    hunterLodge: string;
    noRandomDraws: boolean;
    replayVerified: boolean;
    slyFox: string;
    slyFoxGainedStealthFirst: boolean;
    statePreserved: boolean;
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
    destructionAcceptedActionCount: number;
    exactActivationAvailable: boolean;
    noAffinityOrControlContribution: boolean;
    noRandomDraws: boolean;
    recoveryVerified: boolean;
    replayVerified: boolean;
    seed: number;
    sinkhole: string;
    sourceAndTargetEnteredCemetery: boolean;
    twoNeutralRubbleSites: boolean;
    valley: string;
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
  earthBorderMilitia: Readonly<{
    acceptedActionCount: number;
    borderMilitia: string;
    deck: DeckList;
    footSoldier: string;
    manaPaid: number;
    noRandomDraws: boolean;
    replayVerified: boolean;
    seed: number;
    spellEnteredCemetery: boolean;
    tokensVerified: boolean;
  }>;
  earthHumbleVillage: Readonly<{
    acceptedActionCount: number;
    deck: DeckList;
    declinedKeptManaAndSummonedNothing: boolean;
    exactChoices: boolean;
    footSoldier: string;
    gameRemainedActive: boolean;
    humbleVillage: string;
    noRandomDraws: boolean;
    paidSpentManaAndSummonedToken: boolean;
    replayVerified: boolean;
    seed: number;
    tokenDefinitionVerified: boolean;
  }>;
  earthDuel: Readonly<{
    acceptedActionCount: number;
    allySurvivedWithTwoDamage: boolean;
    boskTroll: string;
    causalEventsVerified: boolean;
    deck: DeckList;
    duel: string;
    elthamTownsfolk: string;
    exactFightPair: boolean;
    gameRemainedActive: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    replayVerified: boolean;
    sitesAndAvatarsPreserved: boolean;
    spellEnteredCemetery: boolean;
    targetDiedAndEnteredCemetery: boolean;
    unitsDidNotMoveOrTap: boolean;
  }>;
  earthGrainSparrow: Readonly<{
    acceptedActionCount: number;
    actualLifeGained: number;
    causalEventsVerified: boolean;
    deck: DeckList;
    grainSparrow: string;
    lesserBloodDemon: string;
    lifeCappedAtMaximum: boolean;
    lifeLostBeforeSummon: boolean;
    noDamageDeathTerminalOrRandomEffects: boolean;
    otherStatePreserved: boolean;
    replayVerified: boolean;
    steppe: string;
    summonedAtC3: boolean;
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
  earthMalakhim: Readonly<{
    acceptedActionCount: number;
    airborneAndWard: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    earthAffinityThree: boolean;
    endPhaseUntapped: boolean;
    malakhim: string;
    manaPaid: number;
    noRandomDraws: boolean;
    normalActionTapped: boolean;
    opponentTurnReady: boolean;
    replayVerified: boolean;
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
  fireAramos: Readonly<{
    acceptedActionCount: number;
    aramosMercenaries: string;
    causalEventsVerified: boolean;
    deck: DeckList;
    discardedNonCastingCard: boolean;
    hiddenInformationVerified: boolean;
    manaPaid: number;
    normalManaSummonUnavailable: boolean;
    paymentModeVerified: boolean;
    raalDromedary: string;
    randomDiscardVerified: boolean;
    replayVerified: boolean;
    summonedAtC3: boolean;
    unrelatedStatePreserved: boolean;
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
  fireVileImp: Readonly<{
    acceptedActionCount: number;
    avatarTookTwoDamage: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    declinePreservedAvatar: boolean;
    exactChoices: boolean;
    legalLowRarityDeck: boolean;
    manaPaid: number;
    noRandomDraws: boolean;
    replayVerified: boolean;
    seed: number;
    summonedAtC3: boolean;
    vileImp: string;
    wasteland: string;
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
  fireLash: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    damageBeforeUntap: boolean;
    deck: DeckList;
    exactNearbyTarget: boolean;
    lash: string;
    manaPaid: number;
    noDeathTerminalOrRandomEffects: boolean;
    otherStatePreserved: boolean;
    raalDromedary: string;
    replayVerified: boolean;
    spellEnteredCemetery: boolean;
    survivedWithOneDamage: boolean;
    tappedThenUntapped: boolean;
  }>;
  fireLeapAttack: Readonly<{
    acceptedActionCount: number;
    allySteppedWithoutTapOrDamage: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactOptionalStepChoices: boolean;
    gameRemainedActive: boolean;
    leapAttack: string;
    manaPaid: number;
    noAttackResponseOrRandomness: boolean;
    raalDromedary: string;
    replayVerified: boolean;
    sitesAndAvatarsPreserved: boolean;
    spellEnteredCemetery: boolean;
    struckAndKilledEveryEnemy: boolean;
  }>;
  fireRecklessSquire: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    deck: DeckList;
    firstStrikeLanceDamage: boolean;
    gameRemainedActive: boolean;
    lanceCreatedAndCarried: boolean;
    lanceUsedAndRemoved: boolean;
    noRandomDraws: boolean;
    raalDromedary: string;
    recklessSquire: string;
    replayVerified: boolean;
    secondStrikeNormal: boolean;
    stateAndCemeteriesVerified: boolean;
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
  fireVikings: Readonly<{
    acceptedActionCount: number;
    activationUnavailableWhileSickAndTapped: boolean;
    artifactCastAndCarried: boolean;
    abilityLethalVerified: boolean;
    boskTroll: string;
    causalEventsVerified: boolean;
    daggerManaPaid: number;
    deck: DeckList;
    exactAdjacentTarget: boolean;
    noCombatOrReturnDamage: boolean;
    noRandomDraws: boolean;
    poisonousDagger: string;
    replayVerified: boolean;
    simultaneousDamageVerified: boolean;
    summonManaPaid: number;
    targetsEnteredCemetery: boolean;
    vikings: string;
    vikingsSurvivedAndTapped: boolean;
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
  waterMesmerism: Readonly<{
    acceptedActionCount: number;
    causalEventsVerified: boolean;
    controlTransferred: boolean;
    deck: DeckList;
    deathriteControllerDrewSite: boolean;
    deathriteOwnerKeptCemetery: boolean;
    exactNearbyTarget: boolean;
    farTargetUnavailable: boolean;
    kettletopLeprechaun: string;
    manaPaid: number;
    mesmerism: string;
    newControllerGainedAction: boolean;
    noRandomDraws: boolean;
    oldControllerHadAction: boolean;
    oldControllerLostAction: boolean;
    replayVerified: boolean;
    seed: number;
    seravaTownsfolk: string;
    waterAffinityFour: boolean;
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
  waterGnarledWendigo: Readonly<{
    acceptedActionCount: number;
    canonicalSacrificeChoice: boolean;
    causalEventsVerified: boolean;
    deck: DeckList;
    exactDiscountedSummonAvailable: boolean;
    gameRemainedActive: boolean;
    ghostTownManaConsumed: boolean;
    gnarledWendigo: string;
    handRealmCemeteryVerified: boolean;
    manaPaid: number;
    noNormalManaSummon: boolean;
    noRandomOrUnrelatedEffects: boolean;
    replayVerified: boolean;
    seravaTownsfolk: string;
    stateVersionAdvancedOnce: boolean;
    summonedAtC4: boolean;
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
    freeze: string;
    nonSubmergeSurfaceAvailable: boolean;
    nonSubmergeUnderwaterUnavailable: boolean;
    replayVerified: boolean;
    seaWitch: string;
    seed: number;
    submergeMinion: string;
    summonedUnderwater: boolean;
    surfaceSummonAvailable: boolean;
    targetIsWaterSite: boolean;
    underwaterFreezeSettlementVerified: boolean;
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
  amazonWarriors: NormalizedCard;
  airborneMinion: NormalizedCard;
  airborneTargetMinion: NormalizedCard;
  aramosMercenaries: NormalizedCard;
  arcLightning: NormalizedCard;
  autumnRiver: NormalizedCard;
  autumnUnicorn: NormalizedCard;
  authorityHash: Hash;
  bladderblimp: NormalizedCard;
  bury: NormalizedCard;
  borderMilitia: NormalizedCard;
  burrowingMinion: NormalizedCard;
  cards: readonly NormalizedCard[];
  cannotDefendMinion: NormalizedCard;
  chargeMagic: NormalizedCard;
  chargeMinion: NormalizedCard;
  config: ScenarioConfig;
  deathriteMinion: NormalizedCard;
  deadOfNightDemon: NormalizedCard;
  darkTower: NormalizedCard;
  dalceanPhalanx: NormalizedCard;
  divineHealing: NormalizedCard;
  duel: NormalizedCard;
  drown: NormalizedCard;
  drowned: NormalizedCard;
  earthProviderMinion: NormalizedCard;
  elthamTownsfolk: NormalizedCard;
  entombed: NormalizedCard;
  format: FormatDefinition;
  formatStableId: string;
  firstStrikeMinion: NormalizedCard;
  firstStrikeTargetMinion: NormalizedCard;
  footSoldier: NormalizedCard;
  forsaken: NormalizedCard;
  freeze: NormalizedCard;
  fatality: NormalizedCard;
  genesisSpellMinion: NormalizedCard;
  genesisMinion: NormalizedCard;
  geomancer: NormalizedCard;
  gothicTower: NormalizedCard;
  sparkmage: NormalizedCard;
  granaryRats: NormalizedCard;
  grainSparrow: NormalizedCard;
  gyreHippogriffs: NormalizedCard;
  gnarledWendigo: NormalizedCard;
  ghostTownSite: NormalizedCard;
  hamlet: NormalizedCard;
  healingMinion: NormalizedCard;
  huntersLodge: NormalizedCard;
  highlandClansmen: NormalizedCard;
  humbleVillage: NormalizedCard;
  lethalMinion: NormalizedCard;
  leylineHenge: NormalizedCard;
  lesserBloodDemon: NormalizedCard;
  ignited: NormalizedCard;
  lash: NormalizedCard;
  leapAttack: NormalizedCard;
  lightningBolt: NormalizedCard;
  lugbogCat: NormalizedCard;
  lure: NormalizedCard;
  malakhim: NormalizedCard;
  mesmerism: NormalizedCard;
  lumberingMinion: NormalizedCard;
  loneTower: NormalizedCard;
  manaMinion: NormalizedCard;
  midnightRogue: NormalizedCard;
  minorExplosion: NormalizedCard;
  monstrousLion: NormalizedCard;
  movementMinion: NormalizedCard;
  movementTwoMinion: NormalizedCard;
  overpower: NormalizedCard;
  providerMinion: NormalizedCard;
  raalDromedary: NormalizedCard;
  recklessSquire: NormalizedCard;
  rangedMinion: NormalizedCard;
  rescue: NormalizedCard;
  rusticVillage: NormalizedCard;
  roamingMinion: NormalizedCard;
  secretTunnel: NormalizedCard;
  sedgeCrabs: NormalizedCard;
  seravaTownsfolk: NormalizedCard;
  shellycoat: NormalizedCard;
  shallowGrave: NormalizedCard;
  sinkhole: NormalizedCard;
  simpleVillage: NormalizedCard;
  slyFox: NormalizedCard;
  spire: NormalizedCard;
  stealthMinion: NormalizedCard;
  stealthTargetMinion: NormalizedCard;
  steppe: NormalizedCard;
  stream: NormalizedCard;
  submergeMinion: NormalizedCard;
  seaWitch: NormalizedCard;
  teleport: NormalizedCard;
  valley: NormalizedCard;
  vileImp: NormalizedCard;
  vikings: NormalizedCard;
  voidwalkMinion: NormalizedCard;
  wardMinion: NormalizedCard;
  wasteland: NormalizedCard;
  wildBoars: NormalizedCard;
  polarBears: NormalizedCard;
  pirateShip: NormalizedCard;
  pudgeButcher: NormalizedCard;
  rainOfArrows: NormalizedCard;
  poisonousDagger: NormalizedCard;
  staticServant: NormalizedCard;
  swordAndShield: NormalizedCard;
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
  const blankOrdinarySite = (
    name: string,
    stableId: string,
    element: GameElement,
  ): NormalizedCard => {
    const card = snapshot.cards.find((candidate) => candidate.name === name);
    if (!card
      || card.stableId !== stableId
      || card.cardType !== 'site'
      || card.rulesText.trim() !== ''
      || card.manaCost !== null
      || card.attack !== null
      || card.defense !== null
      || card.life !== null
      || card.elements.length !== 1
      || card.elements[0] !== element
      || (['air', 'earth', 'fire', 'water'] as const).some((candidate) =>
        card.thresholds[candidate] !== (candidate === element ? 1 : 0))
      || card.rarity !== 'ordinary') {
      throw new Error(`private blank ${element} site ${name} no longer matches its supported facts`);
    }
    return card;
  };
  const ordinarySiteWithRules = (
    name: string,
    stableId: string,
    officialSourceId: string,
    element: GameElement,
    rulesLength: number,
  ): NormalizedCard => {
    const card = snapshot.cards.find((candidate) => candidate.name === name);
    if (!card
      || card.stableId !== stableId
      || card.officialSourceId !== officialSourceId
      || card.cardType !== 'site'
      || card.rulesText.trim().length !== rulesLength
      || card.manaCost !== null
      || card.attack !== null
      || card.defense !== null
      || card.life !== null
      || card.elements.length !== 1
      || card.elements[0] !== element
      || (['air', 'earth', 'fire', 'water'] as const).some((candidate) =>
        card.thresholds[candidate] !== (candidate === element ? 1 : 0))
      || card.rarity !== 'ordinary') {
      throw new Error(`private ${name} no longer matches its supported facts`);
    }
    return card;
  };
  const darkTower = ordinarySiteWithRules(
    'Dark Tower',
    'card:8c61e291e4b91f791fe2a142f154fce92c0066efdfee22f909b4712266d042e1',
    '001-dark_tower-b-f',
    'air',
    71,
  );
  const gothicTower = ordinarySiteWithRules(
    'Gothic Tower',
    'card:f05863c980b6f65f2cf3d1bbc4bbd022bdc83e81a7bbf9dc64180808bf0f764b',
    '001-gothic_tower-b-f',
    'air',
    73,
  );
  const loneTower = ordinarySiteWithRules(
    'Lone Tower',
    'card:3fadcb68035615a1c5b9461fef66b7190113126ebc0b32444d7c5c22586efd47',
    '001-lone_tower-b-f',
    'air',
    71,
  );
  const spire = blankOrdinarySite(
    'Spire',
    'card:e563251eda8b839ca617fb2eb1bfb512b8980747c8618e1e7b8e4e4de2d77e66',
    'air',
  );
  const valley = blankOrdinarySite(
    'Valley',
    'card:11d245c549b417bf6c1bdc675f18594fd9a0c92b560cc2e0c129d03a87b0a9f3',
    'earth',
  );
  const wasteland = blankOrdinarySite(
    'Wasteland',
    'card:12ac69ed727419b132b094e28a149e9d81f6af17f00408e52f700fddce0c9740',
    'fire',
  );
  const granaryRats = snapshot.cards.find(({ name }) => name === 'Granary Rats');
  if (!granaryRats
    || granaryRats.stableId
      !== 'card:d19d5b90a7be41b58e9d21e557f7f09065f33ba4a993d0f6909392c6838df6a0'
    || granaryRats.officialSourceId !== '006-granary_rats-b-f'
    || granaryRats.cardType !== 'minion'
    || granaryRats.rulesText !== "This site doesn't provide threshold."
    || granaryRats.manaCost !== 1
    || granaryRats.attack !== 1
    || granaryRats.defense !== 1
    || granaryRats.life !== null
    || granaryRats.elements.length !== 1
    || granaryRats.elements[0] !== 'fire'
    || granaryRats.thresholds.air !== 0
    || granaryRats.thresholds.earth !== 0
    || granaryRats.thresholds.fire !== 1
    || granaryRats.thresholds.water !== 0
    || granaryRats.rarity !== 'ordinary') {
    throw new Error('private Granary Rats no longer matches its supported facts');
  }
  const hamlet = snapshot.cards.find(({ name }) => name === 'Hamlet');
  if (!hamlet
    || hamlet.stableId
      !== 'card:06566302e2605c4a0dc477e8f1c25eae66c740e566fbd227abd31a526cee4bb7'
    || hamlet.cardType !== 'site'
    || hamlet.rulesText.trim() !== 'Ordinary minions cost (1) less to cast to this site.'
    || hamlet.manaCost !== null
    || hamlet.attack !== null
    || hamlet.defense !== null
    || hamlet.life !== null
    || hamlet.elements.length !== 0
    || hamlet.thresholds.air !== 0
    || hamlet.thresholds.earth !== 0
    || hamlet.thresholds.fire !== 0
    || hamlet.thresholds.water !== 0
    || hamlet.rarity !== 'ordinary') {
    throw new Error('private ordinary-minion discount Site no longer matches its supported facts');
  }
  const stream = blankOrdinarySite(
    'Stream',
    'card:8959e87bfe778fd2bdec4f6355e16d79085fa498040562cabf6f31af40801b6d',
    'water',
  );
  const autumnRiver = snapshot.cards.find(({ name }) => name === 'Autumn River');
  if (!autumnRiver
    || autumnRiver.stableId
      !== 'card:1831cd59a3795825f34b108a1e7bb97173e269d12ec2f8d5664a985ec61cc5da'
    || autumnRiver.cardType !== 'site'
    || autumnRiver.rulesText.trim()
      !== 'Genesis → Look at your next spell. You may put it on the bottom of your spellbook.'
    || autumnRiver.manaCost !== null
    || autumnRiver.attack !== null
    || autumnRiver.defense !== null
    || autumnRiver.life !== null
    || autumnRiver.elements.length !== 1
    || autumnRiver.elements[0] !== 'water'
    || autumnRiver.thresholds.air !== 0
    || autumnRiver.thresholds.earth !== 0
    || autumnRiver.thresholds.fire !== 0
    || autumnRiver.thresholds.water !== 1
    || autumnRiver.rarity !== 'ordinary') {
    throw new Error('private seasonal River no longer matches its supported facts');
  }
  const swordAndShield = snapshot.cards.find(({ name }) => name === 'Sword and Shield');
  if (!swordAndShield
    || swordAndShield.cardType !== 'artifact'
    || swordAndShield.rulesText.trim() !== 'Bearer has +2 power.'
    || swordAndShield.manaCost !== 3
    || swordAndShield.attack !== null
    || swordAndShield.defense !== null
    || swordAndShield.life !== null
    || swordAndShield.elements.length !== 0
    || swordAndShield.thresholds.air !== 0
    || swordAndShield.thresholds.earth !== 0
    || swordAndShield.thresholds.fire !== 0
    || swordAndShield.thresholds.water !== 0
    || swordAndShield.rarity !== 'exceptional') {
    throw new Error('private bearer power Artifact no longer matches its supported facts');
  }
  const poisonousDagger = snapshot.cards.find(({ name }) => name === 'Poisonous Dagger');
  if (!poisonousDagger
    || poisonousDagger.cardType !== 'artifact'
    || poisonousDagger.rulesText.trim() !== 'Bearer has Lethal.'
    || poisonousDagger.manaCost !== 2
    || poisonousDagger.attack !== null
    || poisonousDagger.defense !== null
    || poisonousDagger.life !== null
    || poisonousDagger.elements.length !== 0
    || poisonousDagger.thresholds.air !== 0
    || poisonousDagger.thresholds.earth !== 0
    || poisonousDagger.thresholds.fire !== 0
    || poisonousDagger.thresholds.water !== 0
    || poisonousDagger.rarity !== 'exceptional') {
    throw new Error('private bearer Lethal Artifact no longer matches its supported facts');
  }
  const huntersLodge = snapshot.cards.find(({ name }) => name === "Hunter's Lodge");
  if (!huntersLodge
    || huntersLodge.cardType !== 'site'
    || huntersLodge.rulesText.trim() !== 'Genesis → Enemies lose Stealth.'
    || huntersLodge.manaCost !== null
    || huntersLodge.attack !== null
    || huntersLodge.defense !== null
    || huntersLodge.life !== null
    || huntersLodge.elements.length !== 1
    || huntersLodge.elements[0] !== 'earth'
    || huntersLodge.thresholds.air !== 0
    || huntersLodge.thresholds.earth !== 1
    || huntersLodge.thresholds.fire !== 0
    || huntersLodge.thresholds.water !== 0
    || huntersLodge.rarity !== 'ordinary') {
    throw new Error('private enemy Stealth-removing site no longer matches its supported facts');
  }
  const geomancer = snapshot.cards.find(({ name }) => name === 'Geomancer');
  if (!geomancer
    || geomancer.stableId !== 'card:6fe9e8652f3106e7ae6161ddf2c2f71cc615251462b11040b0fa8128af4863a3'
    || geomancer.officialSourceId !== '002-geomancer-b-f'
    || geomancer.cardType !== 'avatar'
    || geomancer.attack !== 1
    || geomancer.defense !== 1
    || geomancer.life !== 20
    || geomancer.rarity !== null
    || geomancer.elements.length !== 0
    || geomancer.thresholds.air !== 0
    || geomancer.thresholds.earth !== 0
    || geomancer.thresholds.fire !== 0
    || geomancer.thresholds.water !== 0
    || geomancer.rulesText.trim().length !== 169) {
    throw new Error('private Geomancer no longer matches its supported facts');
  }
  const sparkmage = snapshot.cards.find(({ name }) => name === 'Sparkmage');
  if (!sparkmage
    || sparkmage.stableId !== 'card:15f5ffe507d7baba2cf45ce714a03287feb825581d7f2a10ae7baebb93b6c3f7'
    || sparkmage.officialSourceId !== '002-sparkmage-b-f'
    || sparkmage.cardType !== 'avatar'
    || sparkmage.attack !== 1
    || sparkmage.defense !== 1
    || sparkmage.life !== 20
    || sparkmage.rarity !== null
    || sparkmage.elements.length !== 0
    || sparkmage.thresholds.air !== 0
    || sparkmage.thresholds.earth !== 0
    || sparkmage.thresholds.fire !== 0
    || sparkmage.thresholds.water !== 0
    || sparkmage.rulesText.trim().length !== 157) {
    throw new Error('private Sparkmage no longer matches its supported facts');
  }
  const midnightRogue = snapshot.cards.find(({ name }) => name === 'Midnight Rogue');
  const midnightRogueKeywords = midnightRogue?.rulesText.toLowerCase().match(/[a-z]+/g) ?? [];
  if (!midnightRogue
    || midnightRogue.stableId
      !== 'card:c0b54519507e1fc94d51a6b585f1456cbe98850317629489261ae7939c4faf0b'
    || midnightRogue.officialSourceId !== '001-midnight_rogue-b-f'
    || midnightRogue.cardType !== 'minion'
    || midnightRogueKeywords.length !== 2
    || !midnightRogueKeywords.includes('ranged')
    || !midnightRogueKeywords.includes('stealth')
    || midnightRogue.manaCost !== 3
    || midnightRogue.attack !== 2
    || midnightRogue.defense !== 2
    || midnightRogue.life !== null
    || midnightRogue.elements.length !== 1
    || midnightRogue.elements[0] !== 'air'
    || midnightRogue.thresholds.air !== 1
    || midnightRogue.thresholds.earth !== 0
    || midnightRogue.thresholds.fire !== 0
    || midnightRogue.thresholds.water !== 0
    || midnightRogue.rarity !== 'ordinary') {
    throw new Error('private Ranged Stealth minion no longer matches its supported facts');
  }
  const deadOfNightDemon = snapshot.cards.find(({ name }) => name === 'Dead of Night Demon');
  if (!deadOfNightDemon
    || deadOfNightDemon.stableId
      !== 'card:8b8120a300bf81eab49b29ab68aab941c883722a7ec58df93d6f7705a8693319'
    || deadOfNightDemon.officialSourceId !== '001-dead_of_night_demon-b-f'
    || deadOfNightDemon.cardType !== 'minion'
    || deadOfNightDemon.rulesText.trim().toLowerCase() !== 'stealth'
    || deadOfNightDemon.manaCost !== 2
    || deadOfNightDemon.attack !== 2
    || deadOfNightDemon.defense !== 2
    || deadOfNightDemon.life !== null
    || deadOfNightDemon.elements.length !== 1
    || deadOfNightDemon.elements[0] !== 'air'
    || deadOfNightDemon.thresholds.air !== 1
    || deadOfNightDemon.thresholds.earth !== 0
    || deadOfNightDemon.thresholds.fire !== 0
    || deadOfNightDemon.thresholds.water !== 0
    || deadOfNightDemon.rarity !== 'ordinary') {
    throw new Error('private Stealth Demon no longer matches its supported facts');
  }
  const gyreHippogriffs = snapshot.cards.find(({ name }) => name === 'Gyre Hippogriffs');
  const gyreKeywords = gyreHippogriffs?.rulesText.toLowerCase().match(/[a-z]+/g) ?? [];
  if (!gyreHippogriffs
    || gyreHippogriffs.stableId
      !== 'card:0ca5beff70b55e927ac7ea0800a25ca3a820b4a02cf55ec426bc0fa36101584b'
    || gyreHippogriffs.officialSourceId !== '001-gyre_hippogriffs-b-f'
    || gyreHippogriffs.cardType !== 'minion'
    || gyreKeywords.length !== 2
    || !gyreKeywords.includes('airborne')
    || !gyreKeywords.includes('charge')
    || gyreHippogriffs.manaCost !== 4
    || gyreHippogriffs.attack !== 3
    || gyreHippogriffs.defense !== 3
    || gyreHippogriffs.life !== null
    || gyreHippogriffs.elements.length !== 1
    || gyreHippogriffs.elements[0] !== 'air'
    || gyreHippogriffs.thresholds.air !== 2
    || gyreHippogriffs.thresholds.earth !== 0
    || gyreHippogriffs.thresholds.fire !== 0
    || gyreHippogriffs.thresholds.water !== 0
    || gyreHippogriffs.rarity !== 'exceptional') {
    throw new Error('private Airborne Charge Hippogriffs no longer match their supported facts');
  }
  const highlandClansmen = snapshot.cards.find(({ name }) => name === 'Highland Clansmen');
  if (!highlandClansmen
    || highlandClansmen.stableId
      !== 'card:017aecc6e8766f7a5507234db7720a1a85179033b8d6963920522a72fe4e8633'
    || highlandClansmen.officialSourceId !== '001-highland_clansmen-b-f'
    || highlandClansmen.cardType !== 'minion'
    || highlandClansmen.rulesText.trim().toLowerCase() !== 'charge'
    || highlandClansmen.manaCost !== 7
    || highlandClansmen.attack !== 5
    || highlandClansmen.defense !== 5
    || highlandClansmen.life !== null
    || highlandClansmen.elements.length !== 1
    || highlandClansmen.elements[0] !== 'air'
    || highlandClansmen.thresholds.air !== 1
    || highlandClansmen.thresholds.earth !== 0
    || highlandClansmen.thresholds.fire !== 0
    || highlandClansmen.thresholds.water !== 0
    || highlandClansmen.rarity !== 'ordinary') {
    throw new Error('private Charge Clansmen no longer match their supported facts');
  }
  const autumnUnicorn = snapshot.cards.find(({ name }) => name === 'Autumn Unicorn');
  if (!autumnUnicorn
    || autumnUnicorn.stableId
      !== 'card:c9dda83781a55e4af97f34a3cfb3acbb515ba6768c5e5540c2bac91cc0971608'
    || autumnUnicorn.officialSourceId !== '001-autumn_unicorn-b-f'
    || autumnUnicorn.cardType !== 'minion'
    || autumnUnicorn.rulesText.trim() !== ''
    || autumnUnicorn.manaCost !== 3
    || autumnUnicorn.attack !== 4
    || autumnUnicorn.defense !== 4
    || autumnUnicorn.life !== null
    || autumnUnicorn.elements.length !== 1
    || autumnUnicorn.elements[0] !== 'earth'
    || autumnUnicorn.thresholds.air !== 0
    || autumnUnicorn.thresholds.earth !== 2
    || autumnUnicorn.thresholds.fire !== 0
    || autumnUnicorn.thresholds.water !== 0
    || autumnUnicorn.rarity !== 'exceptional') {
    throw new Error('private blank Earth Unicorn no longer matches its supported facts');
  }
  const amazonWarriors = snapshot.cards.find(({ name }) => name === 'Amazon Warriors');
  if (!amazonWarriors
    || amazonWarriors.stableId
      !== 'card:b980d7ff8df3b87d505c00d079d6f3b8676e3beb98cf37ba9d40dbe4e62fc515'
    || amazonWarriors.officialSourceId !== '001-amazon_warriors-b-f'
    || amazonWarriors.cardType !== 'minion'
    || amazonWarriors.rulesText.trim() !== ''
    || amazonWarriors.manaCost !== 5
    || amazonWarriors.attack !== 5
    || amazonWarriors.defense !== 5
    || amazonWarriors.life !== null
    || amazonWarriors.elements.length !== 1
    || amazonWarriors.elements[0] !== 'earth'
    || amazonWarriors.thresholds.air !== 0
    || amazonWarriors.thresholds.earth !== 1
    || amazonWarriors.thresholds.fire !== 0
    || amazonWarriors.thresholds.water !== 0
    || amazonWarriors.rarity !== 'ordinary') {
    throw new Error('private blank Earth Warriors no longer match their supported facts');
  }
  const wildBoars = snapshot.cards.find(({ name }) => name === 'Wild Boars');
  if (!wildBoars
    || wildBoars.stableId !== 'card:9247ae85dd2fd35068d2d6034ccb2a40da872130362e8f82ab9c9a8ddb19037e'
    || wildBoars.cardType !== 'minion'
    || wildBoars.rulesText.trim() !== ''
    || wildBoars.manaCost !== 1
    || wildBoars.attack !== 2
    || wildBoars.defense !== 2
    || wildBoars.life !== null
    || wildBoars.elements.length !== 1
    || wildBoars.elements[0] !== 'earth'
    || wildBoars.thresholds.air !== 0
    || wildBoars.thresholds.earth !== 1
    || wildBoars.thresholds.fire !== 0
    || wildBoars.thresholds.water !== 0
    || wildBoars.rarity !== 'ordinary') {
    throw new Error('private blank Earth Wild Boars no longer matches its supported facts');
  }
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
  const duel = snapshot.cards.find(({ name }) => name === 'Duel');
  if (!duel
    || duel.cardType !== 'magic'
    || duel.rulesText.trim() !== 'An ally fights target enemy adjacent to it.'
    || duel.manaCost !== 3
    || duel.attack !== null
    || duel.defense !== null
    || duel.life !== null
    || duel.elements.length !== 1
    || duel.elements[0] !== 'earth'
    || duel.thresholds.air !== 0
    || duel.thresholds.earth !== 1
    || duel.thresholds.fire !== 0
    || duel.thresholds.water !== 0
    || duel.rarity !== 'ordinary') {
    throw new Error('private ally-versus-adjacent-enemy Duel Magic no longer matches its supported facts');
  }
  const borderMilitia = snapshot.cards.find(({ name }) => name === 'Border Militia');
  if (!borderMilitia
    || borderMilitia.stableId !== 'card:30d6aacf5064c002c058a7a6d5d0b2e3834243fa335c303acb2e9269167281f4'
    || borderMilitia.cardType !== 'magic'
    || borderMilitia.rulesText.trim()
      !== 'Summon a Foot Soldier token to each site you control that borders an enemy site.'
    || borderMilitia.manaCost !== 3
    || borderMilitia.attack !== null
    || borderMilitia.defense !== null
    || borderMilitia.life !== null
    || borderMilitia.elements.length !== 1
    || borderMilitia.elements[0] !== 'earth'
    || borderMilitia.thresholds.air !== 0
    || borderMilitia.thresholds.earth !== 1
    || borderMilitia.thresholds.fire !== 0
    || borderMilitia.thresholds.water !== 0
    || borderMilitia.rarity !== 'ordinary') {
    throw new Error('private Border Militia no longer matches its supported facts');
  }
  const footSoldier = snapshot.cards.find(({ name }) => name === 'Foot Soldier');
  if (!footSoldier
    || footSoldier.stableId !== 'card:064b7fb4b8ef0fcf60aa28f2d6562f70e22bb7f023adfe32353a7a188b79e0a7'
    || footSoldier.cardType !== 'minion'
    || footSoldier.rulesText.trim() !== ''
    || footSoldier.manaCost !== null
    || footSoldier.attack !== 1
    || footSoldier.defense !== 1
    || footSoldier.life !== null
    || footSoldier.elements.length !== 0
    || footSoldier.thresholds.air !== 0
    || footSoldier.thresholds.earth !== 0
    || footSoldier.thresholds.fire !== 0
    || footSoldier.thresholds.water !== 0
    || footSoldier.rarity !== 'ordinary') {
    throw new Error('private Foot Soldier token no longer matches its supported facts');
  }
  const humbleVillage = snapshot.cards.find(({ name }) => name === 'Humble Village');
  if (!humbleVillage
    || humbleVillage.stableId
      !== 'card:d6211d4878dc1d28bb72a5d6a48821807277b9a067d51607514dab004cf5feca'
    || humbleVillage.cardType !== 'site'
    || humbleVillage.rulesText.trim()
      !== 'Genesis → You may pay ① to summon a Foot Soldier token here.'
    || humbleVillage.manaCost !== null
    || humbleVillage.attack !== null
    || humbleVillage.defense !== null
    || humbleVillage.life !== null
    || humbleVillage.elements.length !== 1
    || humbleVillage.elements[0] !== 'earth'
    || humbleVillage.thresholds.air !== 0
    || humbleVillage.thresholds.earth !== 1
    || humbleVillage.thresholds.fire !== 0
    || humbleVillage.thresholds.water !== 0
    || humbleVillage.rarity !== 'ordinary') {
    throw new Error('private optional paid Genesis site no longer matches its supported facts');
  }
  const rusticVillage = ordinarySiteWithRules(
    'Rustic Village',
    'card:0212a50acf38b36f9ce70a348885b587263855c6b174737994ca8cddc408910c',
    '001-rustic_village-b-f',
    'earth',
    60,
  );
  const simpleVillage = ordinarySiteWithRules(
    'Simple Village',
    'card:25d60f9bf9d9a623a101bc4ed5f4e6bace8cc85bf1522df7a04ec25daddf9546',
    '001-simple_village-b-f',
    'earth',
    60,
  );
  if (rusticVillage.rulesText !== humbleVillage.rulesText
    || simpleVillage.rulesText !== humbleVillage.rulesText) {
    throw new Error('private ordinary Village rules no longer match Humble Village');
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
  const bladderblimp = snapshot.cards.find(({ name }) => name === 'Bladderblimp');
  if (!bladderblimp
    || bladderblimp.cardType !== 'minion'
    || bladderblimp.rulesText.trim()
      !== 'Airborne\n\nDeathrite → Players lose 1 life for each nearby site they control.'
    || bladderblimp.manaCost !== 5
    || bladderblimp.attack !== 3
    || bladderblimp.defense !== 3
    || bladderblimp.life !== null
    || bladderblimp.elements.length !== 1
    || bladderblimp.elements[0] !== 'air'
    || bladderblimp.thresholds.air !== 1
    || bladderblimp.thresholds.earth !== 0
    || bladderblimp.thresholds.fire !== 0
    || bladderblimp.thresholds.water !== 0
    || bladderblimp.rarity !== 'exceptional') {
    throw new Error('private nearby-site life-loss Deathrite minion no longer matches its supported facts');
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
  const shellycoat = snapshot.cards.find(({ name }) => name === 'Shellycoat');
  if (!shellycoat
    || shellycoat.stableId !== 'card:deb2bac179433719233b6e569a960329ca2a5ad0a983b076fffdb11e09645d47'
    || shellycoat.cardType !== 'minion'
    || shellycoat.rulesText.trim() !== 'Submerge\r\n\r\nTakes 1 less damage.'
    || shellycoat.manaCost !== 2
    || shellycoat.attack !== 2
    || shellycoat.defense !== 2
    || shellycoat.life !== null
    || shellycoat.elements.length !== 1
    || shellycoat.elements[0] !== 'water'
    || shellycoat.thresholds.air !== 0
    || shellycoat.thresholds.earth !== 0
    || shellycoat.thresholds.fire !== 0
    || shellycoat.thresholds.water !== 1
    || shellycoat.rarity !== 'ordinary') {
    throw new Error('private damage-reduction minion no longer matches its supported facts');
  }
  const staticServant = snapshot.cards.find(({ name }) => name === 'Static Servant');
  if (!staticServant
    || staticServant.cardType !== 'minion'
    || staticServant.rulesText.trim() !== 'Genesis → Each other unit here takes 1 damage.'
    || staticServant.manaCost !== 2
    || staticServant.attack !== 2
    || staticServant.defense !== 2
    || staticServant.life !== null
    || staticServant.elements.length !== 1
    || staticServant.elements[0] !== 'air'
    || staticServant.thresholds.air !== 1
    || staticServant.thresholds.earth !== 0
    || staticServant.thresholds.fire !== 0
    || staticServant.thresholds.water !== 0
    || staticServant.rarity !== 'ordinary') {
    throw new Error('private location-wide Genesis damage minion no longer matches its supported facts');
  }
  const vileImp = snapshot.cards.find(({ name }) => name === 'Vile Imp');
  if (!vileImp
    || vileImp.stableId !== 'card:61054ba4b2dbd90c7b2729eda4a7d24ba509c0d00a4bee8c534eb7ce35bdb856'
    || vileImp.cardType !== 'minion'
    || vileImp.rulesText.trim() !== 'Genesis → May deal 2 damage to target adjacent unit.'
    || vileImp.manaCost !== 2
    || vileImp.attack !== 2
    || vileImp.defense !== 2
    || vileImp.life !== null
    || vileImp.elements.length !== 1
    || vileImp.elements[0] !== 'fire'
    || vileImp.thresholds.air !== 0
    || vileImp.thresholds.earth !== 0
    || vileImp.thresholds.fire !== 1
    || vileImp.thresholds.water !== 0
    || vileImp.rarity !== 'ordinary') {
    throw new Error('private optional targeted Genesis damage minion no longer matches its supported facts');
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
  const aramosMercenaries = snapshot.cards.find(({ name }) => name === 'Aramos Mercenaries');
  if (!aramosMercenaries
    || aramosMercenaries.cardType !== 'minion'
    || aramosMercenaries.rulesText.trim()
      !== "You may discard a random card rather than pay this spell's mana cost."
    || aramosMercenaries.manaCost !== 3
    || aramosMercenaries.attack !== 3
    || aramosMercenaries.defense !== 3
    || aramosMercenaries.life !== null
    || aramosMercenaries.elements.length !== 1
    || aramosMercenaries.elements[0] !== 'fire'
    || aramosMercenaries.thresholds.air !== 0
    || aramosMercenaries.thresholds.earth !== 0
    || aramosMercenaries.thresholds.fire !== 2
    || aramosMercenaries.thresholds.water !== 0
    || aramosMercenaries.rarity !== 'ordinary') {
    throw new Error('private random-discard alternative-cost minion no longer matches its supported facts');
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
  const grainSparrow = snapshot.cards.find(({ name }) => name === 'Grain Sparrow');
  if (!grainSparrow
    || grainSparrow.cardType !== 'minion'
    || grainSparrow.rulesText.trim() !== 'Airborne\r\n\r\nGenesis → Gain 2 life.'
    || grainSparrow.manaCost !== 1
    || grainSparrow.attack !== 1
    || grainSparrow.defense !== 1
    || grainSparrow.life !== null
    || grainSparrow.elements.length !== 1
    || grainSparrow.elements[0] !== 'earth'
    || grainSparrow.thresholds.air !== 0
    || grainSparrow.thresholds.earth !== 1
    || grainSparrow.thresholds.fire !== 0
    || grainSparrow.thresholds.water !== 0
    || grainSparrow.rarity !== 'ordinary') {
    throw new Error('private Airborne Genesis-healing minion no longer matches its supported facts');
  }
  const steppe = snapshot.cards.find(({ name }) => name === 'Steppe');
  if (!steppe
    || steppe.cardType !== 'site'
    || steppe.rulesText.trim() !== ''
    || steppe.manaCost !== null
    || steppe.attack !== null
    || steppe.defense !== null
    || steppe.life !== null
    || steppe.elements.length !== 2
    || steppe.elements[0] !== 'earth'
    || steppe.elements[1] !== 'fire'
    || steppe.thresholds.air !== 0
    || steppe.thresholds.earth !== 1
    || steppe.thresholds.fire !== 1
    || steppe.thresholds.water !== 0
    || steppe.rarity !== 'exceptional') {
    throw new Error('private Earth-Fire support site no longer matches its supported facts');
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
  const lash = snapshot.cards.find(({ name }) => name === 'Lash');
  if (!lash
    || lash.cardType !== 'magic'
    || lash.rulesText.trim() !== 'Deal 1 damage to target nearby minion and untap it.'
    || lash.manaCost !== 3
    || lash.attack !== null
    || lash.defense !== null
    || lash.life !== null
    || lash.elements.length !== 1
    || lash.elements[0] !== 'fire'
    || lash.thresholds.air !== 0
    || lash.thresholds.earth !== 0
    || lash.thresholds.fire !== 1
    || lash.thresholds.water !== 0
    || lash.rarity !== 'ordinary') {
    throw new Error('private nearby damage-and-untap Magic no longer matches its supported facts');
  }
  const leapAttack = snapshot.cards.find(({ name }) => name === 'Leap Attack');
  if (!leapAttack
    || leapAttack.cardType !== 'magic'
    || leapAttack.rulesText.trim()
      !== 'An ally may take a step, and then it strikes each enemy at its location.'
    || leapAttack.manaCost !== 4
    || leapAttack.attack !== null
    || leapAttack.defense !== null
    || leapAttack.life !== null
    || leapAttack.elements.length !== 1
    || leapAttack.elements[0] !== 'fire'
    || leapAttack.thresholds.air !== 0
    || leapAttack.thresholds.earth !== 0
    || leapAttack.thresholds.fire !== 1
    || leapAttack.thresholds.water !== 0
    || leapAttack.rarity !== 'exceptional') {
    throw new Error('private optional-step strike-all Leap Attack Magic no longer matches its supported facts');
  }
  const freeze = snapshot.cards.find(({ name }) => name === 'Freeze');
  if (!freeze
    || freeze.stableId !== 'card:d8081f9a38d1c16caff073da5d5afc9c5acb71b7e8748e73f4d5febb76d83a13'
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
  const mesmerism = snapshot.cards.find(({ name }) => name === 'Mesmerism');
  if (!mesmerism
    || mesmerism.stableId !== 'card:89f8ba7cb2d53b610b0d3307c21b97ebfbddcccf56d07a1bf447e3d2b6d52847'
    || mesmerism.cardType !== 'magic'
    || mesmerism.rulesText.trim() !== 'Gain control of target nearby minion.'
    || mesmerism.manaCost !== 4
    || mesmerism.attack !== null
    || mesmerism.defense !== null
    || mesmerism.life !== null
    || mesmerism.elements.length !== 1
    || mesmerism.elements[0] !== 'water'
    || mesmerism.thresholds.air !== 0
    || mesmerism.thresholds.earth !== 0
    || mesmerism.thresholds.fire !== 0
    || mesmerism.thresholds.water !== 4
    || mesmerism.rarity !== 'unique') {
    throw new Error('private nearby minion control Magic no longer matches its supported facts');
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
    || seravaTownsfolk.stableId !== 'card:71bbb7f8b57789ac2ad9b0062c2440927eff0102f93eb9b093c4d12bbf9684f8'
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
  const gnarledWendigo = snapshot.cards.find(({ name }) => name === 'Gnarled Wendigo');
  if (!gnarledWendigo
    || gnarledWendigo.cardType !== 'minion'
    || gnarledWendigo.rulesText.trim()
      !== 'This costs (2) less to cast for each minion you sacrifice at its summoning location.'
    || gnarledWendigo.manaCost !== 6
    || gnarledWendigo.attack !== 5
    || gnarledWendigo.defense !== 5
    || gnarledWendigo.life !== null
    || gnarledWendigo.elements.length !== 1
    || gnarledWendigo.elements[0] !== 'water'
    || gnarledWendigo.thresholds.air !== 0
    || gnarledWendigo.thresholds.earth !== 0
    || gnarledWendigo.thresholds.fire !== 0
    || gnarledWendigo.thresholds.water !== 1
    || gnarledWendigo.rarity !== 'exceptional') {
    throw new Error('private summon-location sacrifice-discount minion no longer matches its supported facts');
  }
  const raalDromedary = snapshot.cards.find(({ name }) => name === 'Raal Dromedary');
  if (!raalDromedary
    || raalDromedary.stableId !== 'card:933780f9cfe36f9e90affd2c856557cec093e35b97a1059fa2e0606ba2b3d2c7'
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
  const vikings = snapshot.cards.find(({ name }) => name === 'Vikings');
  if (!vikings
    || vikings.cardType !== 'minion'
    || vikings.rulesText.trim() !== 'Tap → Deal 2 damage to each unit at target adjacent location.'
    || vikings.manaCost !== 5
    || vikings.attack !== 4
    || vikings.defense !== 4
    || vikings.life !== null
    || vikings.elements.length !== 1
    || vikings.elements[0] !== 'fire'
    || vikings.thresholds.air !== 0
    || vikings.thresholds.earth !== 0
    || vikings.thresholds.fire !== 2
    || vikings.thresholds.water !== 0
    || vikings.rarity !== 'ordinary') {
    throw new Error('private adjacent-location damage minion no longer matches its supported facts');
  }
  const recklessSquire = snapshot.cards.find(({ name }) => name === 'Reckless Squire');
  if (!recklessSquire
    || recklessSquire.cardType !== 'minion'
    || recklessSquire.rulesText.trim() !== 'Charge, Lance'
    || recklessSquire.manaCost !== 3
    || recklessSquire.attack !== 1
    || recklessSquire.defense !== 1
    || recklessSquire.life !== null
    || recklessSquire.elements.length !== 1
    || recklessSquire.elements[0] !== 'fire'
    || recklessSquire.thresholds.air !== 0
    || recklessSquire.thresholds.earth !== 0
    || recklessSquire.thresholds.fire !== 1
    || recklessSquire.thresholds.water !== 0
    || recklessSquire.rarity !== 'ordinary') {
    throw new Error('private Lance minion no longer matches its supported facts');
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
    || stealthTargetMinion.stableId !== 'card:a842565b1d51d7f2a85c795123c4492c937273e354403718d555f5ee947c0426'
    || stealthTargetMinion.cardType !== 'minion'
    || stealthTargetMinion.rulesText.trim() !== ''
    || stealthTargetMinion.manaCost !== 1
    || stealthTargetMinion.attack !== 2
    || stealthTargetMinion.defense !== 2
    || stealthTargetMinion.life !== null
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
    || submergeMinion.stableId
      !== 'card:d55af34a9a87e85723613b133fdefc561e703cedffbd63bb21d9b6bee59def47'
    || submergeMinion.cardType !== 'minion'
    || submergeMinion.rulesText.trim() !== 'Submerge'
    || submergeMinion.manaCost !== 3
    || submergeMinion.attack !== 3
    || submergeMinion.defense !== 3
    || submergeMinion.life !== null
    || submergeMinion.elements.length !== 1
    || submergeMinion.elements[0] !== 'water'
    || submergeMinion.thresholds.air !== 0
    || submergeMinion.thresholds.earth !== 0
    || submergeMinion.thresholds.fire !== 0
    || submergeMinion.thresholds.water !== 1
    || submergeMinion.rarity !== 'ordinary') {
    throw new Error('private Submerge minion no longer matches its supported facts');
  }
  const seaWitch = snapshot.cards.find(({ name }) => name === 'Sea Witch');
  if (!seaWitch
    || seaWitch.stableId
      !== 'card:a58188635692f16a219b8d53f903f93d55f58e301fda4aea86073f24f2b8d6d6'
    || seaWitch.cardType !== 'minion'
    || seaWitch.rulesText.trim() !== 'Spellcaster, Submerge'
    || seaWitch.manaCost !== 2
    || seaWitch.attack !== 2
    || seaWitch.defense !== 2
    || seaWitch.life !== null
    || seaWitch.elements.length !== 1
    || seaWitch.elements[0] !== 'water'
    || seaWitch.thresholds.air !== 0
    || seaWitch.thresholds.earth !== 0
    || seaWitch.thresholds.fire !== 0
    || seaWitch.thresholds.water !== 1
    || seaWitch.rarity !== 'ordinary') {
    throw new Error('private underwater Spellcaster minion no longer matches its supported facts');
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
  const fatality = snapshot.cards.find(({ name }) => name === 'Fatality');
  if (!fatality
    || fatality.stableId !== 'card:37c0aadb42753d7d4bab3e2155e5f10c7e2537b5fe3cfa16714e5a24ad740cde'
    || fatality.cardType !== 'magic'
    || fatality.rulesText.trim() !== 'Kill target wounded minion.'
    || fatality.manaCost !== 3
    || fatality.attack !== null
    || fatality.defense !== null
    || fatality.life !== null
    || fatality.elements.length !== 2
    || fatality.elements[0] !== 'fire'
    || fatality.elements[1] !== 'air'
    || fatality.thresholds.air !== 1
    || fatality.thresholds.earth !== 0
    || fatality.thresholds.fire !== 1
    || fatality.thresholds.water !== 0
    || fatality.rarity !== 'exceptional') {
    throw new Error('private wounded-minion kill Magic no longer matches its supported facts');
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
  const malakhim = snapshot.cards.find(({ name }) => name === 'Malakhim');
  if (!malakhim
    || malakhim.stableId !== 'card:bae390c28ebe0ec9adbcb37ba77572ac944c54a3de1957a3ab56b8b78f297b33'
    || malakhim.cardType !== 'minion'
    || malakhim.rulesText.trim().replaceAll('\r\n', '\n')
      !== 'Airborne, Ward\n\nAt the end of your turn, untap Malakhim.'
    || malakhim.manaCost !== 6
    || malakhim.attack !== 4
    || malakhim.defense !== 4
    || malakhim.life !== null
    || malakhim.elements.length !== 1
    || malakhim.elements[0] !== 'earth'
    || malakhim.thresholds.air !== 0
    || malakhim.thresholds.earth !== 3
    || malakhim.thresholds.fire !== 0
    || malakhim.thresholds.water !== 0
    || malakhim.rarity !== 'elite') {
    throw new Error('private end-turn untap minion no longer matches its supported facts');
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
    amazonWarriors,
    airborneMinion,
    airborneTargetMinion,
    aramosMercenaries,
    arcLightning,
    autumnRiver,
    autumnUnicorn,
    authorityHash: artifact.contentHash,
    bladderblimp,
    borderMilitia,
    bury,
    burrowingMinion,
    cards: snapshot.cards,
    cannotDefendMinion,
    chargeMagic,
    chargeMinion,
    config,
    dalceanPhalanx,
    deathriteMinion,
    deadOfNightDemon,
    darkTower,
    divineHealing,
    duel,
    drown,
    drowned,
    earthProviderMinion,
    elthamTownsfolk,
    entombed,
    format: selected.identity.payload,
    formatStableId: selected.identity.stableId,
    firstStrikeMinion,
    firstStrikeTargetMinion,
    footSoldier,
    forsaken,
    freeze,
    fatality,
    genesisSpellMinion,
    genesisMinion,
    geomancer,
    gothicTower,
    sparkmage,
    granaryRats,
    grainSparrow,
    gyreHippogriffs,
    gnarledWendigo,
    ghostTownSite,
    hamlet,
    healingMinion,
    huntersLodge,
    highlandClansmen,
    humbleVillage,
    lethalMinion,
    leylineHenge,
    lesserBloodDemon,
    ignited,
    lash,
    leapAttack,
    lightningBolt,
    lugbogCat,
    lure,
    malakhim,
    mesmerism,
    lumberingMinion,
    loneTower,
    manaMinion,
    midnightRogue,
    minorExplosion,
    monstrousLion,
    movementMinion,
    movementTwoMinion,
    overpower,
    polarBears,
    pirateShip,
    poisonousDagger,
    pudgeButcher,
    rainOfArrows,
    staticServant,
    swordAndShield,
    providerMinion,
    raalDromedary,
    recklessSquire,
    rangedMinion,
    rescue,
    rusticVillage,
    roamingMinion,
    secretTunnel,
    sedgeCrabs,
    seravaTownsfolk,
    shellycoat,
    shallowGrave,
    sinkhole,
    simpleVillage,
    slyFox,
    spire,
    stealthMinion,
    stealthTargetMinion,
    steppe,
    stream,
    submergeMinion,
    seaWitch,
    teleport,
    valley,
    vileImp,
    vikings,
    voidwalkMinion,
    wardMinion,
    wasteland,
    wildBoars,
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
  siteGenesisGainManaIfOnlyControlledCopy = false,
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
  genesisHealController: 0 | 2 = 0,
  untapTargetMinionAfterDamage = false,
  discardRandomCardInsteadOfMana = false,
  fightAllyWithAdjacentEnemy = false,
  deathriteLoseLifePerNearbySiteControlled = false,
  leapAttackAlly = false,
  genesisDamageEachOtherUnitHere: 0 | 1 = 0,
  genesisMayDamageTargetAdjacentUnit: 0 | 2 = 0,
  sacrificeMinionAtSummoningLocationForManaDiscount: 0 | 2 = 0,
  lanceCount: 0 | 1 = 0,
  grantsBearerPower: 0 | 2 = 0,
  grantsBearerLethal = false,
  siteGenesisEnemiesLoseStealth = false,
  tapToDamageEachUnitAtAdjacentLocation = false,
  gainControlOfTargetNearbyMinion = false,
  untapsAtEndOfControllerTurn = false,
  killTargetWoundedMinion = false,
  ordinaryMinionManaDiscount: 0 | 1 = 0,
  siteProvidesNoThreshold = false,
  takesLessDamage: 0 | 1 = 0,
  siteGenesisPayOneManaToSummonToken?: string,
  summonTokenToEachControlledSiteBorderingEnemySite?: string,
  token = false,
  siteGenesisMayBottomNextSpell = false,
  earthSitePlayCreatesAdjacentRubble = false,
  replaceAdjacentRubbleWithTopAtlasSite = false,
  tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn = false,
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
      ...(earthSitePlayCreatesAdjacentRubble
        ? { earthSitePlayCreatesAdjacentRubble: true as const }
        : {}),
      life: card.life,
      ...(replaceAdjacentRubbleWithTopAtlasSite
        ? { replaceAdjacentRubbleWithTopAtlasSite: true as const }
        : {}),
      ...(tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn
        ? { tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true as const }
        : {}),
    };
  }
  if (card.cardType === 'artifact'
    && card.manaCost !== null
    && Number(grantsBearerPower === 2) + Number(grantsBearerLethal) === 1) {
    return {
      cardType: 'artifact',
      ...(grantsBearerPower === 2
        ? { grantsBearerPower }
        : { grantsBearerLethal: true as const }),
      manaCost: card.manaCost,
      thresholds: card.thresholds,
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
      ...(siteGenesisGainManaIfOnlyControlledCopy
        ? { genesisGainManaIfOnlyControlledCopy: 1 as const }
        : {}),
      ...(siteGenesisPayOneManaToSummonToken
        ? { genesisPayOneManaToSummonToken: siteGenesisPayOneManaToSummonToken }
        : {}),
      ...(siteGenesisMayBottomNextSpell ? { genesisMayBottomNextSpell: true } : {}),
      ...(ordinaryMinionManaDiscount ? { ordinaryMinionManaDiscount } : {}),
      ...(sacrificeToDestroyNearbySite ? { sacrificeToDestroyNearbySite: true } : {}),
      ...(siteGenesisEnemiesLoseStealth ? { genesisEnemiesLoseStealth: true } : {}),
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
    + Number(burrowTargetMinion)
    + Number(fightAllyWithAdjacentEnemy)
    + Number(gainControlOfTargetNearbyMinion)
    + Number(killTargetWoundedMinion)
    + Number(leapAttackAlly)
    + Number(Boolean(summonTokenToEachControlledSiteBorderingEnemySite));
  if (card.cardType === 'magic'
    && card.manaCost !== null
    && supportedMagicEffects === 1) {
    return {
      ...(burrowTargetMinion ? { burrowTargetMinion: true } : {}),
      cardType: 'magic',
      ...(damageEachAbovegroundMinion !== 0 ? { damageEachAbovegroundMinion } : {}),
      ...(fightAllyWithAdjacentEnemy ? { fightAllyWithAdjacentEnemy: true } : {}),
      ...(gainControlOfTargetNearbyMinion ? { gainControlOfTargetNearbyMinion: true } : {}),
      ...(killTargetWoundedMinion ? { killTargetWoundedMinion: true } : {}),
      ...(grantPowerToAllyThisTurn !== 0 ? { grantPowerToAllyThisTurn } : {}),
      ...(leapAttackAlly ? { leapAttackAlly: true } : {}),
      ...(submergeTargetMinion ? { submergeTargetMinion: true } : {}),
      ...(summonTokenToEachControlledSiteBorderingEnemySite
        ? { summonTokenToEachControlledSiteBorderingEnemySite }
        : {}),
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
      ...(untapTargetMinionAfterDamage ? { untapTargetMinionAfterDamage: true } : {}),
    };
  }
  if (card.cardType === 'minion'
    && card.attack !== null
    && card.defense !== null
    && (card.manaCost !== null || token)) {
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
      ...(deathriteLoseLifePerNearbySiteControlled
        ? { deathriteLoseLifePerNearbySiteControlled: 1 as const }
        : {}),
      defense: card.defense,
      ...(discardRandomCardInsteadOfMana ? { discardRandomCardInsteadOfMana: true } : {}),
      ...(diesAtEndOfControllerTurn ? { diesAtEndOfControllerTurn: true } : {}),
      ...(genesisHealController ? { genesisHealController } : {}),
      ...(genesisDamageEachOtherUnitHere ? { genesisDamageEachOtherUnitHere } : {}),
      ...(genesisMayDamageTargetAdjacentUnit ? { genesisMayDamageTargetAdjacentUnit } : {}),
      genesisDrawSpell,
      genesisDrawSite,
      ...(genesisLoseControllerLife ? { genesisLoseControllerLife } : {}),
      gainsStealthAtEndOfTurn,
      immobile,
      lethal,
      ...(lanceCount ? { lanceCount } : {}),
      manaCost: card.manaCost ?? 0,
      movesOnlyForward,
      mustBeCastBurrowed,
      mustBeCastSubmerged,
      mustBeCastToOuterColumn,
      mustBeCastToWaterSite,
      ...(movementBonus ? { movementBonus } : {}),
      movesOnlySideways,
      ...(card.rarity === 'ordinary' ? { ordinary: true as const } : {}),
      ...(provides ? { provides } : {}),
      ranged,
      ...(sacrificeMinionAtSummoningLocationForManaDiscount
        ? { sacrificeMinionAtSummoningLocationForManaDiscount }
        : {}),
      shootsDragProjectile,
      ...(siteProvidesNoThreshold ? { siteProvidesNoThreshold: true } : {}),
      spellcaster,
      stealth,
      strikesFirstWhileAttacking,
      submerge,
      summonToAnySite,
      ...(untapsAtEndOfControllerTurn ? { untapsAtEndOfControllerTurn: true } : {}),
      ...(tapToDamageEachUnitAtAdjacentLocation
        ? { tapToDamageEachUnitAtAdjacentLocation: 2 as const }
        : {}),
      ...(tapForMana ? { tapForMana } : {}),
      ...(takesLessDamage ? { takesLessDamage } : {}),
      thresholds: card.thresholds,
      ...(token ? { token: true as const } : {}),
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
  scenario: 'air' | 'air-arc-lightning' | 'air-bladderblimp' | 'air-fire-fatality' | 'air-genesis-spell' | 'air-leyline' | 'air-lightning-bolt' | 'air-rain-of-arrows' | 'air-spellcaster-freeze' | 'air-static-servant' | 'air-teleport' | 'air-void-artifact' | 'air-voidwalk' | 'air-zap' | 'airborne' | BetaLessonScenario | 'combat' | 'earth' | 'earth-border-militia' | 'earth-burrowing' | 'earth-bury' | 'earth-divine-healing' | 'earth-duel' | 'earth-entombed' | 'earth-first-strike' | 'earth-forward' | 'earth-grain-sparrow' | 'earth-humble-village' | 'earth-hunters-lodge' | 'earth-immobile' | 'earth-malakhim' | 'earth-overpower' | 'earth-poisonous-dagger' | 'earth-rescue' | 'earth-shallow-grave' | 'earth-sinkhole' | 'earth-sword-and-shield' | 'earth-tunnel' | 'earth-ward' | 'fire' | 'fire-aramos' | 'fire-charge' | 'fire-genesis-life-loss' | 'fire-granary-rats' | 'fire-hamlet' | 'fire-ignited' | 'fire-lash' | 'fire-leap-attack' | 'fire-minor-explosion' | 'fire-reckless-squire' | 'fire-vikings' | 'fire-vile-imp' | 'movement-two' | StarterScenario | 'stealth' | 'water' | 'water-drown' | 'water-drowned' | 'water-edge-connection' | 'water-freeze' | 'water-gnarled-wendigo' | 'water-lugbog' | 'water-lure' | 'water-mesmerism' | 'water-pirate-ship' | 'water-river' | 'water-sideways' | 'water-stealth' | 'water-submerge' = 'combat',
): Readonly<{ manifest: GameManifest; names: ReadonlyMap<string, string> }> {
  const configuredAvatar = input.cards.find(({ stableId }) => stableId === input.config.avatar.stableId);
  if (!configuredAvatar || configuredAvatar.cardType !== 'avatar') {
    throw new Error('private scenario Avatar is missing');
  }
  const avatar = scenario === 'earth-starter'
    ? input.geomancer
    : scenario === 'air-starter' || scenario === 'air-zap'
      ? input.sparkmage
      : configuredAvatar;
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
    featuredArtifacts: readonly NormalizedCard[] = [],
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
    const featuredSpells = [...featuredMinions, ...featuredMagic, ...featuredArtifacts];
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
  const earthMalakhimDeck = elementalDeck(
    'earth',
    [input.malakhim, input.elthamTownsfolk],
    [input.ghostTownSite, input.valley],
  );
  const earthOverpowerDeck = elementalDeck(
    'earth',
    [...earthMinions, input.elthamTownsfolk],
    [],
    [input.overpower],
  );
  const earthBuryDeck = elementalDeck('earth', earthMinions, [], [input.bury]);
  const earthBorderMilitiaDeck = elementalDeck('earth', [], [], [input.borderMilitia]);
  const earthHumbleVillageDeck = elementalDeck('earth', [], [input.humbleVillage]);
  const earthDuelDeck = elementalDeck(
    'earth',
    [...earthMinions, input.elthamTownsfolk],
    [],
    [input.duel],
  );
  const earthSwordAndShieldDeck = elementalDeck(
    'earth',
    [...earthMinions, input.elthamTownsfolk],
    [input.ghostTownSite, input.spire],
    [input.zap],
    [input.swordAndShield],
  );
  const earthPoisonousDaggerDeck = elementalDeck(
    'earth',
    [...earthMinions, input.elthamTownsfolk],
    [input.ghostTownSite],
    [],
    [input.poisonousDagger],
  );
  const earthHuntersLodgeDeck = elementalDeck(
    'water',
    [input.slyFox],
    [input.huntersLodge],
  );
  const earthRescueDeck = elementalDeck('earth', earthMinions, [], [input.bury, input.rescue]);
  const earthDivineHealingDeck = elementalDeck('earth', earthMinions, [], [input.divineHealing]);
  const earthGrainSparrowDeck = elementalDeck(
    'earth',
    [input.grainSparrow, input.lesserBloodDemon],
    [input.ghostTownSite, input.steppe],
  );
  const earthShallowGraveDeck = elementalDeck('earth', earthMinions, [input.shallowGrave]);
  const earthSinkholeDeck = elementalDeck('earth', earthMinions, [input.sinkhole]);
  const airStarterDeck = elementalDeck(
    'air',
    [input.stealthTargetMinion],
    [input.spire],
    [input.zap],
  );
  const earthStarterDeck = elementalDeck('earth', [input.wildBoars], [input.humbleVillage]);
  const airBetaLessonDeck: GameDeckSpec = {
    atlas: [
      ...Array(3).fill(input.darkTower.stableId),
      ...Array(3).fill(input.gothicTower.stableId),
      ...Array(3).fill(input.loneTower.stableId),
    ],
    avatar: input.sparkmage.stableId,
    spellbook: [
      ...Array(2).fill(input.genesisSpellMinion.stableId),
      ...Array(2).fill(input.movementTwoMinion.stableId),
      ...Array(2).fill(input.airborneMinion.stableId),
      ...Array(2).fill(input.stealthTargetMinion.stableId),
      ...Array(2).fill(input.voidwalkMinion.stableId),
      ...Array(2).fill(input.midnightRogue.stableId),
      ...Array(2).fill(input.deadOfNightDemon.stableId),
      input.gyreHippogriffs.stableId,
      input.highlandClansmen.stableId,
      input.roamingMinion.stableId,
      input.teleport.stableId,
      ...Array(3).fill(input.lightningBolt.stableId),
    ],
  };
  const earthBetaLessonDeck: GameDeckSpec = {
    atlas: [
      ...Array(3).fill(input.humbleVillage.stableId),
      ...Array(3).fill(input.rusticVillage.stableId),
      ...Array(3).fill(input.simpleVillage.stableId),
      input.sinkhole.stableId,
    ],
    avatar: input.geomancer.stableId,
    spellbook: [
      ...Array(2).fill(input.wildBoars.stableId),
      ...Array(2).fill(input.genesisMinion.stableId),
      ...Array(2).fill(input.autumnUnicorn.stableId),
      ...Array(3).fill(input.rangedMinion.stableId),
      ...Array(3).fill(input.burrowingMinion.stableId),
      input.dalceanPhalanx.stableId,
      input.pudgeButcher.stableId,
      ...Array(2).fill(input.amazonWarriors.stableId),
      input.borderMilitia.stableId,
      input.divineHealing.stableId,
      ...Array(2).fill(input.overpower.stableId),
    ],
  };
  const fireStarterDeck = elementalDeck(
    'fire',
    [input.raalDromedary],
    [input.wasteland],
    [input.chargeMagic],
  );
  const fireGranaryRatsDeck = elementalDeck(
    'fire',
    [input.granaryRats],
    [input.wasteland],
  );
  const fireHamletDeck = elementalDeck(
    'fire',
    [input.raalDromedary],
    [input.hamlet, input.wasteland],
  );
  const waterStarterDeck = elementalDeck(
    'water',
    [input.seravaTownsfolk],
    [input.autumnRiver, input.stream],
  );
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
  const airBladderblimpDeck = elementalDeck(
    'air',
    [...airMinions, input.bladderblimp],
    [input.ghostTownSite],
    [input.lightningBolt],
  );
  const airLightningBoltDeck = elementalDeck('air', airMinions, [], [input.lightningBolt]);
  const airRainOfArrowsDeck = elementalDeck(
    'air',
    [input.shellycoat, input.stealthTargetMinion],
    [input.spire, input.stream],
    [input.rainOfArrows],
  );
  const airStaticServantDeck = elementalDeck(
    'air',
    [input.staticServant, input.stealthTargetMinion],
  );
  const airTeleportDeck = elementalDeck('air', airMinions, [], [input.teleport]);
  const airZapDeck = elementalDeck('air', airMinions, [], [input.zap]);
  const airFireFatalityDeck = elementalDeck(
    'air',
    [input.stealthTargetMinion, input.raalDromedary],
    [input.spire, input.wasteland],
    [input.zap, input.fatality],
  );
  const airGenesisSpellDeck = elementalDeck('air', [...airMinions, input.genesisSpellMinion]);
  const airLeylineDeck = elementalDeck('air', airMinions, [input.leylineHenge]);
  const airVoidwalkDeck = elementalDeck('air', [
    ...airMinions,
    input.voidwalkMinion,
    input.forsaken,
  ]);
  const airVoidArtifactDeck = elementalDeck(
    'air',
    [input.voidwalkMinion],
    [input.spire],
    [],
    [input.swordAndShield],
  );
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
  const fireVikingsDeck = elementalDeck(
    'fire',
    [input.vikings, input.firstStrikeTargetMinion],
    [input.ghostTownSite, input.valley],
    [],
    [input.poisonousDagger],
  );
  const fireAramosDeck = elementalDeck(
    'fire',
    [input.aramosMercenaries, input.raalDromedary],
  );
  const fireChargeDeck = elementalDeck(
    'fire',
    [input.raalDromedary],
    [],
    [input.chargeMagic],
  );
  const fireGenesisLifeLossDeck = elementalDeck('fire', [input.lesserBloodDemon]);
  const fireVileImpDeck = elementalDeck('fire', [input.vileImp], [input.wasteland]);
  const fireIgnitedDeck = elementalDeck('fire', [input.ignited]);
  const fireLashDeck = elementalDeck(
    'fire',
    [input.raalDromedary],
    [input.ghostTownSite],
    [input.lash],
  );
  const fireLeapAttackDeck = elementalDeck(
    'fire',
    [input.raalDromedary],
    [input.ghostTownSite],
    [input.leapAttack],
  );
  const fireRecklessSquireDeck = elementalDeck(
    'fire',
    [input.recklessSquire, input.raalDromedary],
    [input.ghostTownSite],
  );
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
  const waterMesmerismDeck = elementalDeck(
    'water',
    [input.seravaTownsfolk, input.deathriteMinion],
    [input.stream, input.valley],
    [input.mesmerism],
  );
  const waterPirateShipDeck = elementalDeck(
    'water',
    [input.pirateShip],
    [input.ghostTownSite],
  );
  const waterGnarledWendigoDeck = elementalDeck(
    'water',
    [input.gnarledWendigo, input.seravaTownsfolk],
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
    input.seaWitch,
  ], [], [input.freeze]);
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
    north: scenario === 'air-vs-earth-lesson'
      ? airBetaLessonDeck
      : scenario === 'earth-vs-air-lesson'
      ? earthBetaLessonDeck
      : scenario === 'air-starter'
      ? airStarterDeck
      : scenario === 'air-leyline'
      ? airLeylineDeck
      : scenario === 'air-arc-lightning'
      ? airArcLightningDeck
      : scenario === 'air-bladderblimp'
      ? airBladderblimpDeck
      : scenario === 'air-lightning-bolt'
      ? airLightningBoltDeck
      : scenario === 'air-rain-of-arrows'
      ? airRainOfArrowsDeck
      : scenario === 'air-static-servant'
      ? airStaticServantDeck
      : scenario === 'air-spellcaster-freeze'
      ? airSpellcasterFreezeDeck
      : scenario === 'air-teleport'
      ? airTeleportDeck
      : scenario === 'air-genesis-spell'
      ? airGenesisSpellDeck
      : scenario === 'air-voidwalk'
      ? airVoidwalkDeck
      : scenario === 'air-void-artifact'
      ? airVoidArtifactDeck
      : scenario === 'air-fire-fatality'
      ? airFireFatalityDeck
      : scenario === 'air-zap'
      ? airZapDeck
      : scenario === 'airborne' || scenario === 'movement-two' || scenario === 'stealth'
      ? airborneDeck
      : scenario === 'earth-entombed'
        ? earthEntombedDeck
      : scenario === 'earth-border-militia'
        ? earthBorderMilitiaDeck
      : scenario === 'earth-humble-village'
        ? earthHumbleVillageDeck
      : scenario === 'earth-bury'
        ? earthBuryDeck
      : scenario === 'earth-duel'
        ? earthDuelDeck
      : scenario === 'earth-sword-and-shield'
        ? earthSwordAndShieldDeck
      : scenario === 'earth-poisonous-dagger'
        ? earthPoisonousDaggerDeck
      : scenario === 'earth-hunters-lodge'
        ? earthHuntersLodgeDeck
      : scenario === 'earth-rescue'
        ? earthRescueDeck
      : scenario === 'earth-divine-healing'
        ? earthDivineHealingDeck
      : scenario === 'earth-grain-sparrow'
        ? earthGrainSparrowDeck
      : scenario === 'earth-shallow-grave'
        ? earthShallowGraveDeck
      : scenario === 'earth-sinkhole'
        ? earthSinkholeDeck
      : scenario === 'earth-starter'
        ? earthStarterDeck
      : scenario === 'earth-forward'
        ? earthForwardDeck
      : scenario === 'earth-immobile'
        ? earthImmobileDeck
      : scenario === 'earth-malakhim'
        ? earthMalakhimDeck
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
        : scenario === 'fire-starter'
          ? fireStarterDeck
        : scenario === 'fire-granary-rats'
          ? fireGranaryRatsDeck
        : scenario === 'fire-hamlet'
          ? fireHamletDeck
        : scenario === 'fire-aramos'
          ? fireAramosDeck
        : scenario === 'fire-charge'
          ? fireChargeDeck
        : scenario === 'fire-genesis-life-loss'
          ? fireGenesisLifeLossDeck
        : scenario === 'fire-vile-imp'
          ? fireVileImpDeck
        : scenario === 'fire-ignited'
          ? fireIgnitedDeck
        : scenario === 'fire-lash'
          ? fireLashDeck
        : scenario === 'fire-leap-attack'
          ? fireLeapAttackDeck
        : scenario === 'fire-minor-explosion'
          ? fireMinorExplosionDeck
        : scenario === 'fire-vikings'
          ? fireVikingsDeck
        : scenario === 'fire-reckless-squire'
          ? fireRecklessSquireDeck
        : scenario === 'water-edge-connection'
          ? waterEdgeConnectionDeck
        : scenario === 'water-drowned'
          ? waterDrownedDeck
        : scenario === 'water-drown'
          ? waterDrownDeck
        : scenario === 'water-freeze'
          ? waterFreezeDeck
        : scenario === 'water-gnarled-wendigo'
          ? waterGnarledWendigoDeck
        : scenario === 'water-lure'
          ? waterLureDeck
        : scenario === 'water-mesmerism'
          ? waterMesmerismDeck
        : scenario === 'water-pirate-ship'
          ? waterPirateShipDeck
        : scenario === 'water-river'
          ? waterStarterDeck
        : scenario === 'water-lugbog'
          ? waterLugbogDeck
        : scenario === 'water-submerge'
          ? waterSubmergeDeck
        : scenario === 'water-starter'
          ? waterStarterDeck
        : scenario === 'water' || scenario === 'water-sideways' || scenario === 'water-stealth'
          ? waterDeck
          : deck(false, true),
    south: scenario === 'air-vs-earth-lesson'
      ? earthBetaLessonDeck
      : scenario === 'earth-vs-air-lesson'
      ? airBetaLessonDeck
      : scenario === 'air-starter'
      ? airStarterDeck
      : scenario === 'air-leyline'
      ? airLeylineDeck
      : scenario === 'air-arc-lightning'
      ? airArcLightningDeck
      : scenario === 'air-bladderblimp'
      ? airBladderblimpDeck
      : scenario === 'air-lightning-bolt'
      ? airLightningBoltDeck
      : scenario === 'air-rain-of-arrows'
      ? airRainOfArrowsDeck
      : scenario === 'air-static-servant'
      ? airStaticServantDeck
      : scenario === 'air-spellcaster-freeze'
      ? airSpellcasterFreezeDeck
      : scenario === 'air-teleport'
      ? airTeleportDeck
      : scenario === 'air-genesis-spell'
      ? airGenesisSpellDeck
      : scenario === 'air-voidwalk'
      ? airVoidwalkDeck
      : scenario === 'air-void-artifact'
      ? airVoidArtifactDeck
      : scenario === 'air-fire-fatality'
      ? airFireFatalityDeck
      : scenario === 'air-zap'
      ? airZapDeck
      : scenario === 'airborne' || scenario === 'movement-two' || scenario === 'stealth'
      ? airborneDeck
      : scenario === 'water-starter'
        ? waterStarterDeck
      : scenario === 'water-edge-connection'
        ? waterEdgeConnectionDeck
      : scenario === 'water-drowned'
        ? waterDrownedDeck
      : scenario === 'water-drown'
        ? waterDrownDeck
      : scenario === 'water-freeze'
        ? waterFreezeDeck
      : scenario === 'water-gnarled-wendigo'
        ? waterGnarledWendigoDeck
      : scenario === 'water-lure'
        ? waterLureDeck
      : scenario === 'water-mesmerism'
        ? waterMesmerismDeck
      : scenario === 'water-pirate-ship'
        ? waterPirateShipDeck
      : scenario === 'water-river'
        ? waterStarterDeck
      : scenario === 'water-lugbog'
        ? waterLugbogDeck
      : scenario === 'earth-entombed'
        ? earthEntombedDeck
      : scenario === 'fire-starter'
        ? fireStarterDeck
      : scenario === 'fire-leap-attack'
        ? fireLeapAttackDeck
      : scenario === 'fire-reckless-squire'
        ? fireRecklessSquireDeck
      : scenario === 'fire-vikings'
        ? fireVikingsDeck
      : scenario === 'fire-vile-imp'
        ? fireVileImpDeck
      : scenario === 'earth-border-militia'
        ? earthBorderMilitiaDeck
      : scenario === 'earth-humble-village'
        ? earthHumbleVillageDeck
      : scenario === 'earth-bury'
        ? earthBuryDeck
      : scenario === 'earth-duel'
        ? earthDuelDeck
      : scenario === 'earth-sword-and-shield'
        ? earthSwordAndShieldDeck
      : scenario === 'earth-poisonous-dagger'
        ? earthPoisonousDaggerDeck
      : scenario === 'earth-hunters-lodge'
        ? earthHuntersLodgeDeck
      : scenario === 'earth-rescue'
        ? earthRescueDeck
      : scenario === 'earth-divine-healing'
        ? earthDivineHealingDeck
      : scenario === 'earth-grain-sparrow'
        ? earthGrainSparrowDeck
      : scenario === 'earth-shallow-grave'
        ? earthShallowGraveDeck
      : scenario === 'earth-sinkhole'
        ? earthSinkholeDeck
      : scenario === 'earth-starter'
        ? earthStarterDeck
      : scenario === 'earth-forward'
        ? earthForwardDeck
      : scenario === 'earth-immobile'
        ? earthImmobileDeck
      : scenario === 'earth-malakhim'
        ? earthMalakhimDeck
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
    decks.south.avatar,
    ...decks.north.atlas,
    ...decks.north.spellbook,
    ...decks.south.atlas,
    ...decks.south.spellbook,
    ...(scenario === 'earth-border-militia'
      || scenario === 'earth-humble-village'
      || scenario === 'earth-starter'
      || scenario === 'air-vs-earth-lesson'
      || scenario === 'earth-vs-air-lesson'
      ? [input.footSoldier.stableId]
      : []),
  ]);
  const selectedCards = input.cards.filter(({ stableId }) => referenced.has(stableId));
  const definitions = Object.fromEntries(selectedCards.map((card) => [
    card.stableId,
    gameDefinition(
      card,
      card.stableId === configuredAvatar.stableId && input.config.avatar.drawSpell,
      card.stableId === input.chargeMinion.stableId
        || card.stableId === input.monstrousLion.stableId
        || card.stableId === input.ignited.stableId
        || card.stableId === input.gyreHippogriffs.stableId
        || card.stableId === input.highlandClansmen.stableId,
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
      card.stableId === input.darkTower.stableId
        || card.stableId === input.gothicTower.stableId
        || card.stableId === input.loneTower.stableId,
      card.stableId === input.roamingMinion.stableId
        || card.stableId === input.lugbogCat.stableId,
      card.stableId === input.lumberingMinion.stableId,
      card.stableId === input.monstrousLion.stableId,
      card.stableId === input.rangedMinion.stableId
        || card.stableId === input.midnightRogue.stableId,
      card.stableId === input.firstStrikeMinion.stableId,
      card.stableId === input.wardMinion.stableId
        || card.stableId === input.malakhim.stableId,
      card.stableId === input.airborneMinion.stableId
        || card.stableId === input.gyreHippogriffs.stableId
        || card.stableId === input.movementTwoMinion.stableId
        || card.stableId === input.grainSparrow.stableId
        || card.stableId === input.bladderblimp.stableId
        || card.stableId === input.malakhim.stableId,
      card.stableId === input.stealthMinion.stableId
        || card.stableId === input.midnightRogue.stableId
        || card.stableId === input.deadOfNightDemon.stableId,
      card.stableId === input.slyFox.stableId,
      card.stableId === input.sedgeCrabs.stableId,
      card.stableId === input.submergeMinion.stableId
        || card.stableId === input.drowned.stableId
        || card.stableId === input.shellycoat.stableId
        || card.stableId === input.seaWitch.stableId,
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
        : card.stableId === input.zap.stableId
          || card.stableId === input.lash.stableId ? 1 : 0,
      card.stableId === input.arcLightning.stableId
        || card.stableId === input.lash.stableId,
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
      card.stableId === input.genesisSpellMinion.stableId
        || card.stableId === input.seaWitch.stableId,
      card.stableId === input.grainSparrow.stableId ? 2 : 0,
      card.stableId === input.lash.stableId,
      card.stableId === input.aramosMercenaries.stableId,
      card.stableId === input.duel.stableId,
      card.stableId === input.bladderblimp.stableId,
      card.stableId === input.leapAttack.stableId,
      card.stableId === input.staticServant.stableId ? 1 : 0,
      card.stableId === input.vileImp.stableId ? 2 : 0,
      card.stableId === input.gnarledWendigo.stableId ? 2 : 0,
      card.stableId === input.recklessSquire.stableId ? 1 : 0,
      card.stableId === input.swordAndShield.stableId ? 2 : 0,
      card.stableId === input.poisonousDagger.stableId,
      card.stableId === input.huntersLodge.stableId,
      card.stableId === input.vikings.stableId,
      card.stableId === input.mesmerism.stableId,
      card.stableId === input.malakhim.stableId,
      card.stableId === input.fatality.stableId,
      card.stableId === input.hamlet.stableId ? 1 : 0,
      card.stableId === input.granaryRats.stableId,
      card.stableId === input.shellycoat.stableId ? 1 : 0,
      card.stableId === input.humbleVillage.stableId
        || card.stableId === input.rusticVillage.stableId
        || card.stableId === input.simpleVillage.stableId
        ? input.footSoldier.stableId
        : undefined,
      card.stableId === input.borderMilitia.stableId ? input.footSoldier.stableId : undefined,
      card.stableId === input.footSoldier.stableId,
      card.stableId === input.autumnRiver.stableId,
      card.stableId === input.geomancer.stableId,
      card.stableId === input.geomancer.stableId,
      card.stableId === input.sparkmage.stableId,
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

function findEarthMalakhimOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  malakhimInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  siteInstanceIds: readonly [string, string, string, string, string];
  southSiteInstanceId: string;
  session: GameSession;
}> {
  // ponytail: bounded opening scan avoids another private seed field.
  for (let offset = 1; offset <= 8192; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-malakhim');
    const session = createGameSession(built.manifest);
    const accessibleSites = [
      ...session.state.players.north.hand.atlas,
      ...session.state.players.north.atlas.slice(0, 2),
    ];
    const playedSites = accessibleSites.slice(0, 5);
    const earthSites = playedSites.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
    const ghostTowns = playedSites.filter(({ cardId }) =>
      cardId === input.ghostTownSite.stableId);
    const malakhimInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ].find(({ cardId }) => cardId === input.malakhim.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (earthSites.length >= 3
      && ghostTowns.length >= 2
      && playedSites[4]?.cardId === input.ghostTownSite.stableId
      && malakhimInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        malakhimInstanceId,
        siteInstanceIds: [
          playedSites[0]!.instanceId,
          playedSites[1]!.instanceId,
          playedSites[2]!.instanceId,
          playedSites[3]!.instanceId,
          playedSites[4]!.instanceId,
        ],
        southSiteInstanceId,
        session,
      };
    }
  }
  throw new Error('private Malakhim scenario lacks its supported opening');
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

function findEarthDuelMagicOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  boskTrollInstanceId: string;
  duelInstanceId: string;
  elthamTownsfolkInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded opening scan avoids another private seed/config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-duel');
    const session = createGameSession(built.manifest);
    const northEarthSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
    const southEarthSites = session.state.players.south.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
    const northEarlySpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const northLaterSpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ];
    const southSpells = [
      ...session.state.players.south.hand.spellbook,
      ...session.state.players.south.spellbook.slice(0, 1),
    ];
    const boskTrollInstanceId = northEarlySpells
      .find(({ cardId }) => cardId === input.firstStrikeTargetMinion.stableId)?.instanceId;
    const duelInstanceId = northLaterSpells
      .find(({ cardId }) => cardId === input.duel.stableId)?.instanceId;
    const elthamTownsfolkInstanceId = southSpells
      .find(({ cardId }) => cardId === input.elthamTownsfolk.stableId)?.instanceId;
    if (northEarthSites.length >= 3
      && southEarthSites.length >= 2
      && boskTrollInstanceId
      && duelInstanceId
      && elthamTownsfolkInstanceId) {
      return {
        ...built,
        boskTrollInstanceId,
        duelInstanceId,
        elthamTownsfolkInstanceId,
        northSiteInstanceIds: [
          northEarthSites[0]!.instanceId,
          northEarthSites[1]!.instanceId,
          northEarthSites[2]!.instanceId,
        ],
        session,
        southSiteInstanceIds: [
          southEarthSites[0]!.instanceId,
          southEarthSites[1]!.instanceId,
        ],
      };
    }
  }
  throw new Error('private Duel Magic scenario no longer produces its supported opening');
}

function findEarthBorderMilitiaOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  borderMilitiaInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string, string];
}> {
  const seed = 7688;
  const built = buildManifest(input, seed, 'earth-border-militia');
  const session = createGameSession(built.manifest);
  const earthSites = (seat: GameSeat) => session.state.players[seat].hand.atlas
    .filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
  const northSites = earthSites('north');
  const southSites = earthSites('south');
  const borderMilitiaInstanceId = [
    ...session.state.players.north.hand.spellbook,
    ...session.state.players.north.spellbook.slice(0, 3),
  ].find(({ cardId }) => cardId === input.borderMilitia.stableId)?.instanceId;
  if (northSites.length < 3 || southSites.length < 3 || !borderMilitiaInstanceId) {
    throw new Error('private Border Militia seed no longer produces its supported opening');
  }
  return {
    ...built,
    borderMilitiaInstanceId,
    northSiteInstanceIds: [
      northSites[0]!.instanceId,
      northSites[1]!.instanceId,
      northSites[2]!.instanceId,
    ],
    seed,
    session,
    southSiteInstanceIds: [
      southSites[0]!.instanceId,
      southSites[1]!.instanceId,
      southSites[2]!.instanceId,
    ],
  };
}

function findEarthHumbleVillageOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  humbleVillageInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  seed: number;
  session: GameSession;
}> {
  // ponytail: pinned seed keeps this private proof fast without another config field.
  const seed = 7383;
  const built = buildManifest(input, seed, 'earth-humble-village');
  const session = createGameSession(built.manifest);
  const humbleVillageInstanceId = session.state.players.north.hand.atlas
    .find(({ cardId }) => cardId === input.humbleVillage.stableId)?.instanceId;
  if (!humbleVillageInstanceId) {
    throw new Error('private Humble Village seed no longer produces its supported opening');
  }
  return { ...built, humbleVillageInstanceId, seed, session };
}

function findEarthArtifactOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  artifact: NormalizedCard,
  scenario: 'earth-poisonous-dagger' | 'earth-sword-and-shield',
): Readonly<{
  artifactInstanceId: string;
  boskTrollInstanceId: string;
  elthamTownsfolkInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  session: GameSession;
  southEarthSiteInstanceIds: readonly [string, string];
  zapDrawCount?: number;
  zapInstanceIds?: readonly [string, string];
}> {
  // ponytail: bounded opening scan avoids another private seed/config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, scenario);
    const session = createGameSession(built.manifest);
    const earthSites = (seat: GameSeat) => session.state.players[seat].hand.atlas.filter(({ cardId }) => {
      if (cardId === input.ghostTownSite.stableId) return false;
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('earth');
    });
    const northEarthSites = earthSites('north');
    const southEarthSites = earthSites('south');
    const northSpire = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.spire.stableId);
    const northEarlySpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const northLaterSpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ];
    const northDropSpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 12),
    ];
    const southEarlySpells = [
      ...session.state.players.south.hand.spellbook,
      ...session.state.players.south.spellbook.slice(0, 1),
    ];
    const elthamTownsfolkInstanceId = northEarlySpells
      .find(({ cardId }) => cardId === input.elthamTownsfolk.stableId)?.instanceId;
    const artifactInstanceId = northLaterSpells
      .find(({ cardId }) => cardId === artifact.stableId)?.instanceId;
    const boskTrollInstanceId = southEarlySpells
      .find(({ cardId }) => cardId === input.firstStrikeTargetMinion.stableId)?.instanceId;
    const northSiteInstanceIds: readonly [string, string, string] | undefined =
      scenario === 'earth-sword-and-shield'
        ? northEarthSites[0] && northEarthSites[1] && northSpire
          ? [northEarthSites[0].instanceId, northEarthSites[1].instanceId, northSpire.instanceId]
          : undefined
        : northEarthSites[0] && northEarthSites[1] && northEarthSites[2]
          ? [northEarthSites[0].instanceId, northEarthSites[1].instanceId,
            northEarthSites[2].instanceId]
          : undefined;
    const zaps = northDropSpells.filter(({ cardId }) => cardId === input.zap.stableId);
    const zapInstanceIds: readonly [string, string] | undefined = zaps[0] && zaps[1]
      ? [zaps[0].instanceId, zaps[1].instanceId]
      : undefined;
    const zapDrawCount = zapInstanceIds
      ? Math.max(...zapInstanceIds.map((instanceId) => {
        const index = session.state.players.north.spellbook
          .findIndex((card) => card.instanceId === instanceId);
        return index < 0 ? 0 : Math.max(0, index - 1);
      }))
      : undefined;
    if (northSiteInstanceIds
      && southEarthSites.length >= 2
      && elthamTownsfolkInstanceId
      && artifactInstanceId
      && boskTrollInstanceId
      && (scenario !== 'earth-sword-and-shield' || zapInstanceIds)) {
      return {
        ...built,
        artifactInstanceId,
        boskTrollInstanceId,
        elthamTownsfolkInstanceId,
        northSiteInstanceIds,
        session,
        southEarthSiteInstanceIds: [
          southEarthSites[0]!.instanceId,
          southEarthSites[1]!.instanceId,
        ],
        ...(zapInstanceIds && zapDrawCount !== undefined
          ? { zapDrawCount, zapInstanceIds }
          : {}),
      };
    }
  }
  throw new Error('private carried Artifact scenario no longer produces its supported opening');
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

function findEarthGrainSparrowOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  demonInstanceId: string;
  ghostTownInstanceId: string;
  grainSparrowInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  session: GameSession;
  southSiteInstanceId: string;
  steppeInstanceId: string;
}> {
  // ponytail: a bounded deterministic scan is acceptable for private verification; lock a seed only if material.
  for (let offset = 1; offset <= 4_096; offset += 1) {
    const built = buildManifest(input, input.config.earthSeed + offset, 'earth-grain-sparrow');
    const session = createGameSession(built.manifest);
    const northAtlasHand = session.state.players.north.hand.atlas;
    const steppeInstanceId = northAtlasHand
      .find(({ cardId }) => cardId === input.steppe.stableId)?.instanceId;
    const ghostTownInstanceId = northAtlasHand
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const demonInstanceId = availableMinionInstance(
      session,
      'north',
      input.lesserBloodDemon.stableId,
      1,
    );
    const grainSparrowInstanceId = availableMinionInstance(
      session,
      'north',
      input.grainSparrow.stableId,
      1,
    );
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (demonInstanceId
      && ghostTownInstanceId
      && grainSparrowInstanceId
      && southSiteInstanceId
      && steppeInstanceId) {
      return {
        ...built,
        demonInstanceId,
        ghostTownInstanceId,
        grainSparrowInstanceId,
        session,
        southSiteInstanceId,
        steppeInstanceId,
      };
    }
  }
  throw new Error('private Grain Sparrow Genesis-healing scenario no longer produces its supported opening');
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

function findStarterOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  scenario: StarterScenario | 'fire-granary-rats',
  baseSeed: number,
  site: NormalizedCard,
  minion: NormalizedCard,
  featuredSpell?: NormalizedCard,
): Readonly<{
  manifest: GameManifest;
  minionInstanceId: string;
  names: ReadonlyMap<string, string>;
  session: GameSession;
  siteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, baseSeed + offset, scenario);
    const session = createGameSession(built.manifest);
    const siteInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === site.stableId)?.instanceId;
    const minionInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === minion.stableId)?.instanceId;
    const featuredSpellInstanceId = featuredSpell
      ? session.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === featuredSpell.stableId)?.instanceId
      : undefined;
    const hasSecondFireSite = scenario !== 'fire-starter'
      || session.state.players.north.hand.atlas.some(({ cardId, instanceId }) => {
        const definition = session.state.cards[cardId];
        return instanceId !== siteInstanceId
          && definition?.cardType === 'site'
          && definition.elements.includes('fire');
      });
    if (siteInstanceId
      && minionInstanceId
      && (!featuredSpell || featuredSpellInstanceId)
      && hasSecondFireSite) {
      return { ...built, minionInstanceId, session, siteInstanceId };
    }
  }
  throw new Error(`private ${scenario} scenario no longer produces its supported opening`);
}

export async function loadPrivateStarterCatalog(
  path = DEFAULT_SCENARIO,
): Promise<readonly PrivateStarterPreset[]> {
  const input = await readPrivateInputs(path);
  const lessons = [
    [
      'air-vs-earth-lesson',
      'Air Beta vs Earth Beta — supported cards from one boxed precon each',
      input.config.airSeed,
    ],
    [
      'earth-vs-air-lesson',
      'Earth Beta vs Air Beta — supported cards from one boxed precon each',
      input.config.earthSeed,
    ],
  ] as const;
  const starters = [
    ['air-starter', 'Air Beta precon card lesson — Sparkmage + Snow Leopard', input.config.airSeed, input.spire, input.stealthTargetMinion, input.zap],
    ['earth-starter', 'Earth Beta precon opening — Geomancer + Humble Village + Wild Boars', input.config.earthSeed, input.humbleVillage, input.wildBoars],
    ['fire-starter', 'Fire — Wasteland + Raal Dromedary + Charge', input.config.fireSeed, input.wasteland, input.raalDromedary, input.chargeMagic],
    ['water-starter', 'Water — Autumn River + Serava Townsfolk', input.config.waterSeed, input.autumnRiver, input.seravaTownsfolk],
  ] as const;
  const cardsById = new Map(input.cards.map((card) => [card.stableId, card]));
  const preset = (
    id: PrivateStarterPreset['id'],
    label: string,
    built: ReturnType<typeof buildManifest>,
  ): PrivateStarterPreset => {
    const deckCardIds = [
      ...built.manifest.decks.north.atlas,
      ...built.manifest.decks.north.spellbook,
      ...built.manifest.decks.south.atlas,
      ...built.manifest.decks.south.spellbook,
    ];
    const usesOnlyOrdinaryOrExceptionalCards = deckCardIds.every((cardId) => {
      const rarity = cardsById.get(cardId)?.rarity;
      return rarity === 'ordinary' || rarity === 'exceptional';
    });
    if (!usesOnlyOrdinaryOrExceptionalCards && !id.endsWith('-lesson')) {
      throw new Error('private ' + id + ' teaching deck no longer uses only entry-level rarities');
    }
    return Object.freeze({
      cardNames: Object.freeze(Object.fromEntries(built.names)),
      id,
      label,
      manifest: built.manifest,
      usesOnlyOrdinaryOrExceptionalCards,
    });
  };
  return Object.freeze([
    ...lessons.map(([id, label, seed]) =>
      preset(id, label, buildManifest(input, seed, id))),
    ...starters.map(([id, label, seed, site, minion, featuredSpell]) =>
      preset(id, label, findStarterOpening(input, id, seed, site, minion, featuredSpell))),
  ]);
}

function findFireHamletOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  hamletInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  raalInstanceId: string;
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
  wastelandInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another strict private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.fireSeed + offset;
    const built = buildManifest(input, seed, 'fire-hamlet');
    const session = createGameSession(built.manifest);
    const hamletInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.hamlet.stableId)?.instanceId;
    const wastelandInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.wasteland.stableId)?.instanceId;
    const raalInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.raalDromedary.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (hamletInstanceId && wastelandInstanceId && raalInstanceId && southSiteInstanceId) {
      return {
        ...built,
        hamletInstanceId,
        raalInstanceId,
        seed,
        session,
        southSiteInstanceId,
        wastelandInstanceId,
      };
    }
  }
  throw new Error('private Hamlet scenario no longer produces its supported opening');
}

function findEarthSinkholeOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  recoverySiteInstanceId: string;
  seed: number;
  session: GameSession;
  sinkholeInstanceId: string;
  southSiteInstanceId: string;
  targetSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const seed = input.config.earthSeed + offset;
    const built = buildManifest(input, seed, 'earth-sinkhole');
    const session = createGameSession(built.manifest);
    const sinkholeInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.sinkhole.stableId)?.instanceId;
    const targetSiteInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.valley.stableId)?.instanceId;
    const recoverySiteInstanceId = session.state.players.north.atlas[0]?.cardId
      === input.valley.stableId
      ? session.state.players.north.atlas[0].instanceId
      : undefined;
    const southSiteInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId !== input.sinkhole.stableId)?.instanceId;
    if (sinkholeInstanceId
      && targetSiteInstanceId
      && recoverySiteInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        recoverySiteInstanceId,
        seed,
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

function findAirFireFatalityOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  fatalityInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string];
  session: GameSession;
  snowLeopardInstanceId: string;
  southSpireInstanceId: string;
  zapInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private seed field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-fire-fatality');
    const session = createGameSession(built.manifest);
    const northSites = [
      ...session.state.players.north.hand.atlas,
      ...session.state.players.north.atlas.slice(0, 1),
    ].slice(0, 4);
    const northHasSpire = northSites.some(({ cardId }) => cardId === input.spire.stableId);
    const northHasWasteland = northSites.some(({ cardId }) => cardId === input.wasteland.stableId);
    const southSpireInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId === input.spire.stableId)?.instanceId;
    const fatalityInstanceId = availableMinionInstance(
      session,
      'north',
      input.fatality.stableId,
      2,
    );
    const zapInstanceId = availableMinionInstance(session, 'north', input.zap.stableId, 2);
    const snowLeopardInstanceId = availableMinionInstance(
      session,
      'south',
      input.stealthTargetMinion.stableId,
      1,
    );
    if (northSites.length === 4
      && northHasSpire
      && northHasWasteland
      && southSpireInstanceId
      && fatalityInstanceId
      && zapInstanceId
      && snowLeopardInstanceId) {
      return {
        ...built,
        fatalityInstanceId,
        northSiteInstanceIds: [
          northSites[0]!.instanceId,
          northSites[1]!.instanceId,
          northSites[2]!.instanceId,
          northSites[3]!.instanceId,
        ],
        session,
        snowLeopardInstanceId,
        southSpireInstanceId,
        zapInstanceId,
      };
    }
  }
  throw new Error('private Fatality scenario lacks its supported mixed opening');
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

function findAirBladderblimpOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  bladderblimpInstanceId: string;
  ghostTownSiteInstanceId: string;
  lightningBoltInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northAirSiteInstanceIds: readonly [string, string, string];
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: a bounded opening scan avoids another private seed field for one deterministic proof.
  for (let offset = 1; offset <= 2048; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-bladderblimp');
    const session = createGameSession(built.manifest);
    const northHandAirSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return cardId !== input.ghostTownSite.stableId
        && definition?.cardType === 'site'
        && definition.elements.includes('air');
    });
    const northDrawnAirSite = session.state.players.north.atlas[0];
    const northDrawnAirSiteDefinition = northDrawnAirSite
      ? session.state.cards[northDrawnAirSite.cardId]
      : undefined;
    const ghostTownSiteInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const southSites = session.state.players.south.hand.atlas;
    const bladderblimpInstanceId = availableMinionInstance(
      session,
      'north',
      input.bladderblimp.stableId,
      2,
    );
    const lightningBoltInstanceId = availableMinionInstance(
      session,
      'north',
      input.lightningBolt.stableId,
      3,
    );
    if (northHandAirSites.length >= 2
      && northDrawnAirSite
      && northDrawnAirSite.cardId !== input.ghostTownSite.stableId
      && northDrawnAirSiteDefinition?.cardType === 'site'
      && northDrawnAirSiteDefinition.elements.includes('air')
      && ghostTownSiteInstanceId
      && southSites.length >= 2
      && bladderblimpInstanceId
      && lightningBoltInstanceId) {
      return {
        ...built,
        bladderblimpInstanceId,
        ghostTownSiteInstanceId,
        lightningBoltInstanceId,
        northAirSiteInstanceIds: [
          northHandAirSites[0]!.instanceId,
          northHandAirSites[1]!.instanceId,
          northDrawnAirSite.instanceId,
        ],
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private Bladderblimp nearby-site Deathrite scenario lacks its supported opening');
}

function findAirRainOfArrowsOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSpireInstanceId: string;
  northStreamInstanceId: string;
  rainOfArrowsInstanceId: string;
  seed: number;
  session: GameSession;
  shellycoatInstanceId: string;
  southSnowLeopardInstanceId: string;
  southSpireInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private scenario config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const seed = input.config.airSeed + offset;
    const built = buildManifest(input, seed, 'air-rain-of-arrows');
    const session = createGameSession(built.manifest);
    const northSpireInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.spire.stableId)?.instanceId;
    const northStreamInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.stream.stableId)?.instanceId;
    const southSpireInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId === input.spire.stableId)?.instanceId;
    const shellycoatInstanceId = availableMinionInstance(
      session,
      'north',
      input.shellycoat.stableId,
      1,
    );
    const rainOfArrowsInstanceId = availableMinionInstance(
      session,
      'north',
      input.rainOfArrows.stableId,
      2,
    );
    const southSnowLeopardInstanceId = availableMinionInstance(
      session,
      'south',
      input.stealthTargetMinion.stableId,
      1,
    );
    if (northSpireInstanceId
      && northStreamInstanceId
      && southSpireInstanceId
      && shellycoatInstanceId
      && rainOfArrowsInstanceId
      && southSnowLeopardInstanceId) {
      return {
        ...built,
        northSpireInstanceId,
        northStreamInstanceId,
        rainOfArrowsInstanceId,
        seed,
        session,
        shellycoatInstanceId,
        southSnowLeopardInstanceId,
        southSpireInstanceId,
      };
    }
  }
  throw new Error('private Rain of Arrows damage-reduction scenario lacks its supported opening');
}

function findAirStaticServantOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  snowLeopardInstanceId: string;
  southSiteInstanceId: string;
  staticServantInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.airSeed + offset, 'air-static-servant');
    const session = createGameSession(built.manifest);
    const northSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('air');
    });
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    const snowLeopardInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.stealthTargetMinion.stableId)?.instanceId;
    const staticServantInstanceId = availableMinionInstance(
      session,
      'north',
      input.staticServant.stableId,
      1,
    );
    if (northSites.length >= 2
      && snowLeopardInstanceId
      && southSiteInstanceId
      && staticServantInstanceId) {
      return {
        ...built,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        session,
        snowLeopardInstanceId,
        southSiteInstanceId,
        staticServantInstanceId,
      };
    }
  }
  throw new Error('private location-wide Genesis damage scenario lacks its supported opening');
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

function findFireVileImpOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
  vileImpInstanceId: string;
}> {
  // ponytail: pinned seed keeps this private teaching proof fast without another config field.
  const seed = 141;
  const built = buildManifest(input, seed, 'fire-vile-imp');
  const session = createGameSession(built.manifest);
  const northSites = session.state.players.north.hand.atlas
    .filter(({ cardId }) => cardId === input.wasteland.stableId);
  const vileImpInstanceId = [
    ...session.state.players.north.hand.spellbook,
    ...session.state.players.north.spellbook.slice(0, 1),
  ].find(({ cardId }) => cardId === input.vileImp.stableId)?.instanceId;
  const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
  if (northSites.length >= 2 && southSiteInstanceId && vileImpInstanceId) {
    return {
      ...built,
      northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
      seed,
      session,
      southSiteInstanceId,
      vileImpInstanceId,
    };
  }
  throw new Error('private Vile Imp optional Genesis damage scenario lacks its supported opening');
}

function findFireAramosOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  aramosInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded opening scan avoids another private seed/config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-aramos');
    const session = createGameSession(built.manifest);
    const northFireSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const accessibleSpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ];
    const aramosInstanceId = accessibleSpells
      .find(({ cardId }) => cardId === input.aramosMercenaries.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northFireSites.length >= 2
      && aramosInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        aramosInstanceId,
        northSiteInstanceIds: [northFireSites[0]!.instanceId, northFireSites[1]!.instanceId],
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Aramos random-discard alternative-cost scenario lacks its supported opening');
}

function findFireLashOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  fireSiteInstanceId: string;
  ghostTownInstanceId: string;
  lashInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  raalInstanceId: string;
  session: GameSession;
  southSiteInstanceId: string;
}> {
  // ponytail: bounded seed scan avoids another private config field.
  for (let offset = 1; offset <= 256; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-lash');
    const session = createGameSession(built.manifest);
    const fireSiteInstanceId = session.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    })?.instanceId;
    const ghostTownInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const raalInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.raalDromedary.stableId)?.instanceId;
    const lashInstanceId = availableMinionInstance(
      session,
      'north',
      input.lash.stableId,
      1,
    );
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (fireSiteInstanceId
      && ghostTownInstanceId
      && lashInstanceId
      && raalInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        fireSiteInstanceId,
        ghostTownInstanceId,
        lashInstanceId,
        raalInstanceId,
        session,
        southSiteInstanceId,
      };
    }
  }
  throw new Error('private Lash damage-and-untap scenario no longer produces its supported opening');
}

function findFireLeapAttackOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  ghostTownInstanceId: string;
  leapAttackInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northRaalInstanceId: string;
  northSiteInstanceIds: readonly [string, string];
  session: GameSession;
  southRaalInstanceIds: readonly [string, string];
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: this bounded deterministic scan avoids another private seed field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-leap-attack');
    const session = createGameSession(built.manifest);
    const fireSites = (seat: GameSeat) => session.state.players[seat].hand.atlas
      .filter(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return definition?.cardType === 'site' && definition.elements.includes('fire');
      });
    const northSites = fireSites('north');
    const southSites = fireSites('south');
    const ghostTownInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const northRaalInstanceId = availableMinionInstance(
      session,
      'north',
      input.raalDromedary.stableId,
      1,
    );
    const leapAttackInstanceId = availableMinionInstance(
      session,
      'north',
      input.leapAttack.stableId,
      2,
    );
    const southRaalInstanceIds = [
      ...session.state.players.south.hand.spellbook,
      ...session.state.players.south.spellbook.slice(0, 1),
    ].filter(({ cardId }) => cardId === input.raalDromedary.stableId)
      .map(({ instanceId }) => instanceId);
    if (northSites.length >= 2
      && southSites.length >= 2
      && ghostTownInstanceId
      && northRaalInstanceId
      && leapAttackInstanceId
      && southRaalInstanceIds.length >= 2) {
      return {
        ...built,
        ghostTownInstanceId,
        leapAttackInstanceId,
        northRaalInstanceId,
        northSiteInstanceIds: [northSites[0]!.instanceId, northSites[1]!.instanceId],
        session,
        southRaalInstanceIds: [southRaalInstanceIds[0]!, southRaalInstanceIds[1]!],
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private Leap Attack optional-step strike-all scenario lacks its supported opening');
}

function findFireRecklessSquireOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  ghostTownInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northFireSiteInstanceId: string;
  recklessSquireInstanceId: string;
  session: GameSession;
  southFireSiteInstanceIds: readonly [string, string];
  southRaalInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded deterministic scan avoids another private seed field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-reckless-squire');
    const session = createGameSession(built.manifest);
    const northFireSiteInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => {
        const definition = session.state.cards[cardId];
        return cardId !== input.ghostTownSite.stableId
          && definition?.cardType === 'site'
          && definition.elements.includes('fire');
      })?.instanceId;
    const ghostTownInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const recklessSquireInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 1),
    ].find(({ cardId }) => cardId === input.recklessSquire.stableId)?.instanceId;
    const southFireSites = session.state.players.south.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const southRaalInstanceIds = [
      ...session.state.players.south.hand.spellbook,
      ...session.state.players.south.spellbook.slice(0, 1),
    ].filter(({ cardId }) => cardId === input.raalDromedary.stableId)
      .map(({ instanceId }) => instanceId);
    if (northFireSiteInstanceId
      && ghostTownInstanceId
      && recklessSquireInstanceId
      && southFireSites.length >= 2
      && southRaalInstanceIds.length >= 2) {
      return {
        ...built,
        ghostTownInstanceId,
        northFireSiteInstanceId,
        recklessSquireInstanceId,
        session,
        southFireSiteInstanceIds: [
          southFireSites[0]!.instanceId,
          southFireSites[1]!.instanceId,
        ],
        southRaalInstanceIds: [southRaalInstanceIds[0]!, southRaalInstanceIds[1]!],
      };
    }
  }
  throw new Error('private Reckless Squire Lance scenario lacks its supported opening');
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

function findFireVikingsOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  daggerInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string];
  session: GameSession;
  southBoskTrollInstanceId: string;
  southSiteInstanceIds: readonly [string, string];
  vikingsInstanceId: string;
}> {
  // ponytail: bounded reuse of the Fire seed avoids another private config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.fireSeed + offset, 'fire-vikings');
    const session = createGameSession(built.manifest);
    const northHandSites = session.state.players.north.hand.atlas;
    const northFireSites = northHandSites.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const ghostTownInstanceId = northHandSites
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const thirdNorthSiteInstanceId = session.state.players.north.atlas[0]?.instanceId;
    const southFireSite = session.state.players.south.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('fire');
    });
    const southValleyInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId === input.valley.stableId)?.instanceId;
    const northAccessibleSpells = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ];
    const southAccessibleSpells = [
      ...session.state.players.south.hand.spellbook,
      ...session.state.players.south.spellbook.slice(0, 2),
    ];
    const vikingsInstanceId = northAccessibleSpells
      .find(({ cardId }) => cardId === input.vikings.stableId)?.instanceId;
    const daggerInstanceId = northAccessibleSpells
      .find(({ cardId }) => cardId === input.poisonousDagger.stableId)?.instanceId;
    const southBoskTrollInstanceId = southAccessibleSpells
      .find(({ cardId }) => cardId === input.firstStrikeTargetMinion.stableId)?.instanceId;
    if (northFireSites.length >= 2
      && ghostTownInstanceId
      && thirdNorthSiteInstanceId
      && southFireSite
      && southValleyInstanceId
      && vikingsInstanceId
      && daggerInstanceId
      && southBoskTrollInstanceId) {
      return {
        ...built,
        daggerInstanceId,
        northSiteInstanceIds: [
          northFireSites[0]!.instanceId,
          northFireSites[1]!.instanceId,
          thirdNorthSiteInstanceId,
          ghostTownInstanceId,
        ],
        session,
        southBoskTrollInstanceId,
        southSiteInstanceIds: [
          southFireSite.instanceId,
          southValleyInstanceId,
        ],
        vikingsInstanceId,
      };
    }
  }
  throw new Error('private Vikings area-damage scenario lacks its supported opening');
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

function findAirVoidArtifactOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  artifactInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string];
  seed: number;
  session: GameSession;
  southSiteInstanceId: string;
  stalkerInstanceId: string;
}> {
  // ponytail: this bounded scan avoids adding a private config field for one teaching proof.
  for (let offset = 1; offset <= 256; offset += 1) {
    const seed = input.config.airborneSeed + offset;
    const built = buildManifest(input, seed, 'air-void-artifact');
    const session = createGameSession(built.manifest);
    const northSites = [
      ...session.state.players.north.hand.atlas,
      ...session.state.players.north.atlas.slice(0, 1),
    ];
    const stalkerInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.voidwalkMinion.stableId)?.instanceId;
    const artifactInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.swordAndShield.stableId)?.instanceId;
    const southSiteInstanceId = session.state.players.south.hand.atlas[0]?.instanceId;
    if (northSites.length >= 4
      && stalkerInstanceId
      && artifactInstanceId
      && southSiteInstanceId) {
      return {
        ...built,
        artifactInstanceId,
        northSiteInstanceIds: northSites.slice(0, 4)
          .map(({ instanceId }) => instanceId) as [string, string, string, string],
        seed,
        session,
        southSiteInstanceId,
        stalkerInstanceId,
      };
    }
  }
  throw new Error('private void Artifact relocation scenario lacks its supported opening');
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

function findHuntersLodgeOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  hunterLodgeInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northWaterSiteInstanceId: string;
  session: GameSession;
  slyFoxInstanceId: string;
}> {
  // ponytail: bounded reuse of the Sly Fox seed avoids another private config field.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(input, input.config.slyFoxSeed + offset, 'earth-hunters-lodge');
    const session = createGameSession(built.manifest);
    const northWaterSiteInstanceId = session.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    })?.instanceId;
    const slyFoxInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.slyFox.stableId)?.instanceId;
    const hunterLodgeInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId === input.huntersLodge.stableId)?.instanceId;
    if (northWaterSiteInstanceId && slyFoxInstanceId && hunterLodgeInstanceId) {
      return {
        ...built,
        hunterLodgeInstanceId,
        northWaterSiteInstanceId,
        session,
        slyFoxInstanceId,
      };
    }
  }
  throw new Error("private Hunter's Lodge scenario no longer produces its supported opening");
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

function findWaterSubmergeFreezeOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  comparisonInstanceId: string;
  featuredInstanceId: string;
  freezeInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string];
  seaWitchInstanceId: string;
  seed: number;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  for (let offset = 1; offset <= 512; offset += 1) {
    const seed = input.config.sedgeCrabsSeed + offset;
    const built = buildManifest(input, seed, 'water-submerge');
    const session = createGameSession(built.manifest);
    const north = session.state.players.north;
    const northWaterSites = north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const thirdNorthSite = north.hand.atlas.find(({ instanceId }) =>
      instanceId !== northWaterSites[0]?.instanceId
        && instanceId !== northWaterSites[1]?.instanceId);
    const earlySpells = [...north.hand.spellbook, ...north.spellbook.slice(0, 2)];
    const availableSpells = [...north.hand.spellbook, ...north.spellbook.slice(0, 3)];
    const featuredInstanceId = earlySpells
      .find(({ cardId }) => cardId === input.submergeMinion.stableId)?.instanceId;
    const comparisonInstanceId = earlySpells.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'minion'
        && definition.submerge !== true
        && definition.manaCost <= 3
        && definition.thresholds.air === 0
        && definition.thresholds.earth === 0
        && definition.thresholds.fire === 0
        && definition.thresholds.water <= 3;
    })?.instanceId;
    const seaWitchInstanceId = availableSpells
      .find(({ cardId }) => cardId === input.seaWitch.stableId)?.instanceId;
    const freezeInstanceId = availableSpells
      .find(({ cardId }) => cardId === input.freeze.stableId)?.instanceId;
    const southSites = session.state.players.south.hand.atlas.slice(0, 2);
    if (northWaterSites.length >= 2
      && thirdNorthSite
      && featuredInstanceId
      && comparisonInstanceId
      && seaWitchInstanceId
      && freezeInstanceId
      && southSites.length === 2) {
      return {
        ...built,
        comparisonInstanceId,
        featuredInstanceId,
        freezeInstanceId,
        northSiteInstanceIds: [
          northWaterSites[0]!.instanceId,
          northWaterSites[1]!.instanceId,
          thirdNorthSite.instanceId,
        ],
        seaWitchInstanceId,
        seed,
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private underwater Freeze scenario no longer produces its supported opening');
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

function findWaterGnarledWendigoOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  ghostTownInstanceId: string;
  gnarledWendigoInstanceId: string;
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  northWaterSiteInstanceIds: readonly [string, string];
  seravaInstanceId: string;
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: bounded seed scan avoids another private config field; lock one only if runtime matters.
  for (let offset = 1; offset <= 4096; offset += 1) {
    const built = buildManifest(
      input,
      input.config.waterSeed + offset,
      'water-gnarled-wendigo',
    );
    const session = createGameSession(built.manifest);
    const northWaterSites = session.state.players.north.hand.atlas.filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return cardId !== input.ghostTownSite.stableId
        && definition?.cardType === 'site'
        && definition.elements.includes('water');
    });
    const ghostTownInstanceId = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.ghostTownSite.stableId)?.instanceId;
    const seravaInstanceId = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === input.seravaTownsfolk.stableId)?.instanceId;
    const gnarledWendigoInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 2),
    ].find(({ cardId }) => cardId === input.gnarledWendigo.stableId)?.instanceId;
    const southSites = session.state.players.south.hand.atlas;
    if (northWaterSites.length >= 2
      && ghostTownInstanceId
      && seravaInstanceId
      && gnarledWendigoInstanceId
      && southSites.length >= 2) {
      return {
        ...built,
        ghostTownInstanceId,
        gnarledWendigoInstanceId,
        northWaterSiteInstanceIds: [
          northWaterSites[0]!.instanceId,
          northWaterSites[1]!.instanceId,
        ],
        seravaInstanceId,
        session,
        southSiteInstanceIds: [southSites[0]!.instanceId, southSites[1]!.instanceId],
      };
    }
  }
  throw new Error('private summon-location sacrifice-discount scenario lacks its supported opening');
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

function findWaterMesmerismOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  farSeravaInstanceId: string;
  kettletopInstanceId: string;
  manifest: GameManifest;
  mesmerismInstanceId: string;
  names: ReadonlyMap<string, string>;
  northSiteInstanceIds: readonly [string, string, string, string];
  session: GameSession;
  southSiteInstanceIds: readonly [string, string];
}> {
  // ponytail: pinned offset keeps this private proof fast without another config field.
  for (const offset of [4708]) {
    const built = buildManifest(input, input.config.waterSeed + offset, 'water-mesmerism');
    const session = createGameSession(built.manifest);
    const northSites = [
      ...session.state.players.north.hand.atlas,
      ...session.state.players.north.atlas.slice(0, 1),
    ].filter(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const southWaterSite = session.state.players.south.hand.atlas.find(({ cardId }) => {
      const definition = session.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const southValleyInstanceId = session.state.players.south.hand.atlas
      .find(({ cardId }) => cardId === input.valley.stableId)?.instanceId;
    const mesmerismInstanceId = [
      ...session.state.players.north.hand.spellbook,
      ...session.state.players.north.spellbook.slice(0, 3),
    ].find(({ cardId }) => cardId === input.mesmerism.stableId)?.instanceId;
    const southAccessibleSpells = [
      ...session.state.players.south.hand.spellbook,
      ...session.state.players.south.spellbook.slice(0, 3),
    ];
    const farSeravaInstanceId = southAccessibleSpells
      .find(({ cardId }) => cardId === input.seravaTownsfolk.stableId)?.instanceId;
    const kettletopInstanceId = southAccessibleSpells
      .find(({ cardId }) => cardId === input.deathriteMinion.stableId)?.instanceId;
    if (northSites.length >= 4
      && southWaterSite
      && southValleyInstanceId
      && mesmerismInstanceId
      && farSeravaInstanceId
      && kettletopInstanceId) {
      return {
        ...built,
        farSeravaInstanceId,
        kettletopInstanceId,
        mesmerismInstanceId,
        northSiteInstanceIds: [
          northSites[0]!.instanceId,
          northSites[1]!.instanceId,
          northSites[2]!.instanceId,
          northSites[3]!.instanceId,
        ],
        session,
        southSiteInstanceIds: [southWaterSite.instanceId, southValleyInstanceId],
      };
    }
  }
  throw new Error('private nearby minion control Magic scenario lacks its supported opening');
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

function runStarter(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
  scenario: StarterScenario,
  baseSeed: number,
  siteCard: NormalizedCard,
  minionCard: NormalizedCard,
): StarterCheck {
  const opening = findStarterOpening(input, scenario, baseSeed, siteCard, minionCard);
  let session = keep(opening.session);
  session = keep(session);

  const siteResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.siteInstanceId
      && descriptor.cell === 'C4'
      && descriptor.genesisTokenChoice !== 'pay-one-mana'));
  if (!siteResult.accepted) throw new Error(`private ${siteCard.name} play was rejected`);
  session = siteResult.session;
  if (session.state.phase === 'genesis') {
    const genesisResult = stepGame(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'resolve-genesis-spell' && descriptor.choice === 'keep-next'));
    if (!genesisResult.accepted) throw new Error(`private ${siteCard.name} Genesis was rejected`);
    session = genesisResult.session;
  }
  const manaBeforeSummon = session.state.players.north.mana;

  const summonResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.minionInstanceId
      && descriptor.cell === 'C4'));
  if (!summonResult.accepted) throw new Error(`private ${minionCard.name} summon was rejected`);
  session = summonResult.session;

  const sitePayload = siteResult.receipt.events[0]
    && isJsonRecord(siteResult.receipt.events[0].payload)
    ? siteResult.receipt.events[0].payload
    : undefined;
  const summonPayload = summonResult.receipt.events[0]
    && isJsonRecord(summonResult.receipt.events[0].payload)
    ? summonResult.receipt.events[0].payload
    : undefined;
  const site = session.state.realm.sites.C4;
  const minion = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.minionInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: siteResult.receipt.events.map(({ type }) => type).join(',')
      === (scenario === 'earth-starter' ? 'site-played,rubble-created' : 'site-played')
      && summonResult.receipt.events.map(({ type }) => type).join(',') === 'minion-summoned'
      && sitePayload?.cardId === siteCard.stableId
      && sitePayload.instanceId === opening.siteInstanceId
      && sitePayload.cell === 'C4'
      && summonPayload?.cardId === minionCard.stableId
      && summonPayload.instanceId === opening.minionInstanceId
      && summonPayload.cell === 'C4'
      && summonPayload.manaPaid === 1,
    deck: deckList(opening.manifest.decks.north, opening.names),
    manaPaid: manaBeforeSummon - session.state.players.north.mana,
    minion: minionCard.name,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    replayVerified: verifyGameReplay(session),
    site: siteCard.name,
    siteAndMinionStateVerified: site?.instanceId === opening.siteInstanceId
      && 'cardId' in site
      && site.cardId === siteCard.stableId
      && site.controller === 'north'
      && minion?.cardId === minionCard.stableId
      && minion.controller === 'north'
      && minion.owner === 'north'
      && minion.location === 'C4'
      && minion.region === 'surface'
      && minion.damage === 0
      && !minion.tapped
      && minion.summoningSickness,
  });
}

function runFireGranaryRats(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireGranaryRats'] {
  const opening = findStarterOpening(
    input,
    'fire-granary-rats',
    input.config.fireSeed,
    input.wasteland,
    input.granaryRats,
  );
  let session = keep(opening.session);
  session = keep(session);

  const siteResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.siteInstanceId
      && descriptor.cell === 'C4'));
  if (!siteResult.accepted) throw new Error('private Granary Rats Wasteland play was rejected');
  session = siteResult.session;
  const beforeSummon = observeGame(session.state, 'north');
  const manaBeforeSummon = session.state.players.north.mana;
  const summonAction = action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.minionInstanceId
      && descriptor.cell === 'C4');
  if (summonAction.descriptor.kind !== 'summon-minion') {
    throw new Error('private Granary Rats summon action has the wrong kind');
  }
  const summonResult = stepGame(session, summonAction);
  if (!summonResult.accepted) throw new Error('private Granary Rats summon was rejected');
  session = summonResult.session;

  const sitePayload = siteResult.receipt.events[0]
    && isJsonRecord(siteResult.receipt.events[0].payload)
    ? siteResult.receipt.events[0].payload
    : undefined;
  const summonPayload = summonResult.receipt.events[0]
    && isJsonRecord(summonResult.receipt.events[0].payload)
    ? summonResult.receipt.events[0].payload
    : undefined;
  const afterSummon = observeGame(session.state, 'north');
  const site = session.state.realm.sites.C4;
  const rats = afterSummon.realm.units.find(({ instanceId }) =>
    instanceId === opening.minionInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: siteResult.receipt.events.length === 1
      && siteResult.receipt.events[0]?.type === 'site-played'
      && canonicalJson(sitePayload ?? null) === canonicalJson({
        cardId: input.wasteland.stableId,
        cell: 'C4',
        instanceId: opening.siteInstanceId,
        seat: 'north',
      })
      && summonResult.receipt.events.length === 1
      && summonResult.receipt.events[0]?.type === 'minion-summoned'
      && canonicalJson(summonPayload ?? null) === canonicalJson({
        cardId: input.granaryRats.stableId,
        casterInstanceId: summonAction.descriptor.casterInstanceId,
        cell: 'C4',
        instanceId: opening.minionInstanceId,
        manaPaid: 1,
        seat: 'north',
      }),
    deck: deckList(opening.manifest.decks.north, opening.names),
    fireAffinityBeforeSummon: beforeSummon.players.north.affinity.fire === 1
      && beforeSummon.players.north.affinity.air === 0
      && beforeSummon.players.north.affinity.earth === 0
      && beforeSummon.players.north.affinity.water === 0,
    granaryRats: input.granaryRats.name,
    manaPaid: manaBeforeSummon - session.state.players.north.mana,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    replayVerified: verifyGameReplay(session),
    seed: opening.manifest.seed,
    siteAndMinionStateVerified: site !== undefined
      && 'cardId' in site
      && site.cardId === input.wasteland.stableId
      && site.instanceId === opening.siteInstanceId
      && site.controller === 'north'
      && rats?.cardId === input.granaryRats.stableId
      && rats.attack === 1
      && rats.defense === 1
      && rats.controller === 'north'
      && rats.owner === 'north'
      && rats.location === 'C4'
      && rats.region === 'surface'
      && rats.damage === 0
      && !rats.tapped
      && rats.summoningSickness,
    siteThresholdSuppressed: afterSummon.players.north.affinity.fire === 0
      && afterSummon.players.north.affinity.air === 0
      && afterSummon.players.north.affinity.earth === 0
      && afterSummon.players.north.affinity.water === 0,
    wasteland: input.wasteland.name,
  });
}

function runFireHamlet(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireHamlet'] {
  const opening = findFireHamletOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const wastelandResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.wastelandInstanceId
      && descriptor.cell === 'C4'));
  if (!wastelandResult.accepted) throw new Error('private Wasteland play was rejected');
  session = wastelandResult.session;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const hamletResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.hamletInstanceId
      && descriptor.cell === 'C3'));
  if (!hamletResult.accepted) throw new Error('private Hamlet play was rejected');
  session = hamletResult.session;

  const beforeSummon = observeGame(session.state, 'north');
  const manaBeforeSummon = session.state.players.north.mana;
  const raalSummons = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.raalInstanceId
      && descriptor.region === undefined
      && (descriptor.cell === 'C3' || descriptor.cell === 'C4'));
  const hamletSummon = raalSummons.find(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C3'
      && descriptor.manaCost === 0);
  const wastelandSummon = raalSummons.find(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4'
      && descriptor.manaCost === 1);
  if (!hamletSummon || !wastelandSummon) {
    throw new Error('private Hamlet and Wasteland destination costs are unavailable');
  }
  const summonResult = stepGame(session, hamletSummon);
  if (!summonResult.accepted) throw new Error('private zero-cost Raal summon was rejected');
  session = summonResult.session;

  const wastelandPayload = wastelandResult.receipt.events[0]
    && isJsonRecord(wastelandResult.receipt.events[0].payload)
    ? wastelandResult.receipt.events[0].payload
    : undefined;
  const hamletPayload = hamletResult.receipt.events[0]
    && isJsonRecord(hamletResult.receipt.events[0].payload)
    ? hamletResult.receipt.events[0].payload
    : undefined;
  const summonPayload = summonResult.receipt.events[0]
    && isJsonRecord(summonResult.receipt.events[0].payload)
    ? summonResult.receipt.events[0].payload
    : undefined;
  const wastelandSite = session.state.realm.sites.C4;
  const hamletSite = session.state.realm.sites.C3;
  const raal = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === opening.raalInstanceId);
  const sameCaster = hamletSummon.descriptor.kind === 'summon-minion'
    && wastelandSummon.descriptor.kind === 'summon-minion'
    && hamletSummon.descriptor.casterInstanceId === wastelandSummon.descriptor.casterInstanceId;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: wastelandResult.receipt.events.length === 1
      && wastelandResult.receipt.events[0]?.type === 'site-played'
      && canonicalJson(wastelandPayload ?? null) === canonicalJson({
        cardId: input.wasteland.stableId,
        cell: 'C4',
        instanceId: opening.wastelandInstanceId,
        seat: 'north',
      })
      && hamletResult.receipt.events.length === 1
      && hamletResult.receipt.events[0]?.type === 'site-played'
      && canonicalJson(hamletPayload ?? null) === canonicalJson({
        cardId: input.hamlet.stableId,
        cell: 'C3',
        instanceId: opening.hamletInstanceId,
        seat: 'north',
      })
      && summonResult.receipt.events.length === 1
      && summonResult.receipt.events[0]?.type === 'minion-summoned'
      && hamletSummon.descriptor.kind === 'summon-minion'
      && canonicalJson(summonPayload ?? null) === canonicalJson({
        cardId: input.raalDromedary.stableId,
        casterInstanceId: hamletSummon.descriptor.casterInstanceId,
        cell: 'C3',
        instanceId: opening.raalInstanceId,
        manaPaid: 0,
        seat: 'north',
      }),
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactDestinationCosts: raalSummons.length === 2 && sameCaster,
    fireAffinityVerified: beforeSummon.players.north.affinity.fire === 1
      && beforeSummon.players.north.affinity.air === 0
      && beforeSummon.players.north.affinity.earth === 0
      && beforeSummon.players.north.affinity.water === 0
      && manaBeforeSummon === 2,
    hamlet: input.hamlet.name,
    manaPaid: manaBeforeSummon - session.state.players.north.mana,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    raalDromedary: input.raalDromedary.name,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    siteAndMinionStateVerified: wastelandSite !== undefined
      && 'cardId' in wastelandSite
      && wastelandSite.cardId === input.wasteland.stableId
      && wastelandSite.controller === 'north'
      && hamletSite !== undefined
      && 'cardId' in hamletSite
      && hamletSite.cardId === input.hamlet.stableId
      && hamletSite.controller === 'north'
      && raal?.cardId === input.raalDromedary.stableId
      && raal.controller === 'north'
      && raal.owner === 'north'
      && raal.location === 'C3'
      && raal.region === 'surface'
      && raal.attack === 2
      && raal.defense === 2
      && raal.damage === 0
      && !raal.tapped
      && raal.summoningSickness,
    wasteland: input.wasteland.name,
  });
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

function runEarthBorderMilitia(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthBorderMilitia'] {
  const opening = findEarthBorderMilitiaOpening(input);
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
    && descriptor.cell === 'B4');
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
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[2]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const manaBefore = session.state.players.north.mana;
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.borderMilitiaInstanceId);
  const chosen = choices[0];
  if (!chosen) throw new Error('private Border Militia targetless cast is unavailable');
  const castResult = stepGame(session, chosen);
  if (!castResult.accepted) throw new Error('private Border Militia cast was rejected');
  session = castResult.session;

  const tokens = session.state.realm.units
    .filter(({ cardId }) => cardId === input.footSoldier.stableId)
    .sort((left, right) => left.location.localeCompare(right.location));
  const [b3Token, c4Token] = tokens;
  if (!b3Token || !c4Token) throw new Error('private Border Militia did not summon two tokens');
  const events = castResult.receipt.events;
  const castPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const summonPayloads = events.filter(({ type }) => type === 'minion-summoned')
    .map(({ payload }) => payload);
  const tokenDefinition = session.state.cards[input.footSoldier.stableId];
  const spellEnteredCemetery = session.state.players.north.hand.spellbook
    .every(({ instanceId }) => instanceId !== opening.borderMilitiaInstanceId)
    && session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === opening.borderMilitiaInstanceId);
  const tokensVerified: boolean = choices.length === 1
    && events.map(({ type }) => type).join(',')
      === 'magic-cast,minion-summoned,minion-summoned,magic-resolved'
    && castPayload?.cardId === input.borderMilitia.stableId
    && castPayload.instanceId === opening.borderMilitiaInstanceId
    && castPayload.manaPaid === 3
    && castPayload.seat === 'north'
    && canonicalJson(summonPayloads) === canonicalJson([
      {
        cardId: input.footSoldier.stableId,
        cell: 'B3',
        instanceId: b3Token.instanceId,
        owner: 'north',
        seat: 'north',
        sourceInstanceId: opening.borderMilitiaInstanceId,
        token: true,
      },
      {
        cardId: input.footSoldier.stableId,
        cell: 'C4',
        instanceId: c4Token.instanceId,
        owner: 'north',
        seat: 'north',
        sourceInstanceId: opening.borderMilitiaInstanceId,
        token: true,
      },
    ])
    && tokens.length === 2
    && b3Token.location === 'B3'
    && c4Token.location === 'C4'
    && tokens.every(({ controller, damage, owner, region, source, summoningSickness, tapped }) =>
      controller === 'north'
        && damage === 0
        && owner === 'north'
        && region === 'surface'
        && source === 'token'
        && summoningSickness
        && !tapped)
    && session.state.realm.units.every(({ location }) => location !== 'B4')
    && session.state.realm.sites.B3?.controller === 'north'
    && session.state.realm.sites.B4?.controller === 'north'
    && session.state.realm.sites.C4?.controller === 'north'
    && session.state.realm.sites.C3?.controller === 'south'
    && tokenDefinition?.cardType === 'minion'
    && tokenDefinition.token === true
    && tokenDefinition.attack === 1
    && tokenDefinition.defense === 1
    && tokenDefinition.manaCost === 0
    && (['north', 'south'] as const).every((seat) => session.state.players[seat].cemetery
      .every(({ cardId }) => cardId !== input.footSoldier.stableId));

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    borderMilitia: input.borderMilitia.name,
    deck: deckList(opening.manifest.decks.north, opening.names),
    footSoldier: input.footSoldier.name,
    manaPaid: manaBefore - session.state.players.north.mana,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    spellEnteredCemetery,
    tokensVerified,
  });
}

function runEarthHumbleVillage(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthHumbleVillage'] {
  const opening = findEarthHumbleVillageOpening(input);
  const checkpoint = keep(keep(opening.session));
  const choices = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.humbleVillageInstanceId
      && descriptor.cell === 'C4');
  const declined = choices.find(({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.genesisTokenChoice === 'decline');
  const paid = choices.find(({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.genesisTokenChoice === 'pay-one-mana');
  if (!declined || !paid) throw new Error('private Humble Village choices are unavailable');
  const declinedResult = stepGame(checkpoint, declined);
  const paidResult = stepGame(checkpoint, paid);
  if (!declinedResult.accepted || !paidResult.accepted) {
    throw new Error('private Humble Village choice was rejected');
  }

  const declinedSession = declinedResult.session;
  const paidSession = paidResult.session;
  const token = paidSession.state.realm.units.find(({ cardId }) =>
    cardId === input.footSoldier.stableId);
  if (!token) throw new Error('private Humble Village did not summon its token');
  const source = paidSession.state.realm.sites.C4;
  const tokenDefinition = paidSession.state.cards[input.footSoldier.stableId];
  const exactChoices = choices.length === 2
    && declined.actionId !== paid.actionId
    && new Set(choices.map(({ label }) => label)).size === 2;
  const declinedKeptManaAndSummonedNothing = declinedSession.state.players.north.mana === 1
    && declinedSession.state.players.north.domainEstablished
    && declinedSession.state.players.north.avatar.tapped
    && declinedSession.state.realm.sites.C4 !== undefined
    && 'cardId' in declinedSession.state.realm.sites.C4
    && declinedSession.state.realm.sites.C4.cardId === input.humbleVillage.stableId
    && declinedSession.state.realm.units.every(({ cardId }) => cardId !== input.footSoldier.stableId)
    && declinedResult.receipt.events.map(({ type }) => type).join(',') === 'site-played';
  const paidSpentManaAndSummonedToken = paidSession.state.players.north.mana === 0
    && paidSession.state.players.north.domainEstablished
    && paidSession.state.players.north.avatar.tapped
    && source !== undefined
    && 'cardId' in source
    && source.cardId === input.humbleVillage.stableId
    && source.controller === 'north'
    && paidSession.state.realm.units.filter(({ cardId }) =>
      cardId === input.footSoldier.stableId).length === 1
    && token.controller === 'north'
    && token.damage === 0
    && token.location === 'C4'
    && token.owner === 'north'
    && token.region === 'surface'
    && token.source === 'token'
    && token.summoningSickness
    && !token.tapped
    && canonicalJson(paidResult.receipt.events.map(({ payload, type }) => ({ payload, type })))
      === canonicalJson([
        {
          payload: {
            cardId: input.humbleVillage.stableId,
            cell: 'C4',
            instanceId: source.instanceId,
            seat: 'north',
          },
          type: 'site-played',
        },
        {
          payload: {
            cardId: input.footSoldier.stableId,
            cell: 'C4',
            instanceId: token.instanceId,
            manaPaid: 1,
            owner: 'north',
            seat: 'north',
            sourceInstanceId: source.instanceId,
            token: true,
          },
          type: 'minion-summoned',
        },
      ]);
  const tokenDefinitionVerified = tokenDefinition?.cardType === 'minion'
    && tokenDefinition.token === true
    && tokenDefinition.attack === 1
    && tokenDefinition.defense === 1
    && tokenDefinition.manaCost === 0
    && opening.manifest.decks.north.atlas.every((cardId) =>
      cardId !== input.footSoldier.stableId)
    && opening.manifest.decks.north.spellbook.every((cardId) =>
      cardId !== input.footSoldier.stableId)
    && (['north', 'south'] as const).every((seat) =>
      paidSession.state.players[seat].cemetery.every(({ cardId }) =>
        cardId !== input.footSoldier.stableId));

  return Object.freeze({
    acceptedActionCount: paidSession.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    declinedKeptManaAndSummonedNothing,
    exactChoices,
    footSoldier: input.footSoldier.name,
    gameRemainedActive: declinedSession.state.terminal.status === 'active'
      && paidSession.state.terminal.status === 'active',
    humbleVillage: input.humbleVillage.name,
    noRandomDraws: [...declinedSession.transcript, ...paidSession.transcript]
      .every(({ randomDraws }) => randomDraws.length === 0),
    paidSpentManaAndSummonedToken,
    replayVerified: verifyGameReplay(declinedSession) && verifyGameReplay(paidSession),
    seed: opening.seed,
    tokenDefinitionVerified,
  });
}

function runEarthDuel(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthDuel'] {
  const opening = findEarthDuelMagicOpening(input);
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
    && descriptor.cardInstanceId === opening.boskTrollInstanceId
    && descriptor.cell === 'C3'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.elthamTownsfolkInstanceId
    && descriptor.cell === 'C2'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');

  const boskBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.boskTrollInstanceId);
  const elthamBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  if (!boskBefore || !elthamBefore) throw new Error('private Duel setup lacks its real minions');
  const sitesBefore = canonicalJson(session.state.realm.sites as unknown as JsonValue);
  const avatarsBefore = canonicalJson({
    north: observeGame(session.state, 'north').players.north.avatar,
    south: observeGame(session.state, 'north').players.south.avatar,
  } as unknown as JsonValue);
  const manaBefore = session.state.players.north.mana;
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.duelInstanceId
      && descriptor.ally?.kind === 'minion'
      && descriptor.ally.instanceId === opening.boskTrollInstanceId
      && descriptor.target?.kind === 'minion'
      && descriptor.target.instanceId === opening.elthamTownsfolkInstanceId);
  const chosen = choices[0];
  if (!chosen) throw new Error('private Duel Bosk-to-Eltham fight pair is unavailable');
  const duelResult = stepGame(session, chosen);
  if (!duelResult.accepted) throw new Error('private Duel cast was rejected');
  session = duelResult.session;

  const events = duelResult.receipt.events;
  const payload = (type: string): Readonly<Record<string, JsonValue>> | undefined => {
    const found = events.find((event) => event.type === type);
    return found && isJsonRecord(found.payload) ? found.payload : undefined;
  };
  const castPayload = payload('magic-cast');
  const fightPayload = payload('fight-started');
  const strikePayload = payload('strike-damage-allocated');
  const deathPayload = payload('minion-died');
  const resolvedPayload = payload('magic-resolved');
  const damagePayloads = events.filter(({ type }) => type === 'damage-dealt')
    .map(({ payload: value }) => isJsonRecord(value) ? value : undefined);
  const boskAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.boskTrollInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    allySurvivedWithTwoDamage: boskAfter?.damage === 2
      && boskAfter.cardId === input.firstStrikeTargetMinion.stableId
      && boskAfter.controller === 'north'
      && boskAfter.owner === 'north',
    boskTroll: input.firstStrikeTargetMinion.name,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'magic-cast,fight-started,strike-damage-allocated,damage-dealt,damage-dealt,minion-died,magic-resolved'
      && castPayload?.instanceId === opening.duelInstanceId
      && castPayload.manaPaid === 3
      && castPayload.seat === 'north'
      && castPayload.allyInstanceId === opening.boskTrollInstanceId
      && castPayload.targetInstanceId === opening.elthamTownsfolkInstanceId
      && fightPayload?.attackerInstanceId === opening.boskTrollInstanceId
      && canonicalJson(fightPayload.combatantInstanceIds ?? null)
        === canonicalJson([opening.elthamTownsfolkInstanceId])
      && strikePayload?.amount === 3
      && strikePayload.strikerInstanceId === opening.boskTrollInstanceId
      && strikePayload.targetInstanceId === opening.elthamTownsfolkInstanceId
      && damagePayloads.some((value) => value?.instanceId === opening.boskTrollInstanceId
        && value.amount === 2 && value.accumulated === 2)
      && damagePayloads.some((value) => value?.instanceId === opening.elthamTownsfolkInstanceId
        && value.amount === 3 && value.accumulated === 3)
      && deathPayload?.cardId === input.elthamTownsfolk.stableId
      && deathPayload.instanceId === opening.elthamTownsfolkInstanceId
      && deathPayload.owner === 'south'
      && resolvedPayload?.instanceId === opening.duelInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    duel: input.duel.name,
    elthamTownsfolk: input.elthamTownsfolk.name,
    exactFightPair: choices.length === 1
      && chosen.descriptor.kind === 'cast-magic'
      && chosen.descriptor.ally?.seat === 'north'
      && chosen.descriptor.target?.seat === 'south',
    gameRemainedActive: session.state.phase === 'main' && session.state.terminal.status === 'active',
    manaPaid: manaBefore - session.state.players.north.mana,
    noRandomDraws: duelResult.receipt.randomDraws.length === 0,
    replayVerified: verifyGameReplay(session),
    sitesAndAvatarsPreserved:
      canonicalJson(session.state.realm.sites as unknown as JsonValue) === sitesBefore
      && canonicalJson({
        north: observeGame(session.state, 'north').players.north.avatar,
        south: observeGame(session.state, 'north').players.south.avatar,
      } as unknown as JsonValue) === avatarsBefore,
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.duelInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.duelInstanceId),
    targetDiedAndEnteredCemetery: session.state.realm.units
      .every(({ instanceId }) => instanceId !== opening.elthamTownsfolkInstanceId)
      && session.state.players.south.cemetery.some(({ cardId, instanceId }) =>
        cardId === input.elthamTownsfolk.stableId
          && instanceId === opening.elthamTownsfolkInstanceId),
    unitsDidNotMoveOrTap: boskBefore.location === 'C3'
      && !boskBefore.tapped
      && elthamBefore.location === 'C2'
      && !elthamBefore.tapped
      && boskAfter?.location === boskBefore.location
      && boskAfter.tapped === boskBefore.tapped
      && events.every(({ type }) => type !== 'unit-moved' && type !== 'unit-tapped'),
  });
}

function runEarthArtifactSetup(
  opening: ReturnType<typeof findEarthArtifactOpening>,
  playThirdNorthSite: boolean,
): GameSession {
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
    && descriptor.cardInstanceId === opening.southEarthSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.elthamTownsfolkInstanceId
    && descriptor.cell === 'C3'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southEarthSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.boskTrollInstanceId
    && descriptor.cell === 'C2'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  if (playThirdNorthSite) {
    take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
      && descriptor.cell === 'B3');
  }
  return session;
}

function runEarthSwordAndShield(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthSwordAndShield'] {
  const opening = findEarthArtifactOpening(
    input,
    input.swordAndShield,
    'earth-sword-and-shield',
  );
  if (!opening.zapInstanceIds || opening.zapDrawCount === undefined) {
    throw new Error('private Sword Drop-death opening lacks Zap!');
  }
  let session = runEarthArtifactSetup(opening, true);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const sitesBeforeCombat = canonicalJson(session.state.realm.sites as unknown as JsonValue);
  const avatarsBeforeCombat = {
    north: {
      life: session.state.players.north.avatar.life,
      location: session.state.players.north.avatar.location,
      region: session.state.players.north.avatar.region,
    },
    south: {
      life: session.state.players.south.avatar.life,
      location: session.state.players.south.avatar.location,
      region: session.state.players.south.avatar.region,
    },
  };
  const manaBefore = session.state.players.north.mana;
  const uncarriedCastChoices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-artifact'
      && descriptor.cardInstanceId === opening.artifactInstanceId
      && descriptor.bearer === undefined
      && descriptor.cell === 'C3');
  const chosenArtifact = uncarriedCastChoices[0];
  if (!chosenArtifact) throw new Error('private uncarried Sword and Shield cast is unavailable');
  const castResult = stepGame(session, chosenArtifact);
  if (!castResult.accepted) throw new Error('private Sword and Shield cast was rejected');
  session = castResult.session;

  const castView = observeGame(session.state, 'north');
  const castArtifactState = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const castArtifactView = castView.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const unpoweredEltham = castView.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  const manaBeforePickup = session.state.players.north.mana;
  const pickupChoices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'pick-up-artifacts'
      && descriptor.unit.kind === 'minion'
      && descriptor.unit.instanceId === opening.elthamTownsfolkInstanceId
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === opening.artifactInstanceId);
  const chosenPickup = pickupChoices[0];
  if (!chosenPickup) throw new Error('private Sword and Shield Pick Up is unavailable');
  const pickupResult = stepGame(session, chosenPickup);
  if (!pickupResult.accepted) throw new Error('private Sword and Shield Pick Up was rejected');
  session = pickupResult.session;
  const manaAfterPickup = session.state.players.north.mana;

  const pickedView = observeGame(session.state, 'north');
  const pickedArtifactState = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const pickedArtifactView = pickedView.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const poweredEltham = pickedView.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);

  const dropBranchStart = session;
  const manaBeforeDrop = dropBranchStart.state.players.north.mana;
  const dropChoices = legalGameActions(dropBranchStart.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'drop-artifacts'
      && descriptor.unit.kind === 'minion'
      && descriptor.unit.instanceId === opening.elthamTownsfolkInstanceId
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === opening.artifactInstanceId);
  const chosenDrop = dropChoices[0];
  if (!chosenDrop) throw new Error('private Sword and Shield Drop is unavailable');
  const dropResult = stepGame(dropBranchStart, chosenDrop);
  if (!dropResult.accepted) throw new Error('private Sword and Shield Drop was rejected');
  const dropSession = dropResult.session;
  const droppedView = observeGame(dropSession.state, 'north');
  const droppedArtifactState = dropSession.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const droppedArtifactView = droppedView.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const droppedEltham = droppedView.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  const dropEvent = dropResult.receipt.events.find(({ type }) => type === 'artifacts-dropped');
  const dropPayload = dropEvent && isJsonRecord(dropEvent.payload) ? dropEvent.payload : undefined;
  const secondDropUnavailable = legalGameActions(dropSession.state, 'north')
    .every(({ descriptor }) => descriptor.kind !== 'drop-artifacts'
      || descriptor.unit.kind !== 'minion'
      || descriptor.unit.instanceId !== opening.elthamTownsfolkInstanceId);

  let dropDeathSession = dropBranchStart;
  const takeDropDeath = (predicate: (candidate: GameLegalAction) => boolean): void => {
    dropDeathSession = accept(dropDeathSession, action(dropDeathSession, predicate));
  };
  for (let draw = 0; draw < Math.max(1, opening.zapDrawCount); draw += 1) {
    takeDropDeath(({ descriptor }) => descriptor.kind === 'end-turn');
    takeDropDeath(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    takeDropDeath(({ descriptor }) => descriptor.kind === 'end-turn');
    takeDropDeath(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  }
  for (const zapInstanceId of opening.zapInstanceIds) {
    takeDropDeath(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === zapInstanceId
      && descriptor.target?.kind === 'minion'
      && descriptor.target.instanceId === opening.elthamTownsfolkInstanceId);
  }
  const damagedBearer = observeGame(dropDeathSession.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === opening.elthamTownsfolkInstanceId);
  const dropDeathResult = stepGame(dropDeathSession, action(dropDeathSession, ({ descriptor }) =>
    descriptor.kind === 'drop-artifacts'
      && descriptor.unit.kind === 'minion'
      && descriptor.unit.instanceId === opening.elthamTownsfolkInstanceId
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === opening.artifactInstanceId));
  if (!dropDeathResult.accepted) throw new Error('private lethal Sword Drop was rejected');
  dropDeathSession = dropDeathResult.session;
  const dropDeathArtifact = observeGame(dropDeathSession.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === opening.artifactInstanceId);
  const dropDeathEvents = dropDeathResult.receipt.events;
  const dropDeathDropPayload = dropDeathEvents[0] && isJsonRecord(dropDeathEvents[0].payload)
    ? dropDeathEvents[0].payload
    : undefined;
  const dropDeathMinionPayload = dropDeathEvents[1] && isJsonRecord(dropDeathEvents[1].payload)
    ? dropDeathEvents[1].payload
    : undefined;
  const dropDeathVerified = damagedBearer?.damage === 2
    && damagedBearer.defense === 4
    && dropDeathEvents.map(({ type }) => type).join(',') === 'artifacts-dropped,minion-died'
    && dropDeathDropPayload !== undefined
    && canonicalJson(dropDeathDropPayload) === canonicalJson({
      artifactInstanceIds: [opening.artifactInstanceId],
      seat: 'north',
      unitInstanceId: opening.elthamTownsfolkInstanceId,
      unitKind: 'minion',
    })
    && dropDeathMinionPayload !== undefined
    && canonicalJson(dropDeathMinionPayload) === canonicalJson({
      cardId: input.elthamTownsfolk.stableId,
      instanceId: opening.elthamTownsfolkInstanceId,
      owner: 'north',
    })
    && dropDeathSession.state.realm.units.every(({ instanceId }) =>
      instanceId !== opening.elthamTownsfolkInstanceId)
    && dropDeathSession.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === opening.elthamTownsfolkInstanceId)
    && dropDeathArtifact !== undefined
    && dropDeathArtifact.bearer === undefined
    && dropDeathArtifact.controller === null
    && dropDeathArtifact.location === 'C3'
    && dropDeathArtifact.owner === 'north'
    && dropDeathArtifact.region === 'surface'
    && dropDeathSession.state.players.north.cemetery.every(({ instanceId }) =>
      instanceId !== opening.artifactInstanceId)
    && dropDeathSession.transcript.every(({ randomDraws }) => randomDraws.length === 0);

  const moveResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.elthamTownsfolkInstanceId
      && descriptor.from.cell === 'C3'
      && descriptor.to.cell === 'C2'));
  if (!moveResult.accepted) throw new Error('private Sword bearer move was rejected');
  session = moveResult.session;
  const movedView = observeGame(session.state, 'north');
  const movedArtifact = movedView.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const movedEltham = movedView.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);

  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === opening.boskTrollInstanceId);
  const fightResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  if (!fightResult.accepted) throw new Error('private Sword and Shield fight was rejected');
  session = fightResult.session;

  const castEvent = castResult.receipt.events.find(({ type }) => type === 'artifact-conjured');
  const castPayload = castEvent && isJsonRecord(castEvent.payload) ? castEvent.payload : undefined;
  const pickupEvent = pickupResult.receipt.events.find(({ type }) => type === 'artifacts-picked-up');
  const pickupPayload = pickupEvent && isJsonRecord(pickupEvent.payload)
    ? pickupEvent.payload
    : undefined;
  const fightEvents = fightResult.receipt.events;
  const allocations = fightEvents.filter(({ type }) => type === 'strike-damage-allocated')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const damages = fightEvents.filter(({ type }) => type === 'damage-dealt')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const deaths = fightEvents.filter(({ type }) => type === 'minion-died')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const fightStartedIndex = fightEvents.findIndex(({ type }) => type === 'fight-started');
  const firstAllocationIndex = fightEvents.findIndex(({ type }) => type === 'strike-damage-allocated');
  const firstDamageIndex = fightEvents.findIndex(({ type }) => type === 'damage-dealt');
  const finalArtifactState = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const finalArtifactView = observeGame(session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === opening.artifactInstanceId);
  const finalEltham = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.elthamTownsfolkInstanceId);
  const elthamAbsentFromCemetery = session.state.players.north.cemetery.every(({ instanceId }) =>
    instanceId !== opening.elthamTownsfolkInstanceId);
  const boskInCemetery = session.state.players.south.cemetery.some(({ cardId, instanceId }) =>
    cardId === input.firstStrikeTargetMinion.stableId
      && instanceId === opening.boskTrollInstanceId);
  const artifactAbsentFromCemeteries = (['north', 'south'] as const).every((seat) =>
    session.state.players[seat].cemetery.every(({ instanceId }) =>
      instanceId !== opening.artifactInstanceId));
  const artifactCastUncarried: boolean = uncarriedCastChoices.length === 1
    && castArtifactState !== undefined
    && !('bearer' in castArtifactState)
    && castArtifactState.location === 'C3'
    && castArtifactState.region === 'surface'
    && castArtifactState.owner === 'north'
    && castArtifactView !== undefined
    && castArtifactView.bearer === undefined
    && castArtifactView.controller === null
    && castArtifactView.owner === 'north'
    && castArtifactView.location === 'C3'
    && castArtifactView.region === 'surface'
    && unpoweredEltham?.attack === 2
    && unpoweredEltham.defense === 2;
  const castEventVerified: boolean = castResult.receipt.events.length === 1
    && castResult.receipt.events[0]?.type === 'artifact-conjured'
    && castPayload !== undefined
    && chosenArtifact.descriptor.kind === 'cast-artifact'
    && canonicalJson(castPayload) === canonicalJson({
      cardId: input.swordAndShield.stableId,
      casterInstanceId: chosenArtifact.descriptor.casterInstanceId,
      cell: 'C3',
      instanceId: opening.artifactInstanceId,
      manaPaid: 3,
      owner: 'north',
      region: 'surface',
      seat: 'north',
    });
  const pickupEventVerified: boolean = pickupResult.receipt.events.length === 1
    && pickupResult.receipt.events[0]?.type === 'artifacts-picked-up'
    && pickupPayload !== undefined
    && canonicalJson(pickupPayload) === canonicalJson({
      artifactInstanceIds: [opening.artifactInstanceId],
      seat: 'north',
      unitInstanceId: opening.elthamTownsfolkInstanceId,
      unitKind: 'minion',
    });
  const artifactPickedUpAndCarried: boolean = pickedArtifactState !== undefined
    && 'bearer' in pickedArtifactState
    && pickedArtifactState.bearer.kind === 'minion'
    && pickedArtifactState.bearer.instanceId === opening.elthamTownsfolkInstanceId
    && pickedArtifactState.bearer.seat === 'north'
    && pickedArtifactState.owner === 'north'
    && pickedArtifactView?.bearer?.instanceId === opening.elthamTownsfolkInstanceId
    && pickedArtifactView.controller === 'north'
    && pickedArtifactView.owner === 'north'
    && pickedArtifactView.location === 'C3'
    && pickedArtifactView.region === 'surface'
    && poweredEltham?.attack === 4
    && poweredEltham.defense === 4;
  const pickupSideEffectsAbsent: boolean = manaAfterPickup === manaBeforePickup
    && unpoweredEltham !== undefined
    && poweredEltham !== undefined
    && poweredEltham.tapped === unpoweredEltham.tapped
    && poweredEltham.stealthed === unpoweredEltham.stealthed;
  const dropChoiceVerified: boolean = dropChoices.length === 1
    && chosenDrop.descriptor.kind === 'drop-artifacts'
    && chosenDrop.descriptor.unit.kind === 'minion'
    && chosenDrop.descriptor.unit.instanceId === opening.elthamTownsfolkInstanceId
    && chosenDrop.descriptor.unit.seat === 'north'
    && chosenDrop.descriptor.artifactInstanceIds.length === 1
    && chosenDrop.descriptor.artifactInstanceIds[0] === opening.artifactInstanceId;
  const dropEventVerified: boolean = dropResult.receipt.events.length === 1
    && dropResult.receipt.events[0]?.type === 'artifacts-dropped'
    && dropPayload !== undefined
    && canonicalJson(dropPayload) === canonicalJson({
      artifactInstanceIds: [opening.artifactInstanceId],
      seat: 'north',
      unitInstanceId: opening.elthamTownsfolkInstanceId,
      unitKind: 'minion',
    });
  const dropStateVerified: boolean = droppedArtifactState !== undefined
    && !('bearer' in droppedArtifactState)
    && droppedArtifactState.location === 'C3'
    && droppedArtifactState.region === 'surface'
    && droppedArtifactState.owner === 'north'
    && droppedArtifactView !== undefined
    && droppedArtifactView.bearer === undefined
    && droppedArtifactView.controller === null
    && droppedArtifactView.location === 'C3'
    && droppedArtifactView.region === 'surface'
    && droppedArtifactView.owner === 'north'
    && poweredEltham?.attack === 4
    && poweredEltham.defense === 4
    && droppedEltham?.attack === 2
    && droppedEltham.defense === 2;
  const dropSideEffectsAbsent: boolean = manaBeforeDrop === dropSession.state.players.north.mana
    && poweredEltham !== undefined
    && droppedEltham !== undefined
    && droppedEltham.cardId === poweredEltham.cardId
    && droppedEltham.owner === poweredEltham.owner
    && droppedEltham.controller === poweredEltham.controller
    && droppedEltham.location === poweredEltham.location
    && droppedEltham.region === poweredEltham.region
    && droppedEltham.damage === poweredEltham.damage
    && droppedEltham.tapped === poweredEltham.tapped
    && droppedEltham.summoningSickness === poweredEltham.summoningSickness
    && droppedEltham.stealthed === poweredEltham.stealthed;
  const fightEventOrderVerified: boolean = fightStartedIndex >= 0
    && fightStartedIndex < firstAllocationIndex
    && firstAllocationIndex < firstDamageIndex
    && fightEvents.every(({ type }) => type !== 'artifact-dropped');
  const elthamDealtFour: boolean = allocations.some((payload) => payload.amount === 4
    && payload.strikerInstanceId === opening.elthamTownsfolkInstanceId
    && payload.targetInstanceId === opening.boskTrollInstanceId);
  const damageApplied: boolean = damages.some((payload) => payload.amount === 4
    && payload.instanceId === opening.boskTrollInstanceId)
    && damages.some((payload) => payload.amount === 3
      && payload.instanceId === opening.elthamTownsfolkInstanceId);
  const deathVerified: boolean = deaths.length === 1
    && deaths.some((payload) => payload.cardId === input.firstStrikeTargetMinion.stableId
      && payload.instanceId === opening.boskTrollInstanceId
      && payload.owner === 'south');
  const combatDamageAndSurvivalVerified: boolean = allocations.length === 1
    && elthamDealtFour
    && damageApplied
    && deathVerified
    && finalEltham?.cardId === input.elthamTownsfolk.stableId
    && finalEltham.controller === 'north'
    && finalEltham.owner === 'north'
    && finalEltham.location === 'C2'
    && finalEltham.region === 'surface'
    && finalEltham.damage === 3
    && elthamAbsentFromCemetery
    && session.state.realm.units.every(({ instanceId }) =>
      instanceId !== opening.boskTrollInstanceId)
    && boskInCemetery;
  const exactPickupChoice: boolean = pickupChoices.length === 1
    && chosenPickup.descriptor.kind === 'pick-up-artifacts'
    && chosenPickup.descriptor.unit.kind === 'minion'
    && chosenPickup.descriptor.unit.instanceId === opening.elthamTownsfolkInstanceId
    && chosenPickup.descriptor.unit.seat === 'north'
    && chosenPickup.descriptor.artifactInstanceIds.length === 1
    && chosenPickup.descriptor.artifactInstanceIds[0] === opening.artifactInstanceId;
  const noRandomDraws: boolean = castResult.receipt.randomDraws.length === 0
    && pickupResult.receipt.randomDraws.length === 0
    && dropResult.receipt.randomDraws.length === 0
    && moveResult.receipt.randomDraws.length === 0
    && fightResult.receipt.randomDraws.length === 0;
  const swordFollowedBearer: boolean = movedArtifact?.bearer?.instanceId
    === opening.elthamTownsfolkInstanceId
    && movedArtifact.controller === 'north'
    && movedArtifact.location === 'C2'
    && movedArtifact.region === 'surface'
    && movedEltham?.location === 'C2'
    && movedEltham.region === 'surface';
  const swordStayedOutOfCemetery: boolean = artifactAbsentFromCemeteries
    && session.state.players.north.hand.spellbook.every(({ instanceId }) =>
      instanceId !== opening.artifactInstanceId)
    && finalArtifactState !== undefined;
  const swordRemainedCarried: boolean = finalArtifactState !== undefined
    && 'bearer' in finalArtifactState
    && finalArtifactState.bearer.kind === 'minion'
    && finalArtifactState.bearer.instanceId === opening.elthamTownsfolkInstanceId
    && finalArtifactView?.bearer?.instanceId === opening.elthamTownsfolkInstanceId
    && finalArtifactView.controller === 'north'
    && finalArtifactView.location === 'C2'
    && finalArtifactView.region === 'surface';
  const dropUnavailableAfterInteraction: boolean = legalGameActions(session.state, 'north')
    .every(({ descriptor }) => descriptor.kind !== 'drop-artifacts'
      || descriptor.unit.kind !== 'minion'
      || descriptor.unit.instanceId !== opening.elthamTownsfolkInstanceId);
  const avatarsAfterCombat = {
    north: {
      life: session.state.players.north.avatar.life,
      location: session.state.players.north.avatar.location,
      region: session.state.players.north.avatar.region,
    },
    south: {
      life: session.state.players.south.avatar.life,
      location: session.state.players.south.avatar.location,
      region: session.state.players.south.avatar.region,
    },
  };
  const unrelatedStatePreserved: boolean = canonicalJson(
    session.state.realm.sites as unknown as JsonValue,
  ) === sitesBeforeCombat
    && canonicalJson(avatarsAfterCombat as unknown as JsonValue)
      === canonicalJson(avatarsBeforeCombat as unknown as JsonValue);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    artifactCastUncarried,
    artifactPickedUpAndCarried,
    boskTroll: input.firstStrikeTargetMinion.name,
    causalEventsVerified: castEventVerified
      && pickupEventVerified
      && dropEventVerified
      && fightEventOrderVerified,
    combatDamageAndSurvivalVerified,
    deck: deckList(opening.manifest.decks.north, opening.names),
    dropAcceptedActionCount: dropSession.transcript.length,
    dropChoiceVerified,
    dropDeathAcceptedActionCount: dropDeathSession.transcript.length,
    dropDeathReplayVerified: verifyGameReplay(dropDeathSession),
    dropDeathVerified,
    dropEventVerified,
    dropNoRandomDraws: dropResult.receipt.randomDraws.length === 0,
    dropReplayVerified: verifyGameReplay(dropSession),
    dropSecondUseUnavailable: secondDropUnavailable,
    dropSideEffectsAbsent,
    dropStateVerified,
    dropUnavailableAfterInteraction,
    elthamTownsfolk: input.elthamTownsfolk.name,
    exactPickupChoice,
    gameRemainedActive: session.state.terminal.status === 'active',
    manaPaid: manaBefore - session.state.players.north.mana,
    noRandomDraws,
    pickupSideEffectsAbsent,
    replayVerified: verifyGameReplay(session),
    seed: opening.manifest.seed,
    swordAndShield: input.swordAndShield.name,
    swordFollowedBearer,
    swordRemainedCarried,
    swordStayedOutOfCemetery,
    unrelatedStatePreserved,
  });
}

function runEarthPoisonousDagger(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthPoisonousDagger'] {
  const opening = findEarthArtifactOpening(
    input,
    input.poisonousDagger,
    'earth-poisonous-dagger',
  );
  let session = runEarthArtifactSetup(opening, false);
  const manaBefore = session.state.players.north.mana;
  const bearerChoices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-artifact'
      && descriptor.cardInstanceId === opening.artifactInstanceId
      && descriptor.bearer?.kind === 'minion'
      && descriptor.bearer.instanceId === opening.elthamTownsfolkInstanceId);
  const chosenArtifact = bearerChoices[0];
  if (!chosenArtifact) throw new Error('private Poisonous Dagger bearer cast is unavailable');
  const castResult = stepGame(session, chosenArtifact);
  if (!castResult.accepted) throw new Error('private Poisonous Dagger cast was rejected');
  session = castResult.session;

  const castArtifactState = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const castArtifactView = observeGame(session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === opening.artifactInstanceId);
  const moveResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.elthamTownsfolkInstanceId
      && descriptor.from.cell === 'C3'
      && descriptor.to.cell === 'C2'));
  if (!moveResult.accepted) throw new Error('private Poisonous Dagger bearer move was rejected');
  session = moveResult.session;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === opening.boskTrollInstanceId));
  const fightResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  if (!fightResult.accepted) throw new Error('private Poisonous Dagger fight was rejected');
  session = fightResult.session;

  const castPayload = castResult.receipt.events[0]
    && isJsonRecord(castResult.receipt.events[0].payload)
    ? castResult.receipt.events[0].payload
    : undefined;
  const events = fightResult.receipt.events;
  const allocations = events.filter(({ type }) => type === 'strike-damage-allocated')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const damages = events.filter(({ type }) => type === 'damage-dealt')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const deaths = events.filter(({ type }) => type === 'minion-died')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const dropIndex = events.findIndex(({ type }) => type === 'artifact-dropped');
  const dropEvent = events[dropIndex];
  const dropPayload = dropEvent && isJsonRecord(dropEvent.payload) ? dropEvent.payload : undefined;
  const bearerDeathIndex = events.findIndex(({ payload, type }) =>
    type === 'minion-died'
      && isJsonRecord(payload)
      && payload.instanceId === opening.elthamTownsfolkInstanceId);
  const fightStartedIndex = events.findIndex(({ type }) => type === 'fight-started');
  const attackerAllocationIndex = events.findIndex(({ payload, type }) =>
    type === 'strike-damage-allocated'
      && isJsonRecord(payload)
      && payload.strikerInstanceId === opening.elthamTownsfolkInstanceId);
  const firstDamageIndex = events.findIndex(({ type }) => type === 'damage-dealt');
  const finalArtifactState = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const finalArtifactView = observeGame(session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === opening.artifactInstanceId);
  const castVerified: boolean = castResult.receipt.events.length === 1
    && castResult.receipt.events[0]?.type === 'artifact-conjured'
    && castPayload?.cardId === input.poisonousDagger.stableId
    && castPayload.instanceId === opening.artifactInstanceId
    && castPayload.manaPaid === 2
    && castPayload.owner === 'north'
    && castPayload.seat === 'north'
    && castPayload.bearerInstanceId === opening.elthamTownsfolkInstanceId
    && castPayload.bearerKind === 'minion'
    && castPayload.bearerSeat === 'north';
  const artifactCastAndCarried: boolean = castArtifactState !== undefined
    && 'bearer' in castArtifactState
    && castArtifactState.bearer.kind === 'minion'
    && castArtifactState.bearer.instanceId === opening.elthamTownsfolkInstanceId
    && castArtifactView?.bearer?.instanceId === opening.elthamTownsfolkInstanceId
    && castArtifactView.controller === 'north'
    && castArtifactView.location === 'C3'
    && castArtifactView.region === 'surface';
  const combatLethalVerified: boolean = allocations.some((payload) => payload.amount === 2
    && payload.strikerInstanceId === opening.elthamTownsfolkInstanceId
    && payload.targetInstanceId === opening.boskTrollInstanceId)
    && damages.some((payload) => payload.amount === 2
      && payload.instanceId === opening.boskTrollInstanceId)
    && damages.some((payload) => payload.amount === 3
      && payload.instanceId === opening.elthamTownsfolkInstanceId)
    && deaths.length === 2
    && deaths.some((payload) => payload.cardId === input.firstStrikeTargetMinion.stableId
      && payload.instanceId === opening.boskTrollInstanceId
      && payload.owner === 'south')
    && deaths.some((payload) => payload.cardId === input.elthamTownsfolk.stableId
      && payload.instanceId === opening.elthamTownsfolkInstanceId
      && payload.owner === 'north');
  const daggerDroppedUncontrolled: boolean = dropPayload?.bearerInstanceId
    === opening.elthamTownsfolkInstanceId
    && dropPayload.cardId === input.poisonousDagger.stableId
    && dropPayload.cell === 'C2'
    && dropPayload.instanceId === opening.artifactInstanceId
    && dropPayload.owner === 'north'
    && dropPayload.region === 'surface'
    && finalArtifactState !== undefined
    && !('bearer' in finalArtifactState)
    && finalArtifactState.location === 'C2'
    && finalArtifactState.region === 'surface'
    && finalArtifactView !== undefined
    && finalArtifactView.bearer === undefined
    && finalArtifactView.controller === null
    && finalArtifactView.location === 'C2'
    && finalArtifactView.region === 'surface';
  const stateAndCemeteriesVerified: boolean = session.state.realm.units.every(({ instanceId }) =>
    instanceId !== opening.elthamTownsfolkInstanceId
      && instanceId !== opening.boskTrollInstanceId)
    && session.state.players.north.cemetery.length === 1
    && session.state.players.north.cemetery[0]?.cardId === input.elthamTownsfolk.stableId
    && session.state.players.north.cemetery[0].instanceId === opening.elthamTownsfolkInstanceId
    && session.state.players.south.cemetery.length === 1
    && session.state.players.south.cemetery[0]?.cardId === input.firstStrikeTargetMinion.stableId
    && session.state.players.south.cemetery[0].instanceId === opening.boskTrollInstanceId
    && (['north', 'south'] as const).every((seat) =>
      session.state.players[seat].cemetery.every(({ instanceId }) =>
        instanceId !== opening.artifactInstanceId))
    && session.state.players.north.hand.spellbook.every(({ instanceId }) =>
      instanceId !== opening.artifactInstanceId);
  const causalEventsVerified: boolean = castVerified
    && fightStartedIndex >= 0
    && fightStartedIndex < attackerAllocationIndex
    && attackerAllocationIndex < firstDamageIndex
    && dropIndex >= 0
    && dropIndex < bearerDeathIndex;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    artifactCastAndCarried,
    boskTroll: input.firstStrikeTargetMinion.name,
    causalEventsVerified,
    combatLethalVerified,
    daggerDroppedUncontrolled,
    deck: deckList(opening.manifest.decks.north, opening.names),
    elthamTownsfolk: input.elthamTownsfolk.name,
    exactBearerChoice: bearerChoices.length === 1,
    gameRemainedActive: session.state.terminal.status === 'active',
    manaPaid: manaBefore - session.state.players.north.mana,
    noRandomDraws: castResult.receipt.randomDraws.length === 0
      && moveResult.receipt.randomDraws.length === 0
      && fightResult.receipt.randomDraws.length === 0,
    poisonousDagger: input.poisonousDagger.name,
    replayVerified: verifyGameReplay(session),
    stateAndCemeteriesVerified,
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

function advanceToNorthSiteRecovery(
  checkpoint: GameSession,
  valleyCardId: string,
  recoverySiteInstanceId: string,
  rubbleC3InstanceId: string | undefined,
): GameSession {
  let session = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  const player = session.state.players.north;
  if (session.state.activeSeat !== 'north'
    || session.state.decisionSeat !== 'north'
    || session.state.phase !== 'main'
    || player.avatar.tapped
    || player.mana !== 0) {
    throw new Error('private zero-domain recovery did not reach north ready Main');
  }
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'play-site');
  const exactCells = choices.flatMap(({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardInstanceId === recoverySiteInstanceId
      ? [descriptor.cell]
      : []);
  if (exactCells.length !== 1 || exactCells[0] !== 'C4') {
    throw new Error('private zero-domain recovery did not expose only Avatar-local C4');
  }
  const result = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === recoverySiteInstanceId
      && descriptor.cell === 'C4'));
  if (!result.accepted) {
    throw new Error(`private zero-domain Valley recovery rejected: ${result.reason.code}`);
  }
  session = result.session;
  const site = session.state.realm.sites.C4;
  const realSite = site && !('rubble' in site) ? site : undefined;
  const affinity = observeGame(session.state, 'north').players.north.affinity;
  if (result.receipt.events.map(({ type }) => type).join(',') !== 'rubble-replaced,site-played'
    || result.receipt.randomDraws.length !== 0) {
    throw new Error('private zero-domain recovery emitted unexpected events or randomness');
  }
  if (realSite?.cardId !== valleyCardId
    || realSite.instanceId !== recoverySiteInstanceId
    || realSite.owner !== 'north'
    || realSite.controller !== 'north') {
    throw new Error('private zero-domain recovery did not establish the real owned Valley');
  }
  if (session.state.players.north.avatar.location !== 'C4'
    || session.state.players.north.avatar.region !== 'surface'
    || !session.state.players.north.avatar.tapped
    || session.state.players.north.mana !== 1
    || affinity.earth !== 1) {
    throw new Error('private zero-domain recovery did not restore Avatar-local mana and E1');
  }
  if (session.state.players.north.hand.atlas
    .some(({ instanceId }) => instanceId === recoverySiteInstanceId)
    || session.state.realm.sites.C3?.instanceId !== rubbleC3InstanceId) {
    throw new Error('private zero-domain recovery did not preserve its hand/Rubble transition');
  }
  return session;
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
  const destructionAcceptedActionCount = session.transcript.length;
  const avatarRemainedOnSurface = session.state.players.north.avatar.location === 'C4'
    && session.state.players.north.avatar.region === 'surface';
  const causalEventsVerified = events.map(({ type }) => type).join(',')
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
    && rubbleC4Payload.sourceInstanceId === opening.sinkholeInstanceId;
  const noAffinityOrControlContribution = affinity.air === 0
    && affinity.earth === 0
    && affinity.fire === 0
    && affinity.water === 0
    && Object.values(session.state.realm.sites)
      .every(({ controller }) => controller !== 'north');
  const sourceAndTargetEnteredCemetery = session.state.players.north.cemetery
    .some(({ instanceId }) => instanceId === opening.sinkholeInstanceId)
    && session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.targetSiteInstanceId);
  const twoNeutralRubbleSites = rubbleC3 !== undefined
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
    && rubbleC4.instanceId !== opening.sinkholeInstanceId;

  const recoveredSession = advanceToNorthSiteRecovery(
    session,
    input.valley.stableId,
    opening.recoverySiteInstanceId,
    rubbleC3?.instanceId,
  );

  return Object.freeze({
    acceptedActionCount: recoveredSession.transcript.length,
    avatarRemainedOnSurface,
    causalEventsVerified,
    deck: deckList(opening.manifest.decks.north, opening.names),
    destructionAcceptedActionCount,
    exactActivationAvailable,
    noAffinityOrControlContribution,
    noRandomDraws: recoveredSession.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    recoveryVerified: true,
    replayVerified: verifyGameReplay(recoveredSession),
    seed: opening.seed,
    sinkhole: input.sinkhole.name,
    sourceAndTargetEnteredCemetery,
    twoNeutralRubbleSites,
    valley: input.valley.name,
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

function runEarthGrainSparrow(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthGrainSparrow'] {
  const opening = findEarthGrainSparrowOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.steppeInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'C3');

  const lifeBeforeDemon = session.state.players.north.avatar.life;
  const demonResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.demonInstanceId
      && descriptor.cell === 'C3'
      && descriptor.region === undefined));
  if (!demonResult.accepted) throw new Error('private Grain Sparrow setup life-loss summon was rejected');
  session = demonResult.session;
  const lifeAfterDemon = session.state.players.north.avatar.life;
  const demonEvents = demonResult.receipt.events;
  const demonSummonedPayload = demonEvents[0] && isJsonRecord(demonEvents[0].payload)
    ? demonEvents[0].payload
    : undefined;
  const lifeLostPayload = demonEvents[1] && isJsonRecord(demonEvents[1].payload)
    ? demonEvents[1].payload
    : undefined;

  const northBefore = session.state.players.north;
  const southBefore = session.state.players.south;
  const sitesBefore = session.state.realm.sites;
  const unitsBefore = session.state.realm.units;
  const avatarDefinition = session.state.cards[northBefore.avatar.card.cardId];
  if (avatarDefinition?.cardType !== 'avatar') {
    throw new Error('private Grain Sparrow scenario Avatar is unsupported');
  }
  const grainResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.grainSparrowInstanceId
      && descriptor.cell === 'C3'
      && descriptor.region === undefined));
  if (!grainResult.accepted) throw new Error('private Grain Sparrow summon was rejected');
  session = grainResult.session;
  const northAfter = session.state.players.north;
  const grain = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.grainSparrowInstanceId);
  const grainEvents = grainResult.receipt.events;
  const grainSummonedPayload = grainEvents[0] && isJsonRecord(grainEvents[0].payload)
    ? grainEvents[0].payload
    : undefined;
  const healedPayload = grainEvents[1] && isJsonRecord(grainEvents[1].payload)
    ? grainEvents[1].payload
    : undefined;
  const actualLifeGained = northAfter.avatar.life - northBefore.avatar.life;
  const unrelatedEvents = [...demonEvents, ...grainEvents];

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    actualLifeGained,
    causalEventsVerified: demonEvents.map(({ type }) => type).join(',')
      === 'minion-summoned,avatar-life-lost'
      && demonSummonedPayload?.instanceId === opening.demonInstanceId
      && demonSummonedPayload.manaPaid === 2
      && demonSummonedPayload.seat === 'north'
      && lifeLostPayload?.amount === 2
      && lifeLostPayload.life === 18
      && lifeLostPayload.seat === 'north'
      && lifeLostPayload.sourceInstanceId === opening.demonInstanceId
      && grainEvents.map(({ type }) => type).join(',') === 'minion-summoned,avatar-healed'
      && grainSummonedPayload?.instanceId === opening.grainSparrowInstanceId
      && grainSummonedPayload.cell === 'C3'
      && grainSummonedPayload.manaPaid === 1
      && grainSummonedPayload.seat === 'north'
      && healedPayload?.amount === 2
      && healedPayload.attemptedAmount === 2
      && healedPayload.life === 20
      && healedPayload.seat === 'north'
      && healedPayload.sourceInstanceId === opening.grainSparrowInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    grainSparrow: input.grainSparrow.name,
    lesserBloodDemon: input.lesserBloodDemon.name,
    lifeCappedAtMaximum: northAfter.avatar.life === avatarDefinition.life,
    lifeLostBeforeSummon: lifeBeforeDemon === avatarDefinition.life
      && lifeAfterDemon === avatarDefinition.life - 2
      && northBefore.avatar.life === lifeAfterDemon,
    noDamageDeathTerminalOrRandomEffects: unrelatedEvents.every(({ type }) =>
      type !== 'damage-dealt'
        && type !== 'minion-died'
        && type !== 'death-blow'
        && type !== 'avatar-reached-deaths-door'
        && type !== 'game-ended')
      && demonResult.receipt.randomDraws.length === 0
      && grainResult.receipt.randomDraws.length === 0
      && session.state.terminal.status === 'active',
    otherStatePreserved:
      canonicalJson(session.state.players.south as unknown as JsonValue)
        === canonicalJson(southBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.sites as unknown as JsonValue)
        === canonicalJson(sitesBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.units
        .filter(({ instanceId }) => instanceId !== opening.grainSparrowInstanceId) as unknown as JsonValue)
        === canonicalJson(unitsBefore as unknown as JsonValue)
      && canonicalJson(northAfter.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.spellbook as unknown as JsonValue)
      && canonicalJson(northAfter.hand.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.hand.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.hand.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.hand.spellbook
          .filter(({ instanceId }) => instanceId !== opening.grainSparrowInstanceId) as unknown as JsonValue)
      && canonicalJson(northAfter.cemetery as unknown as JsonValue)
        === canonicalJson(northBefore.cemetery as unknown as JsonValue)
      && northAfter.avatar.card.instanceId === northBefore.avatar.card.instanceId
      && northAfter.avatar.location === northBefore.avatar.location
      && northAfter.avatar.region === northBefore.avatar.region
      && northAfter.avatar.tapped === northBefore.avatar.tapped
      && northAfter.mana === northBefore.mana - 1
      && session.state.phase === 'main',
    replayVerified: verifyGameReplay(session),
    steppe: input.steppe.name,
    summonedAtC3: grain?.cardId === input.grainSparrow.stableId
      && grain.controller === 'north'
      && grain.damage === 0
      && grain.location === 'C3'
      && grain.owner === 'north'
      && grain.region === 'surface'
      && grain.summoningSickness
      && !grain.tapped,
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

function runEarthMalakhimSetup(
  opening: ReturnType<typeof findEarthMalakhimOpening>,
): GameSession {
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.siteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.siteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.siteInstanceIds[2]
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.siteInstanceIds[3]
    && descriptor.cell === 'A3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.siteInstanceIds[4]
    && descriptor.cell === 'A2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.malakhimInstanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  return session;
}

function runEarthMalakhim(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthMalakhim'] {
  const opening = findEarthMalakhimOpening(input);
  let session = runEarthMalakhimSetup(opening);
  const definition = session.state.cards[input.malakhim.stableId];
  const before = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.malakhimInstanceId);
  if (!before) throw new Error('private Malakhim setup did not summon Malakhim');
  const summonEvent = session.transcript.flatMap(({ events }) => events)
    .find(({ payload, type }) => type === 'minion-summoned'
      && isJsonRecord(payload)
      && payload.instanceId === opening.malakhimInstanceId);
  const summonPayload = summonEvent && isJsonRecord(summonEvent.payload)
    ? summonEvent.payload
    : undefined;
  const affinity = observeGame(session.state, 'north').players.north.affinity.earth;
  const move = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.malakhimInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,B3'
      && descriptor.path.every(({ region }) => region === 'surface')));
  if (!move.accepted) throw new Error('private Malakhim normal action was rejected');
  session = move.session;
  const moved = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.malakhimInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  const ended = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  if (!ended.accepted) throw new Error('private Malakhim end phase was rejected');
  session = ended.session;
  const after = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.malakhimInstanceId);
  const events = ended.receipt.events;
  const untapPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const untapIndex = events.findIndex(({ type }) => type === 'minion-untapped');
  const turnEndedIndex = events.findIndex(({ type }) => type === 'turn-ended');
  const airborneAndWard = definition?.cardType === 'minion'
    && definition.airborne === true
    && definition.ward === true
    && before.warded;
  const normalActionTapped = moved?.tapped === true
    && moved.location === 'B3'
    && moved.region === 'surface';
  const endPhaseUntapped = moved?.tapped === true && after?.tapped === false;
  const opponentTurnReady = session.state.decisionSeat === 'south'
    && session.state.phase === 'draw'
    && after?.controller === 'north'
    && after.owner === 'north'
    && !after.summoningSickness
    && !after.tapped;
  const causalEventsVerified = events.map(({ type }) => type).join(',')
    === 'minion-untapped,turn-ended,turn-started'
    && untapPayload?.instanceId === opening.malakhimInstanceId
    && untapPayload.seat === 'north'
    && untapPayload.sourceInstanceId === opening.malakhimInstanceId
    && untapIndex >= 0
    && untapIndex < turnEndedIndex;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    airborneAndWard,
    causalEventsVerified,
    deck: deckList(opening.manifest.decks.north, opening.names),
    earthAffinityThree: affinity === 3,
    endPhaseUntapped,
    malakhim: input.malakhim.name,
    manaPaid: typeof summonPayload?.manaPaid === 'number' ? summonPayload.manaPaid : -1,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    normalActionTapped,
    opponentTurnReady,
    replayVerified: verifyGameReplay(session),
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

function runAirFireFatalitySetup(
  opening: ReturnType<typeof findAirFireFatalityOpening>,
): GameSession {
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
    && descriptor.cardInstanceId === opening.southSpireInstanceId
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
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[3]
    && descriptor.cell === 'A3');
  return session;
}

function runAirFireFatality(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airFireFatality'] {
  const opening = findAirFireFatalityOpening(input);
  let session = runAirFireFatalitySetup(opening);
  const targetBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.snowLeopardInstanceId);
  if (!targetBefore) throw new Error('private Fatality setup lacks its healthy Snow Leopard');
  const healthyChoices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.fatalityInstanceId);
  const affinity = observeGame(session.state, 'north').players.north.affinity;
  const manaBeforeZap = session.state.players.north.mana;
  const zap = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.zapInstanceId
      && descriptor.target?.kind === 'minion'
      && descriptor.target.instanceId === opening.snowLeopardInstanceId
      && descriptor.target.seat === 'south'));
  if (!zap.accepted) throw new Error('private Fatality setup Zap was rejected');
  session = zap.session;
  const targetAfterZap = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.snowLeopardInstanceId);
  if (!targetAfterZap) throw new Error('private Fatality setup Zap killed its target');
  const fatalityChoices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.fatalityInstanceId);
  const chosen = fatalityChoices.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'minion'
    && descriptor.target.instanceId === opening.snowLeopardInstanceId
    && descriptor.target.seat === 'south');
  if (!chosen || fatalityChoices.length !== 1) {
    throw new Error('private Fatality wounded target is not exactly available');
  }
  const manaBeforeFatality = session.state.players.north.mana;
  const killed = stepGame(session, chosen);
  if (!killed.accepted) throw new Error('private Fatality cast was rejected');
  session = killed.session;

  const zapEvents = zap.receipt.events;
  const zapCastPayload = zapEvents[0] && isJsonRecord(zapEvents[0].payload)
    ? zapEvents[0].payload
    : undefined;
  const fatalityEvents = killed.receipt.events;
  const castPayload = fatalityEvents[0] && isJsonRecord(fatalityEvents[0].payload)
    ? fatalityEvents[0].payload
    : undefined;
  const killedPayload = fatalityEvents[1] && isJsonRecord(fatalityEvents[1].payload)
    ? fatalityEvents[1].payload
    : undefined;
  const deathPayload = fatalityEvents[2] && isJsonRecord(fatalityEvents[2].payload)
    ? fatalityEvents[2].payload
    : undefined;
  const resolvedPayload = fatalityEvents[3] && isJsonRecord(fatalityEvents[3].payload)
    ? fatalityEvents[3].payload
    : undefined;
  const causalEventsVerified = zapEvents.map(({ type }) => type).join(',')
    === 'magic-cast,magic-damage-allocated,damage-dealt,magic-resolved'
    && zapCastPayload?.instanceId === opening.zapInstanceId
    && zapCastPayload.manaPaid === 1
    && zapCastPayload.seat === 'north'
    && zapCastPayload.targetInstanceId === opening.snowLeopardInstanceId
    && zapCastPayload.targetSeat === 'south'
    && fatalityEvents.map(({ type }) => type).join(',')
      === 'magic-cast,minion-killed,minion-died,magic-resolved'
    && castPayload?.instanceId === opening.fatalityInstanceId
    && castPayload.manaPaid === 3
    && castPayload.seat === 'north'
    && castPayload.targetInstanceId === opening.snowLeopardInstanceId
    && castPayload.targetSeat === 'south'
    && killedPayload?.cardId === input.stealthTargetMinion.stableId
    && killedPayload.instanceId === opening.snowLeopardInstanceId
    && killedPayload.owner === 'south'
    && killedPayload.seat === 'south'
    && killedPayload.sourceInstanceId === opening.fatalityInstanceId
    && deathPayload?.cardId === input.stealthTargetMinion.stableId
    && deathPayload.instanceId === opening.snowLeopardInstanceId
    && deathPayload.owner === 'south'
    && resolvedPayload?.instanceId === opening.fatalityInstanceId;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    airFireAffinity: affinity.air >= 1 && affinity.fire >= 1,
    causalEventsVerified,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactWoundedTarget: targetAfterZap.damage === 1 && fatalityChoices.length === 1,
    fatality: input.fatality.name,
    fatalityDealtNoDamage: fatalityEvents.every(({ type }) =>
      type !== 'magic-damage-allocated' && type !== 'damage-dealt'),
    fatalityEnteredCemetery: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.fatalityInstanceId),
    healthyTargetUnavailable: targetBefore.damage === 0 && healthyChoices.length === 0,
    manaPaid: manaBeforeFatality - session.state.players.north.mana,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    replayVerified: verifyGameReplay(session),
    snowLeopard: input.stealthTargetMinion.name,
    targetEnteredOwnerCemetery: session.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === opening.snowLeopardInstanceId),
    targetLeftRealm: session.state.realm.units
      .every(({ instanceId }) => instanceId !== opening.snowLeopardInstanceId),
    zap: input.zap.name,
    zapDamageExactlyOne: targetAfterZap.damage - targetBefore.damage === 1,
    zapEnteredCemetery: session.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === opening.zapInstanceId),
    zapManaPaid: manaBeforeZap - zap.session.state.players.north.mana,
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

function runAirBladderblimp(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airBladderblimp'] {
  const opening = findAirBladderblimpOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northAirSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northAirSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northAirSiteInstanceIds[2]
    && descriptor.cell === 'B4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownSiteInstanceId
    && descriptor.cell === 'B3');

  const manaBeforeSummon = session.state.players.north.mana;
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.bladderblimpInstanceId
    && descriptor.cell === 'C3');
  const summonManaPaid = manaBeforeSummon - session.state.players.north.mana;
  const blimpDefinition = session.state.cards[input.bladderblimp.stableId];
  const blimp = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.bladderblimpInstanceId);
  const airborneAtC3 = blimpDefinition?.cardType === 'minion'
    && blimpDefinition.airborne === true
    && blimpDefinition.deathriteLoseLifePerNearbySiteControlled === 1
    && blimp?.location === 'C3'
    && blimp.region === 'surface'
    && blimp.owner === 'north'
    && blimp.controller === 'north';

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const nearbyCells = new Set(['B2', 'B3', 'B4', 'C2', 'C3', 'C4', 'D2', 'D3', 'D4']);
  const nearbySiteCounts = (['north', 'south'] as const).map((seat) =>
    Object.entries(session.state.realm.sites).filter(([cell, site]) =>
      nearbyCells.has(cell) && site.controller === seat).length);
  const exactNearbySiteCounts = nearbySiteCounts[0] === 4 && nearbySiteCounts[1] === 1;
  const lifeBefore = {
    north: session.state.players.north.avatar.life,
    south: session.state.players.south.avatar.life,
  };
  const manaBeforeMagic = session.state.players.north.mana;
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.lightningBoltInstanceId
      && descriptor.targetLocation?.cell === 'C3'
      && descriptor.targetLocation.region === 'surface');
  if (choices.length !== 1) throw new Error('private Bladderblimp Lightning Bolt location is not unique');
  const result = stepGame(session, choices[0]!);
  if (!result.accepted) throw new Error('private Bladderblimp Lightning Bolt cast was rejected');
  session = result.session;

  const events = result.receipt.events;
  const eventTypes = events.map(({ type }) => type);
  const lifeEvents = events.filter(({ type }) => type === 'avatar-life-lost');
  const northLifePayload = lifeEvents[0] && isJsonRecord(lifeEvents[0].payload)
    ? lifeEvents[0].payload
    : undefined;
  const southLifePayload = lifeEvents[1] && isJsonRecord(lifeEvents[1].payload)
    ? lifeEvents[1].payload
    : undefined;
  const deathEvent = events.find(({ type }) => type === 'minion-died');
  const deathPayload = deathEvent && isJsonRecord(deathEvent.payload) ? deathEvent.payload : undefined;
  const damageEvents = events.filter(({ type }) => type === 'damage-dealt');
  const damagePayload = damageEvents[0] && isJsonRecord(damageEvents[0].payload)
    ? damageEvents[0].payload
    : undefined;
  const random = result.receipt.randomDraws[0];
  const randomDomain = random && isJsonRecord(random.domain) ? random.domain : undefined;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    airborneAtC3,
    bladderblimp: input.bladderblimp.name,
    causalEventsVerified: eventTypes.join(',')
      === 'magic-cast,magic-damage-allocated,damage-dealt,avatar-life-lost,avatar-life-lost,minion-died,magic-resolved'
      && northLifePayload?.amount === 4
      && northLifePayload.life === 16
      && northLifePayload.seat === 'north'
      && northLifePayload.sourceInstanceId === opening.bladderblimpInstanceId
      && southLifePayload?.amount === 1
      && southLifePayload.life === 19
      && southLifePayload.seat === 'south'
      && southLifePayload.sourceInstanceId === opening.bladderblimpInstanceId
      && deathPayload?.cardId === input.bladderblimp.stableId
      && deathPayload.instanceId === opening.bladderblimpInstanceId
      && deathPayload.owner === 'north',
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactNearbySiteCounts,
    gameRemainedActive: session.state.terminal.status === 'active'
      && events.every(({ type }) =>
        type !== 'avatar-reached-deaths-door' && type !== 'death-blow' && type !== 'game-ended'),
    lifeLossOnly: session.state.players.north.avatar.life === lifeBefore.north - 4
      && session.state.players.south.avatar.life === lifeBefore.south - 1
      && damageEvents.length === 1
      && damagePayload?.amount === 3
      && damagePayload.instanceId === opening.bladderblimpInstanceId,
    lightningBolt: input.lightningBolt.name,
    magicManaPaid: manaBeforeMagic - session.state.players.north.mana,
    minionAndMagicEnteredCemetery: session.state.realm.units
      .every(({ instanceId }) => instanceId !== opening.bladderblimpInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.bladderblimpInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.lightningBoltInstanceId)
      && session.state.players.north.hand.spellbook
        .every(({ instanceId }) => instanceId !== opening.lightningBoltInstanceId),
    randomSelectionRecorded: result.receipt.randomDraws.length === 1
      && random?.purpose === 'magic_random_unit_at_location'
      && randomDomain?.accepted === true
      && randomDomain.exclusiveMaximum === 1
      && randomDomain.kind === 'unit_index_candidate',
    replayVerified: verifyGameReplay(session),
    summonManaPaid,
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
    && descriptor.cardInstanceId === opening.northSpireInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSpireInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.southSnowLeopardInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northStreamInstanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.shellycoatInstanceId
    && descriptor.cell === 'C3'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const shellycoatBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.shellycoatInstanceId);
  const snowLeopardBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southSnowLeopardInstanceId);
  if (!shellycoatBefore || !snowLeopardBefore) {
    throw new Error('private Rain of Arrows setup lacks both surface comparison minions');
  }
  const avatarsBefore = canonicalJson({
    north: observeGame(session.state, 'north').players.north.avatar,
    south: observeGame(session.state, 'north').players.south.avatar,
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
  const castResult = stepGame(session, selected);
  if (!castResult.accepted) throw new Error('private Rain of Arrows was rejected');
  session = castResult.session;

  const shellycoatAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.shellycoatInstanceId);
  const snowLeopardAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southSnowLeopardInstanceId);
  const events = castResult.receipt.events;
  const allocations = events.filter(({ type }) => type === 'magic-damage-allocated');
  const damages = events.filter(({ type }) => type === 'damage-dealt');
  const shellycoatDamage = damages.find(({ payload }) => isJsonRecord(payload)
    && payload.instanceId === opening.shellycoatInstanceId);
  const snowLeopardDamage = damages.find(({ payload }) => isJsonRecord(payload)
    && payload.instanceId === opening.southSnowLeopardInstanceId);
  const shellycoatDamagePayload = shellycoatDamage && isJsonRecord(shellycoatDamage.payload)
    ? shellycoatDamage.payload
    : undefined;
  const snowLeopardDamagePayload = snowLeopardDamage && isJsonRecord(snowLeopardDamage.payload)
    ? snowLeopardDamage.payload
    : undefined;
  const castPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const resolved = events.at(-1);
  const resolvedPayload = resolved && isJsonRecord(resolved.payload) ? resolved.payload : undefined;
  const targetIds = [
    opening.shellycoatInstanceId,
    opening.southSnowLeopardInstanceId,
  ].sort();
  const definition = session.state.cards[input.shellycoat.stableId];
  const northOtherCemetery = session.state.players.north.cemetery.filter(({ instanceId }) =>
    instanceId !== opening.rainOfArrowsInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarsPreserved: canonicalJson({
      north: observeGame(session.state, 'north').players.north.avatar,
      south: observeGame(session.state, 'north').players.south.avatar,
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
      && shellycoatDamagePayload !== undefined
      && canonicalJson(shellycoatDamagePayload) === canonicalJson({
        accumulated: 0,
        amount: 0,
        attemptedAmount: 1,
        direct: true,
        instanceId: opening.shellycoatInstanceId,
        prevented: true,
        seat: 'north',
      })
      && snowLeopardDamagePayload !== undefined
      && canonicalJson(snowLeopardDamagePayload) === canonicalJson({
        accumulated: 1,
        amount: 1,
        direct: true,
        instanceId: opening.southSnowLeopardInstanceId,
        seat: 'south',
      })
      && resolved?.type === 'magic-resolved'
      && resolvedPayload?.instanceId === opening.rainOfArrowsInstanceId,
    cemeteriesOtherwisePreserved:
      canonicalJson(northOtherCemetery as unknown as JsonValue) === northCemeteryBefore
      && canonicalJson(session.state.players.south.cemetery as unknown as JsonValue)
        === southCemeteryBefore,
    damageReductionVerified: definition?.cardType === 'minion'
      && definition.submerge === true
      && definition.takesLessDamage === 1
      && shellycoatBefore.damage === 0
      && shellycoatAfter?.damage === 0
      && shellycoatAfter.cardId === input.shellycoat.stableId
      && shellycoatAfter.controller === 'north'
      && shellycoatAfter.owner === 'north'
      && shellycoatAfter.location === 'C3'
      && shellycoatAfter.region === 'surface'
      && snowLeopardBefore.damage === 0
      && snowLeopardAfter?.damage === 1
      && snowLeopardAfter.cardId === input.stealthTargetMinion.stableId
      && snowLeopardAfter.controller === 'south'
      && snowLeopardAfter.owner === 'south'
      && snowLeopardAfter.location === 'C1'
      && snowLeopardAfter.region === 'surface',
    deck: deckList(opening.manifest.decks.north, opening.names),
    gameRemainedActive: session.state.terminal.status === 'active',
    manaPaid: manaBefore - session.state.players.north.mana,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    noTargetChoice,
    rainOfArrows: input.rainOfArrows.name,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    shellycoat: input.shellycoat.name,
    sitesPreserved: canonicalJson(session.state.realm.sites as unknown as JsonValue) === sitesBefore,
    snowLeopard: input.stealthTargetMinion.name,
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.rainOfArrowsInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.rainOfArrowsInstanceId),
    surfaceMinionsComparedAndSurvived: shellycoatBefore.damage === 0
      && snowLeopardBefore.damage === 0
      && shellycoatAfter?.damage === 0
      && snowLeopardAfter?.damage === 1
      && events.every(({ type }) => type !== 'minion-died'),
  });
}

function runAirStaticServant(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airStaticServant'] {
  const opening = findAirStaticServantOpening(input);
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
    && descriptor.cell === 'C4'
    && descriptor.region === undefined);
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

  const snowBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.snowLeopardInstanceId);
  if (!snowBefore) throw new Error('private Static Servant setup lacks its Snow Leopard');
  const northBefore = session.state.players.north;
  const southBefore = session.state.players.south;
  const sitesBefore = canonicalJson(session.state.realm.sites as unknown as JsonValue);
  const northCemeteryBefore = canonicalJson(northBefore.cemetery as unknown as JsonValue);
  const southCemeteryBefore = canonicalJson(southBefore.cemetery as unknown as JsonValue);
  const avatarInstanceId = northBefore.avatar.card.instanceId;
  const summons = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.staticServantInstanceId
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
  const selected = summons[0];
  if (!selected || summons.length !== 1) {
    throw new Error('private Static Servant summon is not exactly available');
  }
  const manaBefore = northBefore.mana;
  const summoned = stepGame(session, selected);
  if (!summoned.accepted) throw new Error('private Static Servant summon was rejected');
  session = summoned.session;

  const northAfter = session.state.players.north;
  const snowAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.snowLeopardInstanceId);
  const servantAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.staticServantInstanceId);
  const events = summoned.receipt.events;
  const summonedPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const targetInstanceIds = [avatarInstanceId, opening.snowLeopardInstanceId].sort();
  const allocations = events.filter(({ type }) => type === 'genesis-damage-allocated');
  const damageEvents = events.filter(({ type }) => type === 'damage-dealt');
  const expectedResolutionTypes = targetInstanceIds.flatMap((instanceId) =>
    instanceId === avatarInstanceId
      ? ['damage-dealt', 'avatar-life-lost']
      : ['damage-dealt']);
  const avatarLifeEvent = events.find(({ type }) => type === 'avatar-life-lost');
  const avatarLifePayload = avatarLifeEvent && isJsonRecord(avatarLifeEvent.payload)
    ? avatarLifeEvent.payload
    : undefined;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    avatarAndLeopardDamaged: northBefore.avatar.life === 20
      && northAfter.avatar.life === 19
      && snowBefore.damage === 0
      && snowAfter?.damage === 1
      && snowAfter.cardId === snowBefore.cardId
      && snowAfter.controller === snowBefore.controller
      && snowAfter.location === snowBefore.location
      && snowAfter.owner === snowBefore.owner
      && snowAfter.region === snowBefore.region
      && snowAfter.tapped === snowBefore.tapped,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === [
        'minion-summoned',
        'genesis-damage-allocated',
        'genesis-damage-allocated',
        ...expectedResolutionTypes,
      ].join(',')
      && summonedPayload?.instanceId === opening.staticServantInstanceId
      && summonedPayload.manaPaid === 2
      && summonedPayload.seat === 'north'
      && allocations.map(({ payload }) => isJsonRecord(payload)
        ? String(payload.targetInstanceId)
        : '').join(',') === targetInstanceIds.join(',')
      && allocations.every(({ payload }) => isJsonRecord(payload)
        && payload.amount === 1
        && payload.sourceInstanceId === opening.staticServantInstanceId)
      && damageEvents.map(({ payload }) => isJsonRecord(payload)
        ? String(payload.instanceId)
        : '').join(',') === targetInstanceIds.join(',')
      && damageEvents.every(({ payload }) => isJsonRecord(payload) && payload.amount === 1)
      && avatarLifePayload?.amount === 1
      && avatarLifePayload.life === 19
      && avatarLifePayload.seat === 'north',
    cemeteriesUnchanged:
      canonicalJson(northAfter.cemetery as unknown as JsonValue) === northCemeteryBefore
      && canonicalJson(session.state.players.south.cemetery as unknown as JsonValue)
        === southCemeteryBefore,
    deck: deckList(opening.manifest.decks.north, opening.names),
    gameRemainedActive: session.state.terminal.status === 'active'
      && events.every(({ type }) => type !== 'minion-died' && type !== 'game-ended'),
    manaPaid: manaBefore - northAfter.mana,
    noTargetChoiceOrRandomness: summons.length === 1
      && summoned.receipt.randomDraws.length === 0,
    otherStatePreserved:
      canonicalJson(session.state.players.south as unknown as JsonValue)
        === canonicalJson(southBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.sites as unknown as JsonValue) === sitesBefore
      && canonicalJson(northAfter.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.spellbook as unknown as JsonValue)
      && canonicalJson(northAfter.hand.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.hand.atlas as unknown as JsonValue)
      && northAfter.avatar.card.instanceId === avatarInstanceId
      && northAfter.avatar.location === northBefore.avatar.location
      && northAfter.avatar.region === northBefore.avatar.region
      && northAfter.avatar.tapped === northBefore.avatar.tapped
      && session.state.phase === 'main',
    replayVerified: verifyGameReplay(session),
    snowLeopard: input.stealthTargetMinion.name,
    staticServant: input.staticServant.name,
    staticServantExcludedAndUndamaged: servantAfter?.cardId === input.staticServant.stableId
      && servantAfter.controller === 'north'
      && servantAfter.damage === 0
      && servantAfter.location === 'C4'
      && servantAfter.owner === 'north'
      && servantAfter.region === 'surface'
      && servantAfter.tapped === false
      && allocations.every(({ payload }) => isJsonRecord(payload)
        && payload.targetInstanceId !== opening.staticServantInstanceId)
      && damageEvents.every(({ payload }) => isJsonRecord(payload)
        && payload.instanceId !== opening.staticServantInstanceId),
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

function runAirVoidArtifact(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['airVoidArtifact'] {
  const opening = findAirVoidArtifactOpening(input);
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
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.stalkerInstanceId
    && descriptor.cell === 'B3'
    && descriptor.region === 'void');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B4');

  const castResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-artifact'
      && descriptor.cardInstanceId === opening.artifactInstanceId
      && descriptor.bearer?.kind === 'minion'
      && descriptor.bearer.instanceId === opening.stalkerInstanceId));
  if (!castResult.accepted) throw new Error('private void-carried Sword cast was rejected');
  session = castResult.session;
  const carried = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);

  const dropResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'drop-artifacts'
      && descriptor.unit.kind === 'minion'
      && descriptor.unit.instanceId === opening.stalkerInstanceId
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === opening.artifactInstanceId));
  if (!dropResult.accepted) throw new Error('private void-carried Sword Drop was rejected');
  session = dropResult.session;
  const dropped = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const droppedView = observeGame(session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === opening.artifactInstanceId);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  const coveredVoid = session.state.realm.sites.B3 === undefined;
  const coverResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.northSiteInstanceIds[3]
      && descriptor.cell === 'B3'));
  if (!coverResult.accepted) throw new Error('private site-over-void action was rejected');
  session = coverResult.session;
  const relocated = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.artifactInstanceId);
  const relocatedView = observeGame(session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === opening.artifactInstanceId);
  const surfacedStalker = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.stalkerInstanceId);
  const dropEvent = dropResult.receipt.events[0];
  const relocationVerified = carried !== undefined
    && 'bearer' in carried
    && carried.bearer.instanceId === opening.stalkerInstanceId
    && carried.owner === 'north'
    && dropped !== undefined
    && !('bearer' in dropped)
    && dropped.location === 'B3'
    && dropped.region === 'void'
    && dropped.owner === 'north'
    && droppedView?.controller === null
    && dropResult.receipt.events.length === 1
    && dropEvent?.type === 'artifacts-dropped'
    && coveredVoid
    && relocated !== undefined
    && !('bearer' in relocated)
    && relocated.instanceId === opening.artifactInstanceId
    && relocated.location === 'B3'
    && relocated.region === 'surface'
    && relocated.owner === 'north'
    && relocatedView?.controller === null
    && relocatedView.owner === 'north'
    && relocatedView.location === 'B3'
    && relocatedView.region === 'surface'
    && surfacedStalker?.instanceId === opening.stalkerInstanceId
    && surfacedStalker.location === 'B3'
    && surfacedStalker.region === 'surface'
    && coverResult.receipt.events.length === 1
    && coverResult.receipt.events[0]?.type === 'site-played';

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    relocationVerified,
    replayVerified: verifyGameReplay(session),
    seed: opening.seed,
    spectralStalker: input.voidwalkMinion.name,
    swordAndShield: input.swordAndShield.name,
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

function runFireVileImp(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireVileImp'] {
  const opening = findFireVileImpOpening(input);
  let checkpoint = keep(keep(opening.session));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    checkpoint = accept(checkpoint, action(checkpoint, predicate));
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

  const avatarInstanceId = checkpoint.state.players.north.avatar.card.instanceId;
  const choices = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.vileImpInstanceId
      && descriptor.cell === 'C3');
  const decline = choices.find(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.genesisDamageChoice === 'decline');
  const selfTarget = choices.find(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.genesisDamageChoice === 'target'
    && descriptor.genesisDamageTarget?.kind === 'minion'
    && descriptor.genesisDamageTarget.instanceId === opening.vileImpInstanceId);
  const target = choices.find(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.genesisDamageChoice === 'target'
    && descriptor.genesisDamageTarget?.kind === 'avatar'
    && descriptor.genesisDamageTarget.instanceId === avatarInstanceId);
  if (!decline || !selfTarget || !target) {
    throw new Error('private Vile Imp Genesis choices are unavailable');
  }
  const declined = stepGame(checkpoint, decline);
  const targeted = stepGame(checkpoint, target);
  if (!declined.accepted || !targeted.accepted) {
    throw new Error('private Vile Imp Genesis choice was rejected');
  }

  const events = targeted.receipt.events;
  const summonPayload = events[0] && isJsonRecord(events[0].payload) ? events[0].payload : undefined;
  const allocationPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const damagePayload = events[2] && isJsonRecord(events[2].payload) ? events[2].payload : undefined;
  const lifePayload = events[3] && isJsonRecord(events[3].payload) ? events[3].payload : undefined;
  const imp = targeted.session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.vileImpInstanceId);
  const deckCardIds = [
    ...opening.manifest.decks.north.atlas,
    ...opening.manifest.decks.north.spellbook,
  ];
  const cardsById = new Map(input.cards.map((card) => [card.stableId, card]));
  const legalLowRarityDeck = opening.manifest.decks.north.atlas.length === 30
    && opening.manifest.decks.north.spellbook.length === 60
    && opening.manifest.decks.north.atlas.filter((cardId) =>
      cardId === input.wasteland.stableId).length === input.format.copyLimits.ordinary
    && opening.manifest.decks.north.spellbook.filter((cardId) =>
      cardId === input.vileImp.stableId).length === input.format.copyLimits.ordinary
    && deckCardIds.every((cardId) => {
      const rarity = cardsById.get(cardId)?.rarity;
      return rarity === 'ordinary' || rarity === 'exceptional';
    });
  if (!legalLowRarityDeck) throw new Error('private Vile Imp teaching deck is no longer legal and low-rarity');

  return Object.freeze({
    acceptedActionCount: targeted.session.transcript.length,
    avatarTookTwoDamage: checkpoint.state.players.north.avatar.life === 20
      && targeted.session.state.players.north.avatar.life === 18,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'minion-summoned,genesis-damage-allocated,damage-dealt,avatar-life-lost'
      && summonPayload?.instanceId === opening.vileImpInstanceId
      && summonPayload.manaPaid === 2
      && allocationPayload?.amount === 2
      && allocationPayload.sourceInstanceId === opening.vileImpInstanceId
      && allocationPayload.targetInstanceId === avatarInstanceId
      && damagePayload?.amount === 2
      && damagePayload.instanceId === avatarInstanceId
      && lifePayload?.amount === 2
      && lifePayload.life === 18
      && lifePayload.seat === 'north',
    deck: deckList(opening.manifest.decks.north, opening.names),
    declinePreservedAvatar: declined.session.state.players.north.avatar.life === 20
      && declined.receipt.events.map(({ type }) => type).join(',') === 'minion-summoned',
    exactChoices: choices.length === 3
      && new Set([decline.actionId, selfTarget.actionId, target.actionId]).size === 3,
    legalLowRarityDeck,
    manaPaid: checkpoint.state.players.north.mana
      - targeted.session.state.players.north.mana,
    noRandomDraws: [...declined.session.transcript, ...targeted.session.transcript]
      .every(({ randomDraws }) => randomDraws.length === 0),
    replayVerified: verifyGameReplay(declined.session) && verifyGameReplay(targeted.session),
    seed: opening.seed,
    summonedAtC3: imp?.cardId === input.vileImp.stableId
      && imp.controller === 'north'
      && imp.damage === 0
      && imp.location === 'C3'
      && imp.owner === 'north'
      && imp.region === 'surface',
    vileImp: input.vileImp.name,
    wasteland: input.wasteland.name,
  });
}

function runFireAramos(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireAramos'] {
  const opening = findFireAramosOpening(input);
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
  const sitesBefore = canonicalJson(session.state.realm.sites as unknown as JsonValue);
  const unitsBefore = canonicalJson(session.state.realm.units as unknown as JsonValue);
  const northAvatarBefore = observeGame(session.state, 'north').players.north.avatar;
  const eligibleCards = [
    ...northBefore.hand.atlas.map((card) => ({ ...card, zone: 'atlas' as const })),
    ...northBefore.hand.spellbook
      .filter(({ instanceId }) => instanceId !== opening.aramosInstanceId)
      .map((card) => ({ ...card, zone: 'spellbook' as const })),
  ];
  const northObservedBefore = observeGame(session.state, 'north').players.north.hand;
  const southObservedBefore = observeGame(session.state, 'south').players.north.hand;
  const summonActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.aramosInstanceId);
  const alternative = summonActions.find(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.paymentMode === 'random-card-discard'
      && descriptor.cell === 'C3');
  if (!alternative) throw new Error('private Aramos alternative-cost summon is unavailable');
  const normalManaSummonUnavailable = summonActions.every(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.paymentMode === 'random-card-discard');
  const result = stepGame(session, alternative);
  if (!result.accepted) throw new Error('private Aramos alternative-cost summon was rejected');
  session = result.session;

  const events = result.receipt.events;
  const discardedEvent = events.find(({ type }) => type === 'card-discarded');
  const summonedEvent = events.find(({ type }) => type === 'minion-summoned');
  const discardedPayload = discardedEvent && isJsonRecord(discardedEvent.payload)
    ? discardedEvent.payload
    : undefined;
  const summonedPayload = summonedEvent && isJsonRecord(summonedEvent.payload)
    ? summonedEvent.payload
    : undefined;
  const discardedInstanceId = typeof discardedPayload?.instanceId === 'string'
    ? discardedPayload.instanceId
    : undefined;
  const discardedCardId = typeof discardedPayload?.cardId === 'string'
    ? discardedPayload.cardId
    : undefined;
  const discardedZone = discardedPayload?.zone === 'atlas'
    || discardedPayload?.zone === 'spellbook'
    ? discardedPayload.zone
    : undefined;
  const northAfter = session.state.players.north;
  const aramos = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.aramosInstanceId);
  const southObservedAfter = observeGame(session.state, 'south').players.north.hand;
  const southObservedCemetery = observeGame(session.state, 'south').players.north.cemetery;
  const random = result.receipt.randomDraws[0];
  const randomDomain = random && isJsonRecord(random.domain) ? random.domain : undefined;
  const expectedAtlasHand = northBefore.hand.atlas.filter(({ instanceId }) =>
    instanceId !== discardedInstanceId);
  const expectedSpellbookHand = northBefore.hand.spellbook.filter(({ instanceId }) =>
    instanceId !== opening.aramosInstanceId && instanceId !== discardedInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    aramosMercenaries: input.aramosMercenaries.name,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'card-discarded,minion-summoned'
      && discardedPayload?.owner === 'north'
      && discardedPayload.seat === 'north'
      && discardedPayload.sourceInstanceId === opening.aramosInstanceId
      && discardedZone !== undefined
      && summonedPayload?.instanceId === opening.aramosInstanceId
      && summonedPayload.manaPaid === 0
      && discardedEvent!.eventSequence < summonedEvent!.eventSequence,
    deck: deckList(opening.manifest.decks.north, opening.names),
    discardedNonCastingCard: discardedInstanceId !== undefined
      && discardedCardId !== undefined
      && discardedZone !== undefined
      && discardedInstanceId !== opening.aramosInstanceId
      && eligibleCards.some(({ cardId, instanceId, zone }) =>
        cardId === discardedCardId
          && instanceId === discardedInstanceId
          && zone === discardedZone)
      && northAfter.hand[discardedZone]
        .every(({ instanceId }) => instanceId !== discardedInstanceId)
      && northAfter.cemetery.some(({ cardId, instanceId }) =>
        cardId === discardedCardId && instanceId === discardedInstanceId),
    hiddenInformationVerified: Array.isArray(northObservedBefore.atlas)
      && Array.isArray(northObservedBefore.spellbook)
      && northObservedBefore.spellbook
        .some(({ instanceId }) => instanceId === opening.aramosInstanceId)
      && typeof southObservedBefore.atlas === 'number'
      && typeof southObservedBefore.spellbook === 'number'
      && typeof southObservedAfter.atlas === 'number'
      && typeof southObservedAfter.spellbook === 'number'
      && southObservedAfter.atlas === southObservedBefore.atlas
        - (discardedZone === 'atlas' ? 1 : 0)
      && southObservedAfter.spellbook === southObservedBefore.spellbook
        - 1
        - (discardedZone === 'spellbook' ? 1 : 0)
      && southObservedCemetery.some(({ cardId, instanceId }) =>
        cardId === discardedCardId && instanceId === discardedInstanceId),
    manaPaid: northBefore.mana - northAfter.mana,
    normalManaSummonUnavailable: northBefore.mana === 2 && normalManaSummonUnavailable,
    paymentModeVerified: alternative.descriptor.kind === 'summon-minion'
      && alternative.descriptor.paymentMode === 'random-card-discard',
    raalDromedary: input.raalDromedary.name,
    randomDiscardVerified: result.receipt.randomDraws.length === 1
      && random?.purpose === 'summon_random_card_discard_cost'
      && randomDomain?.accepted === true
      && randomDomain.kind === 'card_index_candidate'
      && randomDomain.exclusiveMaximum === eligibleCards.length,
    replayVerified: verifyGameReplay(session),
    summonedAtC3: aramos?.cardId === input.aramosMercenaries.stableId
      && aramos.controller === 'north'
      && aramos.location === 'C3'
      && aramos.owner === 'north'
      && aramos.region === 'surface',
    unrelatedStatePreserved: northAfter.mana === 2
      && northAfter.cemetery.length === northBefore.cemetery.length + 1
      && canonicalJson(northAfter.hand.atlas as unknown as JsonValue)
        === canonicalJson(expectedAtlasHand as unknown as JsonValue)
      && canonicalJson(northAfter.hand.spellbook as unknown as JsonValue)
        === canonicalJson(expectedSpellbookHand as unknown as JsonValue)
      && canonicalJson(northAfter.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.spellbook as unknown as JsonValue)
      && canonicalJson(observeGame(session.state, 'north').players.north.avatar as unknown as JsonValue)
        === canonicalJson(northAvatarBefore as unknown as JsonValue)
      && canonicalJson(session.state.players.south as unknown as JsonValue)
        === canonicalJson(southBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.sites as unknown as JsonValue) === sitesBefore
      && unitsBefore === '[]'
      && session.state.realm.units.length === 1
      && session.state.terminal.status === 'active'
      && events.every(({ type }) => type !== 'damage-dealt'
        && type !== 'minion-died'
        && type !== 'avatar-life-lost'
        && type !== 'game-ended'),
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

function runFireLash(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireLash'] {
  const opening = findFireLashOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.fireSiteInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.raalInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceId
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'C3');

  const readyRaal = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.raalInstanceId);
  if (!readyRaal) throw new Error('private Lash setup lacks Raal Dromedary');
  const zeroStepMoves = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.raalInstanceId
      && descriptor.path.length === 1
      && descriptor.to.cell === 'C4'
      && descriptor.to.region === 'surface');
  const zeroStepMove = zeroStepMoves[0];
  if (!zeroStepMove || zeroStepMoves.length !== 1) {
    throw new Error('private Lash setup lacks one exact zero-step Raal action');
  }
  session = accept(session, zeroStepMove);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const tappedRaal = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.raalInstanceId);
  if (!tappedRaal) throw new Error('private Lash setup removed Raal Dromedary');

  const northBefore = session.state.players.north;
  const southBefore = session.state.players.south;
  const sitesBefore = session.state.realm.sites;
  const otherUnitsBefore = session.state.realm.units.filter(({ instanceId }) =>
    instanceId !== opening.raalInstanceId);
  const lashActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.lashInstanceId
      && descriptor.casterInstanceId === northBefore.avatar.card.instanceId
      && descriptor.target?.kind === 'minion'
      && descriptor.target.instanceId === opening.raalInstanceId
      && descriptor.target.seat === 'north');
  const lashAction = lashActions[0];
  if (!lashAction || lashActions.length !== 1) {
    throw new Error('private Lash nearby Raal target is not exactly available');
  }
  const lashResult = stepGame(session, lashAction);
  if (!lashResult.accepted) throw new Error('private Lash cast was rejected');
  session = lashResult.session;
  const northAfter = session.state.players.north;
  const raalAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.raalInstanceId);
  const events = lashResult.receipt.events;
  const castPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const allocationPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const damagePayload = events[2] && isJsonRecord(events[2].payload)
    ? events[2].payload
    : undefined;
  const untapPayload = events[3] && isJsonRecord(events[3].payload)
    ? events[3].payload
    : undefined;
  const resolvedPayload = events[4] && isJsonRecord(events[4].payload)
    ? events[4].payload
    : undefined;
  const allocationIndex = events.findIndex(({ type }) => type === 'magic-damage-allocated');
  const damageIndex = events.findIndex(({ type }) => type === 'damage-dealt');
  const untapIndex = events.findIndex(({ type }) => type === 'minion-untapped');
  const resolvedIndex = events.findIndex(({ type }) => type === 'magic-resolved');

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'magic-cast,magic-damage-allocated,damage-dealt,minion-untapped,magic-resolved'
      && castPayload?.casterInstanceId === northBefore.avatar.card.instanceId
      && castPayload.instanceId === opening.lashInstanceId
      && castPayload.manaPaid === 3
      && castPayload.seat === 'north'
      && castPayload.targetInstanceId === opening.raalInstanceId
      && castPayload.targetSeat === 'north'
      && allocationPayload?.amount === 1
      && allocationPayload.sourceInstanceId === opening.lashInstanceId
      && allocationPayload.targetInstanceId === opening.raalInstanceId
      && damagePayload?.accumulated === 1
      && damagePayload.amount === 1
      && damagePayload.instanceId === opening.raalInstanceId
      && damagePayload.seat === 'north'
      && untapPayload?.instanceId === opening.raalInstanceId
      && untapPayload.seat === 'north'
      && untapPayload.sourceInstanceId === opening.lashInstanceId
      && resolvedPayload?.instanceId === opening.lashInstanceId,
    damageBeforeUntap: allocationIndex >= 0
      && allocationIndex < damageIndex
      && damageIndex < untapIndex
      && untapIndex < resolvedIndex,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactNearbyTarget: lashActions.length === 1,
    lash: input.lash.name,
    manaPaid: northBefore.mana - northAfter.mana,
    noDeathTerminalOrRandomEffects: events.every(({ type }) =>
      type !== 'minion-died'
        && type !== 'death-blow'
        && type !== 'avatar-reached-deaths-door'
        && type !== 'game-ended')
      && lashResult.receipt.randomDraws.length === 0
      && session.state.terminal.status === 'active',
    otherStatePreserved: raalAfter !== undefined
      && raalAfter.cardId === tappedRaal.cardId
      && raalAfter.controller === tappedRaal.controller
      && raalAfter.location === tappedRaal.location
      && raalAfter.owner === tappedRaal.owner
      && raalAfter.region === tappedRaal.region
      && raalAfter.stealthed === tappedRaal.stealthed
      && raalAfter.summoningSickness === tappedRaal.summoningSickness
      && raalAfter.warded === tappedRaal.warded
      && canonicalJson(session.state.realm.units
        .filter(({ instanceId }) => instanceId !== opening.raalInstanceId) as unknown as JsonValue)
        === canonicalJson(otherUnitsBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.sites as unknown as JsonValue)
        === canonicalJson(sitesBefore as unknown as JsonValue)
      && canonicalJson(session.state.players.south as unknown as JsonValue)
        === canonicalJson(southBefore as unknown as JsonValue)
      && canonicalJson(northAfter.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.spellbook as unknown as JsonValue)
      && canonicalJson(northAfter.hand.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.hand.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.hand.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.hand.spellbook
          .filter(({ instanceId }) => instanceId !== opening.lashInstanceId) as unknown as JsonValue)
      && canonicalJson(northAfter.cemetery
        .filter(({ instanceId }) => instanceId !== opening.lashInstanceId) as unknown as JsonValue)
        === canonicalJson(northBefore.cemetery as unknown as JsonValue)
      && northAfter.avatar.card.instanceId === northBefore.avatar.card.instanceId
      && northAfter.avatar.life === northBefore.avatar.life
      && northAfter.avatar.location === northBefore.avatar.location
      && northAfter.avatar.region === northBefore.avatar.region
      && northAfter.avatar.tapped === northBefore.avatar.tapped
      && session.state.phase === 'main',
    raalDromedary: input.raalDromedary.name,
    replayVerified: verifyGameReplay(session),
    spellEnteredCemetery: northAfter.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.lashInstanceId)
      && northAfter.cemetery.some(({ instanceId }) => instanceId === opening.lashInstanceId),
    survivedWithOneDamage: raalAfter?.damage === readyRaal.damage + 1
      && northAfter.cemetery.every(({ instanceId }) => instanceId !== opening.raalInstanceId),
    tappedThenUntapped: !readyRaal.tapped && tappedRaal.tapped && raalAfter?.tapped === false,
  });
}

function runFireLeapAttack(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireLeapAttack'] {
  const opening = findFireLeapAttackOpening(input);
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
    && descriptor.cardInstanceId === opening.northRaalInstanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  for (const instanceId of opening.southRaalInstanceIds) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === instanceId
      && descriptor.cell === 'C2');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'B3');

  const allyBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.northRaalInstanceId);
  if (!allyBefore) throw new Error('private Leap Attack setup lacks its allied Raal Dromedary');
  const sitesBefore = session.state.realm.sites;
  const northAvatarBefore = observeGame(session.state, 'north').players.north.avatar;
  const southAvatarBefore = observeGame(session.state, 'north').players.south.avatar;
  const leapActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.leapAttackInstanceId
      && descriptor.ally?.kind === 'minion'
      && descriptor.ally.instanceId === opening.northRaalInstanceId
      && descriptor.ally.seat === 'north'
      && descriptor.target === undefined);
  const noStepActions = leapActions.filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.allyDestination?.cell === 'C3'
      && descriptor.allyDestination.region === 'surface');
  const stepActions = leapActions.filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.allyDestination?.cell === 'C2'
      && descriptor.allyDestination.region === 'surface');
  const stepAction = stepActions[0];
  if (!stepAction || stepActions.length !== 1 || noStepActions.length !== 1) {
    throw new Error('private Leap Attack optional C3 and stepped C2 choices are not exactly available');
  }
  const manaBefore = session.state.players.north.mana;
  const leapResult = stepGame(session, stepAction);
  if (!leapResult.accepted) throw new Error('private Leap Attack cast was rejected');
  session = leapResult.session;

  const allyAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.northRaalInstanceId);
  const events = leapResult.receipt.events;
  const castPayload = events[0] && isJsonRecord(events[0].payload) ? events[0].payload : undefined;
  const stepPayload = events[1] && isJsonRecord(events[1].payload) ? events[1].payload : undefined;
  const stepFrom = stepPayload && isJsonRecord(stepPayload.from) ? stepPayload.from : undefined;
  const stepTo = stepPayload && isJsonRecord(stepPayload.to) ? stepPayload.to : undefined;
  const allocationEvents = events.filter(({ type }) => type === 'strike-damage-allocated');
  const allocationPayloads = allocationEvents.flatMap(({ payload }) =>
    isJsonRecord(payload) ? [payload] : []);
  const damagePayloads = events.filter(({ type }) => type === 'damage-dealt')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const deathPayloads = events.filter(({ type }) => type === 'minion-died')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const enemyIds = [...opening.southRaalInstanceIds].sort((left, right) =>
    left.localeCompare(right));
  const allocationIds = allocationPayloads.map(({ targetInstanceId }) => targetInstanceId);
  const damageIds = damagePayloads.map(({ instanceId }) => instanceId).sort();
  const deathIds = deathPayloads.map(({ instanceId }) => instanceId).sort();
  const eventTypes = events.map(({ type }) => type);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    allySteppedWithoutTapOrDamage: allyAfter?.cardId === allyBefore.cardId
      && allyAfter.controller === allyBefore.controller
      && allyAfter.damage === 0
      && allyAfter.location === 'C2'
      && allyAfter.owner === allyBefore.owner
      && allyAfter.region === 'surface'
      && allyAfter.summoningSickness === allyBefore.summoningSickness
      && !allyAfter.tapped,
    causalEventsVerified: eventTypes.join(',')
      === 'magic-cast,unit-stepped,strike-damage-allocated,strike-damage-allocated,damage-dealt,damage-dealt,minion-died,minion-died,magic-resolved'
      && castPayload?.instanceId === opening.leapAttackInstanceId
      && castPayload.manaPaid === 4
      && castPayload.allyInstanceId === opening.northRaalInstanceId
      && castPayload.allySeat === 'north'
      && isJsonRecord(castPayload.allyDestination)
      && castPayload.allyDestination.cell === 'C2'
      && castPayload.allyDestination.region === 'surface'
      && stepPayload?.instanceId === opening.northRaalInstanceId
      && stepPayload.seat === 'north'
      && stepPayload.sourceInstanceId === opening.leapAttackInstanceId
      && stepPayload.steps === 1
      && stepFrom?.cell === 'C3'
      && stepFrom.region === 'surface'
      && stepTo?.cell === 'C2'
      && stepTo.region === 'surface'
      && allocationPayloads.length === 2
      && allocationPayloads.every(({ amount, strikerInstanceId }) =>
        amount === 2 && strikerInstanceId === opening.northRaalInstanceId)
      && allocationIds.join(',') === enemyIds.join(','),
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactOptionalStepChoices: noStepActions.length === 1 && stepActions.length === 1,
    gameRemainedActive: session.state.terminal.status === 'active',
    leapAttack: input.leapAttack.name,
    manaPaid: manaBefore - session.state.players.north.mana,
    noAttackResponseOrRandomness: leapResult.receipt.randomDraws.length === 0
      && events.every(({ type }) =>
        type !== 'fight-started'
          && type !== 'attack-declared'
          && type !== 'defend-window-opened'
          && type !== 'intercept-window-opened')
      && session.state.pendingCombat === null
      && session.state.phase === 'main'
      && session.state.decisionSeat === 'north',
    raalDromedary: input.raalDromedary.name,
    replayVerified: verifyGameReplay(session),
    sitesAndAvatarsPreserved:
      canonicalJson(session.state.realm.sites as unknown as JsonValue)
        === canonicalJson(sitesBefore as unknown as JsonValue)
      && canonicalJson(observeGame(session.state, 'north').players.north.avatar as unknown as JsonValue)
        === canonicalJson(northAvatarBefore as unknown as JsonValue)
      && canonicalJson(observeGame(session.state, 'north').players.south.avatar as unknown as JsonValue)
        === canonicalJson(southAvatarBefore as unknown as JsonValue),
    spellEnteredCemetery: session.state.players.north.hand.spellbook
      .every(({ instanceId }) => instanceId !== opening.leapAttackInstanceId)
      && session.state.players.north.cemetery
        .some(({ instanceId }) => instanceId === opening.leapAttackInstanceId),
    struckAndKilledEveryEnemy: allocationIds.join(',') === enemyIds.join(',')
      && damageIds.join(',') === enemyIds.join(',')
      && deathIds.join(',') === enemyIds.join(',')
      && opening.southRaalInstanceIds.every((instanceId) =>
        session.state.realm.units.every((unit) => unit.instanceId !== instanceId)
          && session.state.players.south.cemetery.some((card) => card.instanceId === instanceId)),
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

function runFireVikingsSetup(
  opening: ReturnType<typeof findFireVikingsOpening>,
): GameSession {
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
    && descriptor.cardInstanceId === opening.southBoskTrollInstanceId
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.players.south.avatar.card.instanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[3]
    && descriptor.cell === 'B4');
  return session;
}

function runFireVikings(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireVikings'] {
  const opening = findFireVikingsOpening(input);
  let session = runFireVikingsSetup(opening);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const manaBeforeSummon = session.state.players.north.mana;
  const summonResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.vikingsInstanceId
      && descriptor.cell === 'C3'));
  if (!summonResult.accepted) throw new Error('private Vikings summon was rejected');
  session = summonResult.session;
  const manaAfterSummon = session.state.players.north.mana;
  const summonPayload = summonResult.receipt.events
    .map(({ payload }) => isJsonRecord(payload) ? payload : undefined)
    .find((payload) => payload?.instanceId === opening.vikingsInstanceId);
  const sickActivationUnavailable = legalGameActions(session.state, 'north').every(({ descriptor }) =>
    descriptor.kind !== 'activate-area-damage'
      || descriptor.sourceInstanceId !== opening.vikingsInstanceId);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const boskBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southBoskTrollInstanceId);
  const manaBeforeDagger = session.state.players.north.mana;
  const castResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-artifact'
      && descriptor.cardInstanceId === opening.daggerInstanceId
      && descriptor.bearer?.instanceId === opening.vikingsInstanceId));
  if (!castResult.accepted) throw new Error('private Poisonous Dagger cast on Vikings was rejected');
  session = castResult.session;
  const castPayload = castResult.receipt.events[0]
    && isJsonRecord(castResult.receipt.events[0].payload)
    ? castResult.receipt.events[0].payload
    : undefined;

  const activations = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage'
      && descriptor.sourceInstanceId === opening.vikingsInstanceId);
  const targetCells = activations.flatMap(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage' && descriptor.targetLocation.region === 'surface'
      ? [descriptor.targetLocation.cell]
      : []).sort();
  const selected = activations.filter(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage'
      && descriptor.targetLocation.cell === 'C2'
      && descriptor.targetLocation.region === 'surface');
  if (selected.length !== 1 || !selected[0]) {
    throw new Error('private Vikings adjacent C2 activation is unavailable');
  }
  const southAvatarLifeBefore = session.state.players.south.avatar.life;
  const activationResult = stepGame(session, selected[0]);
  if (!activationResult.accepted) throw new Error('private Vikings activation was rejected');
  session = activationResult.session;

  const events = activationResult.receipt.events;
  const activatedPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const boskTargetId = opening.southBoskTrollInstanceId;
  const southAvatarInstanceId = session.state.players.south.avatar.card.instanceId;
  const targetIds = [boskTargetId, southAvatarInstanceId].sort();
  const allocations = events.filter(({ payload, type }) => type === 'area-damage-allocated'
    && isJsonRecord(payload)
    && targetIds.includes(String(payload.targetInstanceId)));
  const damages = events.filter(({ payload, type }) => type === 'damage-dealt'
    && isJsonRecord(payload)
    && targetIds.includes(String(payload.instanceId)));
  const deaths = events.filter(({ payload, type }) => type === 'minion-died'
    && isJsonRecord(payload)
    && payload.instanceId === boskTargetId);
  const lastAllocationIndex = Math.max(...allocations.map((event) => events.indexOf(event)));
  const firstDamageIndex = Math.min(...damages.map((event) => events.indexOf(event)));
  const lastDamageIndex = Math.max(...damages.map((event) => events.indexOf(event)));
  const firstDeathIndex = Math.min(...deaths.map((event) => events.indexOf(event)));
  const vikings = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.vikingsInstanceId);
  const tappedActivationUnavailable = legalGameActions(session.state, 'north').every(({ descriptor }) =>
    descriptor.kind !== 'activate-area-damage'
      || descriptor.sourceInstanceId !== opening.vikingsInstanceId);
  const activationUnavailableWhileSickAndTapped: boolean = sickActivationUnavailable
    && tappedActivationUnavailable;
  const causalEventsVerified: boolean = events[0]?.type === 'area-damage-activated'
    && activatedPayload?.cell === 'C2'
    && activatedPayload.region === 'surface'
    && activatedPayload.seat === 'north'
    && activatedPayload.sourceInstanceId === opening.vikingsInstanceId
    && allocations.every(({ payload }) => isJsonRecord(payload)
      && payload.amount === 2
      && payload.sourceInstanceId === opening.vikingsInstanceId)
    && damages.every(({ payload }) => isJsonRecord(payload) && payload.amount === 2)
    && deaths.every(({ payload }) => isJsonRecord(payload)
      && payload.cardId === input.firstStrikeTargetMinion.stableId
      && payload.instanceId === boskTargetId
      && payload.owner === 'south');
  const dagger = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === opening.daggerInstanceId);
  const boskDefinition = session.state.cards[input.firstStrikeTargetMinion.stableId];
  const artifactCastAndCarried: boolean = castResult.receipt.events.length === 1
    && castResult.receipt.events[0]?.type === 'artifact-conjured'
    && castPayload?.cardId === input.poisonousDagger.stableId
    && castPayload.instanceId === opening.daggerInstanceId
    && castPayload.manaPaid === 2
    && castPayload.bearerInstanceId === opening.vikingsInstanceId
    && dagger !== undefined
    && 'bearer' in dagger
    && dagger.bearer.instanceId === opening.vikingsInstanceId
    && events.every(({ type }) => type !== 'artifact-dropped');
  const abilityLethalVerified: boolean = boskBefore?.damage === 0
    && boskDefinition?.cardType === 'minion'
    && boskDefinition.defense === 3
    && damages.some(({ payload }) => isJsonRecord(payload)
      && payload.instanceId === boskTargetId
      && payload.amount === 2)
    && deaths.length === 1;
  const exactAdjacentTarget: boolean = targetCells.join(',') === 'B3,C2,C4'
    && selected.length === 1;
  const noCombatOrReturnDamage: boolean = vikings?.damage === 0
    && events.every(({ type }) => type !== 'fight-started'
      && type !== 'strike-damage-allocated');
  const simultaneousDamageVerified: boolean = allocations.length === 2
    && damages.length === 2
    && deaths.length === 1
    && allocations.map(({ payload }) => isJsonRecord(payload)
      ? String(payload.targetInstanceId)
      : '').join(',') === targetIds.join(',')
    && lastAllocationIndex < firstDamageIndex
    && lastDamageIndex < firstDeathIndex
    && southAvatarLifeBefore - session.state.players.south.avatar.life === 2;
  const targetsEnteredCemetery: boolean = session.state.players.south.cemetery
    .some((card) => card.instanceId === boskTargetId)
    && session.state.players.south.cemetery.length === 1
    && session.state.players.north.cemetery.length === 0;
  const vikingsSurvivedAndTapped: boolean = vikings?.cardId === input.vikings.stableId
    && vikings.controller === 'north'
    && vikings.owner === 'north'
    && vikings.location === 'C3'
    && vikings.region === 'surface'
    && vikings.tapped
    && summonPayload?.manaPaid === 5
    && manaBeforeSummon === 5;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    activationUnavailableWhileSickAndTapped,
    artifactCastAndCarried,
    abilityLethalVerified,
    boskTroll: input.firstStrikeTargetMinion.name,
    causalEventsVerified,
    daggerManaPaid: manaBeforeDagger - session.state.players.north.mana,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactAdjacentTarget,
    noCombatOrReturnDamage,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    poisonousDagger: input.poisonousDagger.name,
    replayVerified: verifyGameReplay(session),
    simultaneousDamageVerified,
    summonManaPaid: manaBeforeSummon - manaAfterSummon,
    targetsEnteredCemetery,
    vikings: input.vikings.name,
    vikingsSurvivedAndTapped,
  });
}

function runFireRecklessSquire(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['fireRecklessSquire'] {
  const opening = findFireRecklessSquireOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northFireSiteInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southFireSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'C3');
  const summonResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.recklessSquireInstanceId
      && descriptor.cell === 'C3'));
  if (!summonResult.accepted) throw new Error('private Reckless Squire summon was rejected');
  session = summonResult.session;
  const squireAfterSummon = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.recklessSquireInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southFireSiteInstanceIds[1]
    && descriptor.cell === 'C2');
  for (const instanceId of opening.southRaalInstanceIds) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === instanceId
      && descriptor.cell === 'C2');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const sitesBeforeCombat = session.state.realm.sites;
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.recklessSquireInstanceId
    && descriptor.from.cell === 'C3'
    && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === opening.southRaalInstanceIds[0]);
  const firstFight = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  if (!firstFight.accepted) throw new Error('private Lance first fight was rejected');
  session = firstFight.session;
  const squireAfterFirstFight = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.recklessSquireInstanceId);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.recklessSquireInstanceId
    && descriptor.path.length === 1
    && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === opening.southRaalInstanceIds[1]);
  const secondFight = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  if (!secondFight.accepted) throw new Error('private post-Lance fight was rejected');
  session = secondFight.session;

  const summonEvents = summonResult.receipt.events;
  const summonEvent = summonEvents.find(({ type }) => type === 'minion-summoned');
  const summonPayload = summonEvent && isJsonRecord(summonEvent.payload)
    ? summonEvent.payload
    : undefined;
  const gainedEvent = summonEvents.find(({ type }) => type === 'lance-gained');
  const gainedPayload = gainedEvent && isJsonRecord(gainedEvent.payload)
    ? gainedEvent.payload
    : undefined;
  const firstEvents = firstFight.receipt.events;
  const firstAllocations = firstEvents.filter(({ type }) => type === 'strike-damage-allocated')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const firstDamage = firstEvents.filter(({ type }) => type === 'damage-dealt')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const brokenPayload = firstEvents.find(({ type }) => type === 'lance-broken');
  const broken = brokenPayload && isJsonRecord(brokenPayload.payload)
    ? brokenPayload.payload
    : undefined;
  const firstDeath = firstEvents.find(({ type }) => type === 'minion-died');
  const firstDeathPayload = firstDeath && isJsonRecord(firstDeath.payload)
    ? firstDeath.payload
    : undefined;
  const secondEvents = secondFight.receipt.events;
  const secondAllocations = secondEvents.filter(({ type }) => type === 'strike-damage-allocated')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const secondDamage = secondEvents.filter(({ type }) => type === 'damage-dealt')
    .flatMap(({ payload }) => isJsonRecord(payload) ? [payload] : []);
  const secondDeath = secondEvents.find(({ type }) => type === 'minion-died');
  const secondDeathPayload = secondDeath && isJsonRecord(secondDeath.payload)
    ? secondDeath.payload
    : undefined;
  const secondRaal = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.southRaalInstanceIds[1]);
  const summonIndex = summonEvents.findIndex(({ type }) => type === 'minion-summoned');
  const lanceGainedIndex = summonEvents.findIndex(({ type }) => type === 'lance-gained');
  const fightStartedIndex = firstEvents.findIndex(({ type }) => type === 'fight-started');
  const firstAllocationIndex = firstEvents.findIndex(({ type }) =>
    type === 'strike-damage-allocated');
  const firstDamageIndex = firstEvents.findIndex(({ type }) => type === 'damage-dealt');
  const lanceBreakIndex = firstEvents.findIndex(({ type }) => type === 'lance-broken');
  const firstDeathIndex = firstEvents.findIndex(({ type }) => type === 'minion-died');
  const secondSquireAllocation = secondAllocations.some(({
    amount,
    strikerInstanceId,
    targetInstanceId,
  }) => amount === 1
    && strikerInstanceId === opening.recklessSquireInstanceId
    && targetInstanceId === opening.southRaalInstanceIds[1]);
  const secondRaalReturnDamage = secondDamage.some(({ amount, instanceId }) =>
    amount === 2 && instanceId === opening.recklessSquireInstanceId);
  const secondSquireDamage = secondDamage.some(({ amount, instanceId }) =>
    amount === 1 && instanceId === opening.southRaalInstanceIds[1]);
  const secondDeathIsSquire = secondDeathPayload?.instanceId
    === opening.recklessSquireInstanceId;
  const removedFromRealm = session.state.realm.units.every(({ instanceId }) =>
    instanceId !== opening.recklessSquireInstanceId
      && instanceId !== opening.southRaalInstanceIds[0]);
  const squireInCemetery = session.state.players.north.cemetery.some(({ cardId, instanceId }) =>
    cardId === input.recklessSquire.stableId
      && instanceId === opening.recklessSquireInstanceId);
  const firstRaalInCemetery = session.state.players.south.cemetery.some(({ cardId, instanceId }) =>
    cardId === input.raalDromedary.stableId
      && instanceId === opening.southRaalInstanceIds[0]);
  const secondRaalSurvived = secondRaal?.cardId === input.raalDromedary.stableId
    && secondRaal.damage === 1
    && secondRaal.location === 'C2';
  const sitesPreserved = canonicalJson(session.state.realm.sites as unknown as JsonValue)
    === canonicalJson(sitesBeforeCombat as unknown as JsonValue);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified: summonPayload?.instanceId === opening.recklessSquireInstanceId
      && summonPayload.manaPaid === 3
      && summonPayload.seat === 'north'
      && gainedPayload?.bearerInstanceId === opening.recklessSquireInstanceId
      && gainedPayload.count === 1
      && gainedPayload.sourceInstanceId === opening.recklessSquireInstanceId
      && summonIndex >= 0
      && summonIndex < lanceGainedIndex
      && fightStartedIndex >= 0
      && fightStartedIndex < firstAllocationIndex
      && firstAllocationIndex < firstDamageIndex
      && firstDamageIndex < lanceBreakIndex
      && lanceBreakIndex < firstDeathIndex
      && secondEvents.every(({ type }) => type !== 'lance-broken'),
    deck: deckList(opening.manifest.decks.north, opening.names),
    firstStrikeLanceDamage: firstAllocations.length === 1
      && firstAllocations[0]?.amount === 2
      && firstAllocations[0].strikerInstanceId === opening.recklessSquireInstanceId
      && firstAllocations[0].targetInstanceId === opening.southRaalInstanceIds[0]
      && firstDamage.length === 1
      && firstDamage[0]?.amount === 2
      && firstDamage[0].instanceId === opening.southRaalInstanceIds[0]
      && firstDeathPayload?.instanceId === opening.southRaalInstanceIds[0]
      && firstEvents.every(({ payload, type }) =>
        type !== 'strike-damage-allocated'
          || !isJsonRecord(payload)
          || payload.strikerInstanceId !== opening.southRaalInstanceIds[0]),
    gameRemainedActive: session.state.terminal.status === 'active',
    lanceCreatedAndCarried: squireAfterSummon?.carriedLanceCount === 1,
    lanceUsedAndRemoved: broken?.bearerInstanceId === opening.recklessSquireInstanceId
      && broken.count === 1
      && broken.sourceInstanceId === opening.recklessSquireInstanceId
      && (squireAfterFirstFight?.carriedLanceCount ?? 0) === 0,
    noRandomDraws: summonResult.receipt.randomDraws.length === 0
      && firstFight.receipt.randomDraws.length === 0
      && secondFight.receipt.randomDraws.length === 0,
    raalDromedary: input.raalDromedary.name,
    recklessSquire: input.recklessSquire.name,
    replayVerified: verifyGameReplay(session),
    secondStrikeNormal: secondAllocations.length === 1
      && secondSquireAllocation
      && secondRaalReturnDamage
      && secondSquireDamage
      && secondDeathIsSquire,
    stateAndCemeteriesVerified: removedFromRealm
      && squireInCemetery
      && firstRaalInCemetery
      && secondRaalSurvived
      && sitesPreserved,
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

function findWaterRiverOpening(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): Readonly<{
  manifest: GameManifest;
  names: ReadonlyMap<string, string>;
  riverInstanceId: string;
  seed: number;
  session: GameSession;
}> {
  // ponytail: bounded seed scan avoids another private config field.
  for (let offset = 1; offset <= 64; offset += 1) {
    const seed = input.config.waterSeed + offset;
    const built = buildManifest(input, seed, 'water-river');
    const session = createGameSession(built.manifest);
    const river = session.state.players.north.hand.atlas
      .find(({ cardId }) => cardId === input.autumnRiver.stableId);
    if (river && session.state.players.north.spellbook.length > 1) {
      return {
        ...built,
        riverInstanceId: river.instanceId,
        seed,
        session,
      };
    }
  }
  throw new Error('private seasonal River scenario no longer produces its supported opening');
}

function runWaterRiver(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterRiver'] {
  const opening = findWaterRiverOpening(input);
  let checkpoint = keep(opening.session);
  checkpoint = keep(checkpoint);
  const before = checkpoint.state.players.north.spellbook;
  const top = before[0];
  if (!top) throw new Error('private seasonal River scenario lacks a next spell');
  const plays = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.riverInstanceId
      && descriptor.cell === 'C4');
  const play = plays[0];
  if (plays.length !== 1 || !play) throw new Error('private seasonal River play is unavailable');
  const played = stepGame(checkpoint, play);
  if (!played.accepted) throw new Error('private seasonal River play was rejected');
  const choices = legalGameActions(played.session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'resolve-genesis-spell');
  const keepChoice = choices.find(({ descriptor }) =>
    descriptor.kind === 'resolve-genesis-spell' && descriptor.choice === 'keep-next');
  const bottomChoice = choices.find(({ descriptor }) =>
    descriptor.kind === 'resolve-genesis-spell' && descriptor.choice === 'bottom-next');
  if (!keepChoice || !bottomChoice) throw new Error('private seasonal River choices are unavailable');
  const kept = stepGame(played.session, keepChoice);
  const bottomed = stepGame(played.session, bottomChoice);
  if (!kept.accepted || !bottomed.accepted) {
    throw new Error('private seasonal River choice was rejected');
  }
  const beforeOrder = before.map(({ instanceId }) => instanceId);
  const keptOrder = kept.session.state.players.north.spellbook.map(({ instanceId }) => instanceId);
  const bottomedOrder = bottomed.session.state.players.north.spellbook
    .map(({ instanceId }) => instanceId);
  const playEvents = played.receipt.events.map(({ type }) => type).join(',');
  const keepEvents = kept.receipt.events.map(({ type }) => type).join(',');
  const bottomEvents = bottomed.receipt.events.map(({ type }) => type).join(',');
  const bottomEvent = bottomed.receipt.events[0];
  const privateIdentity = [top.cardId, top.instanceId];
  const publicEvents = canonicalJson([
    ...played.receipt.events,
    ...bottomed.receipt.events,
  ] as unknown as JsonValue);
  const deckCardIds = [
    ...opening.manifest.decks.north.atlas,
    ...opening.manifest.decks.north.spellbook,
  ];
  const cardsById = new Map(input.cards.map((card) => [card.stableId, card]));
  if (!deckCardIds.every((cardId) => {
    const rarity = cardsById.get(cardId)?.rarity;
    return rarity === 'ordinary' || rarity === 'exceptional';
  })) {
    throw new Error('private seasonal River teaching deck no longer uses only entry-level rarities');
  }
  return Object.freeze({
    acceptedActionCount: bottomed.session.transcript.length,
    bottomedNextSpell: bottomedOrder.join(',')
      === [...beforeOrder.slice(1), beforeOrder[0]!].join(','),
    causalEventsVerified: playEvents === 'site-played'
      && keepEvents === 'spell-kept'
      && bottomEvents === 'spell-bottomed'
      && bottomEvent?.type === 'spell-bottomed'
      && isJsonRecord(bottomEvent.payload)
      && bottomEvent.payload.seat === 'north'
      && bottomEvent.payload.sourceInstanceId === opening.riverInstanceId,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactChoices: plays.length === 1
      && !play.label.includes(top.cardId)
      && choices.length === 2
      && new Set(choices.map(({ actionId }) => actionId)).size === 2
      && privateIdentity.every((identity) =>
        !canonicalJson(choices.map(({ descriptor }) => descriptor) as unknown as JsonValue)
          .includes(identity)),
    hiddenFromOpponent: canonicalJson(observeGame(kept.session.state, 'south') as unknown as JsonValue)
      === canonicalJson(observeGame(bottomed.session.state, 'south') as unknown as JsonValue)
      && !canonicalJson(observeGame(played.session.state, 'south') as unknown as JsonValue)
        .includes(top.instanceId)
      && privateIdentity.every((identity) => !publicEvents.includes(identity)),
    keptNextSpell: keptOrder.join(',') === beforeOrder.join(','),
    legalLowRarityDeck: true,
    noRandomDraws: played.receipt.randomDraws.length === 0
      && kept.receipt.randomDraws.length === 0
      && bottomed.receipt.randomDraws.length === 0,
    replayVerified: verifyGameReplay(kept.session) && verifyGameReplay(bottomed.session),
    river: input.autumnRiver.name,
    seed: opening.seed,
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
  const opening = findWaterSubmergeFreezeOpening(input);
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

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const manaBeforeSeaWitch = session.state.players.north.mana;
  take(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.seaWitchInstanceId
      && descriptor.cell === 'C3'
      && descriptor.region === 'underwater');
  const manaBeforeFreeze = session.state.players.north.mana;
  const seaWitchBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seaWitchInstanceId);
  const kelpieBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.featuredInstanceId);
  if (!seaWitchBefore || !kelpieBefore) {
    throw new Error('private underwater Freeze setup lacks its caster or target');
  }
  const freezeActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.freezeInstanceId
      && descriptor.casterInstanceId === opening.seaWitchInstanceId
      && descriptor.target?.kind === 'minion'
      && descriptor.target.instanceId === opening.featuredInstanceId
      && descriptor.target.seat === 'north');
  const selectedFreeze = freezeActions[0];
  if (!selectedFreeze || freezeActions.length !== 1) {
    throw new Error('private underwater Spellcaster Freeze target is not exactly available');
  }
  session = accept(session, selectedFreeze);

  const receipt = session.transcript.at(-1);
  const events = receipt?.events ?? [];
  const seaWitchAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.seaWitchInstanceId);
  const cemeteryAfter = session.state.players.north.cemetery;
  const underwaterFreezeSettlementVerified = [
    manaBeforeSeaWitch === 3,
    manaBeforeFreeze === 1,
    session.state.players.north.mana === 0,
    seaWitchBefore.location === 'C3'
      && seaWitchBefore.region === 'underwater'
      && seaWitchBefore.summoningSickness,
    kelpieBefore.location === 'C3' && kelpieBefore.region === 'underwater',
    events.map(({ type }) => type).join(',')
      === 'magic-cast,minion-disabled,minion-died,magic-resolved',
    seaWitchAfter !== undefined
      && seaWitchAfter.controller === 'north'
      && seaWitchAfter.location === 'C3'
      && seaWitchAfter.owner === 'north'
      && seaWitchAfter.region === 'underwater',
    !session.state.realm.units.some(({ instanceId }) =>
      instanceId === opening.featuredInstanceId),
    cemeteryAfter.some(({ instanceId }) => instanceId === opening.featuredInstanceId)
      && cemeteryAfter.some(({ instanceId }) => instanceId === opening.freezeInstanceId),
    session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
  ].every(Boolean);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    deck: deckList(opening.manifest.decks.north, opening.names),
    freeze: input.freeze.name,
    nonSubmergeSurfaceAvailable,
    nonSubmergeUnderwaterUnavailable,
    replayVerified: verifyGameReplay(session),
    seaWitch: input.seaWitch.name,
    seed: opening.seed,
    submergeMinion:
      opening.names.get(input.submergeMinion.stableId) ?? input.submergeMinion.stableId,
    summonedUnderwater,
    surfaceSummonAvailable,
    targetIsWaterSite,
    underwaterFreezeSettlementVerified,
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

function runWaterGnarledWendigo(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterGnarledWendigo'] {
  const opening = findWaterGnarledWendigoOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northWaterSiteInstanceIds[0]
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.seravaInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[0]
    && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northWaterSiteInstanceIds[1]
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.southSiteInstanceIds[1]
    && descriptor.cell === 'B1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const manaBeforeGhostTown = session.state.players.north.mana;
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.ghostTownInstanceId
    && descriptor.cell === 'C2');
  const ghostTownEvents = session.transcript.at(-1)?.events ?? [];
  const ghostTownManaPayload = ghostTownEvents[1] && isJsonRecord(ghostTownEvents[1].payload)
    ? ghostTownEvents[1].payload
    : undefined;
  const northBefore = session.state.players.north;
  const southBefore = session.state.players.south;
  const sitesBefore = session.state.realm.sites;
  const northAvatarBefore = observeGame(session.state, 'north').players.north.avatar;
  const otherUnitsBefore = session.state.realm.units.filter(({ instanceId }) =>
    instanceId !== opening.seravaInstanceId);
  const stateVersionBefore = session.state.stateVersion;
  const allSummons = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === opening.gnarledWendigoInstanceId
      && descriptor.cell === 'C4'
      && (descriptor.region ?? 'surface') === 'surface');
  const normalSummons = allSummons.filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.sacrificedMinionInstanceIds === undefined);
  const discountedSummons = allSummons.filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.manaCost === 4
      && descriptor.sacrificedMinionInstanceIds?.length === 1
      && descriptor.sacrificedMinionInstanceIds[0] === opening.seravaInstanceId);
  const selected = discountedSummons[0];
  if (!selected || discountedSummons.length !== 1) {
    throw new Error('private Gnarled Wendigo sacrifice-discount summon is not exactly available');
  }
  const summoned = stepGame(session, selected);
  if (!summoned.accepted) throw new Error('private Gnarled Wendigo summon was rejected');
  session = summoned.session;

  const northAfter = session.state.players.north;
  const events = summoned.receipt.events;
  const sacrificedPayload = events[0] && isJsonRecord(events[0].payload)
    ? events[0].payload
    : undefined;
  const deathPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const summonPayload = events[2] && isJsonRecord(events[2].payload)
    ? events[2].payload
    : undefined;
  const wendigo = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.gnarledWendigoInstanceId);
  const wendigoDefinition = wendigo ? session.state.cards[wendigo.cardId] : undefined;
  const expectedNorthSpellHand = northBefore.hand.spellbook.filter(({ instanceId }) =>
    instanceId !== opening.gnarledWendigoInstanceId);

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    canonicalSacrificeChoice: selected.descriptor.kind === 'summon-minion'
      && selected.descriptor.sacrificedMinionInstanceIds?.length === 1
      && selected.descriptor.sacrificedMinionInstanceIds[0] === opening.seravaInstanceId,
    causalEventsVerified: events.map(({ type }) => type).join(',')
      === 'minion-sacrificed,minion-died,minion-summoned'
      && sacrificedPayload?.cardId === input.seravaTownsfolk.stableId
      && sacrificedPayload.instanceId === opening.seravaInstanceId
      && sacrificedPayload.owner === 'north'
      && sacrificedPayload.seat === 'north'
      && sacrificedPayload.sourceInstanceId === opening.gnarledWendigoInstanceId
      && deathPayload?.cardId === input.seravaTownsfolk.stableId
      && deathPayload.instanceId === opening.seravaInstanceId
      && deathPayload.owner === 'north'
      && summonPayload?.instanceId === opening.gnarledWendigoInstanceId
      && summonPayload.manaPaid === 4
      && summonPayload.seat === 'north'
      && events[0]!.eventSequence < events[1]!.eventSequence
      && events[1]!.eventSequence < events[2]!.eventSequence,
    deck: deckList(opening.manifest.decks.north, opening.names),
    exactDiscountedSummonAvailable: discountedSummons.length === 1,
    gameRemainedActive: session.state.terminal.status === 'active',
    ghostTownManaConsumed: manaBeforeGhostTown === 2
      && northBefore.mana === 4
      && northAfter.mana === 0
      && ghostTownEvents.map(({ type }) => type).join(',') === 'site-played,mana-gained'
      && ghostTownManaPayload?.amount === 1
      && ghostTownManaPayload.seat === 'north'
      && ghostTownManaPayload.sourceInstanceId === opening.ghostTownInstanceId,
    gnarledWendigo: input.gnarledWendigo.name,
    handRealmCemeteryVerified: canonicalJson(
      northAfter.hand.spellbook as unknown as JsonValue,
    ) === canonicalJson(expectedNorthSpellHand as unknown as JsonValue)
      && northAfter.cemetery.length === northBefore.cemetery.length + 1
      && northAfter.cemetery.some(({ cardId, instanceId }) =>
        cardId === input.seravaTownsfolk.stableId
          && instanceId === opening.seravaInstanceId)
      && session.state.realm.units.every(({ instanceId }) =>
        instanceId !== opening.seravaInstanceId)
      && wendigo !== undefined,
    manaPaid: northBefore.mana - northAfter.mana,
    noNormalManaSummon: normalSummons.length === 0
      && allSummons.every(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.manaCost !== 6),
    noRandomOrUnrelatedEffects: summoned.receipt.randomDraws.length === 0
      && canonicalJson(session.state.players.south as unknown as JsonValue)
        === canonicalJson(southBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.sites as unknown as JsonValue)
        === canonicalJson(sitesBefore as unknown as JsonValue)
      && canonicalJson(session.state.realm.units.filter(({ instanceId }) =>
        instanceId !== opening.gnarledWendigoInstanceId) as unknown as JsonValue)
        === canonicalJson(otherUnitsBefore.filter(({ instanceId }) =>
          instanceId !== opening.seravaInstanceId) as unknown as JsonValue)
      && canonicalJson(northAfter.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.atlas as unknown as JsonValue)
      && canonicalJson(northAfter.spellbook as unknown as JsonValue)
        === canonicalJson(northBefore.spellbook as unknown as JsonValue)
      && canonicalJson(northAfter.hand.atlas as unknown as JsonValue)
        === canonicalJson(northBefore.hand.atlas as unknown as JsonValue)
      && canonicalJson(observeGame(session.state, 'north').players.north.avatar as unknown as JsonValue)
        === canonicalJson(northAvatarBefore as unknown as JsonValue),
    replayVerified: verifyGameReplay(session),
    seravaTownsfolk: input.seravaTownsfolk.name,
    stateVersionAdvancedOnce: session.state.stateVersion === stateVersionBefore + 1,
    summonedAtC4: wendigo?.cardId === input.gnarledWendigo.stableId
      && wendigo.controller === 'north'
      && wendigo.location === 'C4'
      && wendigo.owner === 'north'
      && wendigo.region === 'surface'
      && wendigoDefinition?.cardType === 'minion'
      && wendigoDefinition.attack === 5
      && wendigoDefinition.defense === 5,
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

function runEarthHuntersLodge(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['earthHuntersLodge'] {
  const opening = findHuntersLodgeOpening(input);
  let session = keep(opening.session);
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northWaterSiteInstanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.slyFoxInstanceId
    && descriptor.cell === 'C4');
  const summonedFox = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.slyFoxInstanceId);
  const endResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  if (!endResult.accepted) throw new Error("private Sly Fox end turn before Hunter's Lodge was rejected");
  session = endResult.session;
  const endEvents = endResult.receipt.events;
  const stealthGainedIndex = endEvents.findIndex(({ payload, type }) =>
    type === 'stealth-gained'
      && isJsonRecord(payload)
      && payload.instanceId === opening.slyFoxInstanceId
      && payload.seat === 'north');
  const turnEndedIndex = endEvents.findIndex(({ type }) => type === 'turn-ended');
  const stealthedFox = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.slyFoxInstanceId);
  const slyFoxGainedStealthFirst: boolean = summonedFox?.stealthed === false
    && stealthedFox?.stealthed === true
    && stealthGainedIndex >= 0
    && stealthGainedIndex < turnEndedIndex;

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const northAvatarBefore = session.state.players.north.avatar;
  const southAvatarBefore = session.state.players.south.avatar;
  const northSiteBefore = canonicalJson(
    session.state.realm.sites.C4 as unknown as JsonValue,
  );
  const lodgeResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === opening.hunterLodgeInstanceId
      && descriptor.cell === 'C1'));
  if (!lodgeResult.accepted) throw new Error("private Hunter's Lodge play was rejected");
  session = lodgeResult.session;

  const events = lodgeResult.receipt.events;
  const sitePlayedIndex = events.findIndex(({ type }) => type === 'site-played');
  const stealthLostIndex = events.findIndex(({ payload, type }) =>
    type === 'stealth-lost'
      && isJsonRecord(payload)
      && payload.instanceId === opening.slyFoxInstanceId);
  const sitePlayedPayload = sitePlayedIndex >= 0 && isJsonRecord(events[sitePlayedIndex]!.payload)
    ? events[sitePlayedIndex]!.payload
    : undefined;
  const stealthLostPayload = stealthLostIndex >= 0 && isJsonRecord(events[stealthLostIndex]!.payload)
    ? events[stealthLostIndex]!.payload
    : undefined;
  const revealedFox = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.slyFoxInstanceId);
  const lodgeSite = session.state.realm.sites.C1;
  const causalEventsVerified: boolean = events.length === 2
    && sitePlayedIndex === 0
    && stealthLostIndex === 1
    && sitePlayedPayload?.cardId === input.huntersLodge.stableId
    && sitePlayedPayload.cell === 'C1'
    && sitePlayedPayload.instanceId === opening.hunterLodgeInstanceId
    && sitePlayedPayload.seat === 'south'
    && stealthLostPayload?.instanceId === opening.slyFoxInstanceId
    && stealthLostPayload.seat === 'north'
    && stealthLostPayload.sourceInstanceId === opening.hunterLodgeInstanceId;
  const enemyStealthRemoved: boolean = stealthedFox?.stealthed === true
    && revealedFox?.stealthed === false;
  const statePreserved: boolean = summonedFox !== undefined
    && stealthedFox !== undefined
    && revealedFox !== undefined
    && revealedFox.cardId === stealthedFox.cardId
    && revealedFox.owner === stealthedFox.owner
    && revealedFox.controller === stealthedFox.controller
    && revealedFox.location === stealthedFox.location
    && revealedFox.region === stealthedFox.region
    && revealedFox.damage === stealthedFox.damage
    && revealedFox.tapped === stealthedFox.tapped
    && revealedFox.summoningSickness === stealthedFox.summoningSickness
    && revealedFox.warded === stealthedFox.warded
    && session.state.players.north.avatar.life === northAvatarBefore.life
    && session.state.players.north.avatar.location === northAvatarBefore.location
    && session.state.players.north.avatar.region === northAvatarBefore.region
    && session.state.players.north.avatar.tapped === northAvatarBefore.tapped
    && session.state.players.south.avatar.life === southAvatarBefore.life
    && session.state.players.south.avatar.location === southAvatarBefore.location
    && session.state.players.south.avatar.region === southAvatarBefore.region
    && !southAvatarBefore.tapped
    && session.state.players.south.avatar.tapped
    && canonicalJson(session.state.realm.sites.C4 as unknown as JsonValue) === northSiteBefore
    && lodgeSite !== undefined
    && lodgeSite.controller === 'south'
    && lodgeSite.instanceId === opening.hunterLodgeInstanceId
    && session.state.players.north.cemetery.length === 0
    && session.state.players.south.cemetery.length === 0
    && session.state.terminal.status === 'active';

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified,
    deck: deckList(opening.manifest.decks.north, opening.names),
    enemyStealthRemoved,
    hunterLodge: input.huntersLodge.name,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    replayVerified: verifyGameReplay(session),
    slyFox: input.slyFox.name,
    slyFoxGainedStealthFirst,
    statePreserved,
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

function runWaterMesmerismSetup(
  opening: ReturnType<typeof findWaterMesmerismOpening>,
): GameSession {
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
    && descriptor.cardInstanceId === opening.farSeravaInstanceId
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
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardInstanceId === opening.kettletopInstanceId
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[2]
    && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === opening.northSiteInstanceIds[3]
    && descriptor.cell === 'A3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  return session;
}

function runWaterMesmerism(
  input: Awaited<ReturnType<typeof readPrivateInputs>>,
): PrivateGameCheck['waterMesmerism'] {
  const opening = findWaterMesmerismOpening(input);
  let session = runWaterMesmerismSetup(opening);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const targetBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.kettletopInstanceId);
  const farBefore = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.farSeravaInstanceId);
  if (!targetBefore || !farBefore) throw new Error('private Mesmerism setup lacks its minions');
  const oldControllerHadAction = targetBefore.controller === 'south'
    && !targetBefore.summoningSickness
    && !targetBefore.tapped;
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === opening.mesmerismInstanceId);
  const chosen = choices.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'minion'
    && descriptor.target.instanceId === opening.kettletopInstanceId
    && descriptor.target.seat === 'south');
  if (!chosen || choices.length !== 1) {
    throw new Error('private Mesmerism exact nearby target is not uniquely available');
  }
  const manaBefore = session.state.players.north.mana;
  const affinityBefore = observeGame(session.state, 'north').players.north.affinity.water;
  const cast = stepGame(session, chosen);
  if (!cast.accepted) throw new Error('private Mesmerism cast was rejected');
  session = cast.session;
  const manaAfterCast = session.state.players.north.mana;

  const targetAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.kettletopInstanceId);
  const farAfter = session.state.realm.units.find(({ instanceId }) =>
    instanceId === opening.farSeravaInstanceId);
  if (!targetAfter || !farAfter) throw new Error('private Mesmerism removed a minion');
  const newControllerGainedAction = legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.kettletopInstanceId);
  const events = cast.receipt.events;
  const castPayload = events[0] && isJsonRecord(events[0].payload) ? events[0].payload : undefined;
  const changedPayload = events[1] && isJsonRecord(events[1].payload)
    ? events[1].payload
    : undefined;
  const resolvedPayload = events[2] && isJsonRecord(events[2].payload)
    ? events[2].payload
    : undefined;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const southActions = legalGameActions(session.state, 'south');
  const controlEventsVerified: boolean = events.map(({ type }) => type).join(',')
    === 'magic-cast,minion-control-changed,magic-resolved'
    && castPayload?.instanceId === opening.mesmerismInstanceId
    && castPayload.manaPaid === 4
    && castPayload.seat === 'north'
    && castPayload.targetInstanceId === opening.kettletopInstanceId
    && castPayload.targetSeat === 'south'
    && changedPayload?.fromSeat === 'south'
    && changedPayload.instanceId === opening.kettletopInstanceId
    && changedPayload.seat === 'north'
    && changedPayload.sourceInstanceId === opening.mesmerismInstanceId
    && resolvedPayload?.instanceId === opening.mesmerismInstanceId;
  const controlTransferred: boolean = targetBefore.owner === 'south'
    && targetBefore.controller === 'south'
    && targetAfter.owner === 'south'
    && targetAfter.controller === 'north'
    && targetAfter.cardId === targetBefore.cardId
    && targetAfter.location === targetBefore.location
    && targetAfter.region === targetBefore.region
    && targetAfter.damage === targetBefore.damage
    && targetAfter.stealthed === targetBefore.stealthed
    && targetAfter.summoningSickness === targetBefore.summoningSickness
    && targetAfter.tapped === targetBefore.tapped
    && targetAfter.warded === targetBefore.warded
    && farAfter.owner === farBefore.owner
    && farAfter.controller === farBefore.controller;
  const exactNearbyTarget: boolean = targetBefore.location === 'C2'
    && targetBefore.region === 'surface'
    && choices.length === 1;
  const farTargetUnavailable: boolean = farBefore.location === 'C1'
    && choices.every(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId !== opening.farSeravaInstanceId);
  const oldControllerLostAction: boolean = southActions.every(({ descriptor }) =>
    descriptor.kind !== 'move-and-attack'
      || descriptor.unitInstanceId !== opening.kettletopInstanceId)
    && southActions.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === opening.farSeravaInstanceId);
  const playersBeforeDeathrite = session.state.players;
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === opening.farSeravaInstanceId
    && descriptor.from.cell === 'C1'
    && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === opening.kettletopInstanceId);
  const fight = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  if (!fight.accepted) throw new Error('private Mesmerism Deathrite fight was rejected');
  session = fight.session;
  const deathEvents = fight.receipt.events;
  const drawIndex = deathEvents.findIndex(({ payload, type }) =>
    type === 'site-drawn'
      && isJsonRecord(payload)
      && payload.seat === 'north'
      && payload.sourceInstanceId === opening.kettletopInstanceId);
  const deathIndex = deathEvents.findIndex(({ payload, type }) =>
    type === 'minion-died'
      && isJsonRecord(payload)
      && payload.instanceId === opening.kettletopInstanceId
      && payload.owner === 'south');
  const deathriteControllerDrewSite: boolean =
    session.state.players.north.atlas.length === playersBeforeDeathrite.north.atlas.length - 1
    && session.state.players.north.hand.atlas.length
      === playersBeforeDeathrite.north.hand.atlas.length + 1
    && session.state.players.south.atlas.length === playersBeforeDeathrite.south.atlas.length
    && session.state.players.south.hand.atlas.length === playersBeforeDeathrite.south.hand.atlas.length;
  const deathriteOwnerKeptCemetery: boolean = session.state.players.south.cemetery
    .some(({ instanceId }) => instanceId === opening.kettletopInstanceId)
    && session.state.players.north.cemetery
      .every(({ instanceId }) => instanceId !== opening.kettletopInstanceId)
    && session.state.realm.units.every(({ instanceId }) =>
      instanceId !== opening.kettletopInstanceId);
  const causalEventsVerified: boolean = controlEventsVerified
    && drawIndex >= 0
    && drawIndex < deathIndex;
  const waterAffinityFour: boolean = affinityBefore === 4;

  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    causalEventsVerified,
    controlTransferred,
    deck: deckList(opening.manifest.decks.north, opening.names),
    deathriteControllerDrewSite,
    deathriteOwnerKeptCemetery,
    exactNearbyTarget,
    farTargetUnavailable,
    kettletopLeprechaun: input.deathriteMinion.name,
    manaPaid: manaBefore - manaAfterCast,
    mesmerism: input.mesmerism.name,
    newControllerGainedAction,
    noRandomDraws: session.transcript.every(({ randomDraws }) => randomDraws.length === 0),
    oldControllerHadAction,
    oldControllerLostAction,
    replayVerified: verifyGameReplay(session),
    seed: opening.manifest.seed,
    seravaTownsfolk: input.seravaTownsfolk.name,
    waterAffinityFour,
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
  const airStarter = runStarter(
    input,
    'air-starter',
    input.config.airSeed,
    input.spire,
    input.stealthTargetMinion,
  );
  const airFireFatality = runAirFireFatality(input);
  const airBladderblimp = runAirBladderblimp(input);
  const airGenesisSpell = runAirGenesisSpell(input);
  const airSpellcasterFreeze = runAirSpellcasterFreeze(input);
  const airArcLightning = runAirArcLightning(input);
  const airLightningBolt = runAirLightningBolt(input);
  const airRainOfArrows = runAirRainOfArrows(input);
  const airStaticServant = runAirStaticServant(input);
  const airTeleport = runAirTeleport(input);
  const airLeyline = runAirLeyline(input);
  const airborne = runAirborne(input);
  const airMovement = runAirMovement(input);
  const airMovementTwo = runAirMovementTwo(input);
  const airSummoning = runAirSummoning(input);
  const airVoidArtifact = runAirVoidArtifact(input);
  const airVoidwalk = runAirVoidwalk(input);
  const airZap = runAirZap(input);
  const earthBurrowing = runEarthBurrowing(input);
  const earthStarter = runStarter(
    input,
    'earth-starter',
    input.config.earthSeed,
    input.humbleVillage,
    input.wildBoars,
  );
  const earthOverpower = runEarthOverpower(input);
  const earthBury = runEarthBury(input);
  const earthBorderMilitia = runEarthBorderMilitia(input);
  const earthHumbleVillage = runEarthHumbleVillage(input);
  const earthDuel = runEarthDuel(input);
  const earthHuntersLodge = runEarthHuntersLodge(input);
  const earthPoisonousDagger = runEarthPoisonousDagger(input);
  const earthSwordAndShield = runEarthSwordAndShield(input);
  const earthRescue = runEarthRescue(input);
  const earthDivineHealing = runEarthDivineHealing(input);
  const earthGrainSparrow = runEarthGrainSparrow(input);
  const earthShallowGrave = runEarthShallowGrave(input);
  const earthSinkhole = runEarthSinkhole(input);
  const earthEntombed = runEarthEntombed(input);
  const earthFirstStrike = runEarthFirstStrike(input);
  const earthForwardMovement = runEarthForwardMovement(input);
  const earthImmobile = runEarthImmobile(input);
  const earthRamp = runEarthRamp(input);
  const earthMalakhim = runEarthMalakhim(input);
  const earthRanged = runEarthRanged(input);
  const earthSecretTunnel = runEarthSecretTunnel(input);
  const earthWard = runEarthWard(input);
  const fireStarter = runStarter(
    input,
    'fire-starter',
    input.config.fireSeed,
    input.wasteland,
    input.raalDromedary,
  );
  const fireGranaryRats = runFireGranaryRats(input);
  const fireHamlet = runFireHamlet(input);
  const fireAramos = runFireAramos(input);
  const fireCharge = runFireCharge(input);
  const fireGenesisLifeLoss = runFireGenesisLifeLoss(input);
  const fireVileImp = runFireVileImp(input);
  const fireIgnited = runFireIgnited(input);
  const fireLash = runFireLash(input);
  const fireLeapAttack = runFireLeapAttack(input);
  const fireMinorExplosion = runFireMinorExplosion(input);
  const fireVikings = runFireVikings(input);
  const fireRecklessSquire = runFireRecklessSquire(input);
  const fireResponse = runFireResponse(input);
  const stealth = runStealth(input);
  const waterDrowned = runWaterDrowned(input);
  const waterDrown = runWaterDrown(input);
  const waterEdgeConnection = runWaterEdgeConnection(input);
  const waterLugbog = runWaterLugbog(input);
  const waterLure = runWaterLure(input);
  const waterMesmerism = runWaterMesmerism(input);
  const waterPirateShip = runWaterPirateShip(input);
  const waterEndTurnStealth = runWaterEndTurnStealth(input);
  const waterFreeze = runWaterFreeze(input);
  const waterGnarledWendigo = runWaterGnarledWendigo(input);
  const waterHealing = runWaterHealing(input);
  const waterRiver = runWaterRiver(input);
  const waterSidewaysMovement = runWaterSidewaysMovement(input);
  const waterSubmerge = runWaterSubmerge(input);
  const waterStarter = runStarter(
    input,
    'water-starter',
    input.config.waterSeed,
    input.autumnRiver,
    input.seravaTownsfolk,
  );
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
    airStarter,
    airBladderblimp,
    airGenesisSpell,
    airSpellcasterFreeze,
    airArcLightning,
    airLightningBolt,
    airRainOfArrows,
    airStaticServant,
    airTeleport,
    airLeyline,
    airborne,
    airMovement,
    airMovementTwo,
    airSummoning,
    airVoidArtifact,
    airVoidwalk,
    airZap,
    airFireFatality,
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
    earthStarter,
    earthOverpower,
    earthBury,
    earthBorderMilitia,
    earthHumbleVillage,
    earthDuel,
    earthHuntersLodge,
    earthPoisonousDagger,
    earthSwordAndShield,
    earthRescue,
    earthDivineHealing,
    earthGrainSparrow,
    earthShallowGrave,
    earthSinkhole,
    earthEntombed,
    earthRamp,
    earthMalakhim,
    earthFirstStrike,
    earthForwardMovement,
    earthImmobile,
    earthRanged,
    earthSecretTunnel,
    earthWard,
    fireAramos,
    fireCharge,
    fireGenesisLifeLoss,
    fireVileImp,
    fireGranaryRats,
    fireHamlet,
    fireIgnited,
    fireLash,
    fireLeapAttack,
    fireMinorExplosion,
    fireVikings,
    fireRecklessSquire,
    fireResponse,
    fireStarter,
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
    waterMesmerism,
    waterPirateShip,
    waterEndTurnStealth,
    waterFreeze,
    waterGnarledWendigo,
    waterHealing,
    waterRiver,
    waterSidewaysMovement,
    waterSubmerge,
    waterStarter,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await runPrivateGameCheck(process.argv[2]);
  process.stdout.write(`${canonicalJson(result as unknown as JsonValue)}\n`);
}
