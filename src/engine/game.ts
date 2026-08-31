import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  createAttempt,
  createEvents,
  createReceipt,
  createRejection,
  deepFreeze,
  opaqueActionId,
  orderLegalActions,
  type EngineActionRequest,
  type EngineAttempt,
  type EngineEvent,
  type EngineLegalAction,
  type EngineRandomDraw,
  type EngineReceipt,
  type EngineRejection,
  type EngineSeat,
  type StateHash,
} from './contract.ts';
import { createEngineState, drawUint32, type EngineState } from './determinism.ts';

const UINT32_RANGE = 0x1_0000_0000;
const MAX_DECK_CARDS = 200;
const MAX_COMBAT_STAT = 100;
const CHAIN_MAGIC_DAMAGE = 2;
const CHAIN_MAGIC_EXTRA_TARGET_MANA = 2;
const NORTH_START = 'C4';
const SOUTH_START = 'C1';

export type GameSeat = EngineSeat;
export type DeckZone = 'atlas' | 'spellbook';
export type RealmCell = `${'A' | 'B' | 'C' | 'D' | 'E'}${1 | 2 | 3 | 4}`;
export type GameElement = 'air' | 'earth' | 'fire' | 'water';
export type GameThresholds = Readonly<Record<GameElement, number>>;
export type GameRegion = 'surface' | 'underground' | 'underwater' | 'void';
type MovementPurpose = 'defend' | 'effect' | 'move-and-attack';
export type TwoByTwoArea = readonly [RealmCell, RealmCell, RealmCell, RealmCell];

const REALM_FILES = ['A', 'B', 'C', 'D', 'E'] as const;
const REALM_RANKS = [1, 2, 3, 4] as const;
const REALM_CELLS = REALM_FILES
  .flatMap((file) => REALM_RANKS.map((rank) => `${file}${rank}` as RealmCell));
const TWO_BY_TWO_AREAS: readonly TwoByTwoArea[] = REALM_FILES.slice(0, -1)
  .flatMap((file, fileIndex) => REALM_RANKS.slice(0, -1).map((rank, rankIndex) => [
    `${file}${rank}` as RealmCell,
    `${file}${REALM_RANKS[rankIndex + 1]!}` as RealmCell,
    `${REALM_FILES[fileIndex + 1]!}${rank}` as RealmCell,
    `${REALM_FILES[fileIndex + 1]!}${REALM_RANKS[rankIndex + 1]!}` as RealmCell,
  ] as TwoByTwoArea));

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
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower: 2;
    manaCost: number;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal: true;
    grantsBearerPower?: never;
    manaCost: number;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps: true;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife: number;
    bearerControllerChoosesExtraRandomOutcome?: never;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps?: never;
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps?: never;
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath?: never;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfEachTurnSiteControllerLosesLife?: never;
    bearerControllerChoosesExtraRandomOutcome: true;
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower?: never;
    manaCost: number;
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
  }>
  | Readonly<{
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep?: never;
    cardType: 'aura';
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: true;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep: 3;
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
    discardSiteAsAdditionalCost?: true;
    disableTargetNearbyMinionUntilNextTurn?: true;
    fightAllyWithAdjacentEnemy?: true;
    gainControlOfTargetNearbyMinion?: true;
    grantChargeToAllyThisTurn?: true;
    grantPowerToAllyThisTurn?: 2;
    healController?: number;
    killTargetWoundedMinion?: true;
    leapAttackAlly?: true;
    lureEnemyMinionOneStepCloser?: true;
    manaCost: number;
    returnMinionFromOwnCemetery?: true;
    submergeTargetMinion?: true;
    summonRandomMinionFromAnyCemetery?: true;
    summonTokenToEachControlledSiteBorderingEnemySite?: string;
    destroyTargetSite?: true;
    targetNearby?: boolean;
    teleportAllyToTargetSite?: true;
    teleportNearbyAllyThenDrawCard?: true;
    thresholds: GameThresholds;
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
    deathriteLoseLifePerNearbySiteControlled?: 1;
    defense: number;
    discardSpellToDamageRandomOtherUnitHere?: number;
    discardRandomCardInsteadOfMana?: true;
    diesAtEndOfControllerTurn?: true;
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
    immobile?: boolean;
    lanceCount?: 1 | 2 | 3;
    lethal?: boolean;
    manaCost: number;
    atStartOfControllerTurnTeleportToRandomSiteOrVoid?: true;
    mayRangedStrikeOnceDuringBasicMovement?: true;
    mayStepAfterRangedStrike?: true;
    mortal?: true;
    movementBonus?: 1 | 2;
    movesOnlyForward?: boolean;
    movesOnlySideways?: boolean;
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
    shootsDragProjectile?: boolean;
    siteProvidesNoThreshold?: true;
    spellcaster?: boolean;
    stealth?: boolean;
    strikesFirstWhileAttacking?: boolean;
    submerge?: boolean;
    summonToAnySite?: boolean;
    mustBeCastToOuterColumn?: boolean;
    tapToDamageEachUnitAtAdjacentLocation?: 2;
    tapForMana?: number;
    takesLessDamage?: 1;
    thresholds: GameThresholds;
    token?: true;
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
  cells: TwoByTwoArea;
  controller: GameSeat;
  turnCounters: number;
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
  temporaryChargeSources?: readonly StateHash[];
  temporaryPowerSources?: readonly StateHash[];
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

type GamePhase = 'allocate' | 'attack' | 'cemetery-summon' | 'chain-magic' | 'deathrite-order' | 'defend' | 'draw' | 'end-turn-aura' | 'genesis' | 'intercept' | 'main' | 'movement' | 'mulligan' | 'random-choice' | 'ranged-step' | 'start-turn' | 'terminal';

type PendingDeathrites = Readonly<{
  batches: readonly PendingDeathriteBatch[];
  blinkContinuation?: Readonly<{
    cardId: string;
    instanceId: StateHash;
    owner: GameSeat;
    seat: GameSeat;
    zone: DeckZone;
  }>;
  corpses: readonly UnitInstance[];
  deckLosers: readonly GameSeat[];
  deferredOutcomes?: readonly GameOutcome[];
  endTurnContinuation?: Readonly<{
    remainingInstanceIds: readonly StateHash[];
    seat: GameSeat;
  }>;
  defeatedAvatars: readonly GameSeat[];
  firstStrikeContinuation?: Readonly<{
    attackerStrikesFirst: boolean;
    firstCombatantInstanceIds: readonly StateHash[];
    pending: PendingCombat;
  }>;
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

type DamageAllocationSource = 'attacker-unit' | 'non-unit';

type DamageSourceSnapshot =
  | Readonly<{ currentPower: number; instanceId: StateHash; kind: 'unit' }>
  | Readonly<{ kind: 'non-unit' }>;

type DamageContribution = Readonly<{
  amount: number;
  lethal: boolean;
  source: DamageSourceSnapshot;
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
      cells: TwoByTwoArea;
      controller: GameSeat;
      instanceId: StateHash;
      owner: GameSeat;
      turnCounters: number;
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

type GameActionDescriptor =
  | MulliganDescriptor
  | Readonly<{ kind: 'draw-site' }>
  | Readonly<{ kind: 'draw-spell' }>
  | Readonly<{
    cardId: string;
    cardInstanceId: string;
    cell: RealmCell;
    createRubbleAt?: RealmCell;
    fromTopAtlas?: true;
    genesisTokenChoice?: 'decline' | 'defer' | 'pay-one-mana';
    kind: 'play-site';
  }>
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
  | Readonly<{
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
  }>
  | Readonly<{
    cardId: string;
    cardInstanceId: string;
    casterInstanceId: StateHash;
    cells: TwoByTwoArea;
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
    sourceInstanceId: StateHash;
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
    cells?: TwoByTwoArea;
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

function otherSeat(seat: GameSeat): GameSeat {
  return seat === 'north' ? 'south' : 'north';
}

function borderingCells(cell: RealmCell, connectsTopBottom = false): readonly RealmCell[] {
  const file = cell.charCodeAt(0);
  const rank = Number(cell[1]);
  return [
    [file - 1, rank],
    [file, rank - 1],
    [file + 1, rank],
    [file, rank + 1],
    ...(connectsTopBottom && rank === 1
      ? [[file, 4]]
      : connectsTopBottom && rank === 4 ? [[file, 1]] : []),
  ].filter(([nextFile, nextRank]) =>
    nextFile! >= 65 && nextFile! <= 69 && nextRank! >= 1 && nextRank! <= 4)
    .map(([nextFile, nextRank]) => `${String.fromCharCode(nextFile!)}${nextRank}` as RealmCell);
}

function diagonalCells(cell: RealmCell, connectsTopBottom = false): readonly RealmCell[] {
  const file = cell.charCodeAt(0);
  const rank = Number(cell[1]);
  return [
    [file - 1, rank - 1],
    [file - 1, rank + 1],
    [file + 1, rank - 1],
    [file + 1, rank + 1],
    ...(connectsTopBottom && rank === 1
      ? [[file - 1, 4], [file + 1, 4]]
      : connectsTopBottom && rank === 4 ? [[file - 1, 1], [file + 1, 1]] : []),
  ].filter(([nextFile, nextRank]) =>
    nextFile! >= 65 && nextFile! <= 69 && nextRank! >= 1 && nextRank! <= 4)
    .map(([nextFile, nextRank]) => `${String.fromCharCode(nextFile!)}${nextRank}` as RealmCell);
}

function cardinalCellDistance(left: RealmCell, right: RealmCell): number {
  return Math.abs(left.charCodeAt(0) - right.charCodeAt(0))
    + Math.abs(Number(left[1]) - Number(right[1]));
}

function isRubble(site: RealmSiteInstance): site is RubbleInstance {
  return 'rubble' in site;
}

function legalSiteCells(state: GameState, seat: GameSeat): readonly RealmCell[] {
  const controlledCells = Object.entries(state.realm.sites)
    .filter(([, site]) => site.controller === seat)
    .map(([cell]) => cell as RealmCell);
  if (controlledCells.length === 0) {
    const available = REALM_CELLS
      .filter((cell) => !state.realm.sites[cell] || isRubble(state.realm.sites[cell]!));
    const minimumDistance = Math.min(...available
      .map((cell) => cardinalCellDistance(state.players[seat].avatar.location, cell)));
    return available
      .filter((cell) => cardinalCellDistance(state.players[seat].avatar.location, cell) === minimumDistance)
      .sort();
  }
  return [...new Set(controlledCells
    .flatMap((cell) => borderingCells(cell)))]
    .filter((cell) => !state.realm.sites[cell] || isRubble(state.realm.sites[cell]!))
    .sort();
}

function controlledSiteCells(state: GameState, seat: GameSeat): readonly RealmCell[] {
  return Object.entries(state.realm.sites)
    .filter(([, site]) => site.controller === seat)
    .map(([cell]) => cell as RealmCell)
    .sort();
}

function isWaterSite(state: GameState, cell: RealmCell): boolean {
  const site = state.realm.sites[cell];
  if (!site || isRubble(site)) return false;
  const definition = cardDefinition(state, site.cardId);
  if (definition.cardType !== 'site') throw new Error('realm site lacks site definition');
  return definition.elements.includes('water');
}

function siteCannotBeMovedDestroyedOrModified(state: GameState, site: SiteInstance): boolean {
  const definition = cardDefinition(state, site.cardId);
  if (definition.cardType !== 'site') throw new Error('realm site lacks site definition');
  return definition.cannotBeMovedDestroyedOrModified === true;
}

function meetsThresholds(state: GameState, seat: GameSeat, required: GameThresholds): boolean {
  const available = affinity(state, seat);
  return (['air', 'earth', 'fire', 'water'] as const)
    .every((element) => available[element] >= required[element]);
}

function nonemptyCombinations(
  instanceIds: readonly StateHash[],
  maximumCount: number,
): readonly (readonly StateHash[])[] {
  const combinations: StateHash[][] = [];
  const choose = (start: number, remaining: number, chosen: StateHash[]): void => {
    if (remaining === 0) {
      combinations.push([...chosen]);
      return;
    }
    for (let index = start; index <= instanceIds.length - remaining; index += 1) {
      chosen.push(instanceIds[index]!);
      choose(index + 1, remaining - 1, chosen);
      chosen.pop();
    }
  };
  for (let count = 1; count <= maximumCount; count += 1) choose(0, count, []);
  return combinations;
}

function minionManaCostAtSite(
  state: GameState,
  definition: Extract<GameCardDefinition, Readonly<{ cardType: 'minion' }>>,
  cell: RealmCell,
): number {
  const site = state.realm.sites[cell];
  const siteDefinition = site && !isRubble(site) ? cardDefinition(state, site.cardId) : undefined;
  const discount = definition.ordinary === true
    && siteDefinition?.cardType === 'site'
    && siteDefinition.ordinaryMinionManaDiscount === 1
    ? 1
    : 0;
  return Math.max(0, definition.manaCost - discount);
}

type SummonLocation = Readonly<{
  cell: RealmCell;
  cells?: TwoByTwoArea;
  region?: 'underground' | 'underwater' | 'void';
}>;

function minionSummonLocations(
  state: GameState,
  seat: GameSeat,
  definition: Extract<GameCardDefinition, Readonly<{ cardType: 'minion' }>>,
  anywhere = false,
): readonly SummonLocation[] {
  const controlledCells = controlledSiteCells(state, seat);
  const siteCells = Object.keys(state.realm.sites).sort() as RealmCell[];
  const summonCells = (anywhere || definition.summonToAnySite ? siteCells : controlledCells)
    .filter((cell) => anywhere || !definition.mustBeCastToOuterColumn
      || cell[0] === 'A' || cell[0] === 'E')
    .filter((cell) => anywhere || !definition.mustBeCastToWaterSite || isWaterSite(state, cell));
  const locations: readonly SummonLocation[] = definition.occupiesSquareArea === 2
    ? TWO_BY_TWO_AREAS
      .filter((cells) => cells.some((cell) => summonCells.includes(cell)))
      .filter((cells) => footprintLocationExists(state, cells, 'surface'))
      .map((cells) => ({ cell: cells[0], cells }))
    : [
      ...(anywhere || !definition.mustBeCastBurrowed && !definition.mustBeCastSubmerged
        ? summonCells.map((cell) => ({ cell }))
        : []),
      ...(definition.burrowing && (anywhere || !definition.mustBeCastSubmerged)
        ? summonCells.filter((cell) => !isWaterSite(state, cell))
          .map((cell) => ({ cell, region: 'underground' as const }))
        : []),
      ...(definition.submerge && (anywhere || !definition.mustBeCastBurrowed)
        ? summonCells.filter((cell) => isWaterSite(state, cell))
          .map((cell) => ({ cell, region: 'underwater' as const }))
        : []),
      ...(definition.voidwalk
        && (anywhere || !definition.mustBeCastBurrowed && !definition.mustBeCastSubmerged
          && !definition.mustBeCastToWaterSite)
        ? REALM_CELLS.filter((cell) => !state.realm.sites[cell]
          && (anywhere || !definition.mustBeCastToOuterColumn
            || cell[0] === 'A' || cell[0] === 'E'))
          .map((cell) => ({ cell, region: 'void' as const }))
        : []),
    ];
  return locations.filter(({ cell, cells, region }) =>
    (cells ?? [cell]).every((enteredCell) => unitEntryAllowed(
      state,
      undefined,
      { cell: enteredCell, region: region ?? 'surface' },
      definition.airborne === true,
      true,
      definition.attack,
      'summon',
    )));
}

function genesisDamageChoices(
  state: GameState,
  seat: GameSeat,
  definition: Extract<GameCardDefinition, Readonly<{ cardType: 'minion' }>>,
  instanceId: StateHash,
  cell: RealmCell,
  region: GameRegion,
): readonly Readonly<{
  genesisDamageChoice?: 'decline' | 'target';
  genesisDamageTarget?: GameUnitRef;
}>[] {
  if (definition.genesisMayDamageTargetAdjacentUnit !== 2) return [{}];
  return [
    { genesisDamageChoice: 'decline' as const },
    ...[
      { instanceId, kind: 'minion' as const, seat },
      ...(['north', 'south'] as const)
        .flatMap((targetSeat) => unitRefs(state, targetSeat))
        .filter((target) => {
          const status = unitStatus(state, target);
          return status.region === region
            && status.occupiedCells.some((occupiedCell) =>
              occupiedCell === cell || borderingCells(cell).includes(occupiedCell))
            && (target.seat === seat || !status.stealthed);
        }),
    ].sort((left, right) => left.instanceId.localeCompare(right.instanceId))
      .map((genesisDamageTarget) => ({
        genesisDamageChoice: 'target' as const,
        genesisDamageTarget,
      })),
  ];
}

function summonDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const casters = spellcasterRefs(state, seat);
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'minion'
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    const legalSummonLocations = minionSummonLocations(state, seat, definition);
    return casters.flatMap(({ instanceId: casterInstanceId }) =>
      legalSummonLocations.flatMap(({ cell, cells, region }) => {
        const exactRegion: GameRegion = region ?? 'surface';
        const baseManaCost = minionManaCostAtSite(state, definition, cell);
        const basePaymentOptions: readonly Readonly<{
          manaCost: number;
          paymentMode?: 'random-card-discard';
        }>[] = [
          ...(player.mana >= baseManaCost ? [{ manaCost: baseManaCost }] : []),
          ...(definition.discardRandomCardInsteadOfMana === true
            && (player.hand.atlas.length > 0
              || player.hand.spellbook.some((candidate) => candidate.instanceId !== instanceId))
            ? [{ manaCost: 0, paymentMode: 'random-card-discard' as const }]
            : []),
        ];
        const sacrificeCandidates = state.realm.units
          .filter((unit) => unit.controller === seat
            && unitOccupiedCells(unit).includes(cell)
            && unit.region === exactRegion)
          .map(({ instanceId: candidateId }) => candidateId)
          .sort();
        // ponytail: raise this useful-cost ceiling only if a future ruling permits
        // gratuitous sacrifices after a spell's mana cost has already reached zero.
        const maximumSacrifices = definition
          .sacrificeMinionAtSummoningLocationForManaDiscount === 2
          ? Math.min(sacrificeCandidates.length, Math.ceil(baseManaCost / 2))
          : 0;
        const sacrificePayments = nonemptyCombinations(sacrificeCandidates, maximumSacrifices)
          .map((sacrificedMinionInstanceIds) => ({
            manaCost: Math.max(0, baseManaCost - (2 * sacrificedMinionInstanceIds.length)),
            sacrificedMinionInstanceIds,
          }))
          .filter(({ sacrificedMinionInstanceIds }) => !deathsMayRequireDeathriteContinuation(
            state,
            state.realm.units.filter(({ instanceId: candidateId }) =>
              sacrificedMinionInstanceIds.includes(candidateId)),
          ))
          .filter(({ manaCost }) => player.mana >= manaCost);
        const genesisChoices = genesisDamageChoices(
          state,
          seat,
          definition,
          instanceId,
          cell,
          exactRegion,
        );
        return [...basePaymentOptions, ...sacrificePayments].flatMap((payment) =>
          genesisChoices.map((choice) => ({
            cardId,
            cardInstanceId: instanceId,
            casterInstanceId,
            cell,
            ...(cells ? { cells } : {}),
            ...choice,
            kind: 'summon-minion' as const,
            manaCost: payment.manaCost,
            ...('paymentMode' in payment && payment.paymentMode
              ? { paymentMode: payment.paymentMode }
              : {}),
            ...(region ? { region } : {}),
            ...('sacrificedMinionInstanceIds' in payment
              ? { sacrificedMinionInstanceIds: payment.sacrificedMinionInstanceIds }
              : {}),
          })));
      }));
  });
}

function cemeteryMinionCandidates(state: GameState): readonly CardInstance[] {
  return (['north', 'south'] as const)
    .flatMap((seat) => state.players[seat].cemetery)
    .filter(({ cardId }) => cardDefinition(state, cardId).cardType === 'minion')
    .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
}

function cemeterySummonDescriptors(
  state: GameState,
  seat: GameSeat,
): readonly GameActionDescriptor[] {
  const pending = state.pendingCemeterySummon;
  if (!pending || pending.seat !== seat) {
    throw new Error('unreachable missing pending cemetery summon');
  }
  const card = state.players[pending.cardOwner].cemetery.find(({ instanceId }) =>
    instanceId === pending.cardInstanceId);
  const definition = card && cardDefinition(state, card.cardId);
  if (!card || !definition || definition.cardType !== 'minion') {
    throw new Error('unreachable missing selected cemetery minion');
  }
  return minionSummonLocations(state, seat, definition, true)
    .flatMap(({ cell, cells, region }) => genesisDamageChoices(
      state,
      seat,
      definition,
      card.instanceId,
      cell,
      region ?? 'surface',
    ).map((choice) => ({
      cardId: card.cardId,
      cardInstanceId: card.instanceId,
      casterInstanceId: pending.casterInstanceId,
      cell,
      ...(cells ? { cells } : {}),
      ...choice,
      kind: 'summon-minion' as const,
      manaCost: 0,
      ...(region ? { region } : {}),
    })));
}

function artifactDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const casters = spellcasterRefs(state, seat);
  const bearers = unitRefs(state, seat);
  const cells = controlledSiteCells(state, seat);
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'artifact'
      || player.mana < definition.manaCost
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    return casters.flatMap(({ instanceId: casterInstanceId }) => [
      ...cells.map((cell) => ({
        cardId,
        cardInstanceId: instanceId,
        casterInstanceId,
        cell,
        kind: 'cast-artifact' as const,
        manaCost: definition.manaCost,
      })),
      ...bearers.flatMap((bearer) => {
        const cells = unitStatus(state, bearer).occupiedCells;
        return cells.map((bearerCell) => ({
          bearer,
          ...(cells.length > 1 ? { bearerCell } : {}),
          cardId,
          cardInstanceId: instanceId,
          casterInstanceId,
          kind: 'cast-artifact' as const,
          manaCost: definition.manaCost,
        }));
      }),
    ]);
  });
}

function artifactDamageAbilityDescriptors(
  state: GameState,
  seat: GameSeat,
): readonly GameActionDescriptor[] {
  const allies = unitRefs(state, seat);
  const targets = (['north', 'south'] as const).flatMap((targetSeat) =>
    unitRefs(state, targetSeat));
  return (state.realm.artifacts ?? []).flatMap((artifact) => {
    if (!('bearer' in artifact) || artifact.bearer.seat !== seat) return [];
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps !== 3
      || !readyUnit(state, artifact.bearer)) return [];
    const bearer = unitStatus(state, artifact.bearer);
    const carriedAt = artifactLocation(state, artifact);
    const locations = new Set(locationsWithinMeasuredSteps(
      state,
      carriedAt,
      2,
    ).map(({ cell }) => cell));
    const legalTargets = targets.filter((target) => {
      const status = unitStatus(state, target);
      return status.region === bearer.region
        && status.occupiedCells.some((cell) => locations.has(cell))
        && (target.seat === seat || !status.stealthed);
    });
    return allies.filter((helper) => {
      const status = unitStatus(state, helper);
      return helper.instanceId !== artifact.bearer.instanceId
        && readyUnit(state, helper)
        && status.occupiedCells.includes(carriedAt.cell)
        && status.region === bearer.region;
    }).flatMap((helper) => legalTargets.map((target) => ({
      artifactInstanceId: artifact.instanceId,
      helper,
      kind: 'activate-artifact-damage' as const,
      target,
    })));
  });
}

function artifactDiscardAreaDamageAbilityDescriptors(
  state: GameState,
  seat: GameSeat,
): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const allies = unitRefs(state, seat);
  const discards = (['atlas', 'spellbook'] as const).flatMap((discardZone) =>
    player.hand[discardZone].map(({ instanceId: discardCardInstanceId }) => ({
      discardCardInstanceId,
      discardZone,
    })));
  return (state.realm.artifacts ?? []).flatMap((artifact) => {
    if (!('bearer' in artifact) || artifact.bearer.seat !== seat) return [];
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition
        .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps !== true
      || !readyUnit(state, artifact.bearer)) return [];
    const bearer = unitStatus(state, artifact.bearer);
    const carriedAt = artifactLocation(state, artifact);
    const targetLocations = locationsWithinMeasuredSteps(
      state,
      carriedAt,
      3,
    );
    return allies.filter((helper) => {
      const status = unitStatus(state, helper);
      return helper.instanceId !== artifact.bearer.instanceId
        && readyUnit(state, helper)
        && status.occupiedCells.includes(carriedAt.cell)
        && status.region === bearer.region;
    }).flatMap((helper) => discards.flatMap(({ discardCardInstanceId, discardZone }) =>
      targetLocations.map((targetLocation) => ({
        artifactInstanceId: artifact.instanceId,
        discardCardInstanceId,
        discardZone,
        helper,
        kind: 'activate-artifact-discard-area-damage' as const,
        targetLocation,
      }))));
  });
}

function artifactRollDamageAbilityDescriptors(
  state: GameState,
  seat: GameSeat,
): readonly GameActionDescriptor[] {
  const pushers = unitRefs(state, seat);
  const directions = ['east', 'north', 'south', 'west'] as const;
  return (state.realm.artifacts ?? []).flatMap((artifact) => {
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath !== 4) {
      return [];
    }
    const origin = artifactLocation(state, artifact);
    return pushers.filter((pusher) => {
      const status = unitStatus(state, pusher);
      return readyUnit(state, pusher)
        && status.occupiedCells.includes(origin.cell)
        && status.region === origin.region;
    }).flatMap((pusher) => directions.map((direction) => {
      const path: GameLocation[] = [origin];
      const seen = new Set<RealmCell>([origin.cell]);
      while (true) {
        const nextCell = projectileStep(path.at(-1)!.cell, direction);
        const next = nextCell ? { cell: nextCell, region: origin.region } : undefined;
        if (!next || seen.has(next.cell) || !locationExists(state, next)) break;
        path.push(next);
        seen.add(next.cell);
      }
      return {
        artifactInstanceId: artifact.instanceId,
        direction,
        kind: 'activate-artifact-roll-damage' as const,
        path,
        pusher,
      };
    }));
  });
}

function pickUpArtifactDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const uncarried = (state.realm.artifacts ?? [])
    .flatMap((artifact) => 'bearer' in artifact ? [] : [artifact]);
  return unitRefs(state, seat).flatMap((unit) => {
    const status = unitStatus(state, unit);
    const lastUsedTurn = unit.kind === 'avatar'
      ? state.players[seat].avatar.lastPickedUpArtifactsTurn
      : state.realm.units.find(({ instanceId }) =>
        instanceId === unit.instanceId)?.lastPickedUpArtifactsTurn;
    if (status.disabled || lastUsedTurn === state.turnNumber) return [];
    return status.occupiedCells.flatMap((cell) => {
      const artifactInstanceIds = uncarried
        .filter(({ location, region }) => location === cell && region === status.region)
        .map(({ instanceId }) => instanceId)
        .sort();
      // ponytail: all subsets are exponential; use staged selection if supported local Artifact counts grow large.
      return nonemptyCombinations(artifactInstanceIds, artifactInstanceIds.length)
        .map((ids) => ({
          artifactInstanceIds: ids,
          ...(status.occupiedCells.length > 1 ? { cell } : {}),
          kind: 'pick-up-artifacts' as const,
          unit,
        }));
    });
  });
}

function dropArtifactDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const artifacts = state.realm.artifacts ?? [];
  return unitRefs(state, seat).flatMap((unit) => {
    const status = unitStatus(state, unit);
    const tracked = unit.kind === 'avatar'
      ? state.players[seat].avatar
      : state.realm.units.find(({ instanceId }) => instanceId === unit.instanceId);
    if (status.disabled
      || tracked?.lastDroppedArtifactsTurn === state.turnNumber
      || tracked?.lastInteractedTurn === state.turnNumber) return [];
    const artifactInstanceIds = artifacts
      .filter((artifact) => 'bearer' in artifact
        && artifact.bearer.instanceId === unit.instanceId
        && artifact.bearer.kind === unit.kind
        && artifact.bearer.seat === unit.seat)
      .map(({ instanceId }) => instanceId)
      .sort();
    return nonemptyCombinations(artifactInstanceIds, artifactInstanceIds.length)
      .map((ids) => ({ artifactInstanceIds: ids, kind: 'drop-artifacts' as const, unit }));
  });
}

function auraDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const casters = spellcasterRefs(state, seat);
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'aura'
      || player.mana < definition.manaCost
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    return casters.flatMap(({ instanceId: casterInstanceId }) =>
      TWO_BY_TWO_AREAS.map((cells) => ({
        cardId,
        cardInstanceId: instanceId,
        casterInstanceId,
        cells,
        kind: 'cast-aura' as const,
      })));
  });
}

function chainMagicTargets(
  state: GameState,
  seat: GameSeat,
  casterRef: GameUnitRef,
  previousRef: GameUnitRef,
  selected: readonly GameUnitRef[],
): readonly GameUnitRef[] {
  const caster = unitStatus(state, casterRef);
  const previous = unitStatus(state, previousRef);
  const selectedIds = new Set(selected.map(({ instanceId }) => instanceId));
  return (['north', 'south'] as const)
    .flatMap((targetSeat) => unitRefs(state, targetSeat))
    .filter((target) => {
      if (selectedIds.has(target.instanceId)) return false;
      const status = unitStatus(state, target);
      return status.region === caster.region
        && (target.seat === seat || !status.stealthed)
        && footprintNearby(previous.occupiedCells, status.occupiedCells);
    })
    .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
}

function magicDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const casters = spellcasterRefs(state, seat);
  const targets = (['north', 'south'] as const).flatMap((targetSeat) => unitRefs(state, targetSeat));
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'magic'
      || player.mana < definition.manaCost
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    return casters.flatMap<GameActionDescriptor>((casterRef) => {
      const caster = unitStatus(state, casterRef);
      const cast = {
        cardId,
        cardInstanceId: instanceId,
        casterInstanceId: casterRef.instanceId,
        kind: 'cast-magic' as const,
      };
    if (definition.healController !== undefined) return [cast];
    if (definition.grantChargeToAllyThisTurn === true) {
      return unitRefs(state, seat).map((ally) => ({ ...cast, ally }));
    }
    if (definition.grantPowerToAllyThisTurn === 2) {
      return unitRefs(state, seat).map((ally) => ({ ...cast, ally }));
    }
    if (definition.leapAttackAlly === true) {
      return unitRefs(state, seat).flatMap((ally) => {
        const status = unitStatus(state, ally);
        const destinations = movementPaths(
          state,
          { cell: status.location, region: status.region },
          Math.min(1, status.movementSteps),
          seat,
          status.airborne,
          status.movesOnlySideways,
          status.movesOnlyForward,
          status.burrowing,
          status.submerge,
          status.intrinsicVoidwalk,
          status.planarGateVoidwalk,
          status.connectsTopBottom,
          status.immobile,
          ally.kind === 'minion',
          true,
          'effect',
          status.occupiedCells,
          status.attack,
        ).map((path) => path.at(-1)!);
        return [...new Map(destinations.map((allyDestination) => [
          `${allyDestination.cell}:${allyDestination.region}`,
          allyDestination,
        ])).values()].flatMap((allyDestination) => {
          if (status.occupiedCells.length === 1) return [{ ...cast, ally, allyDestination }];
          const destinationCells = translatedFootprint(
            status.occupiedCells,
            status.location,
            allyDestination.cell,
          ) ?? [];
          return destinationCells.map((cell) => ({
            ...cast,
            ally,
            allyDestination,
            allyStrikeLocation: { cell, region: allyDestination.region },
          }));
        });
      });
    }
    if (definition.fightAllyWithAdjacentEnemy === true) {
      return unitRefs(state, seat).flatMap((ally) => {
        const allyStatus = unitStatus(state, ally);
        return unitRefs(state, otherSeat(seat)).filter((target) => {
          const targetStatus = unitStatus(state, target);
          return targetStatus.region === allyStatus.region
            && !targetStatus.stealthed
            && footprintsHereOrBordering(allyStatus.occupiedCells, targetStatus.occupiedCells);
        }).map((target) => ({ ...cast, ally, target }));
      });
    }
    if (definition.gainControlOfTargetNearbyMinion === true) {
      const nearby = new Set([
        caster.location,
        ...borderingCells(caster.location),
        ...diagonalCells(caster.location),
      ]);
      return targets.filter((target) => {
        if (target.kind !== 'minion') return false;
        const status = unitStatus(state, target);
        return status.region === caster.region
          && status.occupiedCells.some((cell) => nearby.has(cell))
          && (target.seat === seat || !status.stealthed);
      }).map((target) => ({ ...cast, target }));
    }
    if (definition.killTargetWoundedMinion === true) {
      return targets.filter((target) => {
        if (target.kind !== 'minion') return false;
        const targetUnit = state.realm.units.find(({ instanceId }) =>
          instanceId === target.instanceId);
        const status = unitStatus(state, target);
        return targetUnit !== undefined
          && targetUnit.damage > 0
          && status.region === caster.region
          && (target.seat === seat || !status.stealthed);
      }).map((target) => ({ ...cast, target }));
    }
    if (definition.lureEnemyMinionOneStepCloser === true) {
      const choices = unitRefs(state, seat).flatMap((ally) => {
        const allyStatus = unitStatus(state, ally);
        return unitRefs(state, otherSeat(seat)).flatMap((temptedEnemy) => {
          if (temptedEnemy.kind !== 'minion') return [];
          const enemyStatus = unitStatus(state, temptedEnemy);
          if (enemyStatus.disabled
            || enemyStatus.region === 'void'
            || !enemyStatus.occupiedCells.some((cell) => state.realm.sites[cell])
            || !footprintNearby(allyStatus.occupiedCells, enemyStatus.occupiedCells)) return [];
          const from: GameLocation = { cell: enemyStatus.location, region: enemyStatus.region };
          const startingDistance = minimumCardinalDistance(
            enemyStatus.occupiedCells,
            allyStatus.occupiedCells,
          );
          const destinations = movementPaths(
            state,
            from,
            1,
            temptedEnemy.seat,
            enemyStatus.airborne,
            enemyStatus.movesOnlySideways,
            enemyStatus.movesOnlyForward,
            enemyStatus.burrowing,
            enemyStatus.submerge,
            enemyStatus.intrinsicVoidwalk,
            enemyStatus.planarGateVoidwalk,
            enemyStatus.connectsTopBottom,
            enemyStatus.immobile,
            true,
            true,
            'effect',
            enemyStatus.occupiedCells,
            enemyStatus.attack,
          ).flatMap((path) => path.length === 2 ? [path[1]!] : [])
            .filter(({ cell }) => {
              const footprint = translatedFootprint(
                enemyStatus.occupiedCells,
                enemyStatus.location,
                cell,
              );
              return footprint !== undefined
                && minimumCardinalDistance(footprint, allyStatus.occupiedCells) < startingDistance;
            });
          return [...new Map(destinations.map((destination) => [
            `${destination.cell}:${destination.region}`,
            destination,
          ])).values()].map((temptedDestination) => ({
            ...cast,
            ally,
            temptedDestination,
            temptedEnemy,
          }));
        });
      });
      return choices.length > 0 ? choices : [cast];
    }
    if (definition.returnMinionFromOwnCemetery === true) {
      const eligible = player.cemetery.filter(({ cardId }) =>
        cardDefinition(state, cardId).cardType === 'minion');
      return eligible.length > 0
        ? eligible.map(({ instanceId: cemeteryMinionInstanceId }) => ({
          ...cast,
          cemeteryMinionInstanceId,
        }))
        : [cast];
    }
    if (definition.summonTokenToEachControlledSiteBorderingEnemySite !== undefined) {
      return [cast];
    }
    if (definition.summonRandomMinionFromAnyCemetery === true) return [cast];
    if (definition.discardSiteAsAdditionalCost === true) {
      if (caster.region === 'void') return [];
      const discardSiteInstanceIds = player.hand.atlas
        .map(({ instanceId: discardSiteInstanceId }) => discardSiteInstanceId)
        .sort();
      return REALM_CELLS.flatMap((cell) => {
        const site = state.realm.sites[cell];
        if (!site
          || caster.region === 'underwater' && !isWaterSite(state, cell)
          || caster.region === 'underground' && isWaterSite(state, cell)) return [];
        return discardSiteInstanceIds.map((discardSiteInstanceId) => ({
          ...cast,
          discardSiteInstanceId,
          targetLocation: { cell, region: caster.region as Exclude<GameRegion, 'void'> },
          targetSiteInstanceId: site.instanceId,
        }));
      });
    }
    if (definition.teleportNearbyAllyThenDrawCard === true) {
      return unitRefs(state, seat).flatMap((ally) => {
        const status = unitStatus(state, ally);
        const selectedCells = [...new Set(status.occupiedCells.flatMap((cell) => [
          cell,
          ...borderingCells(cell),
          ...diagonalCells(cell),
        ]))].sort() as RealmCell[];
        return selectedCells.flatMap((cell) => {
          const targetLocation = { cell, region: status.region };
          const destinationAreas: readonly (TwoByTwoArea | undefined)[] =
            status.occupiedCells.length > 1
              ? destinationFootprintsContaining(state, targetLocation)
              : locationExists(state, targetLocation) ? [undefined] : [];
          const site = state.realm.sites[cell];
          return destinationAreas
            .filter((allyDestinationCells) =>
              (allyDestinationCells ?? [cell])
                .filter((enteredCell) => !status.occupiedCells.includes(enteredCell))
                .every((enteredCell) => unitEntryAllowed(
                  state,
                  { cell: status.location, region: status.region },
                  { cell: enteredCell, region: targetLocation.region },
                  status.airborne,
                  ally.kind === 'minion',
                  status.attack,
                  'teleport',
                )))
            .flatMap((allyDestinationCells) =>
              (['atlas', 'spellbook'] as const).map((drawZone) => ({
              ...cast,
              ally,
              ...(allyDestinationCells ? { allyDestinationCells } : {}),
              drawZone,
              targetLocation,
              ...(site ? { targetSiteInstanceId: site.instanceId } : {}),
              })));
        });
      });
    }
    if (definition.teleportAllyToTargetSite === true) {
      if (caster.region !== 'surface') return [];
      const targetSites = REALM_CELLS.flatMap((cell) => {
        const site = state.realm.sites[cell];
        return site ? [{ site, targetLocation: { cell, region: 'surface' as const } }] : [];
      });
      return unitRefs(state, seat).flatMap((ally) => {
        const status = unitStatus(state, ally);
        return targetSites.flatMap(({ site, targetLocation }) => {
          const destinationAreas: readonly (TwoByTwoArea | undefined)[] =
            status.occupiedCells.length > 1
              ? destinationFootprintsContaining(state, targetLocation)
              : [undefined];
          return destinationAreas
            .filter((allyDestinationCells) =>
              (allyDestinationCells ?? [targetLocation.cell])
                .filter((enteredCell) => !status.occupiedCells.includes(enteredCell))
                .every((enteredCell) => unitEntryAllowed(
                  state,
                  { cell: status.location, region: status.region },
                  { cell: enteredCell, region: targetLocation.region },
                  status.airborne,
                  ally.kind === 'minion',
                  status.attack,
                  'teleport',
                )))
            .map((allyDestinationCells) => ({
              ...cast,
              ally,
              ...(allyDestinationCells ? { allyDestinationCells } : {}),
              targetLocation,
              targetSiteInstanceId: site.instanceId,
            }));
        });
      });
    }
    if (definition.damageChainNearbyUnits === true) {
      return chainMagicTargets(state, seat, casterRef, casterRef, [])
        .map((target) => ({ ...cast, kind: 'begin-chain-magic' as const, target }));
    }
    if (definition.damageEachAbovegroundMinion === 1) return [cast];
    if (definition.damageEachUnitAtLocationWithinTwoSteps !== undefined) {
      return locationsWithinMeasuredSteps(
        state,
        { cell: caster.location, region: caster.region },
        2,
      ).map((targetLocation) => ({ ...cast, targetLocation }));
    }
    if (definition.damageRandomUnitAtLocation !== undefined) {
      return REALM_CELLS
        .map((cell): GameLocation => ({ cell, region: caster.region }))
        .filter((location) => locationExists(state, location))
        .map((targetLocation) => ({ ...cast, targetLocation }));
    }
    if (definition.burrowAllMinionsAndArtifactsAtTargetLandSite === true) {
      if (caster.region !== 'surface') return [];
      return (Object.entries(state.realm.sites) as Array<[RealmCell, RealmSiteInstance]>)
        .filter(([cell]) => !isWaterSite(state, cell))
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([cell, site]) => ({
          ...cast,
          targetLocation: { cell, region: 'surface' as const },
          targetSiteInstanceId: site.instanceId,
        }));
    }
    if (definition.burrowTargetMinionOrArtifact === true) {
      const minions = targets.filter((target) => {
        if (target.kind !== 'minion') return false;
        const status = unitStatus(state, target);
        return status.region === caster.region
          && (target.seat === seat || !status.stealthed);
      }).map((target) => ({ ...cast, target }));
      const artifacts = (state.realm.artifacts ?? []).flatMap((artifact) => {
        const region = 'bearer' in artifact
          ? unitStatus(state, artifact.bearer).region
          : artifact.region;
        return region === caster.region
          ? [{ ...cast, targetArtifactInstanceId: artifact.instanceId }]
          : [];
      });
      return [...minions, ...artifacts];
    }
    return targets.filter((target) => {
      const status = unitStatus(state, target);
      return (!definition.submergeTargetMinion || target.kind === 'minion')
        && (!definition.disableTargetNearbyMinionUntilNextTurn || target.kind === 'minion')
        && (!definition.untapTargetMinionAfterDamage || target.kind === 'minion')
        && status.region === caster.region
        && (target.seat === seat || !status.stealthed)
        && (!definition.targetNearby && !definition.disableTargetNearbyMinionUntilNextTurn
          || footprintNearby(caster.occupiedCells, status.occupiedCells));
      }).map((target) => ({ ...cast, target }));
    });
  });
}

function chainMagicDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const pending = state.pendingChainMagic;
  const player = state.players[seat];
  if (!pending || pending.seat !== seat || pending.targets.length === 0) {
    throw new Error('unreachable missing pending chained Magic');
  }
  const card = player.hand.spellbook.find(({ cardId, instanceId }) =>
    cardId === pending.cardId && instanceId === pending.cardInstanceId);
  const definition = card && cardDefinition(state, card.cardId);
  const caster = spellcasterRefs(state, seat).find(({ instanceId }) =>
    instanceId === pending.casterInstanceId);
  if (!card || definition?.cardType !== 'magic'
    || definition.damageChainNearbyUnits !== true || !caster) {
    throw new Error('unreachable invalid pending chained Magic');
  }
  const finish: GameActionDescriptor = { kind: 'resolve-chain-magic' };
  const nextManaCost = definition.manaCost
    + CHAIN_MAGIC_EXTRA_TARGET_MANA * pending.targets.length;
  if (player.mana < nextManaCost) return [finish];
  return [
    finish,
    ...chainMagicTargets(
      state,
      seat,
      caster,
      pending.targets.at(-1)!,
      pending.targets,
    ).map((target) => ({ kind: 'extend-chain-magic' as const, target })),
  ];
}

function requireCardId(value: string, path: string): void {
  if (!value.trim() || value.length > 256) throw new RangeError(`${path} must be 1-256 characters`);
}

const SUPPORTED_CARD_FIELDS = {
  artifact: new Set(`
    atEndOfEachTurnSiteControllerLosesLife bearerControllerChoosesExtraRandomOutcome cardType
    grantsBearerLethal grantsBearerPower manaCost
    tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
    tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps
    tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath thresholds
  `.trim().split(/\s+/)),
  aura: new Set(`
    atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep cardType
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
    damageUnitsAboveAndBelowTargetSiteByManhattanDistance discardSiteAsAdditionalCost
    destroyTargetSite
    fightAllyWithAdjacentEnemy gainControlOfTargetNearbyMinion grantChargeToAllyThisTurn
    grantPowerToAllyThisTurn healController killTargetWoundedMinion leapAttackAlly
    lureEnemyMinionOneStepCloser manaCost returnMinionFromOwnCemetery submergeTargetMinion
    summonRandomMinionFromAnyCemetery summonTokenToEachControlledSiteBorderingEnemySite
    targetNearby teleportAllyToTargetSite
    teleportNearbyAllyThenDrawCard thresholds untapTargetMinionAfterDamage
  `.trim().split(/\s+/)),
  minion: new Set(`
    airborne atStartOfControllerTurnTeleportToRandomSiteOrVoid attack burrowing cardType
    cannotAttackSites cannotDefend cannotDefendOrIntercept
    charge connectsTopBottom deathriteDamageEachUnitHere deathriteDrawSite deathriteHeal
    deathriteLoseLifePerNearbySiteControlled defense discardRandomCardInsteadOfMana
    discardSpellToDamageRandomOtherUnitHere diesAtEndOfControllerTurn genesisDamageEachOtherUnitHere
    genesisDisableSelfUntilDamaged genesisDrawSite genesisDrawSpells genesisHealController
    genesisLoseControllerLife genesisMayDamageTargetAdjacentUnit genesisStrikeEachEnemyHere
    gainsPowerRangedAndSpellcasterAtopTower gainsStealthAtEndOfTurn immobile lanceCount lethal
    manaCost mayRangedStrikeOnceDuringBasicMovement mayStepAfterRangedStrike mortal movementBonus
    movesOnlyForward movesOnlySideways
    mustBeCastBurrowed mustBeCastSubmerged mustBeCastToOuterColumn mustBeCastToWaterSite
    nearbyEnemiesPermanentlyLoseStealth occupiesSquareArea ordinary otherControlledMortalsPowerBonus
    otherNearbyAlliesPowerBonus preventsDamageFromUnitsWithPowerAtLeast provides ranged
    sacrificeMinionAtSummoningLocationForManaDiscount shootsDragProjectile siteProvidesNoThreshold
    spellcaster stealth strikesFirstWhileAttacking submerge summonToAnySite
    tapToDamageEachUnitAtAdjacentLocation tapForMana takesLessDamage thresholds token
    untapsAtEndOfControllerTurn voidwalk ward waterbound
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
    preventsUnitsWithPowerAtLeastFromEntering
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
    if (card.grantsBearerPower !== undefined && card.grantsBearerPower !== 2) {
      throw new RangeError(`${path}.grantsBearerPower must be 2`);
    }
    if (card.grantsBearerLethal !== undefined && card.grantsBearerLethal !== true) {
      throw new RangeError(`${path}.grantsBearerLethal must be true`);
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
    if (Number(card.atEndOfEachTurnSiteControllerLosesLife !== undefined)
      + Number(card.bearerControllerChoosesExtraRandomOutcome === true)
      + Number(card.grantsBearerPower === 2)
      + Number(card.grantsBearerLethal === true)
      + Number(card.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps === 3)
      + Number(card
        .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
          === true)
      + Number(card.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath === 4)
        !== 1) {
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
    if (Number(card.immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns === true)
      + Number(card.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep === 3)
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
    if (card.grantChargeToAllyThisTurn !== undefined
      && card.grantChargeToAllyThisTurn !== true) {
      throw new RangeError(`${path}.grantChargeToAllyThisTurn must be true when defined`);
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
    if (card.killTargetWoundedMinion !== undefined
      && card.killTargetWoundedMinion !== true) {
      throw new RangeError(`${path}.killTargetWoundedMinion must be true when defined`);
    }
    if (card.lureEnemyMinionOneStepCloser !== undefined
      && card.lureEnemyMinionOneStepCloser !== true) {
      throw new RangeError(`${path}.lureEnemyMinionOneStepCloser must be true when defined`);
    }
    if (card.damageEachAbovegroundMinion !== undefined
      && card.damageEachAbovegroundMinion !== 1) {
      throw new RangeError(`${path}.damageEachAbovegroundMinion must be 1`);
    }
    if (card.damageChainNearbyUnits !== undefined && card.damageChainNearbyUnits !== true) {
      throw new RangeError(`${path}.damageChainNearbyUnits must be true when defined`);
    }
    if (card.discardSiteAsAdditionalCost !== undefined
      && card.discardSiteAsAdditionalCost !== true) {
      throw new RangeError(`${path}.discardSiteAsAdditionalCost must be true when defined`);
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
    if (targetSiteEffectFacts !== 0 && targetSiteEffectFacts !== 3) {
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
      + Number(card.disableTargetNearbyMinionUntilNextTurn === true)
      + Number(card.fightAllyWithAdjacentEnemy === true)
      + Number(card.gainControlOfTargetNearbyMinion === true)
      + Number(card.grantChargeToAllyThisTurn === true)
      + Number(card.grantPowerToAllyThisTurn === 2)
      + Number(card.healController !== undefined)
      + Number(card.killTargetWoundedMinion === true)
      + Number(card.leapAttackAlly === true)
      + Number(card.lureEnemyMinionOneStepCloser === true)
      + Number(card.returnMinionFromOwnCemetery === true)
      + Number(card.summonRandomMinionFromAnyCemetery === true)
      + Number(card.summonTokenToEachControlledSiteBorderingEnemySite !== undefined)
      + Number(card.teleportAllyToTargetSite === true)
      + Number(card.teleportNearbyAllyThenDrawCard === true);
    if (effectCount !== 1) {
      throw new RangeError(`${path} must define exactly one supported Magic effect`);
    }
    if (card.targetNearby !== undefined && typeof card.targetNearby !== 'boolean') {
      throw new RangeError(`${path}.targetNearby must be boolean`);
    }
    if (card.targetNearby !== undefined && card.damageTargetUnit === undefined) {
      throw new RangeError(`${path}.targetNearby requires damageTargetUnit`);
    }
    if (card.untapTargetMinionAfterDamage !== undefined
      && card.untapTargetMinionAfterDamage !== true) {
      throw new RangeError(`${path}.untapTargetMinionAfterDamage must be true when defined`);
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
  if (card.genesisMayDamageTargetAdjacentUnit === 2
    && (card.discardRandomCardInsteadOfMana === true
      || card.sacrificeMinionAtSummoningLocationForManaDiscount === 2)) {
    throw new RangeError(`${path} targeted Genesis with alternative summon payment is unsupported`);
  }
  if (card.gainsStealthAtEndOfTurn !== undefined && typeof card.gainsStealthAtEndOfTurn !== 'boolean') {
    throw new RangeError(`${path}.gainsStealthAtEndOfTurn must be boolean`);
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
    && (card.ordinary === true
      || card.connectsTopBottom === true
      || card.sacrificeMinionAtSummoningLocationForManaDiscount === 2
      || card.mustBeCastBurrowed === true
      || card.mustBeCastSubmerged === true
      || card.mustBeCastToWaterSite === true
      || card.burrowing === true
      || card.submerge === true
      || card.voidwalk === true
      || card.waterbound === true
      || card.ranged === true
      || card.shootsDragProjectile === true
      || card.siteProvidesNoThreshold === true
      || card.spellcaster === true
      || card.gainsPowerRangedAndSpellcasterAtopTower === 2
      || card.summonToAnySite === true
      || card.mustBeCastToOuterColumn === true
      || card.token === true
      || card.deathriteDamageEachUnitHere !== undefined
      || card.discardSpellToDamageRandomOtherUnitHere !== undefined
      || card.genesisDamageEachOtherUnitHere === 1
      || card.genesisDisableSelfUntilDamaged === true
      || card.genesisMayDamageTargetAdjacentUnit === 2
      || card.genesisStrikeEachEnemyHere === true
      || card.genesisDrawSpells !== undefined
      || card.genesisDrawSite === true
      || card.genesisHealController === 2
      || card.genesisLoseControllerLife === 2)) {
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
  if (card.mustBeCastToWaterSite !== undefined && typeof card.mustBeCastToWaterSite !== 'boolean') {
    throw new RangeError(`${path}.mustBeCastToWaterSite must be boolean`);
  }
  if (card.ranged !== undefined && typeof card.ranged !== 'boolean') {
    throw new RangeError(`${path}.ranged must be boolean`);
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
  if (card.waterbound
    && (card.ward || card.stealth || card.gainsStealthAtEndOfTurn)) {
    throw new RangeError(`${path} Waterbound with Ward or Stealth is unsupported`);
  }
  if (card.waterbound
    && (card.genesisDrawSite || card.genesisDrawSpells !== undefined
      || card.genesisDamageEachOtherUnitHere === 1
      || card.genesisDisableSelfUntilDamaged === true
      || card.genesisMayDamageTargetAdjacentUnit === 2
      || card.genesisStrikeEachEnemyHere === true
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} Waterbound with Genesis is unsupported`);
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
              : card.bearerControllerChoosesExtraRandomOutcome === true
                ? { bearerControllerChoosesExtraRandomOutcome: true as const }
              : card.grantsBearerPower === 2
              ? { grantsBearerPower: 2 as const }
              : card.grantsBearerLethal === true
                ? { grantsBearerLethal: true as const }
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
          }
          : card.cardType === 'aura'
            ? {
              ...(card.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep === 3
                ? {
                  atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep:
                    3 as const,
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
                  : card.grantChargeToAllyThisTurn === true
                    ? { grantChargeToAllyThisTurn: true as const }
                  : card.grantPowerToAllyThisTurn === 2
                    ? { grantPowerToAllyThisTurn: 2 as const }
                  : card.killTargetWoundedMinion === true
                    ? { killTargetWoundedMinion: true as const }
                  : card.leapAttackAlly === true
                    ? { leapAttackAlly: true as const }
                  : card.lureEnemyMinionOneStepCloser === true
                    ? { lureEnemyMinionOneStepCloser: true as const }
                  : card.healController !== undefined
                    ? { healController: card.healController }
                    : card.returnMinionFromOwnCemetery === true
                      ? { returnMinionFromOwnCemetery: true as const }
                      : card.summonRandomMinionFromAnyCemetery === true
                        ? { summonRandomMinionFromAnyCemetery: true as const }
                        : card.summonTokenToEachControlledSiteBorderingEnemySite !== undefined
                        ? {
                          summonTokenToEachControlledSiteBorderingEnemySite:
                            card.summonTokenToEachControlledSiteBorderingEnemySite,
                        }
                        : card.teleportNearbyAllyThenDrawCard === true
                          ? { teleportNearbyAllyThenDrawCard: true as const }
                          : { teleportAllyToTargetSite: true as const }),
              manaCost: card.manaCost,
              ...(card.targetNearby === true ? { targetNearby: true } : {}),
              thresholds: { ...card.thresholds },
              ...(card.untapTargetMinionAfterDamage === true
                ? { untapTargetMinionAfterDamage: true as const }
                : {}),
            }
          : {
            ...(card.airborne === true ? { airborne: true } : {}),
            ...(card.atStartOfControllerTurnTeleportToRandomSiteOrVoid === true
              ? { atStartOfControllerTurnTeleportToRandomSiteOrVoid: true as const }
              : {}),
            attack: card.attack,
            ...(card.burrowing === true ? { burrowing: true } : {}),
            cardType: 'minion' as const,
            ...(card.cannotAttackSites === true ? { cannotAttackSites: true } : {}),
            ...(card.charge === true ? { charge: true } : {}),
            ...(card.cannotDefend === true ? { cannotDefend: true } : {}),
            ...(card.cannotDefendOrIntercept === true ? { cannotDefendOrIntercept: true } : {}),
            ...(card.connectsTopBottom === true ? { connectsTopBottom: true } : {}),
            ...(card.deathriteDamageEachUnitHere
              ? { deathriteDamageEachUnitHere: card.deathriteDamageEachUnitHere }
              : {}),
            ...(card.deathriteDrawSite === true ? { deathriteDrawSite: true } : {}),
            ...(card.deathriteHeal ? { deathriteHeal: card.deathriteHeal } : {}),
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
            ...(card.shootsDragProjectile === true ? { shootsDragProjectile: true } : {}),
            ...(card.siteProvidesNoThreshold === true
              ? { siteProvidesNoThreshold: true as const }
              : {}),
            ...(card.spellcaster === true ? { spellcaster: true } : {}),
            ...(card.stealth === true ? { stealth: true } : {}),
            ...(card.strikesFirstWhileAttacking === true ? { strikesFirstWhileAttacking: true } : {}),
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

function cardInstance(
  manifest: GameManifest,
  owner: GameSeat,
  source: CardInstance['source'],
  ordinal: number,
  cardId: string,
): CardInstance {
  const definition = manifest.cards[cardId];
  if (!definition) throw new Error(`missing manifest card: ${cardId}`);
  return deepFreeze({
    cardId,
    instanceId: identityHash({
      authorityHash: manifest.authority.contentHash,
      cardId,
      definitionHash: identityHash(asJson(definition)),
      engineVersion: manifest.engineVersion,
      firstSeat: manifest.firstSeat,
      ordinal,
      owner,
      seed: manifest.seed,
      source,
    }),
    owner,
    source,
  });
}

function tokenUnit(
  state: GameState,
  owner: GameSeat,
  cardId: string,
  sourceInstanceId: StateHash,
  cell: RealmCell,
  ordinal: number,
): UnitInstance {
  const definition = cardDefinition(state, cardId);
  if (definition.cardType !== 'minion' || definition.token !== true) {
    throw new Error('token effect lacks its referenced token minion definition');
  }
  return deepFreeze({
    cardId,
    ...(definition.lanceCount ? { carriedLanceCount: definition.lanceCount } : {}),
    controller: owner,
    damage: 0,
    instanceId: identityHash({
      cardId,
      cell,
      ordinal,
      owner,
      source: 'token',
      sourceInstanceId,
      stateVersion: state.stateVersion,
    }),
    location: cell,
    owner,
    region: 'surface',
    source: 'token',
    stealthed: definition.stealth === true,
    summoningSickness: true,
    tapped: false,
    warded: definition.ward === true,
  });
}

function drawCandidate(
  engine: EngineState,
  exclusiveMaximum: number,
  purpose: string,
  domainKind = 'shuffle_index_candidate',
): Readonly<{ engine: EngineState; index: number; randomDraws: readonly EngineRandomDraw[] }> {
  const limit = Math.floor(UINT32_RANGE / exclusiveMaximum) * exclusiveMaximum;
  const randomDraws: EngineRandomDraw[] = [];
  let nextEngine = engine;
  while (true) {
    const prePrngStateHash = identityHash(asJson(nextEngine.prng));
    const draw = drawUint32(nextEngine);
    nextEngine = draw.nextState;
    randomDraws.push(deepFreeze({
      domain: { accepted: draw.value < limit, exclusiveMaximum, kind: domainKind },
      drawSequence: nextEngine.prng.draws,
      postPrngStateHash: identityHash(asJson(nextEngine.prng)),
      prePrngStateHash,
      purpose,
      result: draw.value,
    }));
    if (draw.value < limit) {
      return deepFreeze({ engine: nextEngine, index: draw.value % exclusiveMaximum, randomDraws });
    }
  }
}

function luckyCharmCount(state: GameState, seat: GameSeat): number {
  return (state.realm.artifacts ?? []).filter((artifact) => {
    if (!('bearer' in artifact) || artifact.bearer.seat !== seat) return false;
    const definition = cardDefinition(state, artifact.cardId);
    return definition.cardType === 'artifact'
      && definition.bearerControllerChoosesExtraRandomOutcome === true;
  }).length;
}

function drawRandomOutcomes(
  state: GameState,
  seat: GameSeat,
  candidateInstanceIds: readonly StateHash[],
  purpose: string,
  domainKind: string,
): Readonly<{
  engine: EngineState;
  outcomeInstanceIds: readonly StateHash[];
  randomDraws: readonly EngineRandomDraw[];
}> {
  let engine = state.engine;
  const outcomeInstanceIds: StateHash[] = [];
  const randomDraws: EngineRandomDraw[] = [];
  for (let index = 0; index <= luckyCharmCount(state, seat); index += 1) {
    const selected = drawCandidate(engine, candidateInstanceIds.length, purpose, domainKind);
    engine = selected.engine;
    outcomeInstanceIds.push(candidateInstanceIds[selected.index]!);
    randomDraws.push(...selected.randomDraws);
  }
  return deepFreeze({ engine, outcomeInstanceIds, randomDraws });
}

function resolveRandomOutcome(
  state: GameState,
  candidateInstanceIds: readonly StateHash[],
  purpose: string,
  domainKind: string,
  forcedOutcomeInstanceId?: StateHash,
): Readonly<{
  engine: EngineState;
  outcomeInstanceId: StateHash;
  randomDraws: readonly EngineRandomDraw[];
}> {
  if (forcedOutcomeInstanceId !== undefined) {
    if (!candidateInstanceIds.includes(forcedOutcomeInstanceId)) {
      throw new Error('unreachable invalid forced random outcome');
    }
    return deepFreeze({
      engine: state.engine,
      outcomeInstanceId: forcedOutcomeInstanceId,
      randomDraws: [],
    });
  }
  const selected = drawCandidate(
    state.engine,
    candidateInstanceIds.length,
    purpose,
    domainKind,
  );
  const outcomeInstanceId = candidateInstanceIds[selected.index];
  if (!outcomeInstanceId) {
    throw new Error('unreachable invalid random outcome choice');
  }
  return deepFreeze({
    engine: selected.engine,
    outcomeInstanceId,
    randomDraws: selected.randomDraws,
  });
}

function shuffle(
  cards: readonly CardInstance[],
  engine: EngineState,
  purpose: string,
): Readonly<{ cards: readonly CardInstance[]; engine: EngineState; randomDraws: readonly EngineRandomDraw[] }> {
  const shuffled = [...cards];
  const randomDraws: EngineRandomDraw[] = [];
  let nextEngine = engine;
  for (let index = shuffled.length - 1; index > 0; index -= 1) {
    const candidate = drawCandidate(nextEngine, index + 1, purpose);
    nextEngine = candidate.engine;
    randomDraws.push(...candidate.randomDraws);
    [shuffled[index], shuffled[candidate.index]] = [shuffled[candidate.index]!, shuffled[index]!];
  }
  return deepFreeze({ cards: shuffled, engine: nextEngine, randomDraws });
}

function createPlayer(
  manifest: GameManifest,
  seat: GameSeat,
  engine: EngineState,
): Readonly<{ engine: EngineState; player: PlayerState; randomDraws: readonly EngineRandomDraw[] }> {
  const deck = manifest.decks[seat];
  const atlas = deck.atlas.map((cardId, index) => cardInstance(manifest, seat, 'atlas', index, cardId));
  const spellbook = deck.spellbook.map(
    (cardId, index) => cardInstance(manifest, seat, 'spellbook', index, cardId),
  );
  const shuffledAtlas = shuffle(atlas, engine, `setup_${seat}_atlas_shuffle`);
  const shuffledSpellbook = shuffle(spellbook, shuffledAtlas.engine, `setup_${seat}_spellbook_shuffle`);
  const avatarDefinition = manifest.cards[deck.avatar];
  if (!avatarDefinition || avatarDefinition.cardType !== 'avatar') {
    throw new Error('validated deck lacks Avatar definition');
  }
  return deepFreeze({
    engine: shuffledSpellbook.engine,
    player: {
      ...(avatarDefinition.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn === true
        ? { airThresholdsCastThisTurn: 0 }
        : {}),
      atlas: shuffledAtlas.cards.slice(3),
      avatar: {
        card: cardInstance(manifest, seat, 'avatar', 0, deck.avatar),
        deathDoorTurn: null,
        life: avatarDefinition.life,
        location: seat === 'north' ? NORTH_START : SOUTH_START,
        region: 'surface',
        tapped: false,
      },
      cemetery: [],
      domainEstablished: false,
      hand: { atlas: shuffledAtlas.cards.slice(0, 3), spellbook: shuffledSpellbook.cards.slice(0, 3) },
      mana: 0,
      mulliganComplete: false,
      spellbook: shuffledSpellbook.cards.slice(3),
    },
    randomDraws: [...shuffledAtlas.randomDraws, ...shuffledSpellbook.randomDraws],
  });
}

export function createGameSession(manifest: GameManifest): GameSession {
  assertCanonicalGameManifest(manifest);
  const initialEngine = createEngineState(manifest.seed);
  const north = createPlayer(manifest, 'north', initialEngine);
  const south = createPlayer(manifest, 'south', north.engine);
  const state: GameState = deepFreeze({
    activeSeat: 'north',
    cards: manifest.cards,
    decisionSeat: 'north',
    engine: south.engine,
    pendingCombat: null,
    phase: 'mulligan',
    players: { north: north.player, south: south.player },
    realm: { sites: {}, units: [] },
    schemaVersion: 1,
    stateVersion: 0,
    terminal: { status: 'active' },
    turnNumber: 0,
  });
  return deepFreeze({
    attempts: [],
    initialRandomDraws: [...north.randomDraws, ...south.randomDraws],
    manifest,
    state,
    transcript: [],
  });
}

export function hashGameState(state: GameState): StateHash {
  return identityHash(asJson(state));
}

function cardDefinition(state: GameState, cardId: string): GameCardDefinition {
  const card = state.cards[cardId];
  if (!card) throw new Error(`missing manifest card: ${cardId}`);
  return card;
}

function minionDisabled(state: GameState, unit: UnitInstance): boolean {
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion') throw new Error('realm minion lacks minion definition');
  return Boolean(unit.disableEffects?.length) || unit.disabledUntilDamaged === true
    || definition.waterbound === true && !isWaterSite(state, unit.location);
}

function affinity(state: GameState, seat: GameSeat): GameThresholds {
  const total: Record<GameElement, number> = { air: 0, earth: 0, fire: 0, water: 0 };
  const sitesProvidingNoThreshold = new Set(state.realm.units
    .filter((unit) => unit.region !== 'void' && !minionDisabled(state, unit))
    .filter((unit) => {
      const definition = cardDefinition(state, unit.cardId);
      return definition.cardType === 'minion' && definition.siteProvidesNoThreshold === true;
    })
    .map((unit) => unit.location));
  Object.entries(state.realm.sites)
    .filter(([, site]) => site.controller === seat)
    .forEach(([cell, site]) => {
      if (isRubble(site)) return;
      const definition = cardDefinition(state, site.cardId);
      if (definition.cardType !== 'site') throw new Error('realm site lacks site definition');
      if (sitesProvidingNoThreshold.has(cell as RealmCell)
        && !siteCannotBeMovedDestroyedOrModified(state, site)) return;
      definition.elements.forEach((element) => {
        total[element] += 1;
      });
    });
  state.realm.units
    .filter((unit) => unit.controller === seat && !minionDisabled(state, unit))
    .forEach((unit) => {
      const definition = cardDefinition(state, unit.cardId);
      if (definition.cardType !== 'minion') throw new Error('realm minion lacks minion definition');
      if (definition.provides) total[definition.provides] += 1;
    });
  return deepFreeze(total);
}

function observedCard(card: CardInstance): Readonly<{ cardId: string; instanceId: StateHash }> {
  return { cardId: card.cardId, instanceId: card.instanceId };
}

function temporaryPowerBonus(sources: readonly StateHash[] | undefined): number {
  return 2 * (sources?.length ?? 0);
}

function nearbyAlliesPowerBonus(state: GameState, ref: GameUnitRef): number {
  const target = ref.kind === 'avatar'
    ? state.players[ref.seat].avatar
    : state.realm.units.find(({ controller, instanceId }) =>
      controller === ref.seat && instanceId === ref.instanceId);
  if (!target) throw new Error('unreachable nearby-power target');
  const targetCells = ref.kind === 'avatar' ? [target.location] : unitOccupiedCells(target);
  return state.realm.units.filter((source) => {
    if (source.controller !== ref.seat
      || source.instanceId === ref.instanceId
      || source.region !== target.region
      || minionDisabled(state, source)) return false;
    const definition = cardDefinition(state, source.cardId);
    return definition.cardType === 'minion'
      && definition.otherNearbyAlliesPowerBonus === 1
      && footprintNearby(unitOccupiedCells(source), targetCells);
  }).length;
}

function controlledMortalsPowerBonus(state: GameState, ref: GameUnitRef): number {
  if (ref.kind === 'avatar') return 0;
  const target = state.realm.units.find(({ controller, instanceId }) =>
    controller === ref.seat && instanceId === ref.instanceId);
  if (!target) throw new Error('unreachable Mortal-power target');
  const targetDefinition = cardDefinition(state, target.cardId);
  if (targetDefinition.cardType !== 'minion' || targetDefinition.mortal !== true) return 0;
  return state.realm.units.filter((source) => {
    if (source.controller !== target.controller
      || source.instanceId === target.instanceId
      || minionDisabled(state, source)) return false;
    const definition = cardDefinition(state, source.cardId);
    return definition.cardType === 'minion'
      && definition.otherControlledMortalsPowerBonus === 1;
  }).length;
}

function immobileAreaApplies(
  state: GameState,
  area: ImmobileArea,
  location: GameLocation,
  minion: boolean,
): boolean {
  return area.cells.includes(location.cell)
    && (area.minionsAtSitesOnly !== true
      || minion && location.region !== 'void' && state.realm.sites[location.cell] !== undefined);
}

function locationInImmobileArea(
  state: GameState,
  location: GameLocation,
  minion: boolean,
): boolean {
  return state.realm.immobileAreas?.some((area) =>
    immobileAreaApplies(state, area, location, minion)) ?? false;
}

function locationSuppressesAirborne(state: GameState, location: GameLocation): boolean {
  return state.realm.immobileAreas?.some((area) =>
    area.suppressesAirborne === true && immobileAreaApplies(state, area, location, true)) ?? false;
}

function observePlayer(state: GameState, player: PlayerState, owner: GameSeat, viewer: GameSeat): ObservedPlayer {
  const own = owner === viewer;
  const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
  if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
  const status = unitStatus(state, {
    instanceId: player.avatar.card.instanceId,
    kind: 'avatar',
    seat: owner,
  });
  return deepFreeze({
    affinity: affinity(state, owner),
    ...(player.airThresholdsCastThisTurn !== undefined
      ? { airThresholdsCastThisTurn: player.airThresholdsCastThisTurn }
      : {}),
    atlasCount: player.atlas.length,
    avatar: {
      attack: status.attack,
      cardId: player.avatar.card.cardId,
      deathDoorTurn: player.avatar.deathDoorTurn,
      defense: status.defense,
      immobile: status.immobile,
      instanceId: player.avatar.card.instanceId,
      life: player.avatar.life,
      location: player.avatar.location,
      region: player.avatar.region,
      tapped: player.avatar.tapped,
    },
    cemetery: player.cemetery.map(observedCard),
    domainEstablished: player.domainEstablished,
    hand: {
      atlas: own ? player.hand.atlas.map(observedCard) : player.hand.atlas.length,
      spellbook: own ? player.hand.spellbook.map(observedCard) : player.hand.spellbook.length,
    },
    mana: player.mana,
    mulliganComplete: player.mulliganComplete,
    spellbookCount: player.spellbook.length,
  });
}

export function observeGame(state: GameState, viewer: GameSeat): GameObservation {
  const artifacts = state.realm.artifacts?.map((artifact) => {
    if ('bearer' in artifact) {
      const carriedAt = artifactLocation(state, artifact);
      return {
        bearer: artifact.bearer,
        cardId: artifact.cardId,
        controller: artifact.bearer.seat,
        instanceId: artifact.instanceId,
        location: carriedAt.cell,
        owner: artifact.owner,
        region: carriedAt.region,
      };
    }
    return {
      cardId: artifact.cardId,
      controller: null,
      instanceId: artifact.instanceId,
      location: artifact.location,
      owner: artifact.owner,
      region: artifact.region,
    };
  });
  const auras = state.realm.auras?.map((aura) => ({
    cardId: aura.cardId,
    cells: [...aura.cells] as TwoByTwoArea,
    controller: aura.controller,
    instanceId: aura.instanceId,
    owner: aura.owner,
    turnCounters: aura.turnCounters,
  }));
  const sites = Object.fromEntries(
    Object.entries(state.realm.sites).map(([cell, card]) => {
      if (isRubble(card)) {
        return [cell, {
          cardId: 'rubble',
          controller: null,
          elements: [],
          instanceId: card.instanceId,
          rubble: true,
        }];
      }
      const definition = cardDefinition(state, card.cardId);
      if (definition.cardType !== 'site') throw new Error('realm site lacks site definition');
      return [cell, {
        cardId: card.cardId,
        controller: card.controller,
        elements: definition.elements,
        instanceId: card.instanceId,
        owner: card.owner,
      }];
    }),
  ) as GameObservation['realm']['sites'];
  const units = state.realm.units.map((unit) => {
    const definition = cardDefinition(state, unit.cardId);
    if (definition.cardType !== 'minion') throw new Error('unit lacks minion definition');
    const status = unitStatus(state, {
      instanceId: unit.instanceId,
      kind: 'minion',
      seat: unit.controller,
    });
    return {
      airborne: status.airborne,
      attack: status.attack,
      cardId: unit.cardId,
      ...(unit.carriedLanceCount ? { carriedLanceCount: unit.carriedLanceCount } : {}),
      controller: unit.controller,
      damage: unit.damage,
      defense: status.defense,
      disabled: minionDisabled(state, unit),
      immobile: status.immobile,
      instanceId: unit.instanceId,
      location: unit.location,
      ...(unit.occupiedCells ? { occupiedCells: unit.occupiedCells } : {}),
      owner: unit.owner,
      region: unit.region,
      stealthed: unit.stealthed,
      summoningSickness: unit.summoningSickness,
      tapped: unit.tapped,
      ...(definition.token === true ? { token: true as const } : {}),
      warded: unit.warded,
    };
  });
  return deepFreeze({
    activeSeat: state.activeSeat,
    decisionSeat: state.decisionSeat,
    pendingCombat: state.pendingCombat,
    phase: state.phase,
    players: {
      north: observePlayer(state, state.players.north, 'north', viewer),
      south: observePlayer(state, state.players.south, 'south', viewer),
    },
    realm: {
      ...(artifacts ? { artifacts } : {}),
      ...(auras ? { auras } : {}),
      ...(state.realm.immobileAreas
        ? {
          immobileAreas: state.realm.immobileAreas.map((area) => ({
            ...area,
            cells: [...area.cells],
          })),
        }
        : {}),
      sites,
      units,
    },
    schemaVersion: 1,
    stateVersion: state.stateVersion,
    terminal: state.terminal,
    turnNumber: state.turnNumber,
    viewer,
  });
}

function permutations<T>(items: readonly T[]): readonly (readonly T[])[] {
  if (items.length < 2) return [[...items]];
  return items.flatMap((item, index) => permutations([...items.slice(0, index), ...items.slice(index + 1)])
    .map((tail) => [item, ...tail]));
}

function mulliganDescriptors(player: PlayerState): readonly MulliganDescriptor[] {
  const hand = [...player.hand.atlas, ...player.hand.spellbook];
  const descriptors: MulliganDescriptor[] = [];
  for (let mask = 0; mask < 2 ** hand.length; mask += 1) {
    const selected = hand.filter((_, index) => (mask & (1 << index)) !== 0);
    if (selected.length > 3) continue;
    const atlas = selected.filter(({ source }) => source === 'atlas').map(({ instanceId }) => instanceId);
    const spellbook = selected.filter(({ source }) => source === 'spellbook').map(({ instanceId }) => instanceId);
    for (const atlasOrder of permutations(atlas)) {
      for (const spellbookOrder of permutations(spellbook)) {
        descriptors.push(deepFreeze({ atlasOrder, kind: 'mulligan', spellbookOrder }));
      }
    }
  }
  return descriptors;
}

function unitRefs(state: GameState, seat: GameSeat): readonly GameUnitRef[] {
  return [
    {
      instanceId: state.players[seat].avatar.card.instanceId,
      kind: 'avatar',
      seat,
    },
    ...state.realm.units
      .filter((unit) => unit.controller === seat)
      .map((unit) => ({ instanceId: unit.instanceId, kind: 'minion' as const, seat })),
  ];
}

function spellcasterRefs(state: GameState, seat: GameSeat): readonly GameUnitRef[] {
  return unitRefs(state, seat).filter((ref) => unitStatus(state, ref).spellcaster);
}

function atopTowerPowerBonus(
  state: GameState,
  unit: UnitInstance,
  definition: Extract<GameCardDefinition, { readonly cardType: 'minion' }>,
  disabled: boolean,
): number {
  if (disabled
    || unit.region !== 'surface'
    || definition.gainsPowerRangedAndSpellcasterAtopTower !== 2) return 0;
  const site = state.realm.sites[unit.location];
  if (!site || isRubble(site)) return 0;
  const siteDefinition = cardDefinition(state, site.cardId);
  return siteDefinition.cardType === 'site' && siteDefinition.isTower === true ? 2 : 0;
}

function cellsAtPlanarGate(
  state: GameState,
  cells: readonly RealmCell[],
  region: GameRegion,
): boolean {
  if (region === 'void') return false;
  return cells.some((cell) => {
    const site = state.realm.sites[cell];
    if (!site || isRubble(site)) return false;
    const definition = cardDefinition(state, site.cardId);
    return definition.cardType === 'site'
      && definition.minionsHereGainVoidwalkUntilLeavingVoid === true;
  });
}

function minionAtPlanarGate(state: GameState, unit: UnitInstance): boolean {
  return cellsAtPlanarGate(state, unitOccupiedCells(unit), unit.region);
}

function unitStatus(
  state: GameState,
  ref: GameUnitRef,
): Readonly<{
  airborne: boolean;
  attack: number;
  burrowing: boolean;
  canAttackSites: boolean;
  canMoveToDefend: boolean;
  canRespondToAttack: boolean;
  charge: boolean;
  connectsTopBottom: boolean;
  defense: number;
  disabled: boolean;
  immobile: boolean;
  intrinsicVoidwalk: boolean;
  lethal: boolean;
  location: RealmCell;
  occupiedCells: readonly RealmCell[];
  movementSteps: number;
  movesOnlyForward: boolean;
  movesOnlySideways: boolean;
  planarGateVoidwalk: boolean;
  ranged: boolean;
  region: GameRegion;
  spellcaster: boolean;
  stealthed: boolean;
  strikesFirstWhileAttacking: boolean;
  submerge: boolean;
  summoningSickness: boolean;
  tapped: boolean;
  takesLessDamage: number;
  voidwalk: boolean;
}> {
  if (ref.kind === 'avatar') {
    const avatar = state.players[ref.seat].avatar;
    if (avatar.card.instanceId !== ref.instanceId) throw new Error('unreachable Avatar reference');
    const definition = cardDefinition(state, avatar.card.cardId);
    if (definition.cardType !== 'avatar') throw new Error('Avatar lacks Avatar definition');
    const powerBonus = temporaryPowerBonus(avatar.temporaryPowerSources)
      + bearerPowerBonus(state, ref)
      + nearbyAlliesPowerBonus(state, ref);
    return {
      airborne: false,
      attack: definition.attack + powerBonus,
      burrowing: false,
      canAttackSites: true,
      canMoveToDefend: true,
      canRespondToAttack: true,
      charge: false,
      connectsTopBottom: false,
      defense: definition.defense + powerBonus,
      disabled: false,
      immobile: locationInImmobileArea(
        state,
        { cell: avatar.location, region: avatar.region },
        false,
      ),
      intrinsicVoidwalk: false,
      lethal: bearerHasLethal(state, ref),
      location: avatar.location,
      occupiedCells: [avatar.location],
      movementSteps: 1,
      movesOnlyForward: false,
      movesOnlySideways: false,
      planarGateVoidwalk: false,
      ranged: false,
      region: avatar.region,
      spellcaster: true,
      stealthed: false,
      strikesFirstWhileAttacking: false,
      submerge: false,
      summoningSickness: false,
      tapped: avatar.tapped,
      takesLessDamage: 0,
      voidwalk: false,
    };
  }
  const unit = state.realm.units.find(({ instanceId }) => instanceId === ref.instanceId);
  if (!unit || unit.controller !== ref.seat) throw new Error('unreachable minion reference');
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion') throw new Error('minion lacks minion definition');
  const disabled = minionDisabled(state, unit);
  const printedVoidwalk = !disabled && definition.voidwalk === true;
  const planarGateVoidwalk = !disabled
    && (unit.planarGateVoidwalk === true || minionAtPlanarGate(state, unit));
  const towerPowerBonus = atopTowerPowerBonus(state, unit, definition, disabled);
  const powerBonus = temporaryPowerBonus(unit.temporaryPowerSources)
    + bearerPowerBonus(state, ref)
    + controlledMortalsPowerBonus(state, ref)
    + nearbyAlliesPowerBonus(state, ref)
    + towerPowerBonus;
  return {
    airborne: !disabled
      && definition.airborne === true
      && unit.region === 'surface'
      && !unitOccupiedCells(unit).some((cell) =>
        locationSuppressesAirborne(state, { cell, region: unit.region })),
    attack: definition.attack + powerBonus,
    burrowing: !disabled && definition.burrowing === true,
    canAttackSites: !disabled && definition.cannotAttackSites !== true,
    canMoveToDefend: !disabled && definition.cannotDefend !== true,
    canRespondToAttack: !disabled && definition.cannotDefendOrIntercept !== true,
    charge: !disabled
      && (definition.charge === true || Boolean(unit.temporaryChargeSources?.length)),
    connectsTopBottom: !disabled && definition.connectsTopBottom === true,
    defense: definition.defense + powerBonus,
    disabled,
    immobile: unitOccupiedCells(unit).some((cell) =>
      locationInImmobileArea(state, { cell, region: unit.region }, true))
      || (!disabled && definition.immobile === true),
    intrinsicVoidwalk: printedVoidwalk,
    lethal: !disabled && (definition.lethal === true || bearerHasLethal(state, ref)),
    location: unit.location,
    occupiedCells: unitOccupiedCells(unit),
    movementSteps: disabled ? 0 : 1 + (definition.movementBonus ?? 0),
    movesOnlyForward: !disabled && definition.movesOnlyForward === true,
    movesOnlySideways: !disabled && definition.movesOnlySideways === true,
    planarGateVoidwalk,
    ranged: !disabled && (definition.ranged === true || towerPowerBonus > 0),
    region: unit.region,
    spellcaster: !disabled && (definition.spellcaster === true || towerPowerBonus > 0),
    stealthed: !disabled && unit.stealthed,
    strikesFirstWhileAttacking: !disabled && definition.strikesFirstWhileAttacking === true,
    submerge: !disabled && definition.submerge === true,
    summoningSickness: unit.summoningSickness,
    tapped: unit.tapped,
    takesLessDamage: disabled ? 0 : (definition.takesLessDamage ?? 0),
    voidwalk: printedVoidwalk || planarGateVoidwalk,
  };
}

function bearerPowerBonus(state: GameState, ref: GameUnitRef): number {
  return (state.realm.artifacts ?? []).reduce((bonus, artifact) => {
    if (!('bearer' in artifact)
      || artifact.bearer.instanceId !== ref.instanceId
      || artifact.bearer.kind !== ref.kind
      || artifact.bearer.seat !== ref.seat) return bonus;
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact') throw new Error('realm artifact lacks Artifact definition');
    return bonus + (definition.grantsBearerPower ?? 0);
  }, 0);
}

function bearerHasLethal(state: GameState, ref: GameUnitRef): boolean {
  return (state.realm.artifacts ?? []).some((artifact) => {
    if (!('bearer' in artifact)
      || artifact.bearer.instanceId !== ref.instanceId
      || artifact.bearer.kind !== ref.kind
      || artifact.bearer.seat !== ref.seat) return false;
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact') throw new Error('realm artifact lacks Artifact definition');
    return definition.grantsBearerLethal === true;
  });
}

function carriedLanceCount(state: GameState, ref: GameUnitRef): number {
  return ref.kind === 'minion'
    ? state.realm.units.find(({ instanceId }) => instanceId === ref.instanceId)?.carriedLanceCount ?? 0
    : 0;
}

function strikeDamage(state: GameState, ref: GameUnitRef): number {
  return unitStatus(state, ref).attack + carriedLanceCount(state, ref);
}

function readyUnit(state: GameState, ref: GameUnitRef): boolean {
  const unit = unitStatus(state, ref);
  return !unit.disabled && !unit.tapped && (!unit.summoningSickness || unit.charge);
}

function sameLocation(left: GameLocation, right: GameLocation): boolean {
  return left.cell === right.cell && left.region === right.region;
}

function sameOptionalUnitRefs(
  left: readonly GameUnitRef[] | undefined,
  right: readonly GameUnitRef[] | undefined,
): boolean {
  return left === undefined && right === undefined
    || left !== undefined
      && right !== undefined
      && left.length === right.length
      && left.every((target, index) =>
        target.instanceId === right[index]!.instanceId
        && target.kind === right[index]!.kind
        && target.seat === right[index]!.seat);
}

function randomUnitCandidatesAtLocation(
  state: GameState,
  location: GameLocation,
  excludedInstanceId?: StateHash,
): readonly GameUnitRef[] {
  return (['north', 'south'] as const)
    .flatMap((targetSeat) => unitRefs(state, targetSeat))
    .filter((target) => {
      if (target.instanceId === excludedInstanceId) return false;
      const status = unitStatus(state, target);
      return status.region === location.region && status.occupiedCells.includes(location.cell);
    })
    .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
}

function randomSiteOrVoidLocations(
  state: GameState,
): readonly Readonly<{ instanceId: StateHash; location: GameLocation }>[] {
  return REALM_CELLS.flatMap((cell) => {
    const site = state.realm.sites[cell];
    if (site && isRubble(site)) return [];
    const location: GameLocation = { cell, region: site ? 'surface' : 'void' };
    return [{
      instanceId: identityHash(asJson({
        cell,
        kind: 'random-site-or-void-location',
        region: location.region,
      })),
      location,
    }];
  });
}

function luckyRandomOutcomeRequest(
  state: GameState,
  seat: GameSeat,
  descriptor: GameActionDescriptor,
): Readonly<{
  candidateInstanceIds: readonly StateHash[];
  domainKind: string;
  purpose: string;
}> | undefined {
  if (luckyCharmCount(state, seat) === 0) return undefined;
  const player = state.players[seat];
  if (descriptor.kind === 'cast-magic') {
    const card = player.hand.spellbook.find(({ instanceId }) =>
      instanceId === descriptor.cardInstanceId);
    const definition = card && cardDefinition(state, card.cardId);
    if (definition?.cardType === 'magic'
      && definition.summonRandomMinionFromAnyCemetery === true) {
      return {
        candidateInstanceIds: cemeteryMinionCandidates(state)
          .map(({ instanceId }) => instanceId),
        domainKind: 'dead_minion_instance_candidate',
        purpose: 'magic_random_dead_minion',
      };
    }
  }
  if (descriptor.kind === 'resolve-start-turn-trigger') {
    return {
      candidateInstanceIds: randomSiteOrVoidLocations(state)
        .map(({ instanceId }) => instanceId),
      domainKind: 'realm_site_or_void_location',
      purpose: 'start_turn_random_teleport',
    };
  }
  if (descriptor.kind === 'cast-magic' && descriptor.targetLocation) {
    const card = player.hand.spellbook.find(({ instanceId }) =>
      instanceId === descriptor.cardInstanceId);
    const definition = card && cardDefinition(state, card.cardId);
    if (definition?.cardType === 'magic'
      && definition.damageRandomUnitAtLocation !== undefined) {
      return {
        candidateInstanceIds: randomUnitCandidatesAtLocation(
          state,
          descriptor.targetLocation,
        ).map(({ instanceId }) => instanceId),
        domainKind: 'unit_index_candidate',
        purpose: 'magic_random_unit_at_location',
      };
    }
  }
  if (descriptor.kind === 'summon-minion'
    && descriptor.paymentMode === 'random-card-discard') {
    return {
      candidateInstanceIds: [
        ...player.hand.atlas.map(({ instanceId }) => instanceId),
        ...player.hand.spellbook
          .filter(({ instanceId }) => instanceId !== descriptor.cardInstanceId)
          .map(({ instanceId }) => instanceId),
      ],
      domainKind: 'card_index_candidate',
      purpose: 'summon_random_card_discard_cost',
    };
  }
  if (descriptor.kind === 'activate-discard-random-damage') {
    const source = state.realm.units.find(({ instanceId }) =>
      instanceId === descriptor.sourceInstanceId);
    if (source) {
      return {
        candidateInstanceIds: randomUnitCandidatesAtLocation(
          state,
          { cell: source.location, region: source.region },
          source.instanceId,
        ).map(({ instanceId }) => instanceId),
        domainKind: 'unit_index_candidate',
        purpose: 'discard_spell_random_other_unit_here',
      };
    }
  }
  if (descriptor.kind === 'activate-sparkmage') {
    return {
      candidateInstanceIds: randomUnitCandidatesAtLocation(
        state,
        descriptor.targetLocation,
        descriptor.sourceInstanceId,
      ).map(({ instanceId }) => instanceId),
      domainKind: 'unit_index_candidate',
      purpose: 'sparkmage_random_other_unit_at_nearby_location',
    };
  }
  return undefined;
}

function auraOneStepAreas(aura: AuraInstance): readonly TwoByTwoArea[] {
  return TWO_BY_TWO_AREAS.filter((cells) =>
    cells[0] !== aura.cells[0] && cardinalCellDistance(cells[0], aura.cells[0]) === 1);
}

function endTurnAuraCandidates(
  state: GameState,
  aura: AuraInstance,
): readonly GameUnitRef[] {
  const affectedSites = new Set(aura.cells.filter((cell) => state.realm.sites[cell] !== undefined));
  return (['north', 'south'] as const)
    .flatMap((targetSeat) => unitRefs(state, targetSeat))
    .filter((target) => {
      const status = unitStatus(state, target);
      return status.region === 'surface'
        && status.occupiedCells.some((cell) => affectedSites.has(cell));
    })
    .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
}

function endTurnDamageAuraIds(state: GameState, seat: GameSeat): readonly StateHash[] {
  return (state.realm.auras ?? [])
    .filter((aura) => {
      if (aura.controller !== seat) return false;
      const definition = cardDefinition(state, aura.cardId);
      return definition.cardType === 'aura'
        && definition.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep === 3;
    })
    .map(({ instanceId }) => instanceId)
    .sort();
}

function resolveEndTurnAuraDamage(
  state: GameState,
  aura: AuraInstance,
  targetRef: GameUnitRef,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const definition = cardDefinition(state, aura.cardId);
  if (definition.cardType !== 'aura'
    || definition.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep !== 3
    || !endTurnAuraCandidates(state, aura).some(({ instanceId }) =>
      instanceId === targetRef.instanceId)) {
    throw new Error('unreachable invalid end-turn Aura damage');
  }
  const target = unitStatus(state, targetRef);
  const attacker: GameUnitRef = {
    instanceId: state.players[aura.controller].avatar.card.instanceId,
    kind: 'avatar',
    seat: aura.controller,
  };
  const pending: PendingCombat = deepFreeze({
    allocations: [{ amount: 3, targetInstanceId: targetRef.instanceId }],
    attacker,
    attackingSeat: aura.controller,
    cell: target.location,
    combatants: [targetRef],
    defenders: [],
    originalTarget: targetRef,
    targetRemoved: false,
  });
  return resolveFightWindow(
    state,
    pending,
    [{
      payload: {
        amount: 3,
        sourceInstanceId: aura.instanceId,
        targetInstanceId: targetRef.instanceId,
      },
      type: 'aura-end-turn-damage-allocated',
    }],
    'non-unit',
    true,
    false,
    [],
    false,
    false,
  );
}

function beginEndTurnAura(
  state: GameState,
  seat: GameSeat,
  auraInstanceIds: readonly StateHash[],
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] | null {
  const index = auraInstanceIds.findIndex((instanceId) => {
    const aura = state.realm.auras?.find((candidate) => candidate.instanceId === instanceId);
    if (!aura || aura.controller !== seat) return false;
    const definition = cardDefinition(state, aura.cardId);
    return definition.cardType === 'aura'
      && definition.atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep === 3;
  });
  if (index < 0) return null;
  const auraInstanceId = auraInstanceIds[index]!;
  const aura = state.realm.auras!.find(({ instanceId }) => instanceId === auraInstanceId)!;
  const remainingAuraInstanceIds = auraInstanceIds.slice(index + 1);
  const candidates = endTurnAuraCandidates(state, aura);
  const triggered: GameOutcome = {
    payload: {
      cells: aura.cells,
      instanceId: aura.instanceId,
      seat,
      sourceInstanceId: aura.instanceId,
    },
    type: 'aura-end-turn-triggered',
  };
  if (candidates.length === 0) {
    return [
      deepFreeze({
        ...state,
        pendingEndTurnAura: {
          auraInstanceId,
          remainingAuraInstanceIds,
          seat,
          stage: 'move',
        },
        phase: 'end-turn-aura',
      }),
      [triggered, {
        payload: { instanceId: aura.instanceId, seat, sourceInstanceId: aura.instanceId },
        type: 'aura-random-damage-skipped',
      }],
      [],
    ];
  }
  if (luckyCharmCount(state, seat) > 0) {
    const drawn = drawRandomOutcomes(
      state,
      seat,
      candidates.map(({ instanceId }) => instanceId),
      'aura_end_turn_random_unit_at_affected_sites',
      'unit_index_candidate',
    );
    return [
      deepFreeze({
        ...state,
        engine: drawn.engine,
        pendingEndTurnAura: {
          auraInstanceId,
          outcomeInstanceIds: [...new Set(drawn.outcomeInstanceIds)],
          remainingAuraInstanceIds,
          seat,
          stage: 'random',
        },
        phase: 'end-turn-aura',
      }),
      [triggered],
      drawn.randomDraws,
    ];
  }
  const selected = resolveRandomOutcome(
    state,
    candidates.map(({ instanceId }) => instanceId),
    'aura_end_turn_random_unit_at_affected_sites',
    'unit_index_candidate',
  );
  const randomizedState = deepFreeze({ ...state, engine: selected.engine });
  const targetRef = candidates.find(({ instanceId }) =>
    instanceId === selected.outcomeInstanceId)!;
  const [damaged, outcomes, draws] = resolveEndTurnAuraDamage(randomizedState, aura, targetRef);
  if (damaged.terminal.status === 'finished') {
    return [
      deepFreeze({ ...damaged, pendingEndTurnAura: null, phase: 'terminal' }),
      [triggered, ...outcomes],
      [...selected.randomDraws, ...draws],
    ];
  }
  return [
    deepFreeze({
      ...damaged,
      decisionSeat: seat,
      pendingEndTurnAura: {
        auraInstanceId,
        remainingAuraInstanceIds,
        seat,
        stage: 'move',
      },
      phase: 'end-turn-aura',
    }),
    [triggered, ...outcomes],
    [...selected.randomDraws, ...draws],
  ];
}

function unitOccupiedCells(unit: Readonly<Pick<UnitInstance, 'location' | 'occupiedCells'>>):
readonly RealmCell[] {
  return unit.occupiedCells ?? [unit.location];
}

function unitRefOccupiedCells(state: GameState, ref: GameUnitRef): readonly RealmCell[] {
  if (ref.kind === 'avatar') return [state.players[ref.seat].avatar.location];
  const unit = state.realm.units.find(({ controller, instanceId }) =>
    controller === ref.seat && instanceId === ref.instanceId);
  if (!unit) throw new Error('unreachable missing minion footprint');
  return unitOccupiedCells(unit);
}

function artifactLocation(state: GameState, artifact: ArtifactInstance): GameLocation {
  if (!('bearer' in artifact)) return { cell: artifact.location, region: artifact.region };
  const markedBearer = artifact.bearer.kind === 'minion'
    ? state.pendingDeathrites?.corpses.find(({ controller, instanceId }) =>
      controller === artifact.bearer.seat && instanceId === artifact.bearer.instanceId)
    : undefined;
  if (markedBearer) {
    return {
      cell: artifact.bearerCell ?? markedBearer.location,
      region: markedBearer.region,
    };
  }
  const bearer = unitStatus(state, artifact.bearer);
  return { cell: artifact.bearerCell ?? bearer.location, region: bearer.region };
}

function unitOccupiesLocation(
  state: GameState,
  ref: GameUnitRef,
  location: GameLocation,
): boolean {
  const status = unitStatus(state, ref);
  return status.region === location.region
    && unitRefOccupiedCells(state, ref).includes(location.cell);
}

function footprintNearby(
  sourceCells: readonly RealmCell[],
  targetCells: readonly RealmCell[],
): boolean {
  const nearby = new Set(sourceCells.flatMap((cell) => [
    cell,
    ...borderingCells(cell),
    ...diagonalCells(cell),
  ]));
  return targetCells.some((cell) => nearby.has(cell));
}

function footprintsHereOrBordering(
  sourceCells: readonly RealmCell[],
  targetCells: readonly RealmCell[],
): boolean {
  const adjacent = new Set(sourceCells.flatMap((cell) => [cell, ...borderingCells(cell)]));
  return targetCells.some((cell) => adjacent.has(cell));
}

function minimumCardinalDistance(
  sourceCells: readonly RealmCell[],
  targetCells: readonly RealmCell[],
): number {
  return Math.min(...sourceCells.flatMap((source) =>
    targetCells.map((target) => cardinalCellDistance(source, target))));
}

function translatedFootprint(
  occupiedCells: readonly RealmCell[],
  from: RealmCell,
  to: RealmCell,
): readonly RealmCell[] | undefined {
  if (from === to) return occupiedCells;
  const fileDelta = to.charCodeAt(0) - from.charCodeAt(0);
  const rankDelta = Number(to[1]) - Number(from[1]);
  const translated = occupiedCells.map((cell) => {
    const file = cell.charCodeAt(0) + fileDelta;
    const rank = Number(cell[1]) + rankDelta;
    return file >= 65 && file <= 69 && rank >= 1 && rank <= 4
      ? `${String.fromCharCode(file)}${rank}` as RealmCell
      : undefined;
  });
  return translated.every((cell): cell is RealmCell => cell !== undefined)
    ? translated
    : undefined;
}

function footprintLocationExists(
  state: GameState,
  cells: readonly RealmCell[],
  region: GameRegion,
): boolean {
  return cells.every((cell) => locationExists(state, { cell, region }));
}

function destinationFootprintsContaining(
  state: GameState,
  selectedLocation: GameLocation,
): readonly TwoByTwoArea[] {
  return TWO_BY_TWO_AREAS
    .filter((cells) => cells.includes(selectedLocation.cell))
    .filter((cells) => footprintLocationExists(state, cells, selectedLocation.region));
}

function locationExists(state: GameState, location: GameLocation): boolean {
  if (location.region === 'void') return state.realm.sites[location.cell] === undefined;
  return state.realm.sites[location.cell] !== undefined
    && (location.region === 'surface'
      || location.region === (isWaterSite(state, location.cell) ? 'underwater' : 'underground'));
}

function locationsWithinMeasuredSteps(
  state: GameState,
  start: GameLocation,
  maximumSteps: number,
): readonly GameLocation[] {
  if (!locationExists(state, start)) return [];
  const distances = new Map<RealmCell, number>([[start.cell, 0]]);
  const frontier: RealmCell[] = [start.cell];
  while (frontier.length > 0) {
    const current = frontier.shift()!;
    const distance = distances.get(current)!;
    if (distance === maximumSteps) continue;
    for (const cell of borderingCells(current)) {
      if (distances.has(cell)
        || !locationExists(state, { cell, region: start.region })) continue;
      distances.set(cell, distance + 1);
      frontier.push(cell);
    }
  }
  return [...distances.keys()].sort()
    .map((cell) => ({ cell, region: start.region }));
}

function burrowedConnectionLocations(
  state: GameState,
  seat: GameSeat,
  cell: RealmCell,
  connectsTopBottom: boolean,
  submerge: boolean,
): readonly GameLocation[] {
  const controlled = controlledSiteCells(state, seat);
  if (!controlled.includes(cell)) return [];
  const tunnels = controlled.filter((candidate) => {
    const site = state.realm.sites[candidate];
    const definition = site && !isRubble(site) ? cardDefinition(state, site.cardId) : undefined;
    return definition?.cardType === 'site' && definition.connectsBurrowedAllies === true;
  });
  const adjacent = new Set(borderingCells(cell, connectsTopBottom));
  return (tunnels.includes(cell) ? controlled : tunnels)
    .filter((candidate) => candidate !== cell && !adjacent.has(candidate))
    .flatMap((candidate): readonly GameLocation[] => isWaterSite(state, candidate)
      ? submerge ? [{ cell: candidate, region: 'underwater' as const }] : []
      : [{ cell: candidate, region: 'underground' as const }]);
}

function unitEntryAllowed(
  state: GameState,
  current: GameLocation | undefined,
  candidate: GameLocation,
  airborne: boolean,
  movingMinion: boolean,
  power: number,
  method: 'movement' | 'summon' | 'teleport',
): boolean {
  const site = state.realm.sites[candidate.cell];
  if (candidate.region === 'surface' && site && !isRubble(site)) {
    const definition = cardDefinition(state, site.cardId);
    if (definition.cardType === 'site'
      && definition.preventsUnitsWithPowerAtLeastFromEntering !== undefined
      && power >= definition.preventsUnitsWithPowerAtLeastFromEntering) return false;
  }
  if (method !== 'movement'
    || !movingMinion
    || airborne
    || current?.region !== 'surface'
    || candidate.region !== 'surface'
    || current.cell === candidate.cell) return true;
  if (!site || isRubble(site)) return true;
  const definition = cardDefinition(state, site.cardId);
  return definition.cardType !== 'site'
    || definition.blocksGroundMinionEntryWhileMinionAtop !== true
    || !state.realm.units.some((unit) =>
      unitOccupiedCells(unit).includes(candidate.cell) && unit.region === 'surface');
}

function movementStepCost(
  state: GameState,
  current: GameLocation,
  candidate: GameLocation,
  airborne: boolean,
  movingMinion: boolean,
  purpose: MovementPurpose,
): 0 | 1 {
  if (purpose === 'effect'
    || !airborne
    || !movingMinion
    || current.region !== 'surface'
    || current.cell === candidate.cell) return 1;
  const site = state.realm.sites[current.cell];
  if (!site || isRubble(site)) return 1;
  const definition = cardDefinition(state, site.cardId);
  return definition.cardType === 'site'
    && definition.airborneMinionsAtopMoveFreelyAway === true ? 0 : 1;
}

function movementPaths(
  state: GameState,
  start: GameLocation,
  maximumSteps: number,
  seat: GameSeat,
  airborne = false,
  movesOnlySideways = false,
  movesOnlyForward = false,
  burrowing = false,
  submerge = false,
  voidwalk = false,
  planarGateVoidwalk = false,
  connectsTopBottom = false,
  immobile = false,
  movingMinion = false,
  movingUnit = false,
  purpose: MovementPurpose = 'effect',
  occupiedCells: readonly RealmCell[] = [start.cell],
  power = 0,
): readonly (readonly GameLocation[])[] {
  if (!footprintLocationExists(state, occupiedCells, start.region)) return [];
  if (immobile) return [[start]];
  const paths: GameLocation[][] = [[start]];
  let frontier: Array<Readonly<{
    cost: number;
    path: GameLocation[];
    planarGateVoidwalk: boolean;
  }>> = [{ cost: 0, path: [start], planarGateVoidwalk }];
  while (frontier.length > 0) {
    frontier = frontier.flatMap(({ cost, path, planarGateVoidwalk: carriedVoidwalk }) => {
      const current = path.at(-1)!;
      const currentCells = translatedFootprint(occupiedCells, start.cell, current.cell) ?? [];
      const canVoidwalk = voidwalk
        || carriedVoidwalk
        || movingMinion && cellsAtPlanarGate(state, currentCells, current.region);
      if (movingUnit && currentCells.some((cell) =>
        locationInImmobileArea(state, { cell, region: current.region }, movingMinion))) return [];
      const tunnelHops = current.region === 'underground' && burrowing
        ? burrowedConnectionLocations(state, seat, current.cell, connectsTopBottom, submerge)
        : [];
      const candidates: GameLocation[] = current.region === 'surface'
        ? [
          ...borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'surface' as const })),
          ...(airborne
            ? diagonalCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'surface' as const }))
            : []),
          ...(burrowing && !isWaterSite(state, current.cell)
            ? [{ cell: current.cell, region: 'underground' as const }]
            : []),
          ...(submerge && isWaterSite(state, current.cell)
            ? [{ cell: current.cell, region: 'underwater' as const }]
            : []),
          ...(canVoidwalk
            ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'void' as const }))
            : []),
        ]
        : current.region === 'underground' && burrowing
          ? [
            { cell: current.cell, region: 'surface' as const },
            ...borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'underground' as const })),
            ...tunnelHops,
            ...(submerge
              ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'underwater' as const }))
              : []),
            ...(canVoidwalk
              ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'void' as const }))
              : []),
          ]
          : current.region === 'underwater' && submerge
          ? [
            { cell: current.cell, region: 'surface' as const },
            ...borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'underwater' as const })),
            ...(burrowing
              ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'underground' as const }))
              : []),
            ...(canVoidwalk
              ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'void' as const }))
              : []),
          ]
          : current.region === 'void' && canVoidwalk
            ? [
              ...borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'void' as const })),
              ...borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'surface' as const })),
              ...(burrowing
                ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'underground' as const }))
                : []),
              ...(submerge
                ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'underwater' as const }))
                : []),
            ]
          : [];
      return candidates
        .filter((candidate) => {
          const tunnelHop = tunnelHops.some((location) => sameLocation(location, candidate));
          const candidateCells = translatedFootprint(occupiedCells, start.cell, candidate.cell);
          const enteredCells = candidateCells?.filter((cell) => !currentCells.includes(cell)) ?? [];
          // ponytail: tunnel-hop direction stays implicit until direction-sensitive effects need path metadata.
          return candidateCells !== undefined
            && footprintLocationExists(state, candidateCells, candidate.region)
            && enteredCells.every((cell) => unitEntryAllowed(
              state,
              current,
              { cell, region: candidate.region },
              airborne,
              movingMinion,
              power,
              'movement',
            ))
            && (tunnelHop || (
              (!movesOnlySideways
                || candidate.region === current.region && candidate.cell[1] === current.cell[1])
              && (!movesOnlyForward
                || candidate.region === current.region
                  && candidate.cell[0] === current.cell[0]
                  && (Number(candidate.cell[1]) - Number(current.cell[1])
                    === (seat === 'north' ? -1 : 1)
                    || connectsTopBottom && (seat === 'north'
                      ? current.cell[1] === '1' && candidate.cell[1] === '4'
                      : current.cell[1] === '4' && candidate.cell[1] === '1')))))
            && !path.some((from, index) =>
              sameLocation(from, current) && path[index + 1] !== undefined
                && sameLocation(path[index + 1]!, candidate));
        })
        .sort((left, right) => left.cell === right.cell
          ? left.region < right.region ? -1 : left.region > right.region ? 1 : 0
          : left.cell < right.cell ? -1 : 1)
        .map((candidate) => ({
          cost: cost + movementStepCost(
            state,
            current,
            candidate,
            airborne,
            movingMinion,
            purpose,
          ),
          path: [...path, candidate],
          planarGateVoidwalk: !voidwalk && candidate.region === 'void' && canVoidwalk,
        }))
        .filter(({ cost: nextCost }) => nextCost <= maximumSteps);
    });
    paths.push(...frontier.map(({ path }) => path));
  }
  return paths;
}

function pathLocations(path: readonly RealmCell[], region: GameRegion = 'surface'): readonly GameLocation[] {
  return path.map((cell) => ({ cell, region }));
}

function samePath(left: readonly GameLocation[], right: readonly GameLocation[]): boolean {
  return left.length === right.length
    && left.every((location, index) =>
      location.cell === right[index]?.cell && location.region === right[index]?.region);
}

function defendPaths(
  state: GameState,
  ref: GameUnitRef,
  destination: GameLocation,
): readonly (readonly GameLocation[])[] {
  const unit = unitStatus(state, ref);
  if (!readyUnit(state, ref)) return [];
  return movementPaths(
    state,
    { cell: unit.location, region: unit.region },
    unit.canMoveToDefend ? unit.movementSteps : 0,
    ref.seat,
    unit.airborne,
    unit.movesOnlySideways,
    unit.movesOnlyForward,
    unit.burrowing,
    unit.submerge,
    unit.intrinsicVoidwalk,
    unit.planarGateVoidwalk,
    unit.connectsTopBottom,
    unit.immobile,
    ref.kind === 'minion',
    true,
    unit.canMoveToDefend ? 'defend' : 'effect',
    unit.occupiedCells,
    unit.attack,
  ).filter((path) => {
    const end = path.at(-1)!;
    const occupied = translatedFootprint(unit.occupiedCells, unit.location, end.cell);
    return end.region === destination.region && occupied?.includes(destination.cell);
  });
}

function movementDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  return unitRefs(state, seat).flatMap((ref) => {
    const unit = unitStatus(state, ref);
    if (!readyUnit(state, ref)
      || !locationExists(state, { cell: unit.location, region: unit.region })) return [];
    return movementPaths(
      state,
      { cell: unit.location, region: unit.region },
      unit.movementSteps,
      ref.seat,
      unit.airborne,
      unit.movesOnlySideways,
      unit.movesOnlyForward,
      unit.burrowing,
      unit.submerge,
      unit.intrinsicVoidwalk,
      unit.planarGateVoidwalk,
      unit.connectsTopBottom,
      unit.immobile,
      ref.kind === 'minion',
      true,
      'move-and-attack',
      unit.occupiedCells,
      unit.attack,
    )
      .map((path) => ({
        from: { cell: unit.location, region: unit.region },
        kind: 'move-and-attack' as const,
        path,
        to: path.at(-1)!,
        unitInstanceId: ref.instanceId,
      }));
  });
}

function projectileStep(cell: RealmCell, direction: ProjectileDirection): RealmCell | undefined {
  const file = cell.charCodeAt(0) + (direction === 'east' ? 1 : direction === 'west' ? -1 : 0);
  const rank = Number(cell[1]) + (direction === 'north' ? 1 : direction === 'south' ? -1 : 0);
  return file >= 65 && file <= 69 && rank >= 1 && rank <= 4
    ? `${String.fromCharCode(file)}${rank}` as RealmCell
    : undefined;
}

function rangedProjectileRange(
  state: GameState,
  location: RealmCell,
  region: GameRegion,
): number {
  if (region !== 'surface') return 1;
  const site = state.realm.sites[location];
  if (!site || isRubble(site)) return 1;
  const definition = cardDefinition(state, site.cardId);
  return definition.cardType === 'site' && definition.rangedUnitsHereRangeBonus === 1 ? 2 : 1;
}

function projectileDescriptorsForShooter(
  state: GameState,
  shooter: GameUnitRef,
  allowTapped = false,
): readonly GameActionDescriptor[] {
  const seat = shooter.seat;
  const directions = ['east', 'north', 'south', 'west'] as const;
  const allUnits = [...unitRefs(state, 'north'), ...unitRefs(state, 'south')];
  const status = unitStatus(state, shooter);
  if (!status.ranged || (!allowTapped && status.tapped) || status.summoningSickness) return [];
  const range = rangedProjectileRange(state, status.location, status.region);
  const startingEnemies = allUnits
    .filter((ref) => {
      const target = unitStatus(state, ref);
      return ref.seat !== seat
        && !target.stealthed
        && target.occupiedCells.some((cell) => status.occupiedCells.includes(cell))
        && target.region === status.region;
    })
    .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
  return directions.flatMap<GameActionDescriptor>((direction) => {
    if (startingEnemies.length > 0) {
      return startingEnemies.map((hit) => ({
        direction,
        hit,
        kind: 'shoot-projectile' as const,
        path: pathLocations([status.location], status.region),
        shooterInstanceId: shooter.instanceId,
      }));
    }
    const cells: RealmCell[] = [status.location];
    let current = status.location;
    for (let step = 0; step < range; step += 1) {
      const next = projectileStep(current, direction);
      if (!next || !locationExists(state, { cell: next, region: status.region })) break;
      cells.push(next);
      const hits = allUnits
        .filter((ref) => {
          const target = unitStatus(state, ref);
          return !target.stealthed
            && target.occupiedCells.includes(next)
            && target.region === status.region;
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      const path = pathLocations(cells, status.region);
      if (hits.length > 0) {
        return hits.map((hit) => ({
          direction,
          hit,
          kind: 'shoot-projectile' as const,
          path,
          shooterInstanceId: shooter.instanceId,
        }));
      }
      current = next;
    }
    return [{
      direction,
      hit: null,
      kind: 'shoot-projectile' as const,
      path: pathLocations(cells, status.region),
      shooterInstanceId: shooter.instanceId,
    }];
  });
}

function rangedDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  return unitRefs(state, seat).flatMap((shooter) =>
    projectileDescriptorsForShooter(state, shooter));
}

function basicMovementDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const pending = state.pendingBasicMovement;
  if (!pending || pending.seat !== seat) throw new Error('unreachable missing pending basic movement');
  const continuation: GameActionDescriptor = {
    kind: 'continue-basic-movement',
    unitInstanceId: pending.sourceInstanceId,
  };
  if (pending.rangedStrikeUsed) return [continuation];
  const unit = state.realm.units.find(({ controller, instanceId }) =>
    controller === seat && instanceId === pending.sourceInstanceId);
  if (!unit) return [continuation];
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion'
    || definition.mayRangedStrikeOnceDuringBasicMovement !== true) return [continuation];
  const shooter: GameUnitRef = { instanceId: unit.instanceId, kind: 'minion', seat };
  return [continuation, ...projectileDescriptorsForShooter(state, shooter, true)];
}

function rangedStepDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const pending = state.pendingRangedStep;
  if (!pending || pending.seat !== seat) throw new Error('unreachable missing pending Ranged step');
  const unit = state.realm.units.find(({ controller, instanceId }) =>
    controller === seat && instanceId === pending.sourceInstanceId);
  const decline: GameActionDescriptor = {
    choice: 'decline',
    kind: 'resolve-ranged-step',
    unitInstanceId: pending.sourceInstanceId,
  };
  if (!unit) return [decline];
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion'
    || definition.mayStepAfterRangedStrike !== true
    || minionDisabled(state, unit)) return [decline];
  const status = unitStatus(state, { instanceId: unit.instanceId, kind: 'minion', seat });
  const from = { cell: status.location, region: status.region };
  return [
    decline,
    ...movementPaths(
      state,
      from,
      1,
      seat,
      status.airborne,
      status.movesOnlySideways,
      status.movesOnlyForward,
      status.burrowing,
      status.submerge,
      status.intrinsicVoidwalk,
      status.planarGateVoidwalk,
      status.connectsTopBottom,
      status.immobile,
      true,
      true,
      'effect',
      status.occupiedCells,
      status.attack,
    ).filter((path) => path.length > 1).map((path) => ({
      choice: 'step' as const,
      from,
      kind: 'resolve-ranged-step' as const,
      path,
      to: path.at(-1)!,
      unitInstanceId: unit.instanceId,
    })),
  ];
}

function queueRangedStep(state: GameState, sourceInstanceId: StateHash): GameState {
  if (state.terminal.status === 'finished') return state;
  const unit = state.realm.units.find(({ instanceId }) => instanceId === sourceInstanceId);
  if (!unit) return state;
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion'
    || definition.mayStepAfterRangedStrike !== true
    || minionDisabled(state, unit)) return state;
  const pending = deepFreeze({
    ...state,
    decisionSeat: unit.controller,
    pendingRangedStep: { seat: unit.controller, sourceInstanceId },
    phase: 'ranged-step' as const,
  });
  return rangedStepDescriptors(pending, unit.controller)
    .some((descriptor) => descriptor.kind === 'resolve-ranged-step' && descriptor.choice === 'step')
    ? pending
    : state;
}

function dragProjectileDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const directions = ['east', 'north', 'south', 'west'] as const;
  const allUnits = [...unitRefs(state, 'north'), ...unitRefs(state, 'south')];
  return unitRefs(state, seat).flatMap((shooter) => {
    if (shooter.kind !== 'minion') return [];
    const unit = state.realm.units.find(({ instanceId }) => instanceId === shooter.instanceId);
    if (!unit) throw new Error('unreachable drag projectile shooter');
    const definition = cardDefinition(state, unit.cardId);
    const status = unitStatus(state, shooter);
    if (definition.cardType !== 'minion'
      || !definition.shootsDragProjectile
      || status.disabled
      || status.tapped
      || status.summoningSickness) return [];
    return directions.flatMap<GameActionDescriptor>((direction) => {
      const path: GameLocation[] = [{ cell: status.location, region: status.region }];
      while (true) {
        const location = path.at(-1)!;
        const hits = allUnits.filter((ref) => {
          const target = unitStatus(state, ref);
          return !target.stealthed
            && target.occupiedCells.includes(location.cell)
            && target.region === location.region
            && (path.length > 1 || ref.seat !== seat);
        }).sort((left, right) => left.instanceId.localeCompare(right.instanceId));
        if (hits.length > 0) {
          return hits.flatMap((hit) => ([false, true] as const).map((fightOnArrival) => ({
            direction,
            fightOnArrival,
            hit,
            kind: 'shoot-drag-projectile' as const,
            path,
            shooterInstanceId: shooter.instanceId,
          })));
        }
        const nextCell = projectileStep(location.cell, direction);
        const next = nextCell ? { cell: nextCell, region: status.region } : undefined;
        if (!next || !locationExists(state, next)) break;
        path.push(next);
      }
      return [{
        direction,
        fightOnArrival: false,
        hit: null,
        kind: 'shoot-drag-projectile' as const,
        path,
        shooterInstanceId: shooter.instanceId,
      }];
    });
  });
}

function manaAbilityDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  return state.realm.units.flatMap((unit) => {
    if (unit.controller !== seat
      || minionDisabled(state, unit)
      || unit.tapped
      || unit.summoningSickness) return [];
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion' && definition.tapForMana
      ? [{ amount: definition.tapForMana, kind: 'activate-mana' as const, unitInstanceId: unit.instanceId }]
      : [];
  });
}

function areaDamageAbilityDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  return state.realm.units.flatMap((unit) => {
    if (unit.controller !== seat
      || minionDisabled(state, unit)
      || unit.tapped
      || unit.summoningSickness) return [];
    const definition = cardDefinition(state, unit.cardId);
    if (definition.cardType !== 'minion'
      || definition.tapToDamageEachUnitAtAdjacentLocation !== 2) return [];
    const occupied = new Set(unitOccupiedCells(unit));
    return [...new Set(unitOccupiedCells(unit).flatMap((cell) => borderingCells(cell)))]
      .filter((cell) => !occupied.has(cell))
      .map((cell): GameLocation => ({ cell, region: unit.region }))
      .filter((location) => locationExists(state, location))
      .map((targetLocation) => ({
        kind: 'activate-area-damage' as const,
        sourceInstanceId: unit.instanceId,
        targetLocation,
      }));
  });
}

function discardRandomDamageDescriptors(
  state: GameState,
  seat: GameSeat,
): readonly GameActionDescriptor[] {
  const discardCardInstanceIds = state.players[seat].hand.spellbook
    .map(({ instanceId }) => instanceId)
    .sort((left, right) => left.localeCompare(right));
  if (discardCardInstanceIds.length === 0) return [];
  return state.realm.units.flatMap((unit) => {
    if (unit.controller !== seat || minionDisabled(state, unit)) return [];
    const definition = cardDefinition(state, unit.cardId);
    if (definition.cardType !== 'minion'
      || definition.discardSpellToDamageRandomOtherUnitHere === undefined) return [];
    return discardCardInstanceIds.map((discardCardInstanceId) => ({
      discardCardInstanceId,
      kind: 'activate-discard-random-damage' as const,
      sourceInstanceId: unit.instanceId,
    }));
  });
}

function sparkmageDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
  if (avatarDefinition.cardType !== 'avatar'
    || avatarDefinition.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn !== true
    || player.avatar.tapped) return [];
  return [
    player.avatar.location,
    ...borderingCells(player.avatar.location),
    ...diagonalCells(player.avatar.location),
  ]
    .map((cell): GameLocation => ({ cell, region: player.avatar.region }))
    .filter((targetLocation) => locationExists(state, targetLocation))
    .sort((left, right) => left.cell.localeCompare(right.cell))
    .map((targetLocation) => ({
      kind: 'activate-sparkmage' as const,
      sourceInstanceId: player.avatar.card.instanceId,
      targetLocation,
    }));
}

function siteDestructionDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  return REALM_CELLS.flatMap((sourceCell) => {
    const source = state.realm.sites[sourceCell];
    if (!source || isRubble(source) || source.controller !== seat) return [];
    const definition = cardDefinition(state, source.cardId);
    if (definition.cardType !== 'site' || definition.sacrificeToDestroyNearbySite !== true) return [];
    const nearby = new Set([sourceCell, ...borderingCells(sourceCell), ...diagonalCells(sourceCell)]);
    return REALM_CELLS.flatMap((targetCell) => {
      const target = nearby.has(targetCell) ? state.realm.sites[targetCell] : undefined;
      return target
        ? [{
          kind: 'activate-site-destruction' as const,
          sourceSiteInstanceId: source.instanceId,
          targetCell,
          targetSiteInstanceId: target.instanceId,
        }]
        : [];
    });
  });
}

function siteFlightDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  if (affinity(state, seat).air < 3) return [];
  return REALM_CELLS.flatMap((sourceCell) => {
    const source = state.realm.sites[sourceCell];
    if (!source || isRubble(source) || source.controller !== seat
      || source.lastFlightTurn === state.turnNumber
      || siteCannotBeMovedDestroyedOrModified(state, source)) return [];
    const definition = cardDefinition(state, source.cardId);
    if (definition.cardType !== 'site'
      || definition.flyToNearbyVoidOncePerTurnAtAirThreshold !== 3) return [];
    return [...borderingCells(sourceCell), ...diagonalCells(sourceCell)]
      .filter((targetCell) => state.realm.sites[targetCell] === undefined)
      .sort()
      .map((targetCell) => ({
        kind: 'fly-site' as const,
        sourceSiteInstanceId: source.instanceId,
        targetCell,
      }));
  });
}

function attackTargets(state: GameState, pending: PendingCombat): readonly CombatTarget[] {
  const defendingSeat = otherSeat(pending.attackingSeat);
  const attackerAirborne = unitStatus(state, pending.attacker).airborne;
  const region = pending.region ?? 'surface';
  const attackerCells = new Set(unitRefOccupiedCells(state, pending.attacker));
  const targets: CombatTarget[] = unitRefs(state, defendingSeat)
    .filter((ref) => {
      const target = unitStatus(state, ref);
      return target.occupiedCells.some((cell) => attackerCells.has(cell))
        && target.region === region
        && !target.stealthed
        && (!target.airborne || attackerAirborne);
    });
  if (region === 'surface' && unitStatus(state, pending.attacker).canAttackSites) {
    for (const cell of [...attackerCells].sort()) {
      const site = state.realm.sites[cell];
      if (site?.controller === defendingSeat) {
        targets.push({ instanceId: site.instanceId, kind: 'site', seat: defendingSeat });
      }
    }
  }
  return targets;
}

function responseUnitRefs(
  state: GameState,
  pending: PendingCombat,
  intercept: boolean,
): readonly GameUnitRef[] {
  const respondingSeat = otherSeat(pending.attackingSeat);
  const unavailable = new Set([
    ...pending.defenders.map(({ instanceId }) => instanceId),
    ...(pending.originalTarget?.kind === 'site' ? [] : [pending.originalTarget?.instanceId]),
  ].filter((value): value is StateHash => value !== undefined));
  const pendingLocation: GameLocation = { cell: pending.cell, region: pending.region ?? 'surface' };
  const attackerCells = new Set(unitRefOccupiedCells(state, pending.attacker));
  return unitRefs(state, respondingSeat).filter((ref) => {
    if (unavailable.has(ref.instanceId) || !readyUnit(state, ref)) return false;
    const unit = unitStatus(state, ref);
    if (!unit.canRespondToAttack) return false;
    if (intercept && unitStatus(state, pending.attacker).stealthed) return false;
    if (intercept
      && unitStatus(state, pending.attacker).airborne
      && !unit.airborne
      && !unit.ranged) return false;
    return intercept
      ? unit.region === pendingLocation.region
        && attackerCells.has(pending.cell)
        && unit.occupiedCells.includes(pending.cell)
      : defendPaths(state, ref, pendingLocation).length > 0;
  });
}

function pendingCombat(state: GameState): PendingCombat {
  if (!state.pendingCombat) throw new Error('unreachable missing pending combat');
  return state.pendingCombat;
}

function pendingDeathriteOrder(state: GameState): Readonly<{
  seat: GameSeat;
  sources: readonly PendingDeathriteSource[];
}> {
  const batch = state.pendingDeathrites?.batches[0];
  const sources = batch?.stage === 'active-order'
    ? batch.activeRemaining
    : batch?.stage === 'non-active-order'
      ? batch.nonActiveRemaining
      : undefined;
  const seat = sources?.[0]?.controller;
  if (!sources || sources.length < 2 || !seat) {
    throw new Error('unreachable missing pending Deathrite order');
  }
  return { seat, sources };
}

function actionDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  if (state.terminal.status === 'finished' || state.phase === 'terminal' || seat !== state.decisionSeat) return [];
  const player = state.players[seat];
  if (state.phase === 'deathrite-order') {
    const pending = pendingDeathriteOrder(state);
    if (pending.seat !== seat) throw new Error('unreachable wrong Deathrite ordering seat');
    return pending.sources.map(({ instanceId: sourceInstanceId }) => ({
      kind: 'order-deathrites' as const,
      sourceInstanceId,
    }));
  }
  if (state.phase === 'mulligan') return mulliganDescriptors(player);
  if (state.phase === 'draw') return [{ kind: 'draw', zone: 'atlas' }, { kind: 'draw', zone: 'spellbook' }];
  if (state.phase === 'movement') return basicMovementDescriptors(state, seat);
  if (state.phase === 'ranged-step') return rangedStepDescriptors(state, seat);
  if (state.phase === 'cemetery-summon') return cemeterySummonDescriptors(state, seat);
  if (state.phase === 'chain-magic') return chainMagicDescriptors(state, seat);
  if (state.phase === 'start-turn') {
    const pending = state.pendingStartTurn;
    if (!pending || pending.seat !== seat || pending.remainingTriggerInstanceIds.length === 0) {
      throw new Error('unreachable missing pending start-turn trigger');
    }
    return pending.remainingTriggerInstanceIds.map((sourceInstanceId) => ({
      kind: 'resolve-start-turn-trigger' as const,
      sourceInstanceId,
    }));
  }
  if (state.phase === 'random-choice') {
    const pending = state.pendingRandomOutcome;
    if (!pending || pending.seat !== seat || pending.outcomeInstanceIds.length === 0) {
      throw new Error('unreachable missing pending random outcome');
    }
    return pending.outcomeInstanceIds.map((outcomeInstanceId) => ({
      kind: 'resolve-random-outcome' as const,
      outcomeInstanceId,
    }));
  }
  if (state.phase === 'end-turn-aura') {
    const pending = state.pendingEndTurnAura;
    const aura = state.realm.auras?.find(({ instanceId }) =>
      instanceId === pending?.auraInstanceId);
    if (!pending || pending.seat !== seat || !aura) {
      throw new Error('unreachable missing pending end-turn Aura');
    }
    if (pending.stage === 'random') {
      if (!pending.outcomeInstanceIds || pending.outcomeInstanceIds.length === 0) {
        throw new Error('unreachable missing pending end-turn Aura outcomes');
      }
      return pending.outcomeInstanceIds.map((outcomeInstanceId) => ({
        auraInstanceId: aura.instanceId,
        kind: 'resolve-end-turn-aura-random' as const,
        outcomeInstanceId,
      }));
    }
    return [
      { auraInstanceId: aura.instanceId, kind: 'resolve-end-turn-aura-move' },
      ...auraOneStepAreas(aura).map((cells) => ({
        auraInstanceId: aura.instanceId,
        cells,
        kind: 'resolve-end-turn-aura-move' as const,
      })),
    ];
  }
  if (state.phase === 'genesis') {
    if (state.pendingGenesisSpellOrder?.seat === seat) {
      return permutations(Array.from(
        { length: state.pendingGenesisSpellOrder.count },
        (_, index) => index,
      )).map((order) => ({ kind: 'resolve-genesis-spell-order' as const, order }));
    }
    if (state.pendingGenesisToken?.seat === seat) {
      return [
        { choice: 'decline', kind: 'resolve-genesis-token' },
        ...(player.mana > 0
          ? [{ choice: 'pay-one-mana' as const, kind: 'resolve-genesis-token' as const }]
          : []),
      ];
    }
    if (!state.pendingGenesisSpell
      || state.pendingGenesisSpell.seat !== seat
      || player.spellbook.length === 0) {
      throw new Error('unreachable missing pending Genesis spell');
    }
    return [
      { choice: 'keep-next', kind: 'resolve-genesis-spell' },
      { choice: 'bottom-next', kind: 'resolve-genesis-spell' },
    ];
  }
  if (state.phase === 'attack') {
    const pending = pendingCombat(state);
    return [
      ...attackTargets(state, pending).map((target) => ({
        kind: 'declare-attack' as const,
        target,
      })),
      { kind: 'decline-attack' },
    ];
  }
  if (state.phase === 'defend') {
    const pending = pendingCombat(state);
    const destination: GameLocation = { cell: pending.cell, region: pending.region ?? 'surface' };
    const defenders = responseUnitRefs(state, pending, false).flatMap((ref) => {
      const unit = unitStatus(state, ref);
      return defendPaths(state, ref, destination).map((path) => ({
        from: { cell: unit.location, region: unit.region },
        kind: 'defend' as const,
        path,
        to: destination,
        unitInstanceId: ref.instanceId,
      }));
    });
    const choices = pending.originalTarget?.kind === 'site' || pending.defenders.length === 0
      ? [pending.originalTarget?.kind !== 'site']
      : [true, false];
    return [
      ...defenders,
      ...choices.map((originalTargetParticipates) => ({
        kind: 'close-defend' as const,
        originalTargetParticipates,
      })),
    ];
  }
  if (state.phase === 'intercept') {
    const pending = pendingCombat(state);
    return [
      ...responseUnitRefs(state, pending, true).map((ref) => ({
        kind: 'intercept' as const,
        unitInstanceId: ref.instanceId,
      })),
      { kind: 'close-intercept' },
    ];
  }
  if (state.phase === 'allocate') {
    const pending = pendingCombat(state);
    const target = pending.combatants[pending.allocations.length];
    if (!target) throw new Error('unreachable completed strike allocation');
    const assigned = pending.allocations.reduce((total, allocation) => total + allocation.amount, 0);
    const remaining = strikeDamage(state, pending.attacker) - assigned;
    const last = pending.allocations.length === pending.combatants.length - 1;
    const amounts = last ? [remaining] : Array.from({ length: remaining + 1 }, (_, amount) => amount);
    return amounts.map((amount) => ({
      amount,
      kind: 'allocate-strike' as const,
      targetInstanceId: target.instanceId,
    }));
  }
  const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
  if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
  const siteDescriptors = (cells: readonly RealmCell[]): readonly GameActionDescriptor[] =>
    player.hand.atlas.flatMap((card) => cells.flatMap((cell) => {
      const { cardId, instanceId } = card;
      if (sitePlayNeedsDeathriteContinuation(state, seat, card, cell)) return [];
      const base = { cardId, cardInstanceId: instanceId, cell, kind: 'play-site' as const };
      const definition = cardDefinition(state, cardId);
      const choices = definition.cardType === 'site'
        && definition.genesisPayOneManaToSummonToken !== undefined
        ? [
          { ...base, genesisTokenChoice: 'decline' as const },
          { ...base, genesisTokenChoice: 'pay-one-mana' as const },
        ]
        : [base];
      if (definition.cardType !== 'site'
        || !definition.elements.includes('earth')
        || avatarDefinition.earthSitePlayCreatesAdjacentRubble !== true) return choices;
      const rubbleCells = borderingCells(player.avatar.location)
        .filter((rubbleCell) => rubbleCell !== cell && state.realm.sites[rubbleCell] === undefined);
      return rubbleCells.length === 0
        ? choices
        : choices.flatMap((choice) => rubbleCells.map((createRubbleAt) => ({
          ...choice,
          createRubbleAt,
        })));
    }));
  if (!player.domainEstablished) return siteDescriptors([player.avatar.location]);
  const cells = player.avatar.tapped ? [] : legalSiteCells(state, seat);
  const rubbleReplacementDescriptors: readonly GameActionDescriptor[] = player.avatar.tapped
    || avatarDefinition.replaceAdjacentRubbleWithTopAtlasSite !== true
    || player.atlas.length === 0
    ? []
    : borderingCells(player.avatar.location).flatMap((cell) => {
      const site = state.realm.sites[cell];
      const top = player.atlas[0];
      return site && isRubble(site) && top
        && !sitePlayNeedsDeathriteContinuation(state, seat, top, cell)
        ? [{
          kind: 'replace-rubble-with-top-atlas-site' as const,
          targetCell: cell,
          targetRubbleInstanceId: site.instanceId,
        }]
        : [];
    });
  return [
    ...siteDescriptors(cells),
    ...rubbleReplacementDescriptors,
    ...(player.avatar.tapped ? [] : [{ kind: 'draw-site' as const }]),
    ...(!player.avatar.tapped && avatarDefinition.drawSpell ? [{ kind: 'draw-spell' as const }] : []),
    ...summonDescriptors(state, seat),
    ...artifactDescriptors(state, seat),
    ...auraDescriptors(state, seat),
    ...magicDescriptors(state, seat),
    ...artifactDamageAbilityDescriptors(state, seat),
    ...artifactDiscardAreaDamageAbilityDescriptors(state, seat),
    ...artifactRollDamageAbilityDescriptors(state, seat),
    ...pickUpArtifactDescriptors(state, seat),
    ...dropArtifactDescriptors(state, seat),
    ...siteFlightDescriptors(state, seat),
    ...siteDestructionDescriptors(state, seat),
    ...areaDamageAbilityDescriptors(state, seat),
    ...discardRandomDamageDescriptors(state, seat),
    ...sparkmageDescriptors(state, seat),
    ...manaAbilityDescriptors(state, seat),
    ...movementDescriptors(state, seat),
    ...dragProjectileDescriptors(state, seat),
    ...rangedDescriptors(state, seat),
    { kind: 'end-turn' },
  ];
}

function actionLabel(state: GameState, descriptor: GameActionDescriptor): string {
  if (descriptor.kind === 'mulligan') {
    const count = descriptor.atlasOrder.length + descriptor.spellbookOrder.length;
    return count === 0
      ? 'Keep opening hand'
      : `Mulligan ${count} (${descriptor.atlasOrder.length} atlas, ${descriptor.spellbookOrder.length} spellbook)`;
  }
  if (descriptor.kind === 'draw') return `Draw from ${descriptor.zone}`;
  if (descriptor.kind === 'draw-site') return 'Draw a site with Avatar';
  if (descriptor.kind === 'draw-spell') return 'Draw a spell with Avatar';
  if (descriptor.kind === 'replace-rubble-with-top-atlas-site') {
    return `Replace Rubble at ${descriptor.targetCell} with the top site of your Atlas`;
  }
  if (descriptor.kind === 'resolve-genesis-token') {
    const pending = state.pendingGenesisToken;
    const source = pending && state.realm.sites[pending.cell];
    const definition = source && !isRubble(source) ? cardDefinition(state, source.cardId) : undefined;
    const token = definition?.cardType === 'site'
      ? definition.genesisPayOneManaToSummonToken
      : undefined;
    return descriptor.choice === 'pay-one-mana'
      ? `Pay 1 to summon ${token ?? 'the Genesis token'}`
      : 'Decline the optional Genesis token';
  }
  if (descriptor.kind === 'play-site') {
    const definition = cardDefinition(state, descriptor.cardId);
    const choice = definition.cardType === 'site'
      && definition.genesisPayOneManaToSummonToken !== undefined
      ? descriptor.genesisTokenChoice === 'pay-one-mana'
        ? ' (pay 1 for Genesis)'
        : ' (decline Genesis)'
      : '';
    return `Play ${descriptor.cardId} at ${descriptor.cell}${choice}`
      + (descriptor.createRubbleAt ? ` — create Rubble at ${descriptor.createRubbleAt}` : '')
      + (descriptor.fromTopAtlas ? ' from the top of your Atlas' : '');
  }
  if (descriptor.kind === 'resolve-genesis-spell') {
    const nextSpell = state.players[state.decisionSeat].spellbook[0];
    return descriptor.choice === 'bottom-next'
      ? `Put ${nextSpell?.cardId ?? 'next spell'} on bottom`
      : `Keep ${nextSpell?.cardId ?? 'next spell'} on top`;
  }
  if (descriptor.kind === 'resolve-genesis-spell-order') {
    const top = state.players[state.decisionSeat].spellbook.slice(0, descriptor.order.length);
    return `Order next spells ${descriptor.order.map((index) => top[index]?.cardId ?? '?').join(', ')}`;
  }
  if (descriptor.kind === 'resolve-end-turn-aura-random') {
    return `Choose unit ${descriptor.outcomeInstanceId.slice(0, 15)}… for the Aura's random damage`;
  }
  if (descriptor.kind === 'resolve-end-turn-aura-move') {
    return descriptor.cells
      ? `Move Aura to ${descriptor.cells.join(', ')}`
      : 'Keep Aura in place';
  }
  if (descriptor.kind === 'cast-artifact') {
    const destination = descriptor.bearer
      ? `carried by ${descriptor.bearer.kind} ${descriptor.bearer.instanceId.slice(0, 15)}…`
      : `uncarried at ${descriptor.cell}`;
    return `Cast ${descriptor.cardId} ${destination} (${descriptor.manaCost} mana)`;
  }
  if (descriptor.kind === 'cast-aura') {
    return `Conjure ${descriptor.cardId} across ${descriptor.cells.join(', ')}`;
  }
  if (descriptor.kind === 'begin-chain-magic') {
    return `Choose ${descriptor.target.kind} ${descriptor.target.instanceId.slice(0, 15)}… as the first target for ${descriptor.cardId}`;
  }
  if (descriptor.kind === 'extend-chain-magic') {
    return `Add ${descriptor.target.kind} ${descriptor.target.instanceId.slice(0, 15)}… as a chained target (+${CHAIN_MAGIC_EXTRA_TARGET_MANA} mana)`;
  }
  if (descriptor.kind === 'resolve-chain-magic') {
    const pending = state.pendingChainMagic;
    if (!pending) throw new Error('unreachable missing chained Magic label');
    const definition = cardDefinition(state, pending.cardId);
    if (definition.cardType !== 'magic') throw new Error('chained targets require Magic');
    const manaCost = definition.manaCost
      + CHAIN_MAGIC_EXTRA_TARGET_MANA * (pending.targets.length - 1);
    return `Cast ${pending.cardId} through ${pending.targets.length} chosen unit${pending.targets.length === 1 ? '' : 's'} (${manaCost} mana)`;
  }
  if (descriptor.kind === 'activate-site-destruction') {
    return `Sacrifice site to destroy ${descriptor.targetCell}`;
  }
  if (descriptor.kind === 'fly-site') {
    return `Fly site to ${descriptor.targetCell}`;
  }
  if (descriptor.kind === 'summon-minion') {
    const genesis = descriptor.genesisDamageChoice === 'target' && descriptor.genesisDamageTarget
      ? `; Genesis targets ${descriptor.genesisDamageTarget.kind} ${descriptor.genesisDamageTarget.instanceId.slice(0, 15)}…`
      : descriptor.genesisDamageChoice === 'decline' ? '; decline Genesis' : '';
    if (state.phase === 'cemetery-summon') {
      return `Summon selected dead minion ${descriptor.cardId} at ${descriptor.cell}${descriptor.region ? ` ${descriptor.region}` : ''}${genesis}`;
    }
    const caster = state.realm.units.find(({ instanceId }) =>
      instanceId === descriptor.casterInstanceId);
    const payment = descriptor.paymentMode === 'random-card-discard'
      ? 'discard random card'
      : descriptor.sacrificedMinionInstanceIds
        ? `${descriptor.manaCost} mana + sacrifice ${descriptor.sacrificedMinionInstanceIds.length} minion${descriptor.sacrificedMinionInstanceIds.length === 1 ? '' : 's'}`
      : `${descriptor.manaCost} mana`;
    return `Summon ${descriptor.cardId} at ${descriptor.cell}${descriptor.region ? ` ${descriptor.region}` : ''} (${payment})${genesis}`
      + (caster ? ` with minion ${caster.instanceId.slice(0, 15)}…` : '');
  }
  if (descriptor.kind === 'cast-magic') {
    const caster = state.realm.units.find(({ instanceId }) =>
      instanceId === descriptor.casterInstanceId);
    const withCaster = (label: string): string =>
      label + (caster ? ` with minion ${caster.instanceId.slice(0, 15)}…` : '');
    if (descriptor.cemeteryMinionInstanceId) {
      return withCaster(
        `Cast ${descriptor.cardId} to return minion ${descriptor.cemeteryMinionInstanceId.slice(0, 15)}…`,
      );
    }
    if (descriptor.discardSiteInstanceId && descriptor.targetLocation) {
      return withCaster(
        `Cast ${descriptor.cardId} at ${descriptor.targetLocation.cell}; discard site ${descriptor.discardSiteInstanceId.slice(0, 15)}…`,
      );
    }
    if (descriptor.ally && descriptor.temptedEnemy && descriptor.temptedDestination) {
      return withCaster(
        `Cast ${descriptor.cardId}: ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}… tempts minion ${descriptor.temptedEnemy.instanceId.slice(0, 15)}… to ${descriptor.temptedDestination.cell}`,
      );
    }
    if (descriptor.target) {
      if (descriptor.ally) {
        return withCaster(
          `Cast ${descriptor.cardId}: ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}… fights ${descriptor.target.kind} ${descriptor.target.instanceId.slice(0, 15)}…`,
        );
      }
      return withCaster(
        `Cast ${descriptor.cardId} on ${descriptor.target.kind} ${descriptor.target.instanceId.slice(0, 15)}…`,
      );
    }
    if (descriptor.targetArtifactInstanceId) {
      return withCaster(
        `Cast ${descriptor.cardId} on artifact ${descriptor.targetArtifactInstanceId.slice(0, 15)}…`,
      );
    }
    if (descriptor.ally && descriptor.targetLocation) {
      return withCaster(
        `Cast ${descriptor.cardId} to teleport ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}… to ${descriptor.targetLocation.cell}`,
      );
    }
    if (descriptor.ally && descriptor.allyDestination) {
      const ally = unitStatus(state, descriptor.ally);
      const stays = ally.location === descriptor.allyDestination.cell
        && ally.region === descriptor.allyDestination.region;
      const strikeLocation = descriptor.allyStrikeLocation ?? descriptor.allyDestination;
      return withCaster(
        `Cast ${descriptor.cardId}: ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}… `
          + (stays
            ? `stays and strikes enemies at ${strikeLocation.cell}`
            : `steps to ${descriptor.allyDestination.cell} and strikes enemies at ${strikeLocation.cell}`),
      );
    }
    if (descriptor.ally) {
      const definition = cardDefinition(state, descriptor.cardId);
      const effect = definition.cardType === 'magic'
        && definition.grantPowerToAllyThisTurn === 2
        ? 'grant +2 power'
        : 'grant Charge';
      return withCaster(
        `Cast ${descriptor.cardId} to ${effect} to ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}…`,
      );
    }
    return withCaster(descriptor.targetLocation
      ? `Cast ${descriptor.cardId} at ${descriptor.targetLocation.cell} ${descriptor.targetLocation.region}`
      : `Cast ${descriptor.cardId}`);
  }
  if (descriptor.kind === 'move-and-attack') {
    return descriptor.path.length === 1
      ? `Tap ${descriptor.unitInstanceId.slice(0, 15)}… without moving${descriptor.to.region === 'surface' ? '' : ` ${descriptor.to.region}`}`
      : `Move ${descriptor.unitInstanceId.slice(0, 15)}… ${descriptor.path.map(({ cell, region }) => `${cell}${region === 'surface' ? '' : ` ${region}`}`).join(' → ')}`;
  }
  if (descriptor.kind === 'continue-basic-movement') {
    const pending = state.pendingBasicMovement;
    const destination = pending?.path[pending.pathIndex + 1];
    return destination
      ? `Continue ${descriptor.unitInstanceId.slice(0, 15)}… to ${destination.cell}`
      : `Finish ${pending?.purpose === 'defend' ? 'Defend' : 'Move and Attack'}`;
  }
  if (descriptor.kind === 'shoot-projectile') {
    const target = descriptor.hit
      ? `${descriptor.hit.kind} ${descriptor.hit.instanceId.slice(0, 15)}…`
      : 'nothing';
    return `Shoot ${descriptor.direction} at ${target}`;
  }
  if (descriptor.kind === 'resolve-ranged-step') {
    return descriptor.choice === 'decline'
      ? 'Decline the optional step after the Ranged strike'
      : `Step ${descriptor.unitInstanceId.slice(0, 15)}… to ${descriptor.to.cell}`;
  }
  if (descriptor.kind === 'shoot-drag-projectile') {
    const target = descriptor.hit
      ? `${descriptor.hit.kind} ${descriptor.hit.instanceId.slice(0, 15)}…`
      : 'nothing';
    return `Hook ${descriptor.direction} at ${target}${descriptor.fightOnArrival ? ' and fight' : ''}`;
  }
  if (descriptor.kind === 'pick-up-artifacts') {
    return `Pick up ${descriptor.artifactInstanceIds.length} artifact${descriptor.artifactInstanceIds.length === 1 ? '' : 's'} with ${descriptor.unit.kind} ${descriptor.unit.instanceId.slice(0, 15)}…`;
  }
  if (descriptor.kind === 'drop-artifacts') {
    return `Drop ${descriptor.artifactInstanceIds.length} artifact${descriptor.artifactInstanceIds.length === 1 ? '' : 's'} with ${descriptor.unit.kind} ${descriptor.unit.instanceId.slice(0, 15)}…`;
  }
  if (descriptor.kind === 'activate-artifact-damage') {
    return `Tap bearer and ally to activate artifact ${descriptor.artifactInstanceId.slice(0, 15)}… on ${descriptor.target.kind} ${descriptor.target.instanceId.slice(0, 15)}…`;
  }
  if (descriptor.kind === 'activate-artifact-discard-area-damage') {
    return `Tap bearer and ally, discard ${descriptor.discardCardInstanceId.slice(0, 15)}…, and activate artifact ${descriptor.artifactInstanceId.slice(0, 15)}… at ${descriptor.targetLocation.cell}`;
  }
  if (descriptor.kind === 'activate-artifact-roll-damage') {
    return `Tap ${descriptor.pusher.kind} ${descriptor.pusher.instanceId.slice(0, 15)}… to roll artifact ${descriptor.direction} through ${descriptor.path.map(({ cell }) => cell).join(' → ')}`;
  }
  if (descriptor.kind === 'decline-attack') return 'Decline attack';
  if (descriptor.kind === 'declare-attack') return `Attack ${descriptor.target.kind} ${descriptor.target.instanceId.slice(0, 15)}…`;
  if (descriptor.kind === 'defend') {
    return `Defend with ${descriptor.unitInstanceId.slice(0, 15)}… via ${descriptor.path.map(({ cell, region }) => `${cell}${region === 'surface' ? '' : ` ${region}`}`).join(' → ')}`;
  }
  if (descriptor.kind === 'close-defend') {
    return descriptor.originalTargetParticipates ? 'Close defend window; keep target' : 'Close defend window; remove target';
  }
  if (descriptor.kind === 'intercept') return `Intercept with ${descriptor.unitInstanceId.slice(0, 15)}…`;
  if (descriptor.kind === 'close-intercept') return 'Close intercept window';
  if (descriptor.kind === 'allocate-strike') {
    return `Assign ${descriptor.amount} damage to ${descriptor.targetInstanceId.slice(0, 15)}…`;
  }
  if (descriptor.kind === 'activate-area-damage') {
    return `Tap ${descriptor.sourceInstanceId.slice(0, 15)}… to damage every unit at ${descriptor.targetLocation.cell}`;
  }
  if (descriptor.kind === 'activate-discard-random-damage') {
    const source = state.realm.units.find(({ instanceId }) =>
      instanceId === descriptor.sourceInstanceId);
    const discard = source
      ? state.players[source.controller].hand.spellbook.find(({ instanceId }) =>
        instanceId === descriptor.discardCardInstanceId)
      : undefined;
    return `Discard ${discard?.cardId ?? 'selected Spellbook card'} to activate ${descriptor.sourceInstanceId.slice(0, 15)}…`;
  }
  if (descriptor.kind === 'activate-sparkmage') {
    const amount = state.players[state.decisionSeat].airThresholdsCastThisTurn ?? 0;
    return `Tap Sparkmage to deal ${amount} to a random other unit at ${descriptor.targetLocation.cell}`;
  }
  if (descriptor.kind === 'resolve-random-outcome') {
    const pendingAction = state.pendingRandomOutcome?.action;
    const raiseDeadCard = pendingAction?.kind === 'cast-magic'
      ? state.players[state.decisionSeat].hand.spellbook.find(({ instanceId }) =>
        instanceId === pendingAction.cardInstanceId)
      : undefined;
    const raiseDeadDefinition = raiseDeadCard && cardDefinition(state, raiseDeadCard.cardId);
    const deadMinion = raiseDeadDefinition?.cardType === 'magic'
      && raiseDeadDefinition.summonRandomMinionFromAnyCemetery === true
      ? cemeteryMinionCandidates(state).find(({ instanceId }) =>
        instanceId === descriptor.outcomeInstanceId)
      : undefined;
    if (deadMinion) {
      return `Lucky Charm chooses ${deadMinion.cardId} ${deadMinion.instanceId.slice(0, 15)}…`;
    }
    const location = state.pendingRandomOutcome?.action.kind === 'resolve-start-turn-trigger'
      ? randomSiteOrVoidLocations(state).find(({ instanceId }) =>
        instanceId === descriptor.outcomeInstanceId)?.location
      : undefined;
    if (location) return `Lucky Charm chooses ${location.cell} ${location.region}`;
    return `Lucky Charm chooses ${descriptor.outcomeInstanceId.slice(0, 15)}…`;
  }
  if (descriptor.kind === 'resolve-start-turn-trigger') {
    return `Resolve start-turn trigger for ${descriptor.sourceInstanceId.slice(0, 15)}…`;
  }
  if (descriptor.kind === 'order-deathrites') {
    const source = state.pendingDeathrites?.batches[0]
      ?.stage === 'active-order'
      ? state.pendingDeathrites.batches[0].activeRemaining.find(({ instanceId }) =>
        instanceId === descriptor.sourceInstanceId)
      : state.pendingDeathrites?.batches[0]?.nonActiveRemaining.find(({ instanceId }) =>
        instanceId === descriptor.sourceInstanceId);
    return `Order ${source?.unit.cardId ?? descriptor.sourceInstanceId.slice(0, 15) + '…'} first within your Deathrites`;
  }
  if (descriptor.kind === 'activate-mana') {
    return 'Tap ' + descriptor.unitInstanceId.slice(0, 15) + '… for ' + descriptor.amount + ' mana';
  }
  return 'End turn';
}

const legalActionCache = new WeakMap<GameState, Map<GameSeat, readonly GameLegalAction[]>>();

export function legalGameActions(state: GameState, seat: GameSeat): readonly GameLegalAction[] {
  const cached = legalActionCache.get(state)?.get(seat);
  if (cached) return cached;
  const actions = orderLegalActions(actionDescriptors(state, seat).map((descriptor) => ({
    actionId: opaqueActionId('sorcery-core-v1', seat, state.stateVersion, descriptor),
    descriptor,
    label: actionLabel(state, descriptor),
    seat,
    stateVersion: state.stateVersion,
  })));
  const bySeat = legalActionCache.get(state) ?? new Map();
  bySeat.set(seat, actions);
  legalActionCache.set(state, bySeat);
  return actions;
}

function withStateVersion(state: GameState, changes: Partial<GameState>): GameState {
  const stateVersion = state.stateVersion + 1;
  if (!Number.isSafeInteger(stateVersion)) throw new RangeError('game state version exhausted');
  return deepFreeze({
    ...state,
    ...changes,
    engine: deepFreeze({ ...(changes.engine ?? state.engine), stateVersion }),
    stateVersion,
  });
}

function replacePlayer(
  state: GameState,
  seat: GameSeat,
  player: PlayerState,
): Readonly<Record<GameSeat, PlayerState>> {
  return deepFreeze({ ...state.players, [seat]: player });
}

function healAvatar(
  player: PlayerState,
  maximumLife: number,
  attemptedAmount: number,
): readonly [PlayerState, number] {
  const life = player.avatar.life === 0
    ? 0
    : Math.min(maximumLife, player.avatar.life + attemptedAmount);
  return [deepFreeze({ ...player, avatar: { ...player.avatar, life } }), life - player.avatar.life];
}

function loseAvatarLife(
  player: PlayerState,
  attemptedAmount: number,
  turnNumber: number,
): readonly [PlayerState, number, boolean] {
  const life = Math.max(0, player.avatar.life - attemptedAmount);
  const reachedDeathsDoor = player.avatar.life > 0 && life === 0;
  return [
    deepFreeze({
      ...player,
      avatar: {
        ...player.avatar,
        ...(reachedDeathsDoor ? { deathDoorTurn: turnNumber } : {}),
        life,
      },
    }),
    player.avatar.life - life,
    reachedDeathsDoor,
  ];
}

function orderedCards(hand: readonly CardInstance[], ids: readonly string[]): readonly CardInstance[] {
  return ids.map((id) => {
    const card = hand.find(({ instanceId }) => instanceId === id);
    if (!card) throw new Error('unreachable mulligan card');
    return card;
  });
}

function resolveMulliganZone(
  hand: readonly CardInstance[],
  deck: readonly CardInstance[],
  order: readonly string[],
): Readonly<{ deck: readonly CardInstance[]; hand: readonly CardInstance[] }> {
  const returned = orderedCards(hand, order);
  const selected = new Set(order);
  const kept = hand.filter(({ instanceId }) => !selected.has(instanceId));
  const withReturned = [...deck, ...returned];
  return deepFreeze({
    deck: withReturned.slice(order.length),
    hand: [...kept, ...withReturned.slice(0, order.length)],
  });
}

function siteCount(state: GameState, seat: GameSeat): number {
  return Object.values(state.realm.sites).filter((site) => site.controller === seat).length;
}

function moveUnit(
  state: GameState,
  ref: GameUnitRef,
  location: GameLocation,
  tap: boolean,
): Readonly<{ players: GameState['players']; realm: GameState['realm'] }> {
  if (ref.kind === 'avatar') {
    const player = state.players[ref.seat];
    if (player.avatar.card.instanceId !== ref.instanceId) throw new Error('unreachable Avatar move');
    return {
      players: replacePlayer(state, ref.seat, deepFreeze({
        ...player,
        avatar: {
          ...player.avatar,
          location: location.cell,
          region: location.region,
          tapped: tap || player.avatar.tapped,
        },
      })),
      realm: state.realm,
    };
  }
  if (!state.realm.units.some(({ instanceId }) => instanceId === ref.instanceId)) {
    throw new Error('unreachable minion move');
  }
  const moving = state.realm.units.find(({ instanceId }) => instanceId === ref.instanceId)!;
  const movingDefinition = cardDefinition(state, moving.cardId);
  if (movingDefinition.cardType !== 'minion') throw new Error('realm minion lacks minion definition');
  const retainsPlanarGateVoidwalk = movingDefinition.voidwalk !== true
    && !minionDisabled(state, moving)
    && location.region === 'void'
    && (moving.planarGateVoidwalk === true || minionAtPlanarGate(state, moving));
  const artifacts = state.realm.artifacts?.map((artifact) =>
    'bearer' in artifact
      && artifact.bearer.instanceId === ref.instanceId
      && artifact.bearer.kind === ref.kind
      && artifact.bearer.seat === ref.seat
      && artifact.bearerCell
      ? deepFreeze({
        ...artifact,
        bearerCell: translatedFootprint(
          [artifact.bearerCell],
          moving.location,
          location.cell,
        )![0]!,
      })
      : artifact);
  return {
    players: state.players,
    realm: {
      ...state.realm,
      ...(artifacts ? { artifacts } : {}),
      units: state.realm.units.map((unit) => {
        if (unit.instanceId !== ref.instanceId) return unit;
        const baseUnit = { ...unit };
        delete baseUnit.planarGateVoidwalk;
        return deepFreeze({
          ...baseUnit,
          location: location.cell,
          ...(unit.occupiedCells
            ? {
              occupiedCells: translatedFootprint(
                unit.occupiedCells,
                unit.location,
                location.cell,
              ) as TwoByTwoArea,
            }
            : {}),
          region: location.region,
          tapped: tap || unit.tapped,
          ...(retainsPlanarGateVoidwalk ? { planarGateVoidwalk: true as const } : {}),
        });
      }),
    },
  };
}

function startTurnTriggerUnit(
  state: GameState,
  seat: GameSeat,
  sourceInstanceId: StateHash,
): UnitInstance | undefined {
  const unit = state.realm.units.find(({ instanceId }) => instanceId === sourceInstanceId);
  if (!unit || unit.controller !== seat || minionDisabled(state, unit)) return undefined;
  const definition = cardDefinition(state, unit.cardId);
  return definition.cardType === 'minion'
    && definition.atStartOfControllerTurnTeleportToRandomSiteOrVoid === true
    ? unit
    : undefined;
}

function startTurnTriggerInstanceIds(
  state: GameState,
  seat: GameSeat,
): readonly StateHash[] {
  return state.realm.units
    .flatMap(({ instanceId }) => startTurnTriggerUnit(state, seat, instanceId)
      ? [instanceId]
      : [])
    .sort((left, right) => left.localeCompare(right));
}

function finishStartTurnTrigger(
  state: GameState,
  sourceInstanceId: StateHash,
): GameState {
  const pending = state.pendingStartTurn;
  if (!pending) throw new Error('unreachable missing pending start-turn trigger');
  const remainingTriggerInstanceIds = pending.remainingTriggerInstanceIds
    .filter((instanceId) => instanceId !== sourceInstanceId)
    .filter((instanceId) => startTurnTriggerUnit(state, pending.seat, instanceId));
  if (remainingTriggerInstanceIds.length > 0 && state.terminal.status === 'active') {
    return deepFreeze({
      ...state,
      pendingStartTurn: { ...pending, remainingTriggerInstanceIds },
      phase: 'start-turn',
    });
  }
  const withoutPending = { ...state };
  delete withoutPending.pendingStartTurn;
  return deepFreeze({
    ...withoutPending,
    phase: state.terminal.status === 'finished' ? 'terminal' : 'draw',
  });
}

function resolveStartTurnTrigger(
  state: GameState,
  sourceInstanceId: StateHash,
  forcedRandomOutcomeInstanceId?: StateHash,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const pending = state.pendingStartTurn;
  const source = pending && startTurnTriggerUnit(state, pending.seat, sourceInstanceId);
  if (state.phase !== 'start-turn'
    || !pending
    || pending.seat !== state.decisionSeat
    || !pending.remainingTriggerInstanceIds.includes(sourceInstanceId)
    || !source) {
    throw new Error('unreachable illegal start-turn trigger');
  }
  const candidates = randomSiteOrVoidLocations(state);
  if (candidates.length === 0) {
    const completed = finishStartTurnTrigger(state, sourceInstanceId);
    return [
      withStateVersion(completed, {}),
      [{
        payload: { reason: 'no-site-or-void', seat: pending.seat, sourceInstanceId },
        type: 'unit-teleport-failed',
      }],
      [],
    ];
  }
  const selected = resolveRandomOutcome(
    state,
    candidates.map(({ instanceId }) => instanceId),
    'start_turn_random_teleport',
    'realm_site_or_void_location',
    forcedRandomOutcomeInstanceId,
  );
  const destination = candidates.find(({ instanceId }) =>
    instanceId === selected.outcomeInstanceId)!.location;
  const ref: GameUnitRef = {
    instanceId: source.instanceId,
    kind: 'minion',
    seat: source.controller,
  };
  const status = unitStatus(state, ref);
  const from: GameLocation = { cell: status.location, region: status.region };
  const stays = from.cell === destination.cell && from.region === destination.region;
  const legal = stays || locationExists(state, destination)
    && (destination.region !== 'void' || status.voidwalk)
    && unitEntryAllowed(
      state,
      from,
      destination,
      status.airborne,
      true,
      status.attack,
      'teleport',
    );
  const selectedState = deepFreeze({ ...state, engine: selected.engine });
  const outcomes: GameOutcome[] = [];
  let effectState = selectedState;
  if (legal && !stays) {
    const moved = moveUnit(selectedState, ref, destination, false);
    const teleported = deepFreeze({ ...selectedState, players: moved.players, realm: moved.realm });
    outcomes.push({
      payload: {
        from,
        outcomeInstanceId: selected.outcomeInstanceId,
        seat: pending.seat,
        sourceInstanceId,
        targetInstanceId: sourceInstanceId,
        to: destination,
      },
      type: 'unit-teleported',
    });
    const regionSettlement = settleRegionOccupancy(teleported);
    const powerSettlement = settleStaticPowerDeaths(regionSettlement.state);
    effectState = powerSettlement.state;
    outcomes.push(...regionSettlement.outcomes, ...powerSettlement.outcomes);
  } else {
    outcomes.push({
      payload: {
        from,
        outcomeInstanceId: selected.outcomeInstanceId,
        reason: legal ? 'already-there' : 'illegal-entry',
        seat: pending.seat,
        sourceInstanceId,
        to: destination,
      },
      type: legal ? 'unit-teleport-resolved' : 'unit-teleport-failed',
    });
  }
  const completed = finishStartTurnTrigger(effectState, sourceInstanceId);
  return [withStateVersion(completed, {}), outcomes, selected.randomDraws];
}

function moveAndTapUnit(
  state: GameState,
  ref: GameUnitRef,
  location: GameLocation,
): Readonly<{ players: GameState['players']; realm: GameState['realm'] }> {
  return moveUnit(state, ref, location, true);
}

function loseStealth(
  units: readonly UnitInstance[],
  refs: readonly GameUnitRef[],
  sourceInstanceId?: string,
): readonly [readonly UnitInstance[], readonly GameOutcome[]] {
  const interacting = new Set(refs
    .filter(({ kind }) => kind === 'minion')
    .map(({ instanceId }) => instanceId));
  const revealed = units.filter(({ instanceId, stealthed }) =>
    stealthed && interacting.has(instanceId));
  return [
    units.map((unit) => revealed.some(({ instanceId }) => instanceId === unit.instanceId)
      ? deepFreeze({ ...unit, stealthed: false })
      : unit),
    revealed.map(({ controller, instanceId }) => ({
      payload: { instanceId, seat: controller, ...(sourceInstanceId ? { sourceInstanceId } : {}) },
      type: 'stealth-lost',
    })),
  ];
}

function settleNearbyEnemyStealth(
  state: GameState,
): Readonly<{ outcomes: readonly GameOutcome[]; state: GameState }> {
  if (state.terminal.status === 'finished') return { outcomes: [], state };
  const sources = state.realm.units.filter((unit) => {
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion'
      && definition.nearbyEnemiesPermanentlyLoseStealth === true
      && !minionDisabled(state, unit);
  }).sort((left, right) => left.instanceId.localeCompare(right.instanceId));
  let units = state.realm.units;
  const outcomes: GameOutcome[] = [];
  for (const source of sources) {
    const refs = units.filter((unit) => unit.controller !== source.controller
      && unit.region === source.region
      && unit.stealthed
      && footprintNearby(unitOccupiedCells(source), unitOccupiedCells(unit)))
      .map(({ controller, instanceId }) => ({
        instanceId,
        kind: 'minion' as const,
        seat: controller,
      }));
    const [revealed, lost] = loseStealth(units, refs, source.instanceId);
    units = revealed;
    outcomes.push(...lost);
  }
  return outcomes.length === 0
    ? { outcomes, state }
    : {
      outcomes,
      state: deepFreeze({ ...state, realm: { ...state.realm, units } }),
    };
}

function recordInteraction(
  state: GameState,
  refs: readonly GameUnitRef[],
  sourceInstanceId?: string,
): Readonly<{
  outcomes: readonly GameOutcome[];
  players: GameState['players'];
  units: readonly UnitInstance[];
}> {
  const [revealedUnits, outcomes] = loseStealth(state.realm.units, refs, sourceInstanceId);
  const minionIds = new Set(refs
    .filter(({ kind }) => kind === 'minion')
    .map(({ instanceId }) => instanceId));
  const avatarSeats = new Set(refs
    .filter(({ kind }) => kind === 'avatar')
    .map(({ seat }) => seat));
  const interactedAvatar = (seat: GameSeat): PlayerState => {
    const player = state.players[seat];
    return avatarSeats.has(seat)
      ? deepFreeze({
        ...player,
        avatar: { ...player.avatar, lastInteractedTurn: state.turnNumber },
      })
      : player;
  };
  return {
    outcomes,
    players: deepFreeze({ north: interactedAvatar('north'), south: interactedAvatar('south') }),
    units: revealedUnits.map((unit) => minionIds.has(unit.instanceId)
      ? deepFreeze({ ...unit, lastInteractedTurn: state.turnNumber })
      : unit),
  };
}

function dropArtifactsCarriedBy(
  artifacts: readonly ArtifactInstance[] | undefined,
  bearer: Readonly<Pick<UnitInstance, 'instanceId' | 'location' | 'region'>>,
  artifactInstanceIds?: ReadonlySet<StateHash>,
): Readonly<{
  artifacts: readonly ArtifactInstance[] | undefined;
  outcomes: readonly GameOutcome[];
}> {
  if (!artifacts) return { artifacts, outcomes: [] };
  const dropped = artifacts.filter((artifact) =>
    'bearer' in artifact
      && artifact.bearer.instanceId === bearer.instanceId
      && (!artifactInstanceIds || artifactInstanceIds.has(artifact.instanceId)));
  if (dropped.length === 0) return { artifacts, outcomes: [] };
  return {
    artifacts: artifacts.map((artifact) =>
      dropped.some(({ instanceId }) => instanceId === artifact.instanceId)
        ? deepFreeze({
          cardId: artifact.cardId,
          instanceId: artifact.instanceId,
          location: 'bearer' in artifact ? artifact.bearerCell ?? bearer.location : bearer.location,
          owner: artifact.owner,
          region: bearer.region,
          source: artifact.source,
        })
        : artifact),
    outcomes: dropped.map((artifact) => ({
      payload: {
        bearerInstanceId: bearer.instanceId,
        cardId: artifact.cardId,
        cell: 'bearer' in artifact ? artifact.bearerCell ?? bearer.location : bearer.location,
        instanceId: artifact.instanceId,
        owner: artifact.owner,
        region: bearer.region,
      },
      type: 'artifact-dropped',
    })),
  };
}

type MinionDeathResolution = Readonly<{
  artifacts: readonly ArtifactInstance[] | undefined;
  outcomes: readonly GameOutcome[];
  pendingDeathrites: PendingDeathrites | null;
  players: GameState['players'];
  terminal: GameTerminal;
  units: readonly UnitInstance[];
}>;

function hasDeathrite(definition: GameCardDefinition): boolean {
  return definition.cardType === 'minion'
    && (definition.deathriteDamageEachUnitHere !== undefined
      || definition.deathriteDrawSite === true
      || definition.deathriteHeal !== undefined
      || definition.deathriteLoseLifePerNearbySiteControlled !== undefined);
}

function deathsMayRequireDeathriteContinuation(
  state: GameState,
  deaths: readonly UnitInstance[],
): boolean {
  const sources = deaths.filter((unit) => {
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion'
      && hasDeathrite(definition)
      && !minionDisabled(state, unit);
  });
  const counts = sources.reduce<Record<GameSeat, number>>(
    (current, { controller }) => ({ ...current, [controller]: current[controller] + 1 }),
    { north: 0, south: 0 },
  );
  return counts.north > 1
    || counts.south > 1
    || sources.some((unit) => {
      const definition = cardDefinition(state, unit.cardId);
      return definition.cardType === 'minion'
        && definition.deathriteDamageEachUnitHere !== undefined;
    });
}

function sitePlayNeedsDeathriteContinuation(
  state: GameState,
  seat: GameSeat,
  card: CardInstance,
  cell: RealmCell,
): boolean {
  const previous = state.realm.sites[cell];
  const definition = cardDefinition(state, card.cardId);
  if (!previous || !isRubble(previous) || definition.cardType !== 'site') return false;
  const units = state.realm.units.map((unit) => {
    if (unit.location !== cell) return unit;
    if (unit.region === 'void') return deepFreeze({ ...unit, region: 'surface' as const });
    return unit.region === 'underground' && definition.elements.includes('water')
      ? deepFreeze({ ...unit, region: 'underwater' as const })
      : unit;
  });
  const prospective = deepFreeze({
    ...state,
    realm: {
      ...state.realm,
      sites: { ...state.realm.sites, [cell]: deepFreeze({ ...card, controller: seat }) },
      units,
    },
  });
  return deathsMayRequireDeathriteContinuation(
    prospective,
    units.filter((unit) => minionRegionDisposition(prospective, unit) === 'dies'),
  );
}

function makeDeathriteBatch(
  state: GameState,
  sources: readonly PendingDeathriteSource[],
): PendingDeathriteBatch | null {
  if (sources.length === 0) return null;
  const active = sources.filter(({ controller }) => controller === state.activeSeat);
  const nonActive = sources.filter(({ controller }) => controller !== state.activeSeat);
  const activeNeedsOrder = active.length > 1;
  const nonActiveNeedsOrder = nonActive.length > 1;
  return deepFreeze({
    activeOrder: activeNeedsOrder ? [] : active,
    activeRemaining: activeNeedsOrder ? active : [],
    nonActiveOrder: nonActiveNeedsOrder ? [] : nonActive,
    nonActiveRemaining: nonActiveNeedsOrder ? nonActive : [],
    resolving: !activeNeedsOrder && !nonActiveNeedsOrder ? [...nonActive, ...active] : [],
    stage: activeNeedsOrder
      ? 'active-order'
      : nonActiveNeedsOrder ? 'non-active-order' : 'resolve',
  });
}

function collectMinionDeaths(
  state: GameState,
  startingPlayers: GameState['players'],
  units: readonly UnitInstance[],
  startingArtifacts: readonly ArtifactInstance[] | undefined,
  deaths: readonly UnitInstance[],
): Readonly<{
  artifacts: readonly ArtifactInstance[] | undefined;
  corpses: readonly UnitInstance[];
  outcomes: readonly GameOutcome[];
  players: GameState['players'];
  sources: readonly PendingDeathriteSource[];
  units: readonly UnitInstance[];
}> {
  const resolvedDeaths: UnitInstance[] = [];
  const deadIds = new Set<StateHash>();
  const sources: PendingDeathriteSource[] = [];
  let survivingUnits = [...units];
  const sourceState = (): GameState => deepFreeze({
    ...state,
    players: startingPlayers,
    realm: {
      ...state.realm,
      ...(startingArtifacts ? { artifacts: startingArtifacts } : {}),
      units: survivingUnits,
    },
  });
  const addDeaths = (newDeaths: readonly UnitInstance[]): void => {
    const snapshotState = sourceState();
    for (const dead of newDeaths) {
      if (deadIds.has(dead.instanceId)) continue;
      const definition = cardDefinition(state, dead.cardId);
      if (definition.cardType === 'minion'
        && hasDeathrite(definition)
        && !minionDisabled(snapshotState, dead)) {
        sources.push(deepFreeze({
          controller: dead.controller,
          currentPower: unitStatus(snapshotState, {
            instanceId: dead.instanceId,
            kind: 'minion',
            seat: dead.controller,
          }).attack,
          instanceId: dead.instanceId,
          lethal: definition.lethal === true || bearerHasLethal(snapshotState, {
            instanceId: dead.instanceId,
            kind: 'minion',
            seat: dead.controller,
          }),
          unit: dead,
        }));
      }
      deadIds.add(dead.instanceId);
      resolvedDeaths.push(dead);
    }
    survivingUnits = survivingUnits.filter(({ instanceId }) => !deadIds.has(instanceId));
  };

  addDeaths(deaths);
  while (true) {
    const projectedState = sourceState();
    const newlyLethal = survivingUnits.filter((unit) => unit.damage > 0
      && unit.damage >= unitStatus(projectedState, {
        instanceId: unit.instanceId,
        kind: 'minion',
        seat: unit.controller,
      }).defense);
    if (newlyLethal.length === 0) break;
    addDeaths(newlyLethal);
  }

  return {
    artifacts: startingArtifacts,
    corpses: resolvedDeaths,
    outcomes: [],
    players: startingPlayers,
    sources,
    units: survivingUnits,
  };
}

function finishMinionDeaths(
  state: GameState,
  startingPlayers: GameState['players'],
  units: readonly UnitInstance[],
  startingArtifacts: readonly ArtifactInstance[] | undefined,
  pending: PendingDeathrites,
  outcomes: readonly GameOutcome[],
): MinionDeathResolution {
  const players: Record<GameSeat, PlayerState> = {
    north: startingPlayers.north,
    south: startingPlayers.south,
  };
  let artifacts = startingArtifacts;
  const completed = [...outcomes];
  for (const dead of pending.corpses) {
    const definition = cardDefinition(state, dead.cardId);
    const token = definition.cardType === 'minion' && definition.token === true;
    if (!token) {
      const owner = players[dead.owner];
      players[dead.owner] = deepFreeze({
        ...owner,
        cemetery: [...owner.cemetery, {
          cardId: dead.cardId,
          instanceId: dead.instanceId,
          owner: dead.owner,
          source: dead.source,
        }],
      });
    }
    const drop = dropArtifactsCarriedBy(artifacts, dead);
    artifacts = drop.artifacts;
    completed.push(...drop.outcomes, {
      payload: { cardId: dead.cardId, instanceId: dead.instanceId, owner: dead.owner },
      type: 'minion-died',
    });
    if (token) {
      completed.push({
        payload: { cardId: dead.cardId, instanceId: dead.instanceId, owner: dead.owner },
        type: 'minion-banished',
      });
    }
  }
  const defeatedAvatars = new Set(pending.defeatedAvatars);
  const deckLosers = new Set(pending.deckLosers);
  const losers = new Set([...defeatedAvatars, ...deckLosers]);
  let terminal: GameTerminal = { status: 'active' };
  if (losers.size === 2) {
    const reason = defeatedAvatars.size === 2 && deckLosers.size === 0
      ? 'simultaneous_avatar_defeat'
      : 'simultaneous_defeat';
    terminal = { reason, result: 'draw', status: 'finished' };
    completed.push({ payload: { reason, result: 'draw' }, type: 'game-ended' });
  } else if (losers.size === 1) {
    const loser = [...losers][0]!;
    const winner = otherSeat(loser);
    const reason = defeatedAvatars.has(loser) ? 'avatar_defeated' : 'deck_empty';
    terminal = { loser, reason, status: 'finished', winner };
    completed.push({ payload: { loser, reason, winner }, type: 'game-ended' });
  }
  return {
    artifacts,
    outcomes: completed,
    pendingDeathrites: null,
    players: deepFreeze(players),
    terminal,
    units,
  };
}

function driveDeathrites(
  state: GameState,
  startingPlayers: GameState['players'],
  startingUnits: readonly UnitInstance[],
  startingArtifacts: readonly ArtifactInstance[] | undefined,
  startingPending: PendingDeathrites,
  startingOutcomes: readonly GameOutcome[] = [],
): MinionDeathResolution {
  let players = startingPlayers;
  let units = startingUnits;
  let artifacts = startingArtifacts;
  let pending = startingPending;
  const outcomes: GameOutcome[] = [...startingOutcomes];

  while (pending.batches.length > 0) {
    const [batch, ...olderBatches] = pending.batches;
    if (!batch) break;
    if (batch.stage !== 'resolve') {
      return {
        artifacts,
        outcomes,
        pendingDeathrites: pending,
        players,
        terminal: { status: 'active' },
        units,
      };
    }
    const [source, ...remainingSources] = batch.resolving;
    if (!source) {
      pending = deepFreeze({ ...pending, batches: olderBatches });
      continue;
    }
    pending = deepFreeze({
      ...pending,
      batches: [deepFreeze({ ...batch, resolving: remainingSources }), ...olderBatches],
    });
    const definition = cardDefinition(state, source.unit.cardId);
    if (definition.cardType !== 'minion') throw new Error('Deathrite source lacks minion definition');
    const currentState = (): GameState => deepFreeze({
      ...state,
      pendingDeathrites: pending,
      players,
      realm: {
        ...state.realm,
        ...(artifacts ? { artifacts } : {}),
        units,
      },
    });
    const triggeredDeaths: UnitInstance[] = [];
    const deathriteDamage = definition.deathriteDamageEachUnitHere;
    if (deathriteDamage) {
      const projectedState = currentState();
      const targets = (['north', 'south'] as const)
        .flatMap((seat) => unitRefs(projectedState, seat))
        .filter((ref) => {
          const status = unitStatus(projectedState, ref);
          return status.region === source.unit.region
            && status.occupiedCells.includes(source.unit.location);
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      outcomes.push(...targets.map((target) => ({
        payload: {
          amount: deathriteDamage,
          sourceInstanceId: source.instanceId,
          targetInstanceId: target.instanceId,
        },
        type: 'deathrite-damage-allocated',
      })));
      const sourceSnapshot: DamageSourceSnapshot = {
        currentPower: source.currentPower,
        instanceId: source.instanceId,
        kind: 'unit',
      };
      for (const target of targets) {
        const amount = deathriteDamage;
        if (target.kind === 'avatar') {
          const player = players[target.seat];
          const avatar = player.avatar;
          if (avatar.life === 0) {
            if (avatar.deathDoorTurn !== state.turnNumber) {
              pending = deepFreeze({
                ...pending,
                defeatedAvatars: [...new Set([...pending.defeatedAvatars, target.seat])],
              });
              outcomes.push(
                {
                  payload: { amount, direct: true, instanceId: target.instanceId, seat: target.seat },
                  type: 'damage-dealt',
                },
                {
                  payload: { instanceId: target.instanceId, seat: target.seat },
                  type: 'death-blow',
                },
              );
            } else {
              outcomes.push({
                payload: {
                  amount: 0,
                  attemptedAmount: amount,
                  direct: true,
                  instanceId: target.instanceId,
                  prevented: true,
                  seat: target.seat,
                },
                type: 'damage-dealt',
              });
            }
            continue;
          }
          const life = Math.max(0, avatar.life - amount);
          const lost = avatar.life - life;
          players = replacePlayer(currentState(), target.seat, deepFreeze({
            ...player,
            avatar: { ...avatar, ...(life === 0 ? { deathDoorTurn: state.turnNumber } : {}), life },
          }));
          outcomes.push(
            {
              payload: { amount, direct: true, instanceId: target.instanceId, seat: target.seat },
              type: 'damage-dealt',
            },
            { payload: { amount: lost, life, seat: target.seat }, type: 'avatar-life-lost' },
          );
          if (life === 0) {
            outcomes.push({
              payload: { seat: target.seat, turnNumber: state.turnNumber },
              type: 'avatar-reached-deaths-door',
            });
          }
          continue;
        }
        const index = units.findIndex(({ instanceId }) => instanceId === target.instanceId);
        const unit = units[index];
        if (!unit) continue;
        const contributions: readonly DamageContribution[] = [{
          amount,
          lethal: source.lethal,
          source: sourceSnapshot,
        }];
        const unprevented = unpreventedDamageContributions(projectedState, target, contributions);
        const unpreventedAmount = unprevented.reduce((total, contribution) =>
          total + contribution.amount, 0);
        if (unpreventedAmount > 0 && unit.warded) {
          units = units.map((candidate, candidateIndex) => candidateIndex === index
            ? deepFreeze({ ...candidate, warded: false })
            : candidate);
          outcomes.push(
            {
              payload: {
                amount: 0,
                attemptedAmount: amount,
                direct: true,
                instanceId: target.instanceId,
                prevented: true,
                seat: target.seat,
              },
              type: 'damage-dealt',
            },
            {
              payload: { instanceId: target.instanceId, seat: target.seat },
              type: 'ward-broken',
            },
          );
          continue;
        }
        const status = unitStatus(projectedState, target);
        const dealt = unprevented.reduce((total, contribution) =>
          total + Math.max(0, contribution.amount - status.takesLessDamage), 0);
        const lethalDealt = unprevented.some((contribution) =>
          contribution.lethal
            && Math.max(0, contribution.amount - status.takesLessDamage) > 0);
        const accumulated = unit.damage + dealt;
        const awakened = dealt > 0 && unit.disabledUntilDamaged === true;
        const updated = { ...unit, damage: accumulated };
        if (awakened) delete updated.disabledUntilDamaged;
        const updatedUnit = deepFreeze(updated);
        units = units.map((candidate, candidateIndex) =>
          candidateIndex === index ? updatedUnit : candidate);
        outcomes.push({
          payload: {
            accumulated,
            amount: dealt,
            ...(dealt < amount ? { attemptedAmount: amount, prevented: true } : {}),
            direct: true,
            instanceId: target.instanceId,
            seat: target.seat,
          },
          type: 'damage-dealt',
        });
        if (awakened) {
          outcomes.push({
            payload: { instanceId: target.instanceId, seat: target.seat },
            type: 'minion-awakened',
          });
        }
        if (accumulated > 0 && (accumulated >= status.defense || lethalDealt)) {
          triggeredDeaths.push(updatedUnit);
        }
      }
    }
    if (definition.deathriteHeal) {
      const controller = players[source.controller];
      const avatarDefinition = cardDefinition(state, controller.avatar.card.cardId);
      if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
      const [healed, amount] = healAvatar(controller, avatarDefinition.life, definition.deathriteHeal);
      players = replacePlayer(currentState(), source.controller, healed);
      if (amount > 0) {
        outcomes.push({
          payload: {
            amount,
            attemptedAmount: definition.deathriteHeal,
            life: healed.avatar.life,
            seat: source.controller,
            sourceInstanceId: source.instanceId,
          },
          type: 'avatar-healed',
        });
      }
    }
    if (definition.deathriteLoseLifePerNearbySiteControlled === 1) {
      const nearbyCells = new Set(unitOccupiedCells(source.unit).flatMap((cell) => [
        cell,
        ...borderingCells(cell),
        ...diagonalCells(cell),
      ]));
      for (const seat of ['north', 'south'] as const) {
        const amount = Object.entries(state.realm.sites).filter(([cell, site]) =>
          site.controller === seat
            && nearbyCells.has(cell as RealmCell)
            && locationExists(state, { cell: cell as RealmCell, region: source.unit.region })).length;
        const [lifePlayer, lost, reachedDeathsDoor] = loseAvatarLife(
          players[seat],
          amount,
          state.turnNumber,
        );
        players = replacePlayer(currentState(), seat, lifePlayer);
        if (lost > 0) {
          outcomes.push({
            payload: { amount: lost, life: lifePlayer.avatar.life, seat, sourceInstanceId: source.instanceId },
            type: 'avatar-life-lost',
          });
        }
        if (reachedDeathsDoor) {
          outcomes.push({
            payload: { seat, sourceInstanceId: source.instanceId, turnNumber: state.turnNumber },
            type: 'avatar-reached-deaths-door',
          });
        }
      }
    }
    if (definition.deathriteDrawSite) {
      const controller = players[source.controller];
      const [drawn, ...atlas] = controller.atlas;
      if (!drawn) {
        pending = deepFreeze({
          ...pending,
          deckLosers: [...new Set([...pending.deckLosers, source.controller])],
        });
      } else {
        players = replacePlayer(currentState(), source.controller, deepFreeze({
          ...controller,
          atlas,
          hand: { ...controller.hand, atlas: [...controller.hand.atlas, drawn] },
        }));
        outcomes.push({
          payload: { seat: source.controller, sourceInstanceId: source.instanceId },
          type: 'site-drawn',
        });
      }
    }

    if (triggeredDeaths.length > 0) {
      const collected = collectMinionDeaths(
        currentState(),
        players,
        units,
        artifacts,
        triggeredDeaths,
      );
      players = collected.players;
      units = collected.units;
      artifacts = collected.artifacts;
      outcomes.push(...collected.outcomes);
      const nested = makeDeathriteBatch(currentState(), collected.sources);
      pending = deepFreeze({
        ...pending,
        batches: [...(nested ? [nested] : []), ...pending.batches],
        corpses: [...pending.corpses, ...collected.corpses],
      });
    }
  }
  return finishMinionDeaths(state, players, units, artifacts, pending, outcomes);
}

function resolveMinionDeaths(
  state: GameState,
  startingPlayers: GameState['players'],
  units: readonly UnitInstance[],
  deaths: readonly UnitInstance[],
  defeatedAvatars: ReadonlySet<GameSeat>,
): MinionDeathResolution {
  const collected = collectMinionDeaths(
    state,
    startingPlayers,
    units,
    state.realm.artifacts,
    deaths,
  );
  const batch = makeDeathriteBatch(state, collected.sources);
  const existing = state.pendingDeathrites;
  const pending: PendingDeathrites = deepFreeze({
    ...(existing ?? {}),
    batches: [...(batch ? [batch] : []), ...(existing?.batches ?? [])],
    corpses: [...(existing?.corpses ?? []), ...collected.corpses],
    deckLosers: existing?.deckLosers ?? [],
    defeatedAvatars: [
      ...new Set([...(existing?.defeatedAvatars ?? []), ...defeatedAvatars]),
    ],
  });
  return driveDeathrites(
    state,
    collected.players,
    collected.units,
    collected.artifacts,
    pending,
    collected.outcomes,
  );
}

function commitDeathriteOrder(
  pending: PendingDeathrites,
  sourceInstanceId: StateHash,
): PendingDeathrites {
  const [batch, ...olderBatches] = pending.batches;
  if (!batch || batch.stage === 'resolve') throw new Error('unreachable Deathrite order stage');
  const activeStage = batch.stage === 'active-order';
  const remaining = activeStage ? batch.activeRemaining : batch.nonActiveRemaining;
  const selected = remaining.find(({ instanceId }) => instanceId === sourceInstanceId);
  if (!selected || remaining.length < 2) throw new Error('unreachable illegal Deathrite order');
  const rest = remaining.filter(({ instanceId }) => instanceId !== sourceInstanceId);
  const committed = [
    ...(activeStage ? batch.activeOrder : batch.nonActiveOrder),
    selected,
    ...(rest.length === 1 ? rest : []),
  ];
  const stillUnordered = rest.length > 1 ? rest : [];
  let updated: PendingDeathriteBatch;
  if (activeStage && stillUnordered.length > 0) {
    updated = deepFreeze({
      ...batch,
      activeOrder: committed,
      activeRemaining: stillUnordered,
    });
  } else if (activeStage && batch.nonActiveRemaining.length > 1) {
    updated = deepFreeze({
      ...batch,
      activeOrder: committed,
      activeRemaining: [],
      stage: 'non-active-order',
    });
  } else if (activeStage) {
    updated = deepFreeze({
      ...batch,
      activeOrder: committed,
      activeRemaining: [],
      resolving: [...batch.nonActiveOrder, ...committed],
      stage: 'resolve',
    });
  } else if (stillUnordered.length > 0) {
    updated = deepFreeze({
      ...batch,
      nonActiveOrder: committed,
      nonActiveRemaining: stillUnordered,
    });
  } else {
    updated = deepFreeze({
      ...batch,
      nonActiveOrder: committed,
      nonActiveRemaining: [],
      resolving: [...committed, ...batch.activeOrder],
      stage: 'resolve',
    });
  }
  return deepFreeze({ ...pending, batches: [updated, ...olderBatches] });
}

function exposeDeathriteOrder(state: GameState): GameState {
  const pending = state.pendingDeathrites;
  if (!pending) return state;
  const captured = pending.returnPhase
    ? pending
    : deepFreeze({
      ...pending,
      returnDecisionSeat: state.decisionSeat,
      returnPhase: state.phase,
    });
  const orderingState = deepFreeze({ ...state, pendingDeathrites: captured });
  const order = pendingDeathriteOrder(orderingState);
  return deepFreeze({
    ...orderingState,
    decisionSeat: order.seat,
    phase: 'deathrite-order',
  });
}

function applyDeathriteOrder(
  state: GameState,
  sourceInstanceId: StateHash,
  manifest: GameManifest,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const pending = state.pendingDeathrites;
  const order = pendingDeathriteOrder(state);
  if (state.phase !== 'deathrite-order'
    || !pending
    || order.seat !== state.decisionSeat
    || !order.sources.some(({ instanceId }) => instanceId === sourceInstanceId)) {
    throw new Error('unreachable illegal Deathrite order action');
  }
  const committed = commitDeathriteOrder(pending, sourceInstanceId);
  const resolution = driveDeathrites(
    state,
    state.players,
    state.realm.units,
    state.realm.artifacts,
    committed,
  );
  const restoredPhase = resolution.terminal.status === 'finished'
    ? 'terminal'
    : pending.returnPhase ?? 'main';
  const restoredDecisionSeat = pending.returnDecisionSeat ?? state.activeSeat;
  const withoutPending = { ...state };
  delete withoutPending.pendingDeathrites;
  if (resolution.terminal.status === 'finished') {
    delete withoutPending.pendingBasicMovement;
    delete withoutPending.pendingCemeterySummon;
    delete withoutPending.pendingChainMagic;
    delete withoutPending.pendingEndTurnAura;
    delete withoutPending.pendingGenesisSpell;
    delete withoutPending.pendingGenesisSpellOrder;
    delete withoutPending.pendingGenesisToken;
    delete withoutPending.pendingRandomOutcome;
    delete withoutPending.pendingRangedStep;
    delete withoutPending.pendingStartTurn;
  }
  const resolvedState = deepFreeze({
    ...withoutPending,
    decisionSeat: restoredDecisionSeat,
    ...(resolution.pendingDeathrites
      ? { pendingDeathrites: resolution.pendingDeathrites }
      : {}),
    ...(resolution.terminal.status === 'finished' ? { pendingCombat: null } : {}),
    phase: restoredPhase,
    players: resolution.players,
    realm: {
      ...state.realm,
      ...(resolution.artifacts ? { artifacts: resolution.artifacts } : {}),
      units: resolution.units,
    },
    terminal: resolution.terminal,
  });
  const reconciledCombat = resolvedState.pendingCombat === null
    ? null
    : reconcilePendingCombat(resolvedState, resolvedState.pendingCombat);
  const basicMovement = resolvedState.pendingBasicMovement;
  const staleBasicMovement = basicMovement
    && (!resolvedState.realm.units.some(({ instanceId }) =>
      instanceId === basicMovement.sourceInstanceId)
      || basicMovement.purpose === 'defend' && reconciledCombat === null);
  const movementState = staleBasicMovement
    ? deepFreeze({
      ...resolvedState,
      decisionSeat: basicMovement.purpose === 'defend' && reconciledCombat
        ? basicMovement.seat
        : resolvedState.activeSeat,
      pendingBasicMovement: null,
      pendingCombat: reconciledCombat,
      phase: basicMovement.purpose === 'defend' && reconciledCombat
        ? 'defend' as const
        : 'main' as const,
    })
    : resolvedState.pendingCombat === reconciledCombat
      ? resolvedState
      : deepFreeze({ ...resolvedState, pendingCombat: reconciledCombat });
  const staleRangedStep = movementState.pendingRangedStep
    && !movementState.realm.units.some(({ instanceId }) =>
      instanceId === movementState.pendingRangedStep!.sourceInstanceId);
  const resumedState = staleRangedStep
    ? deepFreeze({
      ...movementState,
      pendingRangedStep: null,
      ...(movementState.phase === 'ranged-step'
        ? { decisionSeat: movementState.activeSeat, phase: 'main' as const }
        : {}),
    })
    : movementState;
  let continued: readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]];
  let continuedStateVersioned = false;
  if (!resolution.pendingDeathrites
    && resolution.terminal.status === 'active'
    && pending.endTurnContinuation) {
    const endTurnDeaths = resolveEndOfTurnDeaths(
      resumedState,
      pending.endTurnContinuation.seat,
      pending.endTurnContinuation.remainingInstanceIds,
    );
    if (endTurnDeaths.state.pendingDeathrites) {
      continued = [endTurnDeaths.state, endTurnDeaths.outcomes, []];
    } else {
      const ended = applyDescriptor(
        endTurnDeaths.state,
        { kind: 'end-turn' },
        manifest,
        undefined,
        'after-deaths',
      );
      continued = [ended[0], [...endTurnDeaths.outcomes, ...ended[1]], ended[2]];
      continuedStateVersioned = true;
    }
  } else if (!resolution.pendingDeathrites
    && resolution.terminal.status === 'active'
    && pending.blinkContinuation) {
    const blink = pending.blinkContinuation;
    const drawingPlayer = resumedState.players[blink.seat];
    const [drawn, ...remaining] = drawingPlayer[blink.zone];
    const resolved: GameOutcome = {
      payload: { cardId: blink.cardId, instanceId: blink.instanceId, owner: blink.owner },
      type: 'magic-resolved',
    };
    if (!drawn) {
      const winner = otherSeat(blink.seat);
      continued = [
        deepFreeze({
          ...resumedState,
          pendingCombat: null,
          phase: 'terminal',
          terminal: {
            loser: blink.seat,
            reason: 'deck_empty',
            status: 'finished',
            winner,
          },
        }),
        [resolved, {
          payload: { loser: blink.seat, reason: 'deck_empty', winner },
          type: 'game-ended',
        }],
        [],
      ];
    } else {
      const updatedPlayer = deepFreeze({
        ...drawingPlayer,
        [blink.zone]: remaining,
        hand: {
          ...drawingPlayer.hand,
          [blink.zone]: [...drawingPlayer.hand[blink.zone], drawn],
        },
      });
      continued = [
        deepFreeze({
          ...resumedState,
          players: replacePlayer(resumedState, blink.seat, updatedPlayer),
        }),
        [{
          payload: { seat: blink.seat, sourceInstanceId: blink.instanceId },
          type: blink.zone === 'atlas' ? 'site-drawn' : 'spell-drawn',
        }, resolved],
        [],
      ];
    }
  } else {
    continued = !resolution.pendingDeathrites && pending.firstStrikeContinuation
      ? continueFirstStrikeAfterDeathrites(resumedState, pending.firstStrikeContinuation)
      : [resumedState, [] as readonly GameOutcome[], [] as readonly EngineRandomDraw[]];
  }
  const nextState = continued[0].pendingDeathrites
    ? exposeDeathriteOrder(continued[0])
    : continued[0];
  const deferredOutcomes = resolution.pendingDeathrites
    ? []
    : [
      ...(pending.deferredOutcomes ?? []),
      ...(resolution.terminal.status === 'finished' && pending.blinkContinuation
        ? [{
          payload: {
            cardId: pending.blinkContinuation.cardId,
            instanceId: pending.blinkContinuation.instanceId,
            owner: pending.blinkContinuation.owner,
          },
          type: 'magic-resolved',
        }]
        : []),
    ];
  const terminalIndex = resolution.outcomes.findIndex(({ type }) => type === 'game-ended');
  const completedResolutionOutcomes = terminalIndex < 0
    ? [...resolution.outcomes, ...deferredOutcomes]
    : [
      ...resolution.outcomes.slice(0, terminalIndex),
      ...deferredOutcomes,
      ...resolution.outcomes.slice(terminalIndex),
    ];
  return [
    continuedStateVersioned ? nextState : withStateVersion(nextState, {}),
    [
      {
        payload: { seat: order.seat, sourceInstanceId },
        type: 'deathrite-order-committed',
      },
      ...completedResolutionOutcomes,
      ...continued[1],
    ],
    continued[2],
  ];
}

function settleStaticPowerDeaths(
  state: GameState,
): Readonly<{ outcomes: readonly GameOutcome[]; state: GameState }> {
  const deaths = state.realm.units.filter((unit) => unit.damage > 0
    && unit.damage >= unitStatus(state, {
      instanceId: unit.instanceId,
      kind: 'minion',
      seat: unit.controller,
    }).defense);
  if (deaths.length === 0) return { outcomes: [], state };
  const resolution = resolveMinionDeaths(
    state,
    state.players,
    state.realm.units,
    deaths,
    new Set<GameSeat>(),
  );
  const pendingCombat = state.pendingCombat === null
    ? null
    : reconcilePendingCombat(
      deepFreeze({ ...state, realm: { ...state.realm, units: resolution.units } }),
      state.pendingCombat,
    );
  const pendingInvalid = state.pendingCombat !== null && pendingCombat === null;
  const movement = state.pendingBasicMovement;
  const movementInvalid = movement !== null && movement !== undefined
    && (!resolution.units.some(({ instanceId }) => instanceId === movement.sourceInstanceId)
      || (movement.purpose === 'defend' && pendingCombat === null));
  const terminal = state.terminal.status === 'finished'
    ? state.terminal
    : resolution.terminal;
  return {
    outcomes: resolution.outcomes,
    state: deepFreeze({
      ...state,
      ...(terminal.status === 'finished'
        ? {
          ...(movement === undefined ? {} : { pendingBasicMovement: null }),
          pendingCombat: null,
          phase: 'terminal' as const,
        }
        : movementInvalid
          ? {
            decisionSeat: movement.purpose === 'defend' && pendingCombat !== null
              ? movement.seat
              : state.activeSeat,
            pendingBasicMovement: null,
            pendingCombat,
            phase: movement.purpose === 'defend' && pendingCombat !== null
              ? 'defend' as const
              : 'main' as const,
          }
          : pendingInvalid
            ? { decisionSeat: state.activeSeat, pendingCombat: null, phase: 'main' as const }
            : { pendingCombat }),
      players: resolution.players,
      ...(resolution.pendingDeathrites ? { pendingDeathrites: resolution.pendingDeathrites } : {}),
      realm: {
        ...state.realm,
        ...(resolution.artifacts ? { artifacts: resolution.artifacts } : {}),
        units: resolution.units,
      },
      terminal,
    }),
  };
}

function reconcilePendingCombat(state: GameState, pending: PendingCombat): PendingCombat | null {
  const survivingIds = new Set(state.realm.units.map(({ instanceId }) => instanceId));
  const unitSurvives = (ref: GameUnitRef): boolean =>
    ref.kind === 'avatar' || survivingIds.has(ref.instanceId);
  if (!unitSurvives(pending.attacker)
    || (pending.originalTarget?.kind === 'minion' && !unitSurvives(pending.originalTarget))) return null;
  return deepFreeze({
    ...pending,
    combatants: pending.combatants.filter(unitSurvives),
    defenders: pending.defenders.filter(unitSurvives),
  });
}

function resolveEndOfTurnDeaths(
  state: GameState,
  seat: GameSeat,
  remainingInstanceIds?: readonly StateHash[],
): Readonly<{ outcomes: readonly GameOutcome[]; state: GameState }> {
  const triggeredIds = remainingInstanceIds ?? state.realm.units.flatMap((unit) => {
    if (unit.controller !== seat || minionDisabled(state, unit)) return [];
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion'
      && definition.diesAtEndOfControllerTurn === true
      ? [unit.instanceId]
      : [];
  });
  let current = state;
  const outcomes: GameOutcome[] = [];
  for (const [index, instanceId] of triggeredIds.entries()) {
    const dead = current.realm.units.find((unit) => unit.instanceId === instanceId);
    if (!dead) continue;
    const resolution = resolveMinionDeaths(
      current,
      current.players,
      current.realm.units,
      [dead],
      new Set<GameSeat>(),
    );
    const pendingDeathrites = resolution.pendingDeathrites
      ? deepFreeze({
        ...resolution.pendingDeathrites,
        endTurnContinuation: {
          remainingInstanceIds: triggeredIds.slice(index + 1),
          seat,
        },
      })
      : null;
    current = deepFreeze({
      ...current,
      ...(resolution.terminal.status === 'finished'
        ? { pendingCombat: null, phase: 'terminal' as const }
        : {}),
      players: resolution.players,
      ...(pendingDeathrites ? { pendingDeathrites } : {}),
      realm: {
        ...current.realm,
        ...(resolution.artifacts ? { artifacts: resolution.artifacts } : {}),
        units: resolution.units,
      },
      terminal: resolution.terminal,
    });
    outcomes.push(...resolution.outcomes);
    if (pendingDeathrites) break;
    if (resolution.terminal.status === 'finished') break;
  }
  return { outcomes, state: current };
}

function resolveEndOfEachTurnSiteControllerLifeLoss(
  state: GameState,
  activeSeat: GameSeat,
): Readonly<{ outcomes: readonly GameOutcome[]; state: GameState }> {
  const nonActiveSeat = otherSeat(activeSeat);
  const triggeredIds = (state.realm.artifacts ?? []).flatMap((artifact) => {
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition.atEndOfEachTurnSiteControllerLosesLife === undefined) return [];
    const orderingSeat = 'bearer' in artifact ? artifact.bearer.seat : artifact.owner;
    return [{
      instanceId: artifact.instanceId,
      orderingGroup: orderingSeat === nonActiveSeat ? 0 : 1,
    }];
  }).sort((left, right) => left.orderingGroup - right.orderingGroup
    || left.instanceId.localeCompare(right.instanceId));
  let current = state;
  const outcomes: GameOutcome[] = [];
  for (const { instanceId } of triggeredIds) {
    const artifact = current.realm.artifacts?.find((candidate) =>
      candidate.instanceId === instanceId);
    if (!artifact) continue;
    const definition = cardDefinition(current, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition.atEndOfEachTurnSiteControllerLosesLife === undefined) continue;
    const location = artifactLocation(current, artifact);
    const site = location.region === 'void' ? undefined : current.realm.sites[location.cell];
    if (!site || isRubble(site)) continue;
    const seat = site.controller;
    const [player, lost, reachedDeathsDoor] = loseAvatarLife(
      current.players[seat],
      definition.atEndOfEachTurnSiteControllerLosesLife,
      current.turnNumber,
    );
    current = deepFreeze({ ...current, players: replacePlayer(current, seat, player) });
    outcomes.push({
      payload: {
        amount: definition.atEndOfEachTurnSiteControllerLosesLife,
        seat,
        siteInstanceId: site.instanceId,
        sourceInstanceId: artifact.instanceId,
      },
      type: 'end-turn-site-life-loss-triggered',
    });
    if (lost > 0) {
      outcomes.push({
        payload: { amount: lost, life: player.avatar.life, seat, sourceInstanceId: artifact.instanceId },
        type: 'avatar-life-lost',
      });
    }
    if (reachedDeathsDoor) {
      outcomes.push({
        payload: { seat, sourceInstanceId: artifact.instanceId, turnNumber: current.turnNumber },
        type: 'avatar-reached-deaths-door',
      });
    }
  }
  return { outcomes, state: current };
}

type MinionRegionDisposition = 'banished' | 'dies' | 'survives';

function minionRegionDisposition(state: GameState, unit: UnitInstance): MinionRegionDisposition {
  if (!footprintLocationExists(state, unitOccupiedCells(unit), unit.region)) {
    return unit.region === 'void' ? 'banished' : 'dies';
  }
  if (unit.region === 'surface') return 'survives';
  if (unit.region === 'void') {
    return unitStatus(state, {
      instanceId: unit.instanceId,
      kind: 'minion',
      seat: unit.controller,
    }).voidwalk ? 'survives' : 'banished';
  }
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion') throw new Error('realm minion lacks minion definition');
  const disabled = minionDisabled(state, unit);
  if (unit.region === 'underground') return !disabled && definition.burrowing === true ? 'survives' : 'dies';
  return !disabled && definition.submerge === true ? 'survives' : 'dies';
}

function settleRegionOccupancy(state: GameState): Readonly<{
  outcomes: readonly GameOutcome[];
  removals: readonly Readonly<{
    disposition: Exclude<MinionRegionDisposition, 'survives'>;
    instanceId: StateHash;
  }>[];
  state: GameState;
}> {
  const removals = state.realm.units.flatMap((unit) => {
    const disposition = minionRegionDisposition(state, unit);
    return disposition === 'survives' ? [] : [{ disposition, instanceId: unit.instanceId }];
  });
  if (removals.length === 0) return { outcomes: [], removals, state };
  const banishedIds = new Set(removals
    .filter(({ disposition }) => disposition === 'banished')
    .map(({ instanceId }) => instanceId));
  const deathIds = new Set(removals
    .filter(({ disposition }) => disposition === 'dies')
    .map(({ instanceId }) => instanceId));
  const banished = state.realm.units.filter(({ instanceId }) => banishedIds.has(instanceId));
  const units = state.realm.units.filter(({ instanceId }) => !banishedIds.has(instanceId));
  const deaths = units.filter(({ instanceId }) => deathIds.has(instanceId));
  let artifacts = state.realm.artifacts;
  const banishedOutcomes: GameOutcome[] = [];
  for (const unit of banished) {
    const drop = dropArtifactsCarriedBy(artifacts, unit);
    artifacts = drop.artifacts;
    banishedOutcomes.push(...drop.outcomes, {
      payload: { cardId: unit.cardId, instanceId: unit.instanceId, owner: unit.owner },
      type: 'minion-banished',
    });
  }
  const banishedState = deepFreeze({
    ...state,
    realm: { ...state.realm, ...(artifacts ? { artifacts } : {}), units },
  });
  const deathResolution = resolveMinionDeaths(
    banishedState,
    state.players,
    units,
    deaths,
    new Set<GameSeat>(),
  );
  const terminalIndex = deathResolution.outcomes.findIndex(({ type }) => type === 'game-ended');
  const outcomes = terminalIndex < 0
    ? [...deathResolution.outcomes, ...banishedOutcomes]
    : [
      ...deathResolution.outcomes.slice(0, terminalIndex),
      ...banishedOutcomes,
      ...deathResolution.outcomes.slice(terminalIndex),
    ];
  const settledState = deepFreeze({
    ...state,
    ...(deathResolution.terminal.status === 'finished'
      ? { pendingCombat: null, phase: 'terminal' as const }
      : {}),
    players: deathResolution.players,
    ...(deathResolution.pendingDeathrites
      ? { pendingDeathrites: deathResolution.pendingDeathrites }
      : {}),
    realm: {
      ...state.realm,
      ...(deathResolution.artifacts ? { artifacts: deathResolution.artifacts } : {}),
      units: deathResolution.units,
    },
    terminal: deathResolution.terminal,
  });
  return { outcomes, removals, state: settledState };
}

function resolveDeclaredPath(
  state: GameState,
  ref: GameUnitRef,
  path: readonly GameLocation[],
  tap: boolean,
): Readonly<{
  outcomes: readonly GameOutcome[];
  path: readonly GameLocation[];
  removals: readonly Readonly<{
    disposition: Exclude<MinionRegionDisposition, 'survives'>;
    instanceId: StateHash;
  }>[];
  state: GameState;
}> {
  const start = path[0];
  if (!start) return { outcomes: [], path: [], removals: [], state };
  const startingStatus = unitStatus(state, ref);
  if (!sameLocation({ cell: startingStatus.location, region: startingStatus.region }, start)) {
    return { outcomes: [], path: [], removals: [], state };
  }
  const tapped = moveUnit(state, ref, start, tap);
  let current = deepFreeze({ ...state, players: tapped.players, realm: tapped.realm });
  const actualPath: GameLocation[] = [start];
  const outcomes: GameOutcome[] = [];
  const removals: Array<Readonly<{
    disposition: Exclude<MinionRegionDisposition, 'survives'>;
    instanceId: StateHash;
  }>> = [];
  for (let index = 1; index < path.length; index += 1) {
    const expectedFrom = path[index - 1]!;
    const next = path[index]!;
    const currentUnit = ref.kind === 'avatar'
      ? current.players[ref.seat].avatar.card.instanceId === ref.instanceId
        ? current.players[ref.seat].avatar
        : undefined
      : current.realm.units.find(({ instanceId }) => instanceId === ref.instanceId);
    if (!currentUnit
      || currentUnit.location !== expectedFrom.cell
      || currentUnit.region !== expectedFrom.region) break;
    const currentStatus = unitStatus(current, ref);
    const nextCells = translatedFootprint(
      currentStatus.occupiedCells,
      currentStatus.location,
      next.cell,
    );
    const enteredCells = nextCells?.filter((cell) => !currentStatus.occupiedCells.includes(cell));
    if (!nextCells
      || !footprintLocationExists(current, nextCells, next.region)
      || !enteredCells?.every((cell) => unitEntryAllowed(
        current,
        expectedFrom,
        { cell, region: next.region },
        currentStatus.airborne,
        ref.kind === 'minion',
        currentStatus.attack,
        'movement',
      ))) break;
    const moved = moveUnit(current, ref, next, false);
    current = deepFreeze({ ...current, players: moved.players, realm: moved.realm });
    actualPath.push(next);
    const settlement = settleRegionOccupancy(current);
    current = settlement.state;
    outcomes.push(...settlement.outcomes);
    removals.push(...settlement.removals);
    if (current.pendingDeathrites) {
      // ponytail: resume the remaining declared path when an actual-card scenario
      // first combines multi-edge movement with ordered Deathrites.
      if (index < path.length - 1) {
        throw new Error('unsupported Deathrite ordering during multi-step movement');
      }
      break;
    }
    const stealthSettlement = settleNearbyEnemyStealth(current);
    current = stealthSettlement.state;
    outcomes.push(...stealthSettlement.outcomes);
    if (current.pendingDeathrites) {
      if (index < path.length - 1) {
        throw new Error('unsupported Deathrite ordering during multi-step movement');
      }
      break;
    }
    const powerSettlement = settleStaticPowerDeaths(current);
    current = powerSettlement.state;
    outcomes.push(...powerSettlement.outcomes);
    if (current.pendingDeathrites) {
      if (index < path.length - 1) {
        throw new Error('unsupported Deathrite ordering during multi-step movement');
      }
      break;
    }
    if (settlement.removals.some(({ instanceId }) => instanceId === ref.instanceId)
      || current.terminal.status === 'finished') break;
  }
  return { outcomes, path: actualPath, removals, state: current };
}

function beginBasicMovement(
  state: GameState,
  ref: GameUnitRef,
  path: readonly GameLocation[],
  purpose: PendingBasicMovement['purpose'],
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const started = resolveDeclaredPath(state, ref, path.slice(0, 1), true);
  return [
    withStateVersion(started.state, {
      decisionSeat: ref.seat,
      pendingBasicMovement: {
        path: [...path],
        pathIndex: 0,
        purpose,
        rangedStrikeUsed: false,
        seat: ref.seat,
        sourceInstanceId: ref.instanceId,
      },
      phase: 'movement',
    }),
    [{
      payload: {
        from: path[0]!,
        path,
        purpose,
        seat: ref.seat,
        sourceInstanceId: ref.instanceId,
        to: path.at(-1)!,
      },
      type: 'basic-movement-started',
    }],
    [],
  ];
}

function finishMoveAndAttackMovement(
  state: GameState,
  ref: GameUnitRef,
  declaredTo: GameLocation,
  path: readonly GameLocation[],
  pathOutcomes: readonly GameOutcome[],
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const from = path[0]!;
  const actualTo = path.at(-1) ?? from;
  const activated: GameOutcome = {
    payload: {
      from,
      path,
      seat: ref.seat,
      steps: path.length - 1,
      to: actualTo,
      unitInstanceId: ref.instanceId,
    },
    type: 'move-and-attack-activated',
  };
  const moverArrived = ref.kind === 'avatar'
    ? state.players[ref.seat].avatar.card.instanceId === ref.instanceId
      && state.players[ref.seat].avatar.location === declaredTo.cell
      && state.players[ref.seat].avatar.region === declaredTo.region
    : state.realm.units.some(({ instanceId, location, region }) =>
      instanceId === ref.instanceId && location === declaredTo.cell && region === declaredTo.region);
  if (!moverArrived || state.terminal.status === 'finished') {
    return [
      withStateVersion(state, {
        decisionSeat: state.activeSeat,
        ...(state.pendingBasicMovement === undefined ? {} : { pendingBasicMovement: null }),
        phase: state.terminal.status === 'finished' ? 'terminal' : 'main',
      }),
      [...(state.pendingBasicMovement?.activationEmitted ? [] : [activated]), ...pathOutcomes],
      [],
    ];
  }
  const pending: PendingCombat = deepFreeze({
    allocations: [],
    attacker: ref,
    attackingSeat: ref.seat,
    cell: declaredTo.cell,
    combatants: [],
    defenders: [],
    originalTarget: null,
    ...(declaredTo.region === 'surface' ? {} : { region: declaredTo.region }),
    targetRemoved: false,
  });
  return [
    withStateVersion(state, {
      ...(state.pendingBasicMovement === undefined ? {} : { pendingBasicMovement: null }),
      pendingCombat: pending,
      phase: 'attack',
    }),
    [...(state.pendingBasicMovement?.activationEmitted ? [] : [activated]), ...pathOutcomes],
    [],
  ];
}

function finishDefendMovement(
  state: GameState,
  ref: GameUnitRef,
  path: readonly GameLocation[],
  pathOutcomes: readonly GameOutcome[],
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const from = path[0]!;
  const pending = state.pendingCombat;
  const destination = pending
    ? { cell: pending.cell, region: pending.region ?? 'surface' as const }
    : state.pendingBasicMovement?.path.at(-1) ?? path.at(-1)!;
  const defenderArrived = pending !== null && unitRefs(state, ref.seat).some((candidate) =>
    candidate.instanceId === ref.instanceId
    && candidate.kind === ref.kind
    && unitOccupiesLocation(state, candidate, destination));
  const movement: GameOutcome = {
    payload: {
      from,
      instanceId: ref.instanceId,
      path,
      seat: ref.seat,
      steps: path.length - 1,
      to: path.at(-1) ?? from,
    },
    type: defenderArrived ? 'defender-joined' : 'defender-moved',
  };
  if (!defenderArrived || state.terminal.status === 'finished' || pending === null) {
    return [
      withStateVersion(state, {
        decisionSeat: pending === null ? state.activeSeat : ref.seat,
        ...(state.pendingBasicMovement === undefined ? {} : { pendingBasicMovement: null }),
        phase: state.terminal.status === 'finished'
          ? 'terminal'
          : pending === null ? 'main' : 'defend',
      }),
      [...(state.pendingBasicMovement?.activationEmitted ? [] : [movement]), ...pathOutcomes],
      [],
    ];
  }
  const removesSite = pending.originalTarget?.kind === 'site' && !pending.targetRemoved;
  return [
    withStateVersion(state, {
      decisionSeat: ref.seat,
      ...(state.pendingBasicMovement === undefined ? {} : { pendingBasicMovement: null }),
      pendingCombat: deepFreeze({
        ...pending,
        defenders: [...pending.defenders, ref],
        targetRemoved: pending.targetRemoved || removesSite,
      }),
      phase: 'defend',
    }),
    [
      ...(state.pendingBasicMovement?.activationEmitted ? [] : [movement]),
      ...pathOutcomes,
      ...(removesSite
        ? [{
          payload: { instanceId: pending.originalTarget!.instanceId, kind: 'site' },
          type: 'original-target-removed',
        }]
        : []),
    ],
    [],
  ];
}

function applySiteDestructionTerrain(
  state: GameState,
  destroyed: readonly Readonly<{ cell: RealmCell; site: SiteInstance }>[],
  sourceInstanceId: StateHash,
): Readonly<{
  outcomes: readonly GameOutcome[];
  state: GameState;
  unique: readonly Readonly<{ cell: RealmCell; site: SiteInstance }>[];
}> {
  const unique = [...new Map(destroyed.map((entry) => [entry.site.instanceId, entry])).values()]
    .sort((left, right) => left.cell.localeCompare(right.cell));
  const floodedCells = new Set(unique
    .filter(({ site }) => {
      const definition = cardDefinition(state, site.cardId);
      return definition.cardType === 'site' && definition.elements.includes('water');
    })
    .map(({ cell }) => cell));
  const sites = { ...state.realm.sites };
  const rubbleOutcomes: GameOutcome[] = [];
  for (const { cell, site } of unique) {
    const rubble = deepFreeze({
      controller: null,
      instanceId: identityHash(asJson({
        cell,
        destroyedSiteInstanceId: site.instanceId,
        kind: 'rubble',
        sourceInstanceId,
      })),
      rubble: true as const,
    });
    sites[cell] = rubble;
    rubbleOutcomes.push({
      payload: { cell, instanceId: rubble.instanceId, sourceInstanceId },
      type: 'rubble-created',
    });
  }
  const units = state.realm.units.map((unit) =>
    unitOccupiedCells(unit).some((cell) => floodedCells.has(cell)) && unit.region === 'underwater'
      ? deepFreeze({ ...unit, region: 'underground' as const })
      : unit);
  const artifacts = state.realm.artifacts?.map((artifact) =>
    !('bearer' in artifact)
      && floodedCells.has(artifact.location)
      && artifact.region === 'underwater'
      ? deepFreeze({ ...artifact, region: 'underground' as const })
      : artifact);
  const terrainState = deepFreeze({
    ...state,
    realm: {
      ...state.realm,
      ...(artifacts ? { artifacts } : {}),
      sites,
      units,
    },
  });
  return { outcomes: rubbleOutcomes, state: terrainState, unique };
}

function addDestroyedSitesToCemeteries(
  players: GameState['players'],
  destroyed: readonly Readonly<{ site: SiteInstance }>[],
): GameState['players'] {
  const updated: Record<GameSeat, PlayerState> = {
    north: players.north,
    south: players.south,
  };
  for (const { site } of destroyed) {
    const owner = updated[site.owner];
    updated[site.owner] = deepFreeze({
      ...owner,
      cemetery: [...owner.cemetery, {
        cardId: site.cardId,
        instanceId: site.instanceId,
        owner: site.owner,
        source: site.source,
      }],
    });
  }
  return deepFreeze(updated);
}

function resolveSiteDeaths(
  state: GameState,
  destroyed: readonly Readonly<{ cell: RealmCell; site: SiteInstance }>[],
  sourceInstanceId: StateHash,
): Readonly<{
  outcomes: readonly GameOutcome[];
  pendingDeathrites: PendingDeathrites | null;
  players: GameState['players'];
  realm: GameState['realm'];
  terminal: GameTerminal;
}> {
  const terrain = applySiteDestructionTerrain(state, destroyed, sourceInstanceId);
  const settlement = settleRegionOccupancy(terrain.state);
  const players = addDestroyedSitesToCemeteries(settlement.state.players, terrain.unique);
  const terminalIndex = settlement.outcomes.findIndex(({ type }) => type === 'game-ended');
  return {
    outcomes: terminalIndex < 0
      ? [...settlement.outcomes, ...terrain.outcomes]
      : [
        ...settlement.outcomes.slice(0, terminalIndex),
        ...terrain.outcomes,
        ...settlement.outcomes.slice(terminalIndex),
      ],
    pendingDeathrites: settlement.state.pendingDeathrites ?? null,
    players,
    realm: settlement.state.realm,
    terminal: settlement.state.terminal,
  };
}

function unpreventedDamageContributions(
  state: GameState,
  target: GameUnitRef,
  sources: readonly DamageContribution[],
): readonly DamageContribution[] {
  if (target.kind === 'avatar') return sources;
  const unit = state.realm.units.find(({ instanceId }) => instanceId === target.instanceId);
  if (!unit) throw new Error('unreachable damage-prevention minion');
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion') {
    throw new Error('damage-prevention minion lacks minion definition');
  }
  const status = unitStatus(state, target);
  const threshold = status.disabled
    ? undefined
    : definition.preventsDamageFromUnitsWithPowerAtLeast;
  if (threshold === undefined) return sources;
  if (unit.warded || status.takesLessDamage > 0) {
    throw new Error('unsupported competing damage prevention effects');
  }
  return sources.filter(({ source }) =>
    source.kind !== 'unit' || source.currentPower < threshold);
}

function resolveFightWindow(
  state: GameState,
  pending: PendingCombat,
  outcomes: readonly GameOutcome[],
  allocationSource: DamageAllocationSource,
  attackerStrikes: boolean,
  combatantsStrike: boolean | readonly GameUnitRef[],
  interactingRefs?: readonly GameUnitRef[],
  allocationsAreStrikes = true,
  allocationsUseAttackerLethal = allocationsAreStrikes,
  simultaneousSiteDestruction?: Readonly<{
    destroyed: readonly Readonly<{ cell: RealmCell; site: SiteInstance }>[];
    sourceInstanceId: StateHash;
  }>,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const allocations = new Map(pending.allocations.map(({ amount, targetInstanceId }) =>
    [targetInstanceId, amount]));
  const unitSource = (ref: GameUnitRef): DamageSourceSnapshot => ({
    currentPower: unitStatus(state, ref).attack,
    instanceId: ref.instanceId,
    kind: 'unit',
  });
  const damageSources = new Map<StateHash, readonly DamageContribution[]>();
  const attackerStatus = unitStatus(state, pending.attacker);
  const attackerCanStrike = attackerStrikes && !attackerStatus.disabled;
  const requestedCombatants = typeof combatantsStrike === 'boolean'
    ? combatantsStrike ? pending.combatants : []
    : combatantsStrike;
  const strikingCombatants = requestedCombatants
    .filter((ref) => !unitStatus(state, ref).disabled);
  if (strikingCombatants.length > 0) {
    damageSources.set(
      pending.attacker.instanceId,
      strikingCombatants.map((ref) => ({
        amount: strikeDamage(state, ref),
        lethal: unitStatus(state, ref).lethal,
        source: unitSource(ref),
      })),
    );
  }
  if (attackerCanStrike) {
    const source = allocationSource === 'attacker-unit'
      ? unitSource(pending.attacker)
      : { kind: 'non-unit' as const };
    pending.combatants.forEach((ref) =>
      damageSources.set(ref.instanceId, [{
        amount: allocations.get(ref.instanceId) ?? 0,
        lethal: allocationSource === 'attacker-unit'
          && allocationsUseAttackerLethal
          && attackerStatus.lethal,
        source,
      }]));
  }

  const interaction = recordInteraction(state, interactingRefs ?? [
    ...(attackerCanStrike ? [pending.attacker] : []),
    ...strikingCombatants,
  ]);
  const players: Record<GameSeat, PlayerState> = {
    north: interaction.players.north,
    south: interaction.players.south,
  };
  let units = [...interaction.units];
  const stealthOutcomes = interaction.outcomes;
  const defeatedAvatars = new Set<GameSeat>();
  const deaths: UnitInstance[] = [];
  const damageOutcomes: GameOutcome[] = [];

  const damagedRefs = [
    ...(strikingCombatants.length > 0 ? [pending.attacker] : []),
    ...(attackerCanStrike ? pending.combatants : []),
  ];
  for (const ref of damagedRefs) {
    const sources = damageSources.get(ref.instanceId) ?? [];
    const amount = sources.reduce((total, source) => total + source.amount, 0);
    if (ref.kind === 'avatar') {
      const player = players[ref.seat];
      const avatar = player.avatar;
      if (avatar.life === 0) {
        if (amount > 0 && avatar.deathDoorTurn !== state.turnNumber) {
          defeatedAvatars.add(ref.seat);
          damageOutcomes.push(
            { payload: { amount, direct: true, instanceId: ref.instanceId, seat: ref.seat }, type: 'damage-dealt' },
            { payload: { instanceId: ref.instanceId, seat: ref.seat }, type: 'death-blow' },
          );
        } else if (amount > 0) {
          damageOutcomes.push({
            payload: {
              amount: 0,
              attemptedAmount: amount,
              direct: true,
              instanceId: ref.instanceId,
              prevented: amount > 0,
              seat: ref.seat,
            },
            type: 'damage-dealt',
          });
        }
        continue;
      }
      const life = Math.max(0, avatar.life - amount);
      const lost = avatar.life - life;
      players[ref.seat] = deepFreeze({
        ...player,
        avatar: {
          ...avatar,
          ...(life === 0 ? { deathDoorTurn: state.turnNumber } : {}),
          life,
        },
      });
      if (amount > 0) {
        damageOutcomes.push(
          { payload: { amount, direct: true, instanceId: ref.instanceId, seat: ref.seat }, type: 'damage-dealt' },
          { payload: { amount: lost, life, seat: ref.seat }, type: 'avatar-life-lost' },
        );
      }
      if (avatar.life > 0 && life === 0) {
        damageOutcomes.push({
          payload: { seat: ref.seat, turnNumber: state.turnNumber },
          type: 'avatar-reached-deaths-door',
        });
      }
      continue;
    }

    const index = units.findIndex(({ instanceId }) => instanceId === ref.instanceId);
    const unit = units[index];
    if (!unit) throw new Error('unreachable fight minion');
    const status = unitStatus(state, ref);
    const unpreventedSources = unpreventedDamageContributions(state, ref, sources);
    const unpreventedAmount = unpreventedSources.reduce((total, source) =>
      total + source.amount, 0);
    if (unpreventedAmount > 0 && unit.warded) {
      units[index] = deepFreeze({ ...unit, warded: false });
      damageOutcomes.push(
        {
          payload: {
            amount: 0,
            attemptedAmount: amount,
            direct: true,
            instanceId: ref.instanceId,
            prevented: true,
            seat: ref.seat,
          },
          type: 'damage-dealt',
        },
        { payload: { instanceId: ref.instanceId, seat: ref.seat }, type: 'ward-broken' },
      );
      continue;
    }
    const reduction = status.takesLessDamage;
    const dealt = unpreventedSources.reduce((total, source) =>
      total + Math.max(0, source.amount - reduction), 0);
    const lethalDealt = unpreventedSources.some((source) =>
      source.lethal && Math.max(0, source.amount - reduction) > 0);
    const accumulated = unit.damage + dealt;
    const awakened = dealt > 0 && unit.disabledUntilDamaged === true;
    const updated = { ...unit, damage: accumulated };
    if (awakened) delete updated.disabledUntilDamaged;
    units[index] = deepFreeze(updated);
    damageOutcomes.push({
      payload: {
        accumulated,
        amount: dealt,
        ...(dealt < amount ? { attemptedAmount: amount, prevented: true } : {}),
        direct: true,
        instanceId: ref.instanceId,
        seat: ref.seat,
      },
      type: 'damage-dealt',
    });
    if (awakened) {
      damageOutcomes.push({
        payload: { instanceId: ref.instanceId, seat: ref.seat },
        type: 'minion-awakened',
      });
    }
    if (accumulated > 0
      && (accumulated >= status.defense || lethalDealt)) {
      deaths.push(units[index]!);
    }
  }
  damageOutcomes.push(...stealthOutcomes);

  const lanceOutcomes: GameOutcome[] = [];
  if (allocationsAreStrikes) {
    const strikers = [
      ...(attackerCanStrike ? [pending.attacker] : []),
      ...strikingCombatants,
    ];
    for (const ref of strikers) {
      const count = carriedLanceCount(state, ref);
      if (ref.kind !== 'minion' || count === 0) continue;
      units = units.map((unit) => {
        if (unit.instanceId !== ref.instanceId) return unit;
        const cleared = { ...unit };
        delete cleared.carriedLanceCount;
        return deepFreeze(cleared);
      });
      lanceOutcomes.push({
        payload: {
          bearerInstanceId: ref.instanceId,
          count,
          sourceInstanceId: ref.instanceId,
        },
        type: 'lance-broken',
      });
    }
  }

  let deathState = state;
  let deathUnits: readonly UnitInstance[] = units;
  let simultaneousOutcomes: readonly GameOutcome[] = [];
  let destroyedSites: readonly Readonly<{ cell: RealmCell; site: SiteInstance }>[] = [];
  let simultaneousDeaths = deaths;
  if (simultaneousSiteDestruction) {
    const damagedState = deepFreeze({
      ...state,
      players,
      realm: { ...state.realm, units },
    });
    const terrain = applySiteDestructionTerrain(
      damagedState,
      simultaneousSiteDestruction.destroyed,
      simultaneousSiteDestruction.sourceInstanceId,
    );
    const damagedIds = new Set(deaths.map(({ instanceId }) => instanceId));
    deathState = terrain.state;
    deathUnits = terrain.state.realm.units;
    simultaneousOutcomes = terrain.outcomes;
    destroyedSites = terrain.unique;
    simultaneousDeaths = deathUnits.filter((unit) =>
      damagedIds.has(unit.instanceId) || minionRegionDisposition(deathState, unit) === 'dies');
  }

  const deathResolution = resolveMinionDeaths(
    deathState,
    players,
    deathUnits,
    simultaneousDeaths,
    defeatedAvatars,
  );
  const resolvedPlayers = addDestroyedSitesToCemeteries(
    deathResolution.players,
    destroyedSites,
  );

  return [
    deepFreeze({
      ...state,
      decisionSeat: state.activeSeat,
      pendingCombat: null,
      ...(deathResolution.pendingDeathrites
        ? { pendingDeathrites: deathResolution.pendingDeathrites }
        : {}),
      phase: deathResolution.terminal.status === 'finished' ? 'terminal' : 'main',
      players: resolvedPlayers,
      realm: {
        ...deathState.realm,
        ...(deathResolution.artifacts ? { artifacts: deathResolution.artifacts } : {}),
        units: deathResolution.units,
      },
      terminal: deathResolution.terminal,
    }),
    [
      ...outcomes,
      ...damageOutcomes,
      ...lanceOutcomes,
      ...simultaneousOutcomes,
      ...deathResolution.outcomes,
    ],
    [],
  ];
}

function resolveChainMagicDamage(
  state: GameState,
  seat: GameSeat,
  caster: GameUnitRef,
  targets: readonly GameUnitRef[],
  sourceInstanceId: StateHash,
  outcomes: readonly GameOutcome[],
  resolved: GameOutcome,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const pending: PendingCombat = deepFreeze({
    allocations: targets.map(({ instanceId }) => ({
      amount: CHAIN_MAGIC_DAMAGE,
      targetInstanceId: instanceId,
    })),
    attacker: caster,
    attackingSeat: seat,
    cell: unitStatus(state, caster).location,
    combatants: targets,
    defenders: [],
    originalTarget: targets[0]!,
    targetRemoved: false,
  });
  const allocationOutcomes: readonly GameOutcome[] = targets.map(({ instanceId }) => ({
    payload: {
      amount: CHAIN_MAGIC_DAMAGE,
      sourceInstanceId,
      targetInstanceId: instanceId,
    },
    type: 'magic-damage-allocated',
  }));
  const [damaged, damageOutcomes, randomDraws] = resolveFightWindow(
    state,
    pending,
    [...outcomes, ...allocationOutcomes],
    'non-unit',
    true,
    false,
    [caster],
    false,
  );
  const terminalIndex = damageOutcomes.findIndex(({ type }) => type === 'game-ended');
  return [
    withStateVersion(damaged, {}),
    terminalIndex < 0
      ? [...damageOutcomes, resolved]
      : [...damageOutcomes.slice(0, terminalIndex), resolved, ...damageOutcomes.slice(terminalIndex)],
    randomDraws,
  ];
}

function continueFirstStrikeAfterDeathrites(
  state: GameState,
  continuation: NonNullable<PendingDeathrites['firstStrikeContinuation']>,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const survivors = continuation.pending.combatants.filter((ref) =>
    ref.kind === 'avatar'
      || state.realm.units.some(({ instanceId }) => instanceId === ref.instanceId));
  const attackerSurvived = continuation.pending.attacker.kind === 'avatar'
    || state.realm.units.some(({ instanceId }) =>
      instanceId === continuation.pending.attacker.instanceId);
  if (state.terminal.status === 'finished' || !attackerSurvived || survivors.length === 0) {
    return [state, [], []];
  }
  const firstIds = new Set(continuation.firstCombatantInstanceIds);
  return resolveFightWindow(
    state,
    deepFreeze({ ...continuation.pending, combatants: survivors }),
    [],
    'attacker-unit',
    !continuation.attackerStrikesFirst,
    survivors.filter(({ instanceId }) => !firstIds.has(instanceId)),
  );
}

function finishFight(
  state: GameState,
  pending: PendingCombat,
  outcomes: readonly GameOutcome[],
  returnStrikes = true,
  incrementVersion = true,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const finish = (candidate: GameState): GameState =>
    incrementVersion ? withStateVersion(candidate, {}) : candidate;
  const attacker = unitStatus(state, pending.attacker);
  const attackerStrikesFirst = returnStrikes
    && !attacker.disabled
    && (attacker.strikesFirstWhileAttacking || carriedLanceCount(state, pending.attacker) > 0);
  const firstCombatants = returnStrikes
    ? pending.combatants.filter((ref) =>
      !unitStatus(state, ref).disabled && carriedLanceCount(state, ref) > 0)
    : [];
  if (attackerStrikesFirst || firstCombatants.length > 0) {
    const [earlyState, earlyOutcomes, earlyDraws] = resolveFightWindow(
      state,
      pending,
      outcomes,
      'attacker-unit',
      attackerStrikesFirst,
      firstCombatants,
    );
    if (earlyState.pendingDeathrites) {
      return [
        finish(deepFreeze({
          ...earlyState,
          pendingDeathrites: {
            ...earlyState.pendingDeathrites,
            firstStrikeContinuation: {
              attackerStrikesFirst,
              firstCombatantInstanceIds: firstCombatants.map(({ instanceId }) => instanceId),
              pending,
            },
          },
        })),
        earlyOutcomes,
        earlyDraws,
      ];
    }
    const survivors = pending.combatants.filter((ref) =>
      ref.kind === 'avatar'
        || earlyState.realm.units.some(({ instanceId }) => instanceId === ref.instanceId));
    const attackerSurvived = pending.attacker.kind === 'avatar'
      || earlyState.realm.units.some(({ instanceId }) => instanceId === pending.attacker.instanceId);
    if (earlyState.terminal.status === 'finished' || !attackerSurvived || survivors.length === 0) {
      return [finish(earlyState), earlyOutcomes, earlyDraws];
    }
    const firstIds = new Set(firstCombatants.map(({ instanceId }) => instanceId));
    const [resolved, resolvedOutcomes, normalDraws] = resolveFightWindow(
      earlyState,
      deepFreeze({ ...pending, combatants: survivors }),
      earlyOutcomes,
      'attacker-unit',
      !attackerStrikesFirst,
      survivors.filter(({ instanceId }) => !firstIds.has(instanceId)),
    );
    return [finish(resolved), resolvedOutcomes, [...earlyDraws, ...normalDraws]];
  }
  const [resolved, resolvedOutcomes, draws] = resolveFightWindow(
    state,
    pending,
    outcomes,
    'attacker-unit',
    true,
    returnStrikes,
  );
  return [finish(resolved), resolvedOutcomes, draws];
}

function beginFight(
  state: GameState,
  pending: PendingCombat,
  combatants: readonly GameUnitRef[],
  outcomes: readonly GameOutcome[],
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const ordered = [...combatants].sort((left, right) =>
    left.instanceId < right.instanceId ? -1 : left.instanceId > right.instanceId ? 1 : 0);
  const fight = deepFreeze({ ...pending, allocations: [], combatants: ordered });
  const started: GameOutcome = {
    payload: {
      attackerInstanceId: pending.attacker.instanceId,
      combatantInstanceIds: ordered.map(({ instanceId }) => instanceId),
    },
    type: 'fight-started',
  };
  if (ordered.length === 1) {
    const amount = strikeDamage(state, pending.attacker);
    const allocated = deepFreeze({
      ...fight,
      allocations: [{ amount, targetInstanceId: ordered[0]!.instanceId }],
    });
    return finishFight(state, allocated, [
      ...outcomes,
      started,
      {
        payload: { amount, strikerInstanceId: pending.attacker.instanceId, targetInstanceId: ordered[0]!.instanceId },
        type: 'strike-damage-allocated',
      },
    ]);
  }
  return [
    withStateVersion(state, {
      decisionSeat: pending.attackingSeat,
      pendingCombat: fight,
      phase: 'allocate',
    }),
    [...outcomes, started],
    [],
  ];
}

function strikeUndefendedSite(
  state: GameState,
  pending: PendingCombat,
  outcomes: readonly GameOutcome[],
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const target = pending.originalTarget;
  if (!target || target.kind !== 'site' || pending.region) {
    throw new Error('unreachable site strike target');
  }
  const site = state.realm.sites[pending.cell];
  if (!site || site.instanceId !== target.instanceId || site.controller !== target.seat) {
    throw new Error('unreachable missing site strike target');
  }
  const amount = unitStatus(state, pending.attacker).attack;
  const interaction = recordInteraction(state, [pending.attacker]);
  const player = interaction.players[target.seat];
  const [updatedPlayer, lost, reachedDeathsDoor] = loseAvatarLife(player, amount, state.turnNumber);
  const life = updatedPlayer.avatar.life;
  return [
    withStateVersion(state, {
      decisionSeat: state.activeSeat,
      pendingCombat: null,
      phase: 'main',
      players: deepFreeze({ ...interaction.players, [target.seat]: updatedPlayer }),
      realm: { ...state.realm, units: interaction.units },
    }),
    [
      ...outcomes,
      {
        payload: {
          amount,
          attackerInstanceId: pending.attacker.instanceId,
          cell: pending.cell,
          siteInstanceId: target.instanceId,
        },
        type: 'undefended-site-struck',
      },
      ...(lost > 0
        ? [{ payload: { amount: lost, life, seat: target.seat }, type: 'avatar-life-lost' }]
        : []),
      ...(reachedDeathsDoor
        ? [{
          payload: { seat: target.seat, turnNumber: state.turnNumber },
          type: 'avatar-reached-deaths-door',
        }]
        : []),
      ...interaction.outcomes,
    ],
    [],
  ];
}

function applyDescriptor(
  state: GameState,
  descriptor: GameActionDescriptor,
  manifest: GameManifest,
  forcedRandomOutcomeInstanceId?: StateHash,
  resumeEndTurn: false | 'after-auras' | 'after-deaths' = false,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const seat = state.decisionSeat;
  const player = state.players[seat];
  if (descriptor.kind === 'order-deathrites') {
    return applyDeathriteOrder(state, descriptor.sourceInstanceId, manifest);
  }
  if (descriptor.kind === 'resolve-random-outcome') {
    const pending = state.pendingRandomOutcome;
    if (state.phase !== 'random-choice'
      || !pending
      || pending.seat !== seat
      || !pending.outcomeInstanceIds.includes(descriptor.outcomeInstanceId)) {
      throw new Error('unreachable illegal random outcome choice');
    }
    return applyDescriptor(
      deepFreeze({
        ...state,
        pendingRandomOutcome: null,
        phase: pending.action.kind === 'resolve-start-turn-trigger' ? 'start-turn' : 'main',
      }),
      pending.action,
      manifest,
      descriptor.outcomeInstanceId,
    );
  }
  if (descriptor.kind === 'resolve-end-turn-aura-random') {
    const { auraInstanceId, outcomeInstanceId } = descriptor;
    const pending = state.pendingEndTurnAura;
    const aura = state.realm.auras?.find(({ instanceId }) =>
      instanceId === auraInstanceId);
    const target = aura
      ? endTurnAuraCandidates(state, aura).find(({ instanceId }) =>
        instanceId === outcomeInstanceId)
      : undefined;
    if (state.phase !== 'end-turn-aura'
      || !pending
      || pending.stage !== 'random'
      || pending.seat !== seat
      || pending.auraInstanceId !== auraInstanceId
      || !pending.outcomeInstanceIds?.includes(outcomeInstanceId)
      || !aura
      || !target) {
      throw new Error('unreachable illegal end-turn Aura random choice');
    }
    const [damaged, outcomes, draws] = resolveEndTurnAuraDamage(state, aura, target);
    if (damaged.terminal.status === 'finished') {
      return [
        withStateVersion(damaged, { pendingEndTurnAura: null, phase: 'terminal' }),
        outcomes,
        draws,
      ];
    }
    return [
      withStateVersion(damaged, {
        decisionSeat: seat,
        pendingEndTurnAura: {
          auraInstanceId: pending.auraInstanceId,
          remainingAuraInstanceIds: pending.remainingAuraInstanceIds,
          seat,
          stage: 'move',
        },
        phase: 'end-turn-aura',
      }),
      outcomes,
      draws,
    ];
  }
  if (descriptor.kind === 'resolve-end-turn-aura-move') {
    const { auraInstanceId, cells: destinationCells } = descriptor;
    const pending = state.pendingEndTurnAura;
    const aura = state.realm.auras?.find(({ instanceId }) =>
      instanceId === auraInstanceId);
    const legalMove = destinationCells === undefined || (aura !== undefined
      && auraOneStepAreas(aura).some((cells) =>
        cells.every((cell, index) => cell === destinationCells[index])));
    if (state.phase !== 'end-turn-aura'
      || !pending
      || pending.stage !== 'move'
      || pending.seat !== seat
      || pending.auraInstanceId !== auraInstanceId
      || !aura
      || !legalMove) {
      throw new Error('unreachable illegal end-turn Aura move');
    }
    const moved = deepFreeze({
      ...state,
      pendingEndTurnAura: null,
      phase: 'main' as const,
      realm: {
        ...state.realm,
        auras: state.realm.auras!.map((candidate) =>
          candidate.instanceId === aura.instanceId && destinationCells
            ? deepFreeze({ ...candidate, cells: [...destinationCells] as TwoByTwoArea })
            : candidate),
      },
    });
    const movement: GameOutcome = destinationCells
      ? {
        payload: {
          cells: destinationCells,
          instanceId: aura.instanceId,
          seat,
          sourceInstanceId: aura.instanceId,
        },
        type: 'aura-moved',
      }
      : {
        payload: { instanceId: aura.instanceId, seat, sourceInstanceId: aura.instanceId },
        type: 'aura-move-declined',
      };
    const nextAura = beginEndTurnAura(moved, seat, pending.remainingAuraInstanceIds);
    if (nextAura) {
      return [
        withStateVersion(nextAura[0], {}),
        [movement, ...nextAura[1]],
        nextAura[2],
      ];
    }
    const ended = applyDescriptor(
      moved,
      { kind: 'end-turn' },
      manifest,
      undefined,
      'after-auras',
    );
    return [ended[0], [movement, ...ended[1]], ended[2]];
  }
  const randomRequest = forcedRandomOutcomeInstanceId === undefined
    ? luckyRandomOutcomeRequest(state, seat, descriptor)
    : undefined;
  if (randomRequest && randomRequest.candidateInstanceIds.length > 0) {
    const drawn = drawRandomOutcomes(
      state,
      seat,
      randomRequest.candidateInstanceIds,
      randomRequest.purpose,
      randomRequest.domainKind,
    );
    return [
      withStateVersion(state, {
        engine: drawn.engine,
        pendingRandomOutcome: {
          action: descriptor,
          outcomeInstanceIds: [...new Set(drawn.outcomeInstanceIds)],
          seat,
        },
        phase: 'random-choice',
      }),
      [],
      drawn.randomDraws,
    ];
  }
  if (descriptor.kind === 'resolve-start-turn-trigger') {
    return resolveStartTurnTrigger(state, descriptor.sourceInstanceId, forcedRandomOutcomeInstanceId);
  }
  if (descriptor.kind === 'mulligan') {
    const atlas = resolveMulliganZone(player.hand.atlas, player.atlas, descriptor.atlasOrder);
    const spellbook = resolveMulliganZone(player.hand.spellbook, player.spellbook, descriptor.spellbookOrder);
    const updatedPlayer = deepFreeze({
      ...player,
      atlas: atlas.deck,
      hand: { atlas: atlas.hand, spellbook: spellbook.hand },
      mulliganComplete: true,
      spellbook: spellbook.deck,
    });
    const players = replacePlayer(state, seat, updatedPlayer);
    const outcome: GameOutcome = {
      payload: {
        atlasCount: descriptor.atlasOrder.length,
        seat,
        spellbookCount: descriptor.spellbookOrder.length,
      },
      type: 'mulligan-completed',
    };
    if (seat === 'north') {
      return [withStateVersion(state, { activeSeat: 'south', decisionSeat: 'south', players }), [outcome], []];
    }
    const firstPlayer = players[manifest.firstSeat];
    const startedPlayer = deepFreeze({ ...firstPlayer, mana: siteCount(state, manifest.firstSeat) });
    return [
      withStateVersion(state, {
        activeSeat: manifest.firstSeat,
        decisionSeat: manifest.firstSeat,
        phase: 'main',
        players: deepFreeze({ ...players, [manifest.firstSeat]: startedPlayer }),
        turnNumber: 1,
      }),
      [outcome, {
        payload: { drawSkipped: true, seat: manifest.firstSeat, turnNumber: 1 },
        type: 'turn-started',
      }],
      [],
    ];
  }

  if (descriptor.kind === 'begin-chain-magic') {
    const selected = descriptor;
    const legal = magicDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'begin-chain-magic'
        && candidate.cardInstanceId === selected.cardInstanceId
        && candidate.casterInstanceId === selected.casterInstanceId
        && candidate.target.instanceId === selected.target.instanceId
        && candidate.target.kind === selected.target.kind
        && candidate.target.seat === selected.target.seat);
    if (!legal) throw new Error('unreachable illegal chained Magic target');
    return [
      withStateVersion(state, {
        pendingChainMagic: {
          cardId: selected.cardId,
          cardInstanceId: selected.cardInstanceId,
          casterInstanceId: selected.casterInstanceId,
          seat,
          targets: [selected.target],
        },
        phase: 'chain-magic',
      }),
      [],
      [],
    ];
  }

  if (descriptor.kind === 'extend-chain-magic') {
    const selected = descriptor;
    const pending = state.pendingChainMagic;
    const legal = chainMagicDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'extend-chain-magic'
        && candidate.target.instanceId === selected.target.instanceId
        && candidate.target.kind === selected.target.kind
        && candidate.target.seat === selected.target.seat);
    if (!pending || !legal) throw new Error('unreachable illegal chained Magic extension');
    return [
      withStateVersion(state, {
        pendingChainMagic: {
          ...pending,
          targets: [...pending.targets, selected.target],
        },
      }),
      [],
      [],
    ];
  }

  if (descriptor.kind === 'resolve-chain-magic') {
    const pending = state.pendingChainMagic;
    const legal = chainMagicDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'resolve-chain-magic');
    if (!pending || !legal) throw new Error('unreachable illegal chained Magic resolution');
    descriptor = {
      cardId: pending.cardId,
      cardInstanceId: pending.cardInstanceId,
      casterInstanceId: pending.casterInstanceId,
      kind: 'cast-magic',
      targets: pending.targets,
    };
  }

  if (descriptor.kind === 'resolve-ranged-step') {
    const pending = state.pendingRangedStep;
    const legal = state.phase === 'ranged-step'
      && pending?.seat === seat
      && rangedStepDescriptors(state, seat).some((candidate) =>
        candidate.kind === 'resolve-ranged-step'
        && candidate.choice === descriptor.choice
        && candidate.unitInstanceId === descriptor.unitInstanceId
        && (candidate.choice === 'decline' && descriptor.choice === 'decline'
          || candidate.choice === 'step' && descriptor.choice === 'step'
            && samePath(candidate.path, descriptor.path)));
    if (!legal || !pending) throw new Error('unreachable illegal Ranged step choice');
    if (descriptor.choice === 'decline') {
      return [withStateVersion(state, {
        decisionSeat: state.activeSeat,
        pendingRangedStep: null,
        phase: 'main',
      }), [], []];
    }
    const ref: GameUnitRef = {
      instanceId: pending.sourceInstanceId,
      kind: 'minion',
      seat,
    };
    const path = resolveDeclaredPath(state, ref, descriptor.path, false);
    const from = path.path[0]!;
    const to = path.path.at(-1)!;
    return [
      withStateVersion(path.state, {
        decisionSeat: path.state.activeSeat,
        pendingRangedStep: null,
        phase: path.state.terminal.status === 'finished' ? 'terminal' : 'main',
      }),
      [{
        payload: {
          from,
          instanceId: pending.sourceInstanceId,
          seat,
          sourceInstanceId: pending.sourceInstanceId,
          steps: path.path.length - 1,
          to,
        },
        type: 'unit-stepped',
      }, ...path.outcomes],
      [],
    ];
  }

  if (descriptor.kind === 'activate-site-destruction') {
    const legal = siteDestructionDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-site-destruction'
        && candidate.sourceSiteInstanceId === descriptor.sourceSiteInstanceId
        && candidate.targetCell === descriptor.targetCell
        && candidate.targetSiteInstanceId === descriptor.targetSiteInstanceId);
    const sourceEntry = REALM_CELLS
      .map((cell) => ({ cell, site: state.realm.sites[cell] }))
      .find(({ site }) => site?.instanceId === descriptor.sourceSiteInstanceId);
    const target = state.realm.sites[descriptor.targetCell];
    if (!legal || !sourceEntry?.site || isRubble(sourceEntry.site)
      || !target || target.instanceId !== descriptor.targetSiteInstanceId) {
      throw new Error('unreachable illegal site destruction');
    }
    const source = sourceEntry.site;
    const targetProtected = !isRubble(target)
      && siteCannotBeMovedDestroyedOrModified(state, target);
    const resolved = resolveSiteDeaths(
      state,
      [
        { cell: sourceEntry.cell, site: source },
        ...(isRubble(target) || targetProtected
          ? []
          : [{ cell: descriptor.targetCell, site: target }]),
      ],
      source.instanceId,
    );
    return [
      withStateVersion(state, {
        ...(resolved.terminal.status === 'finished' ? { phase: 'terminal' as const } : {}),
        ...(resolved.pendingDeathrites ? { pendingDeathrites: resolved.pendingDeathrites } : {}),
        players: resolved.players,
        realm: resolved.realm,
        terminal: resolved.terminal,
      }),
      [
        {
          payload: {
            cell: sourceEntry.cell,
            instanceId: source.instanceId,
            owner: source.owner,
            sourceInstanceId: source.instanceId,
          },
          type: 'site-sacrificed',
        },
        {
          payload: {
            cell: descriptor.targetCell,
            instanceId: target.instanceId,
            ...(!isRubble(target) ? { owner: target.owner } : {}),
            sourceInstanceId: source.instanceId,
          },
          type: targetProtected ? 'site-destruction-prevented' : 'site-destroyed',
        },
        ...resolved.outcomes,
      ],
      [],
    ];
  }

  if (descriptor.kind === 'fly-site') {
    const legal = siteFlightDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'fly-site'
        && candidate.sourceSiteInstanceId === descriptor.sourceSiteInstanceId
        && candidate.targetCell === descriptor.targetCell);
    const sourceEntry = REALM_CELLS
      .map((cell) => ({ cell, site: state.realm.sites[cell] }))
      .find(({ site }) => site?.instanceId === descriptor.sourceSiteInstanceId);
    if (!legal || !sourceEntry?.site || isRubble(sourceEntry.site)) {
      throw new Error('unreachable illegal site flight');
    }
    const sourceCell = sourceEntry.cell;
    const carriedAvatarInstanceIds = (['north', 'south'] as const)
      .filter((avatarSeat) => state.players[avatarSeat].avatar.location === sourceCell)
      .map((avatarSeat) => state.players[avatarSeat].avatar.card.instanceId);
    const carriedMinionInstanceIds = state.realm.units
      .filter((unit) => unit.location === sourceCell && unit.occupiedCells === undefined)
      .map(({ instanceId }) => instanceId);
    const carriedArtifactInstanceIds = (state.realm.artifacts ?? [])
      .filter((artifact) => !('bearer' in artifact) && artifact.location === sourceCell)
      .map(({ instanceId }) => instanceId);
    const sites = { ...state.realm.sites };
    delete sites[sourceCell];
    sites[descriptor.targetCell] = deepFreeze({
      ...sourceEntry.site,
      lastFlightTurn: state.turnNumber,
    });
    const moved = deepFreeze({
      ...state,
      players: deepFreeze(Object.fromEntries((['north', 'south'] as const).map((avatarSeat) => {
        const movingPlayer = state.players[avatarSeat];
        return [avatarSeat, movingPlayer.avatar.location === sourceCell
          ? deepFreeze({
            ...movingPlayer,
            avatar: { ...movingPlayer.avatar, location: descriptor.targetCell },
          })
          : movingPlayer];
      })) as Record<GameSeat, PlayerState>),
      realm: {
        ...state.realm,
        ...(state.realm.artifacts
          ? {
            artifacts: state.realm.artifacts.map((artifact) =>
              !('bearer' in artifact) && artifact.location === sourceCell
                ? deepFreeze({ ...artifact, location: descriptor.targetCell })
                : artifact),
          }
          : {}),
        sites,
        units: state.realm.units.map((unit) =>
          unit.location === sourceCell && unit.occupiedCells === undefined
            ? deepFreeze({ ...unit, location: descriptor.targetCell })
            : unit),
      },
    });
    const settlement = settleRegionOccupancy(moved);
    return [
      withStateVersion(settlement.state, {}),
      [{
        payload: {
          carriedArtifactInstanceIds,
          carriedAvatarInstanceIds,
          carriedMinionInstanceIds,
          from: sourceCell,
          instanceId: sourceEntry.site.instanceId,
          seat,
          to: descriptor.targetCell,
        },
        type: 'site-flown',
      }, ...settlement.outcomes],
      [],
    ];
  }

  if (descriptor.kind === 'resolve-genesis-spell') {
    const pending = state.pendingGenesisSpell;
    const nextSpell = player.spellbook[0];
    if (state.phase !== 'genesis'
      || !pending
      || pending.seat !== seat
      || !nextSpell) {
      throw new Error('unreachable illegal Genesis spell choice');
    }
    const updatedPlayer = descriptor.choice === 'bottom-next'
      ? deepFreeze({ ...player, spellbook: [...player.spellbook.slice(1), nextSpell] })
      : player;
    return [
      withStateVersion(state, {
        pendingGenesisSpell: null,
        phase: 'main',
        players: replacePlayer(state, seat, updatedPlayer),
      }),
      [{
        payload: { seat, sourceInstanceId: pending.sourceInstanceId },
        type: descriptor.choice === 'bottom-next' ? 'spell-bottomed' : 'spell-kept',
      }],
      [],
    ];
  }

  if (descriptor.kind === 'resolve-genesis-spell-order') {
    const pending = state.pendingGenesisSpellOrder;
    if (state.phase !== 'genesis'
      || !pending
      || pending.seat !== seat
      || descriptor.order.length !== pending.count
      || new Set(descriptor.order).size !== pending.count
      || descriptor.order.some((index) => !Number.isSafeInteger(index)
        || index < 0 || index >= pending.count)) {
      throw new Error('unreachable illegal Genesis spell order');
    }
    const top = player.spellbook.slice(0, pending.count);
    const updatedPlayer = deepFreeze({
      ...player,
      spellbook: [
        ...descriptor.order.map((index) => top[index]!),
        ...player.spellbook.slice(pending.count),
      ],
    });
    return [
      withStateVersion(state, {
        pendingGenesisSpellOrder: null,
        phase: 'main',
        players: replacePlayer(state, seat, updatedPlayer),
      }),
      [{
        payload: { count: pending.count, seat, sourceInstanceId: pending.sourceInstanceId },
        type: 'spells-reordered',
      }],
      [],
    ];
  }

  if (descriptor.kind === 'resolve-genesis-token') {
    const pending = state.pendingGenesisToken;
    const source = pending && state.realm.sites[pending.cell];
    const definition = source && !isRubble(source) ? cardDefinition(state, source.cardId) : undefined;
    const tokenCardId = definition?.cardType === 'site'
      ? definition.genesisPayOneManaToSummonToken
      : undefined;
    if (state.phase !== 'genesis'
      || !pending
      || pending.seat !== seat
      || !source
      || isRubble(source)
      || source.instanceId !== pending.sourceInstanceId
      || !tokenCardId
      || descriptor.choice === 'pay-one-mana' && player.mana < 1) {
      throw new Error('unreachable illegal Genesis token choice');
    }
    const token = descriptor.choice === 'pay-one-mana'
      ? tokenUnit(state, seat, tokenCardId, source.instanceId, pending.cell, 0)
      : undefined;
    return [
      withStateVersion(state, {
        pendingGenesisToken: null,
        phase: state.pendingGenesisSpell ? 'genesis' : 'main',
        players: token
          ? replacePlayer(state, seat, deepFreeze({ ...player, mana: player.mana - 1 }))
          : state.players,
        realm: token
          ? deepFreeze({ ...state.realm, units: [...state.realm.units, token] })
          : state.realm,
      }),
      token
        ? [{
          payload: {
            cardId: token.cardId,
            cell: token.location,
            instanceId: token.instanceId,
            manaPaid: 1,
            owner: token.owner,
            seat: token.controller,
            sourceInstanceId: source.instanceId,
            token: true,
          },
          type: 'minion-summoned',
        }]
        : [],
      [],
    ];
  }

  if (descriptor.kind === 'replace-rubble-with-top-atlas-site') {
    const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
    const target = state.realm.sites[descriptor.targetCell];
    const card = player.atlas[0];
    const definition = card && cardDefinition(state, card.cardId);
    if (avatarDefinition.cardType !== 'avatar'
      || avatarDefinition.replaceAdjacentRubbleWithTopAtlasSite !== true
      || player.avatar.tapped
      || !target
      || !isRubble(target)
      || target.instanceId !== descriptor.targetRubbleInstanceId
      || !borderingCells(player.avatar.location).includes(descriptor.targetCell)
      || !card
      || definition?.cardType !== 'site') {
      throw new Error('unreachable illegal Rubble replacement');
    }
    return applyDescriptor(state, {
      cardId: card.cardId,
      cardInstanceId: card.instanceId,
      cell: descriptor.targetCell,
      fromTopAtlas: true,
      ...(definition.genesisPayOneManaToSummonToken
        ? { genesisTokenChoice: 'defer' as const }
        : {}),
      kind: 'play-site',
    }, manifest);
  }

  if (descriptor.kind === 'play-site') {
    const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
    if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
    const card = descriptor.fromTopAtlas
      ? player.atlas[0]
      : player.hand.atlas.find(({ cardId, instanceId }) =>
        instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    if (card?.instanceId !== descriptor.cardInstanceId || card.cardId !== descriptor.cardId) {
      throw new Error('unreachable site card');
    }
    const definition = cardDefinition(state, card.cardId);
    if (definition.cardType !== 'site') throw new Error('unreachable non-site card');
    const previousSite = state.realm.sites[descriptor.cell];
    const replacingRubble = previousSite !== undefined && isRubble(previousSite);
    const legalCell = descriptor.fromTopAtlas
      ? !player.avatar.tapped
        && avatarDefinition.replaceAdjacentRubbleWithTopAtlasSite === true
        && replacingRubble
        && borderingCells(player.avatar.location).includes(descriptor.cell)
      : !player.domainEstablished
      ? !player.avatar.tapped
        && descriptor.cell === player.avatar.location
        && (!previousSite || replacingRubble)
      : !player.avatar.tapped && legalSiteCells(state, seat).includes(descriptor.cell);
    if (!legalCell) throw new Error('unreachable illegal site cell');
    const site = deepFreeze({ ...card, controller: seat });
    const genesisSpellDrawCount = definition.genesisDrawSpellPerAdjacentSameCard
      ? borderingCells(descriptor.cell)
        .filter((cell) => {
          const adjacent = state.realm.sites[cell];
          return adjacent !== undefined && !isRubble(adjacent) && adjacent.cardId === card.cardId;
        }).length
      : 0;
    const genesisSpellDraws = player.spellbook.slice(0, genesisSpellDrawCount);
    const genesisSpellDiscards = definition.genesisDiscardTopSpells
      ? player.spellbook.slice(0, definition.genesisDiscardTopSpells)
      : [];
    const genesisGainMana = definition.genesisGainMana
      ?? (definition.genesisGainManaIfOnlyControlledCopy === 1
        && Object.values(state.realm.sites).every((existing) =>
          isRubble(existing) || existing.controller !== seat || existing.cardId !== card.cardId)
        ? 1
        : 0);
    const genesisDrawFailed = genesisSpellDraws.length < genesisSpellDrawCount;
    const updatedPlayer = deepFreeze({
      ...player,
      avatar: { ...player.avatar, tapped: true },
      cemetery: [...player.cemetery, ...genesisSpellDiscards],
      domainEstablished: true,
      hand: {
        ...player.hand,
        atlas: descriptor.fromTopAtlas
          ? player.hand.atlas
          : player.hand.atlas.filter(({ instanceId }) => instanceId !== card.instanceId),
        spellbook: [...player.hand.spellbook, ...genesisSpellDraws],
      },
      mana: player.mana + 1 + genesisGainMana
        - Number(descriptor.genesisTokenChoice === 'pay-one-mana'),
      atlas: descriptor.fromTopAtlas ? player.atlas.slice(1) : player.atlas,
      spellbook: player.spellbook.slice(genesisSpellDraws.length + genesisSpellDiscards.length),
    });
    const winner = otherSeat(seat);
    const placedUnits = state.realm.units.map((unit) => {
      if (unit.location !== descriptor.cell) return unit;
      if (unit.region === 'void') return deepFreeze({ ...unit, region: 'surface' as const });
      if (replacingRubble && unit.region === 'underground' && definition.elements.includes('water')) {
        return deepFreeze({ ...unit, region: 'underwater' as const });
      }
      return unit;
    });
    const placedArtifacts = state.realm.artifacts?.map((artifact) => {
      if ('bearer' in artifact || artifact.location !== descriptor.cell) return artifact;
      if (artifact.region === 'void') {
        return deepFreeze({ ...artifact, region: 'surface' as const });
      }
      if (replacingRubble
        && artifact.region === 'underground'
        && definition.elements.includes('water')) {
        return deepFreeze({ ...artifact, region: 'underwater' as const });
      }
      return artifact;
    });
    const placedState = deepFreeze({
      ...state,
      players: replacePlayer(state, seat, updatedPlayer),
      realm: {
        ...state.realm,
        ...(placedArtifacts ? { artifacts: placedArtifacts } : {}),
        sites: { ...state.realm.sites, [descriptor.cell]: site },
        units: placedUnits,
      },
    });
    const settlement = settleRegionOccupancy(placedState);
    // ponytail: site Genesis needs a serializable post-Deathrite continuation only
    // when a real-card test first combines terrain replacement, two Deathrites,
    // and a remaining Genesis effect. Fail closed instead of reordering it.
    if (settlement.state.pendingDeathrites) {
      throw new Error('unsupported Deathrite ordering during site Genesis continuation');
    }
    const nearbyCells = new Set([
      descriptor.cell,
      ...borderingCells(descriptor.cell),
      ...diagonalCells(descriptor.cell),
    ]);
    let genesisPlayers = settlement.state.players;
    const genesisHealOutcomes: GameOutcome[] = [];
    if (definition.genesisHealNearbyAvatars === 3) {
      for (const healedSeat of ['north', 'south'] as const) {
        const healedPlayer = genesisPlayers[healedSeat];
        if (!nearbyCells.has(healedPlayer.avatar.location)) continue;
        const avatar = cardDefinition(settlement.state, healedPlayer.avatar.card.cardId);
        if (avatar.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
        const [healed, amount] = healAvatar(
          healedPlayer,
          avatar.life,
          definition.genesisHealNearbyAvatars,
        );
        genesisPlayers = deepFreeze({ ...genesisPlayers, [healedSeat]: healed });
        if (amount > 0) {
          genesisHealOutcomes.push({
            payload: {
              amount,
              attemptedAmount: definition.genesisHealNearbyAvatars,
              life: healed.avatar.life,
              seat: healedSeat,
              sourceInstanceId: card.instanceId,
            },
            type: 'avatar-healed',
          });
        }
      }
    }
    const immobileArea = definition.genesisImmobilizeNearbyUntilNextTurn
      ? deepFreeze({
        cells: REALM_CELLS.filter((cell) => {
          const nearbySite = settlement.state.realm.sites[cell];
          return nearbyCells.has(cell) && nearbySite !== undefined && !isRubble(nearbySite);
        }),
        expiresAtSeat: seat,
        sourceInstanceId: card.instanceId,
      })
      : undefined;
    const enemyStealthRefs: readonly GameUnitRef[] = definition.genesisEnemiesLoseStealth
      ? settlement.state.realm.units
        .filter(({ controller, stealthed }) => controller !== seat && stealthed)
        .map(({ controller, instanceId }) => ({ instanceId, kind: 'minion' as const, seat: controller }))
      : [];
    const [genesisUnits, enemyStealthOutcomes] = loseStealth(
      settlement.state.realm.units,
      enemyStealthRefs,
      card.instanceId,
    );
    const genesisToken = descriptor.genesisTokenChoice === 'pay-one-mana'
      && definition.genesisPayOneManaToSummonToken
      ? tokenUnit(
        settlement.state,
        seat,
        definition.genesisPayOneManaToSummonToken,
        card.instanceId,
        descriptor.cell,
        0,
      )
      : undefined;
    const terminal = genesisDrawFailed
      ? { loser: seat, reason: 'deck_empty' as const, status: 'finished' as const, winner }
      : settlement.state.terminal;
    const settlementOutcomes = genesisDrawFailed
      ? settlement.outcomes.filter(({ type }) => type !== 'game-ended')
      : settlement.outcomes;
    const pendingGenesisSpell = terminal.status === 'active'
      && definition.genesisMayBottomNextSpell === true
      && settlement.state.players[seat].spellbook.length > 0
      ? deepFreeze({ seat, sourceInstanceId: card.instanceId })
      : undefined;
    const pendingGenesisSpellOrderCount = definition.genesisReorderNextSpells === 3
      ? Math.min(definition.genesisReorderNextSpells, settlement.state.players[seat].spellbook.length)
      : 0;
    const pendingGenesisSpellOrder = terminal.status === 'active'
      && pendingGenesisSpellOrderCount > 0
      ? deepFreeze({
        count: pendingGenesisSpellOrderCount,
        seat,
        sourceInstanceId: card.instanceId,
      })
      : undefined;
    const pendingGenesisToken = terminal.status === 'active'
      && descriptor.genesisTokenChoice === 'defer'
      && definition.genesisPayOneManaToSummonToken !== undefined
      ? deepFreeze({ cell: descriptor.cell, seat, sourceInstanceId: card.instanceId })
      : undefined;
    const resolvedState = deepFreeze({
        ...(terminal.status === 'finished'
          ? { phase: 'terminal' as const }
          : pendingGenesisToken || pendingGenesisSpell || pendingGenesisSpellOrder
            ? {
              ...(pendingGenesisSpell ? { pendingGenesisSpell } : {}),
              ...(pendingGenesisSpellOrder ? { pendingGenesisSpellOrder } : {}),
              ...(pendingGenesisToken ? { pendingGenesisToken } : {}),
              phase: 'genesis' as const,
            }
            : {}),
        players: genesisPlayers,
        realm: {
          ...settlement.state.realm,
          ...(immobileArea
            ? {
              immobileAreas: [
                ...(settlement.state.realm.immobileAreas ?? []),
                immobileArea,
              ],
            }
            : {}),
          units: [...genesisUnits, ...(genesisToken ? [genesisToken] : [])],
        },
        terminal,
      });
    const createRubbleAt = terminal.status === 'active' ? descriptor.createRubbleAt : undefined;
    const rubble = createRubbleAt
      ? deepFreeze({
        controller: null,
        instanceId: identityHash(asJson({
          cell: createRubbleAt,
          kind: 'rubble',
          sourceInstanceId: player.avatar.card.instanceId,
          stateVersion: state.stateVersion,
        })),
        rubble: true as const,
      })
      : undefined;
    const finalState = rubble && createRubbleAt
      ? deepFreeze({
        ...resolvedState,
        realm: {
          ...resolvedState.realm,
          ...(resolvedState.realm.artifacts
            ? {
              artifacts: resolvedState.realm.artifacts.map((artifact) =>
                !('bearer' in artifact)
                  && artifact.location === createRubbleAt
                  && artifact.region === 'void'
                  ? deepFreeze({ ...artifact, region: 'surface' as const })
                  : artifact),
            }
            : {}),
          sites: { ...resolvedState.realm.sites, [createRubbleAt]: rubble },
          units: resolvedState.realm.units.map((unit) =>
            unit.location === createRubbleAt && unit.region === 'void'
              ? deepFreeze({ ...unit, region: 'surface' as const })
              : unit),
        },
      })
      : resolvedState;
    return [
      withStateVersion(state, finalState),
      [
        ...(replacingRubble
          ? [{
            payload: {
              cell: descriptor.cell,
              instanceId: previousSite.instanceId,
              targetSiteInstanceId: card.instanceId,
            },
            type: 'rubble-replaced',
          }]
          : []),
        { payload: { cardId: card.cardId, cell: descriptor.cell, instanceId: card.instanceId, seat }, type: 'site-played' },
        ...genesisHealOutcomes,
        ...(genesisGainMana
          ? [{
            payload: { amount: genesisGainMana, seat, sourceInstanceId: card.instanceId },
            type: 'mana-gained',
          }]
          : []),
        ...(genesisToken
          ? [{
            payload: {
              cardId: genesisToken.cardId,
              cell: genesisToken.location,
              instanceId: genesisToken.instanceId,
              manaPaid: 1,
              owner: genesisToken.owner,
              seat: genesisToken.controller,
              sourceInstanceId: card.instanceId,
              token: true,
            },
            type: 'minion-summoned' as const,
          }]
          : []),
        ...genesisSpellDraws.map(() => ({
          payload: { seat, sourceInstanceId: card.instanceId },
          type: 'spell-drawn',
        })),
        ...genesisSpellDiscards.map((discarded) => ({
          payload: {
            cardId: discarded.cardId,
            instanceId: discarded.instanceId,
            owner: discarded.owner,
            seat,
            sourceInstanceId: card.instanceId,
          },
          type: 'spell-discarded',
        })),
        ...enemyStealthOutcomes,
        ...settlementOutcomes,
        ...(genesisDrawFailed
          ? [{ payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' }]
          : []),
        ...(rubble && createRubbleAt
          ? [{
            payload: {
              cell: createRubbleAt,
              instanceId: rubble.instanceId,
              sourceInstanceId: player.avatar.card.instanceId,
            },
            type: 'rubble-created',
          }]
          : []),
      ],
      [],
    ];
  }

  if (descriptor.kind === 'cast-artifact') {
    const card = player.hand.spellbook.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    const definition = card && cardDefinition(state, card.cardId);
    const legal = artifactDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'cast-artifact'
        && candidate.cardInstanceId === descriptor.cardInstanceId
        && candidate.casterInstanceId === descriptor.casterInstanceId
        && candidate.cell === descriptor.cell
        && candidate.bearerCell === descriptor.bearerCell
        && candidate.manaCost === descriptor.manaCost
        && (candidate.bearer === undefined && descriptor.bearer === undefined
          || candidate.bearer !== undefined
            && descriptor.bearer !== undefined
            && candidate.bearer.instanceId === descriptor.bearer.instanceId
            && candidate.bearer.kind === descriptor.bearer.kind
            && candidate.bearer.seat === descriptor.bearer.seat));
    const caster = spellcasterRefs(state, seat).find(({ instanceId }) =>
      instanceId === descriptor.casterInstanceId);
    if (!card || !definition || definition.cardType !== 'artifact' || !legal || !caster) {
      throw new Error('unreachable illegal Artifact cast');
    }
    const paidPlayer = deepFreeze({
      ...player,
      ...(player.airThresholdsCastThisTurn !== undefined
        ? {
          airThresholdsCastThisTurn:
            player.airThresholdsCastThisTurn + definition.thresholds.air,
        }
        : {}),
      hand: {
        ...player.hand,
        spellbook: player.hand.spellbook.filter(({ instanceId }) => instanceId !== card.instanceId),
      },
      mana: player.mana - descriptor.manaCost,
    });
    const paidState = deepFreeze({ ...state, players: replacePlayer(state, seat, paidPlayer) });
    const interaction = recordInteraction(paidState, [caster]);
    const artifact: ArtifactInstance = descriptor.bearer
      ? deepFreeze({
        ...card,
        bearer: descriptor.bearer,
        ...(descriptor.bearerCell ? { bearerCell: descriptor.bearerCell } : {}),
      })
      : deepFreeze({ ...card, location: descriptor.cell!, region: 'surface' as const });
    return [
      withStateVersion(paidState, {
        players: interaction.players,
        realm: {
          ...paidState.realm,
          artifacts: [...(paidState.realm.artifacts ?? []), artifact],
          units: interaction.units,
        },
      }),
      [
        ...interaction.outcomes,
        {
          payload: {
            cardId: card.cardId,
            casterInstanceId: descriptor.casterInstanceId,
            instanceId: card.instanceId,
            manaPaid: descriptor.manaCost,
            owner: card.owner,
            seat,
            ...(descriptor.bearer
              ? {
                bearerInstanceId: descriptor.bearer.instanceId,
                ...(descriptor.bearerCell ? { cell: descriptor.bearerCell } : {}),
                bearerKind: descriptor.bearer.kind,
                bearerSeat: descriptor.bearer.seat,
              }
              : { cell: descriptor.cell!, region: 'surface' }),
          },
          type: 'artifact-conjured',
        },
      ],
      [],
    ];
  }

  if (descriptor.kind === 'pick-up-artifacts') {
    const legal = pickUpArtifactDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'pick-up-artifacts'
        && candidate.unit.instanceId === descriptor.unit.instanceId
        && candidate.unit.kind === descriptor.unit.kind
        && candidate.unit.seat === descriptor.unit.seat
        && candidate.cell === descriptor.cell
        && candidate.artifactInstanceIds.length === descriptor.artifactInstanceIds.length
        && candidate.artifactInstanceIds.every((instanceId, index) =>
          instanceId === descriptor.artifactInstanceIds[index]));
    if (!legal) throw new Error('unreachable illegal Artifact Pick Up');

    const selected = new Set(descriptor.artifactInstanceIds);
    const artifacts = (state.realm.artifacts ?? []).map((artifact): ArtifactInstance => {
      if (!selected.has(artifact.instanceId)) return artifact;
      if ('bearer' in artifact) throw new Error('unreachable carried Artifact Pick Up');
      return deepFreeze({
        bearer: descriptor.unit,
        ...(descriptor.cell ? { bearerCell: descriptor.cell } : {}),
        cardId: artifact.cardId,
        instanceId: artifact.instanceId,
        owner: artifact.owner,
        source: artifact.source,
      });
    });
    const players = descriptor.unit.kind === 'avatar'
      ? replacePlayer(state, seat, deepFreeze({
        ...player,
        avatar: {
          ...player.avatar,
          lastPickedUpArtifactsTurn: state.turnNumber,
        },
      }))
      : state.players;
    const units = descriptor.unit.kind === 'minion'
      ? state.realm.units.map((unit) => unit.instanceId === descriptor.unit.instanceId
        ? deepFreeze({ ...unit, lastPickedUpArtifactsTurn: state.turnNumber })
        : unit)
      : state.realm.units;
    return [
      withStateVersion(state, {
        players,
        realm: { ...state.realm, artifacts, units },
      }),
      [{
        payload: {
          artifactInstanceIds: descriptor.artifactInstanceIds,
          seat,
          unitInstanceId: descriptor.unit.instanceId,
          unitKind: descriptor.unit.kind,
        },
        type: 'artifacts-picked-up',
      }],
      [],
    ];
  }

  if (descriptor.kind === 'drop-artifacts') {
    const legal = dropArtifactDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'drop-artifacts'
        && candidate.unit.instanceId === descriptor.unit.instanceId
        && candidate.unit.kind === descriptor.unit.kind
        && candidate.unit.seat === descriptor.unit.seat
        && candidate.artifactInstanceIds.length === descriptor.artifactInstanceIds.length
        && candidate.artifactInstanceIds.every((instanceId, index) =>
          instanceId === descriptor.artifactInstanceIds[index]));
    if (!legal) throw new Error('unreachable illegal Artifact Drop');
    const status = unitStatus(state, descriptor.unit);
    const dropped = dropArtifactsCarriedBy(
      state.realm.artifacts,
      { instanceId: descriptor.unit.instanceId, location: status.location, region: status.region },
      new Set(descriptor.artifactInstanceIds),
    );
    const players = descriptor.unit.kind === 'avatar'
      ? replacePlayer(state, seat, deepFreeze({
        ...player,
        avatar: { ...player.avatar, lastDroppedArtifactsTurn: state.turnNumber },
      }))
      : state.players;
    const units = descriptor.unit.kind === 'minion'
      ? state.realm.units.map((unit) => unit.instanceId === descriptor.unit.instanceId
        ? deepFreeze({ ...unit, lastDroppedArtifactsTurn: state.turnNumber })
        : unit)
      : state.realm.units;
    const droppedState = deepFreeze({
      ...state,
      players,
      realm: {
        ...state.realm,
        ...(dropped.artifacts ? { artifacts: dropped.artifacts } : {}),
        units,
      },
    });
    const bearer = descriptor.unit.kind === 'minion'
      ? units.find(({ instanceId }) => instanceId === descriptor.unit.instanceId)
      : undefined;
    const deaths = bearer
      && bearer.damage > 0
      && bearer.damage >= unitStatus(droppedState, descriptor.unit).defense
      ? [bearer]
      : [];
    const deathResolution = resolveMinionDeaths(
      droppedState,
      droppedState.players,
      droppedState.realm.units,
      deaths,
      new Set(),
    );
    return [
      withStateVersion(droppedState, {
        ...(deathResolution.terminal.status === 'finished' ? { phase: 'terminal' } : {}),
        ...(deathResolution.pendingDeathrites
          ? { pendingDeathrites: deathResolution.pendingDeathrites }
          : {}),
        players: deathResolution.players,
        realm: {
          ...droppedState.realm,
          ...(deathResolution.artifacts ? { artifacts: deathResolution.artifacts } : {}),
          units: deathResolution.units,
        },
        terminal: deathResolution.terminal,
      }),
      [
        {
          payload: {
            artifactInstanceIds: descriptor.artifactInstanceIds,
            seat,
            unitInstanceId: descriptor.unit.instanceId,
            unitKind: descriptor.unit.kind,
          },
          type: 'artifacts-dropped',
        },
        ...deathResolution.outcomes,
      ],
      [],
    ];
  }

  if (descriptor.kind === 'cast-aura') {
    const card = player.hand.spellbook.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    const definition = card && cardDefinition(state, card.cardId);
    const legal = auraDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'cast-aura'
        && candidate.cardInstanceId === descriptor.cardInstanceId
        && candidate.casterInstanceId === descriptor.casterInstanceId
        && candidate.cells.every((cell, index) => cell === descriptor.cells[index]));
    const caster = spellcasterRefs(state, seat).find(({ instanceId }) =>
      instanceId === descriptor.casterInstanceId);
    if (!card || !definition || definition.cardType !== 'aura' || !legal || !caster) {
      throw new Error('unreachable illegal Aura cast');
    }
    const paidPlayer = deepFreeze({
      ...player,
      ...(player.airThresholdsCastThisTurn !== undefined
        ? {
          airThresholdsCastThisTurn:
            player.airThresholdsCastThisTurn + definition.thresholds.air,
        }
        : {}),
      hand: {
        ...player.hand,
        spellbook: player.hand.spellbook.filter(({ instanceId }) => instanceId !== card.instanceId),
      },
      mana: player.mana - definition.manaCost,
    });
    const paidState = deepFreeze({ ...state, players: replacePlayer(state, seat, paidPlayer) });
    const interaction = recordInteraction(paidState, [caster]);
    const aura: AuraInstance = deepFreeze({
      ...card,
      cells: [...descriptor.cells] as TwoByTwoArea,
      controller: seat,
      turnCounters: 0,
    });
    const immobileArea: ImmobileArea | undefined =
      definition.immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns === true
        ? deepFreeze({
          cells: [...descriptor.cells],
          minionsAtSitesOnly: true,
          sourceInstanceId: card.instanceId,
          suppressesAirborne: true,
        })
        : undefined;
    return [
      withStateVersion(paidState, {
        players: interaction.players,
        realm: {
          ...paidState.realm,
          auras: [...(paidState.realm.auras ?? []), aura],
          ...(immobileArea
            ? { immobileAreas: [...(paidState.realm.immobileAreas ?? []), immobileArea] }
            : {}),
          units: interaction.units,
        },
      }),
      [
        ...interaction.outcomes,
        {
          payload: {
            cardId: card.cardId,
            casterInstanceId: descriptor.casterInstanceId,
            cells: descriptor.cells,
            instanceId: card.instanceId,
            manaPaid: definition.manaCost,
            owner: card.owner,
            seat,
          },
          type: 'aura-conjured',
        },
      ],
      [],
    ];
  }

  if (descriptor.kind === 'cast-magic') {
    const card = player.hand.spellbook.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    const definition = card && cardDefinition(state, card.cardId);
    const chainPending = state.pendingChainMagic;
    const legal = definition?.cardType === 'magic'
      && definition.damageChainNearbyUnits === true
      ? state.phase === 'chain-magic'
        && chainPending?.seat === seat
        && chainPending.cardId === descriptor.cardId
        && chainPending.cardInstanceId === descriptor.cardInstanceId
        && chainPending.casterInstanceId === descriptor.casterInstanceId
        && sameOptionalUnitRefs(chainPending.targets, descriptor.targets)
      : magicDescriptors(state, seat).some((candidate) =>
        candidate.kind === 'cast-magic'
        && candidate.cardInstanceId === descriptor.cardInstanceId
        && candidate.casterInstanceId === descriptor.casterInstanceId
        && candidate.cemeteryMinionInstanceId === descriptor.cemeteryMinionInstanceId
        && candidate.discardSiteInstanceId === descriptor.discardSiteInstanceId
        && (candidate.ally === undefined && descriptor.ally === undefined
          || candidate.ally !== undefined
            && descriptor.ally !== undefined
            && candidate.ally.instanceId === descriptor.ally.instanceId
            && candidate.ally.kind === descriptor.ally.kind
            && candidate.ally.seat === descriptor.ally.seat)
        && (candidate.allyDestination === undefined && descriptor.allyDestination === undefined
          || candidate.allyDestination !== undefined
            && descriptor.allyDestination !== undefined
            && sameLocation(candidate.allyDestination, descriptor.allyDestination))
        && (candidate.allyDestinationCells === undefined
          && descriptor.allyDestinationCells === undefined
          || candidate.allyDestinationCells !== undefined
            && descriptor.allyDestinationCells !== undefined
            && candidate.allyDestinationCells.every((cell, index) =>
              cell === descriptor.allyDestinationCells![index]))
        && (candidate.allyStrikeLocation === undefined
          && descriptor.allyStrikeLocation === undefined
          || candidate.allyStrikeLocation !== undefined
            && descriptor.allyStrikeLocation !== undefined
            && sameLocation(candidate.allyStrikeLocation, descriptor.allyStrikeLocation))
        && (candidate.target === undefined && descriptor.target === undefined
          || candidate.target !== undefined
            && descriptor.target !== undefined
            && candidate.target.instanceId === descriptor.target.instanceId
            && candidate.target.kind === descriptor.target.kind
            && candidate.target.seat === descriptor.target.seat)
        && sameOptionalUnitRefs(candidate.targets, descriptor.targets)
        && candidate.targetArtifactInstanceId === descriptor.targetArtifactInstanceId
        && (candidate.targetLocation === undefined && descriptor.targetLocation === undefined
          || candidate.targetLocation !== undefined
            && descriptor.targetLocation !== undefined
            && sameLocation(candidate.targetLocation, descriptor.targetLocation))
        && candidate.targetSiteInstanceId === descriptor.targetSiteInstanceId
        && (candidate.temptedDestination === undefined && descriptor.temptedDestination === undefined
          || candidate.temptedDestination !== undefined
            && descriptor.temptedDestination !== undefined
            && sameLocation(candidate.temptedDestination, descriptor.temptedDestination))
        && (candidate.temptedEnemy === undefined && descriptor.temptedEnemy === undefined
          || candidate.temptedEnemy !== undefined
            && descriptor.temptedEnemy !== undefined
            && candidate.temptedEnemy.instanceId === descriptor.temptedEnemy.instanceId
            && candidate.temptedEnemy.kind === descriptor.temptedEnemy.kind
            && candidate.temptedEnemy.seat === descriptor.temptedEnemy.seat));
    const caster = spellcasterRefs(state, seat).find(({ instanceId }) =>
      instanceId === descriptor.casterInstanceId);
    if (!card || !definition || definition.cardType !== 'magic' || !legal || !caster) {
      throw new Error('unreachable illegal Magic cast');
    }
    const discardedSite = definition.discardSiteAsAdditionalCost === true
      ? player.hand.atlas.find(({ instanceId }) =>
        instanceId === descriptor.discardSiteInstanceId)
      : undefined;
    if (definition.discardSiteAsAdditionalCost === true && !discardedSite) {
      throw new Error('unreachable missing Magic site-discard cost');
    }
    const manaPaid = definition.manaCost + (definition.damageChainNearbyUnits === true
      ? CHAIN_MAGIC_EXTRA_TARGET_MANA * (descriptor.targets!.length - 1)
      : 0);
    const paidPlayer = deepFreeze({
      ...player,
      ...(player.airThresholdsCastThisTurn !== undefined
        ? {
          airThresholdsCastThisTurn:
            player.airThresholdsCastThisTurn + definition.thresholds.air,
        }
        : {}),
      hand: {
        ...player.hand,
        ...(discardedSite
          ? {
            atlas: player.hand.atlas.filter(({ instanceId }) =>
              instanceId !== discardedSite.instanceId),
          }
          : {}),
        spellbook: player.hand.spellbook.filter(({ instanceId }) => instanceId !== card.instanceId),
      },
      ...(discardedSite ? { cemetery: [...player.cemetery, discardedSite] } : {}),
      mana: player.mana - manaPaid,
    });
    const paidState = deepFreeze({
      ...state,
      ...(definition.damageChainNearbyUnits === true
        ? { decisionSeat: state.activeSeat, pendingChainMagic: null, phase: 'main' as const }
        : {}),
      players: replacePlayer(state, seat, paidPlayer),
    });
    const interaction = recordInteraction(paidState, [caster]);
    const owner = interaction.players[card.owner];
    const castState = deepFreeze({
      ...paidState,
      players: deepFreeze({
        ...interaction.players,
        [card.owner]: deepFreeze({ ...owner, cemetery: [...owner.cemetery, card] }),
      }),
      realm: { ...paidState.realm, units: interaction.units },
    });
    const castOutcome = {
      payload: {
        cardId: card.cardId,
        casterInstanceId: descriptor.casterInstanceId,
        instanceId: card.instanceId,
        manaPaid,
        seat,
        ...(descriptor.target
          ? {
            targetInstanceId: descriptor.target.instanceId,
            targetSeat: descriptor.target.seat,
          }
          : {}),
        ...(descriptor.targets
          ? { targetInstanceIds: descriptor.targets.map(({ instanceId }) => instanceId) }
          : {}),
        ...(descriptor.targetArtifactInstanceId
          ? { targetArtifactInstanceId: descriptor.targetArtifactInstanceId }
          : {}),
        ...(descriptor.targetLocation ? { targetLocation: descriptor.targetLocation } : {}),
        ...(descriptor.ally
          ? { allyInstanceId: descriptor.ally.instanceId, allySeat: descriptor.ally.seat }
          : {}),
        ...(descriptor.allyDestination ? { allyDestination: descriptor.allyDestination } : {}),
        ...(descriptor.allyDestinationCells
          ? { allyDestinationCells: descriptor.allyDestinationCells }
          : {}),
        ...(descriptor.allyStrikeLocation
          ? { allyStrikeLocation: descriptor.allyStrikeLocation }
          : {}),
        ...(descriptor.targetSiteInstanceId
          ? { targetSiteInstanceId: descriptor.targetSiteInstanceId }
          : {}),
        ...(descriptor.cemeteryMinionInstanceId
          ? { cemeteryMinionInstanceId: descriptor.cemeteryMinionInstanceId }
          : {}),
        ...(descriptor.discardSiteInstanceId
          ? { discardSiteInstanceId: descriptor.discardSiteInstanceId }
          : {}),
        ...(descriptor.temptedDestination
          ? { temptedDestination: descriptor.temptedDestination }
          : {}),
        ...(descriptor.temptedEnemy
          ? {
            temptedEnemyInstanceId: descriptor.temptedEnemy.instanceId,
            temptedEnemySeat: descriptor.temptedEnemy.seat,
          }
          : {}),
      },
      type: 'magic-cast',
    } as const;
    const castOutcomes: readonly GameOutcome[] = [
      ...(discardedSite
        ? [{
          payload: {
            cardId: discardedSite.cardId,
            instanceId: discardedSite.instanceId,
            owner: discardedSite.owner,
            seat,
            sourceInstanceId: card.instanceId,
            zone: 'atlas',
          },
          type: 'card-discarded',
        } as const]
        : []),
      castOutcome,
      ...interaction.outcomes,
    ];
    const resolved = {
      payload: { cardId: card.cardId, instanceId: card.instanceId, owner: card.owner },
      type: 'magic-resolved',
    } as const;
    if (definition.summonRandomMinionFromAnyCemetery === true) {
      const candidates = cemeteryMinionCandidates(castState);
      if (candidates.length === 0) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const selected = resolveRandomOutcome(
        castState,
        candidates.map(({ instanceId }) => instanceId),
        'magic_random_dead_minion',
        'dead_minion_instance_candidate',
        forcedRandomOutcomeInstanceId,
      );
      const deadMinion = candidates.find(({ instanceId }) =>
        instanceId === selected.outcomeInstanceId)!;
      const deadDefinition = cardDefinition(castState, deadMinion.cardId);
      if (deadDefinition.cardType !== 'minion') {
        throw new Error('unreachable non-minion Raise Dead candidate');
      }
      const randomizedState = deepFreeze({ ...castState, engine: selected.engine });
      const selectedOutcome: GameOutcome = {
        payload: {
          cardId: deadMinion.cardId,
          instanceId: deadMinion.instanceId,
          owner: deadMinion.owner,
          seat,
          sourceInstanceId: card.instanceId,
        },
        type: 'dead-minion-selected',
      };
      if (minionSummonLocations(randomizedState, seat, deadDefinition, true).length === 0) {
        return [
          withStateVersion(randomizedState, {}),
          [
            ...castOutcomes,
            selectedOutcome,
            {
              payload: {
                instanceId: deadMinion.instanceId,
                owner: deadMinion.owner,
                reason: 'no-legal-location',
                seat,
                sourceInstanceId: card.instanceId,
              },
              type: 'minion-summon-failed',
            },
            resolved,
          ],
          selected.randomDraws,
        ];
      }
      return [
        withStateVersion(randomizedState, {
          pendingCemeterySummon: {
            cardInstanceId: deadMinion.instanceId,
            cardOwner: deadMinion.owner,
            casterInstanceId: descriptor.casterInstanceId,
            seat,
            sourceMagicInstanceId: card.instanceId,
          },
          phase: 'cemetery-summon',
        }),
        [...castOutcomes, selectedOutcome],
        selected.randomDraws,
      ];
    }
    if (definition.healController !== undefined) {
      const controller = castState.players[seat];
      const avatarDefinition = cardDefinition(castState, controller.avatar.card.cardId);
      if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
      const [healed, amount] = healAvatar(
        controller,
        avatarDefinition.life,
        definition.healController,
      );
      return [
        withStateVersion(castState, { players: replacePlayer(castState, seat, healed) }),
        [
          ...castOutcomes,
          ...(amount > 0
            ? [{
              payload: {
                amount,
                attemptedAmount: definition.healController,
                life: healed.avatar.life,
                seat,
                sourceInstanceId: card.instanceId,
              },
              type: 'avatar-healed' as const,
            }]
            : []),
          resolved,
        ],
        [],
      ];
    }
    if (definition.returnMinionFromOwnCemetery === true) {
      if (!descriptor.cemeteryMinionInstanceId) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const returningPlayer = castState.players[seat];
      const selected = returningPlayer.cemetery.find(({ instanceId }) =>
        instanceId === descriptor.cemeteryMinionInstanceId);
      if (!selected
        || selected.owner !== seat
        || cardDefinition(castState, selected.cardId).cardType !== 'minion') {
        throw new Error('unreachable Rescue choice');
      }
      const returnedPlayer = deepFreeze({
        ...returningPlayer,
        cemetery: returningPlayer.cemetery.filter(({ instanceId }) =>
          instanceId !== selected.instanceId),
        hand: {
          ...returningPlayer.hand,
          spellbook: [...returningPlayer.hand.spellbook, selected],
        },
      });
      return [
        withStateVersion(castState, {
          players: replacePlayer(castState, seat, returnedPlayer),
        }),
        [
          ...castOutcomes,
          {
            payload: {
              cardId: selected.cardId,
              instanceId: selected.instanceId,
              owner: selected.owner,
              seat,
              sourceInstanceId: card.instanceId,
            },
            type: 'minion-returned-to-hand',
          },
          resolved,
        ],
        [],
      ];
    }
    if (definition.summonTokenToEachControlledSiteBorderingEnemySite !== undefined) {
      const tokenCardId = definition.summonTokenToEachControlledSiteBorderingEnemySite;
      const enemySeat = otherSeat(seat);
      const cells = controlledSiteCells(castState, seat).filter((cell) =>
        borderingCells(cell).some((borderingCell) =>
          castState.realm.sites[borderingCell]?.controller === enemySeat));
      const tokens = cells.map((cell, ordinal) =>
        tokenUnit(castState, seat, tokenCardId, card.instanceId, cell, ordinal));
      return [
        withStateVersion(castState, {
          realm: { ...castState.realm, units: [...castState.realm.units, ...tokens] },
        }),
        [
          ...castOutcomes,
          ...tokens.map((token) => ({
            payload: {
              cardId: token.cardId,
              cell: token.location,
              instanceId: token.instanceId,
              owner: token.owner,
              seat: token.controller,
              sourceInstanceId: card.instanceId,
              token: true,
            },
            type: 'minion-summoned' as const,
          })),
          resolved,
        ],
        [],
      ];
    }
    if (definition.grantChargeToAllyThisTurn === true) {
      if (!descriptor.ally) throw new Error('unreachable Charge cast');
      if (descriptor.ally.kind === 'avatar') {
        return [
          withStateVersion(castState, {}),
          [
            ...castOutcomes,
            {
              payload: {
                instanceId: descriptor.ally.instanceId,
                seat,
                sourceInstanceId: card.instanceId,
              },
              type: 'charge-granted',
            },
            resolved,
          ],
          [],
        ];
      }
      const allyIndex = castState.realm.units.findIndex(({ controller, instanceId }) =>
        controller === seat && instanceId === descriptor.ally!.instanceId);
      const ally = castState.realm.units[allyIndex];
      if (!ally) throw new Error('unreachable Charge ally');
      const charged = deepFreeze({
        ...ally,
        temporaryChargeSources: [...(ally.temporaryChargeSources ?? []), card.instanceId],
      });
      const chargedState = deepFreeze({
        ...castState,
        realm: {
          ...castState.realm,
          units: castState.realm.units.map((unit, index) => index === allyIndex ? charged : unit),
        },
      });
      return [
        withStateVersion(chargedState, {}),
        [
          ...castOutcomes,
          {
            payload: {
              instanceId: ally.instanceId,
              seat,
              sourceInstanceId: card.instanceId,
            },
            type: 'charge-granted',
          },
          resolved,
        ],
        [],
      ];
    }
    if (definition.grantPowerToAllyThisTurn === 2) {
      if (!descriptor.ally) throw new Error('unreachable Overpower cast');
      const grantOutcome: GameOutcome = {
        payload: {
          amount: definition.grantPowerToAllyThisTurn,
          instanceId: descriptor.ally.instanceId,
          seat,
          sourceInstanceId: card.instanceId,
        },
        type: 'power-granted',
      };
      if (descriptor.ally.kind === 'avatar') {
        const allyPlayer = castState.players[seat];
        const poweredState = deepFreeze({
          ...castState,
          players: replacePlayer(castState, seat, deepFreeze({
            ...allyPlayer,
            avatar: {
              ...allyPlayer.avatar,
              temporaryPowerSources: [
                ...(allyPlayer.avatar.temporaryPowerSources ?? []),
                card.instanceId,
              ],
            },
          })),
        });
        return [
          withStateVersion(poweredState, {}),
          [...castOutcomes, grantOutcome, resolved],
          [],
        ];
      }
      const allyIndex = castState.realm.units.findIndex(({ controller, instanceId }) =>
        controller === seat && instanceId === descriptor.ally!.instanceId);
      const ally = castState.realm.units[allyIndex];
      if (!ally) throw new Error('unreachable Overpower ally');
      const poweredState = deepFreeze({
        ...castState,
        realm: {
          ...castState.realm,
          units: castState.realm.units.map((unit, index) => index === allyIndex
            ? deepFreeze({
              ...ally,
              temporaryPowerSources: [...(ally.temporaryPowerSources ?? []), card.instanceId],
            })
            : unit),
        },
      });
      return [
        withStateVersion(poweredState, {}),
        [...castOutcomes, grantOutcome, resolved],
        [],
      ];
    }
    if (definition.leapAttackAlly === true) {
      if (!descriptor.ally || !descriptor.allyDestination) {
        throw new Error('unreachable Leap Attack cast');
      }
      const startingStatus = unitStatus(castState, descriptor.ally);
      const from: GameLocation = {
        cell: startingStatus.location,
        region: startingStatus.region,
      };
      const declaredPath = sameLocation(from, descriptor.allyDestination)
        ? [from]
        : [from, descriptor.allyDestination];
      const path = resolveDeclaredPath(castState, descriptor.ally, declaredPath, false);
      const steppedTo = path.path.at(-1) ?? from;
      const stepOutcomes: readonly GameOutcome[] = path.path.length > 1
        ? [{
          payload: {
            from,
            instanceId: descriptor.ally.instanceId,
            seat: descriptor.ally.seat,
            sourceInstanceId: card.instanceId,
            steps: path.path.length - 1,
            to: steppedTo,
          },
          type: 'unit-stepped',
        }]
        : [];
      const outcomesBeforeStrike = [...castOutcomes, ...stepOutcomes, ...path.outcomes];
      if (path.state.pendingDeathrites) {
        // ponytail: serialize Leap Attack's destination strike when an actual-card
        // scenario first combines it with ordered movement deaths.
        throw new Error('unsupported Deathrite ordering during Leap Attack continuation');
      }
      const moverRemoved = path.removals.some(({ instanceId }) =>
        instanceId === descriptor.ally!.instanceId);
      if (moverRemoved || path.state.terminal.status === 'finished') {
        const terminalIndex = outcomesBeforeStrike.findIndex(({ type }) => type === 'game-ended');
        return [
          withStateVersion(path.state, {}),
          terminalIndex < 0
            ? [...outcomesBeforeStrike, resolved]
            : [
              ...outcomesBeforeStrike.slice(0, terminalIndex),
              resolved,
              ...outcomesBeforeStrike.slice(terminalIndex),
            ],
          [],
        ];
      }
      const striker = unitStatus(path.state, descriptor.ally);
      const strikeLocation = descriptor.allyStrikeLocation ?? steppedTo;
      const enemies = striker.region === strikeLocation.region
        && striker.occupiedCells.includes(strikeLocation.cell)
        ? unitRefs(path.state, otherSeat(seat))
        .filter((enemy) => {
          const status = unitStatus(path.state, enemy);
          return status.region === striker.region
            && status.occupiedCells.includes(strikeLocation.cell);
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId))
        : [];
      if (striker.disabled || enemies.length === 0) {
        return [
          withStateVersion(path.state, {}),
          [...outcomesBeforeStrike, resolved],
          [],
        ];
      }
      const amount = strikeDamage(path.state, descriptor.ally);
      const pending: PendingCombat = deepFreeze({
        allocations: enemies.map(({ instanceId }) => ({
          amount,
          targetInstanceId: instanceId,
        })),
        attacker: descriptor.ally,
        attackingSeat: seat,
        cell: strikeLocation.cell,
        combatants: enemies,
        defenders: [],
        originalTarget: null,
        ...(striker.region === 'surface' ? {} : { region: striker.region }),
        targetRemoved: false,
      });
      const allocationOutcomes: readonly GameOutcome[] = enemies.map(({ instanceId }) => ({
        payload: {
          amount,
          strikerInstanceId: descriptor.ally!.instanceId,
          targetInstanceId: instanceId,
        },
        type: 'strike-damage-allocated',
      }));
      const [struck, strikeOutcomes, randomDraws] = resolveFightWindow(
        path.state,
        pending,
        [...outcomesBeforeStrike, ...allocationOutcomes],
        'attacker-unit',
        true,
        false,
        [descriptor.ally],
      );
      const terminalIndex = strikeOutcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(struck, {}),
        terminalIndex < 0
          ? [...strikeOutcomes, resolved]
          : [
            ...strikeOutcomes.slice(0, terminalIndex),
            resolved,
            ...strikeOutcomes.slice(terminalIndex),
          ],
        randomDraws,
      ];
    }
    if (definition.fightAllyWithAdjacentEnemy === true) {
      if (!descriptor.ally || !descriptor.target) throw new Error('unreachable Duel cast');
      const allyStatus = unitStatus(castState, descriptor.ally);
      if (descriptor.target.kind === 'minion') {
        const targetIndex = castState.realm.units.findIndex(({ controller, instanceId }) =>
          controller === descriptor.target!.seat && instanceId === descriptor.target!.instanceId);
        const target = castState.realm.units[targetIndex];
        if (!target) throw new Error('unreachable Duel target');
        if (target.warded) {
          const wardedState = deepFreeze({
            ...castState,
            realm: {
              ...castState.realm,
              units: castState.realm.units.map((unit, index) => index === targetIndex
                ? deepFreeze({ ...unit, warded: false })
                : unit),
            },
          });
          return [
            withStateVersion(wardedState, {}),
            [
              ...castOutcomes,
              { payload: { instanceId: target.instanceId, seat: target.controller }, type: 'ward-broken' },
              resolved,
            ],
            [],
          ];
        }
      }
      const amount = strikeDamage(castState, descriptor.ally);
      const pending: PendingCombat = deepFreeze({
        allocations: [{ amount, targetInstanceId: descriptor.target.instanceId }],
        attacker: descriptor.ally,
        attackingSeat: seat,
        cell: allyStatus.location,
        combatants: [descriptor.target],
        defenders: [],
        originalTarget: descriptor.target,
        ...(allyStatus.region === 'surface' ? {} : { region: allyStatus.region }),
        targetRemoved: false,
      });
      const [foughtState, fightOutcomes, randomDraws] = beginFight(
        castState,
        pending,
        [descriptor.target],
        [
          ...castOutcomes,
        ],
      );
      const terminalIndex = fightOutcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        foughtState,
        terminalIndex < 0
          ? [...fightOutcomes, resolved]
          : [
            ...fightOutcomes.slice(0, terminalIndex),
            resolved,
            ...fightOutcomes.slice(terminalIndex),
          ],
        randomDraws,
      ];
    }
    if (definition.lureEnemyMinionOneStepCloser === true) {
      if (!descriptor.ally || !descriptor.temptedEnemy || !descriptor.temptedDestination) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const enemyStatus = unitStatus(castState, descriptor.temptedEnemy);
      const from: GameLocation = { cell: enemyStatus.location, region: enemyStatus.region };
      const to = descriptor.temptedDestination;
      const path = resolveDeclaredPath(castState, descriptor.temptedEnemy, [from, to], false);
      const lured: GameOutcome = {
        payload: {
          allyInstanceId: descriptor.ally.instanceId,
          from,
          path: path.path,
          seat: descriptor.temptedEnemy.seat,
          sourceInstanceId: card.instanceId,
          steps: path.path.length - 1,
          targetInstanceId: descriptor.temptedEnemy.instanceId,
          to,
        },
        type: 'unit-lured',
      };
      const outcomes = [...castOutcomes, lured, ...path.outcomes];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(path.state, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
      ];
    }
    if (definition.gainControlOfTargetNearbyMinion === true) {
      if (descriptor.target?.kind !== 'minion') throw new Error('unreachable Mesmerism cast');
      const targetIndex = castState.realm.units.findIndex(({ instanceId, controller }) =>
        instanceId === descriptor.target!.instanceId && controller === descriptor.target!.seat);
      const target = castState.realm.units[targetIndex];
      if (!target) throw new Error('unreachable Mesmerism target');
      if (target.controller === seat) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      if (target.warded) {
        const wardedState = deepFreeze({
          ...castState,
          realm: {
            ...castState.realm,
            units: castState.realm.units.map((unit, index) => index === targetIndex
              ? deepFreeze({ ...unit, warded: false })
              : unit),
          },
        });
        return [
          withStateVersion(wardedState, {}),
          [
            ...castOutcomes,
            { payload: { instanceId: target.instanceId, seat: target.controller }, type: 'ward-broken' },
            resolved,
          ],
          [],
        ];
      }
      const controlledState = deepFreeze({
        ...castState,
        realm: {
          ...castState.realm,
          ...(castState.realm.artifacts
            ? {
              artifacts: castState.realm.artifacts.map((artifact) =>
                'bearer' in artifact
                  && artifact.bearer.kind === 'minion'
                  && artifact.bearer.instanceId === target.instanceId
                  ? deepFreeze({
                    ...artifact,
                    bearer: deepFreeze({ ...artifact.bearer, seat }),
                  })
                  : artifact),
            }
            : {}),
          units: castState.realm.units.map((unit, index) => index === targetIndex
            ? deepFreeze({ ...unit, controller: seat })
            : unit),
        },
      });
      const powerSettlement = settleStaticPowerDeaths(controlledState);
      const outcomes: readonly GameOutcome[] = [
        ...castOutcomes,
        {
          payload: {
            fromSeat: target.controller,
            instanceId: target.instanceId,
            seat,
            sourceInstanceId: card.instanceId,
          },
          type: 'minion-control-changed',
        },
        ...powerSettlement.outcomes,
      ];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(powerSettlement.state, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
      ];
    }
    if (definition.killTargetWoundedMinion === true) {
      if (descriptor.target?.kind !== 'minion') throw new Error('unreachable Fatality cast');
      const targetIndex = castState.realm.units.findIndex(({ instanceId, controller }) =>
        instanceId === descriptor.target!.instanceId && controller === descriptor.target!.seat);
      const target = castState.realm.units[targetIndex];
      if (!target) throw new Error('unreachable Fatality target');
      if (target.warded) {
        const wardedState = deepFreeze({
          ...castState,
          realm: {
            ...castState.realm,
            units: castState.realm.units.map((unit, index) => index === targetIndex
              ? deepFreeze({ ...unit, warded: false })
              : unit),
          },
        });
        return [
          withStateVersion(wardedState, {}),
          [
            ...castOutcomes,
            { payload: { instanceId: target.instanceId, seat: target.controller }, type: 'ward-broken' },
            resolved,
          ],
          [],
        ];
      }
      const deathResolution = resolveMinionDeaths(
        castState,
        castState.players,
        castState.realm.units,
        [target],
        new Set<GameSeat>(),
      );
      const killedState = deepFreeze({
        ...castState,
        ...(deathResolution.terminal.status === 'finished'
          ? { pendingCombat: null, phase: 'terminal' as const }
          : {}),
        players: deathResolution.players,
        ...(deathResolution.pendingDeathrites
          ? { pendingDeathrites: deathResolution.pendingDeathrites }
          : {}),
        realm: {
          ...castState.realm,
          ...(deathResolution.artifacts ? { artifacts: deathResolution.artifacts } : {}),
          units: deathResolution.units,
        },
        terminal: deathResolution.terminal,
      });
      const outcomes: readonly GameOutcome[] = [
        ...castOutcomes,
        {
          payload: {
            cardId: target.cardId,
            instanceId: target.instanceId,
            owner: target.owner,
            seat: target.controller,
            sourceInstanceId: card.instanceId,
          },
          type: 'minion-killed',
        },
        ...deathResolution.outcomes,
      ];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(killedState, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
      ];
    }
    if (definition.disableTargetNearbyMinionUntilNextTurn === true) {
      if (descriptor.target?.kind !== 'minion') throw new Error('unreachable Freeze cast');
      const targetIndex = castState.realm.units.findIndex(({ instanceId, controller }) =>
        instanceId === descriptor.target!.instanceId && controller === descriptor.target!.seat);
      const target = castState.realm.units[targetIndex];
      if (!target) throw new Error('unreachable Freeze target');
      if (target.warded && target.controller !== seat) {
        const wardedState = deepFreeze({
          ...castState,
          realm: {
            ...castState.realm,
            units: castState.realm.units.map((unit, index) => index === targetIndex
              ? deepFreeze({ ...unit, warded: false })
              : unit),
          },
        });
        return [
          withStateVersion(wardedState, {}),
          [
            ...castOutcomes,
            { payload: { instanceId: target.instanceId, seat: target.controller }, type: 'ward-broken' },
            resolved,
          ],
          [],
        ];
      }
      const effect: DisableEffect = deepFreeze({
        expiresAtSeat: seat,
        sourceInstanceId: card.instanceId,
      });
      const disabledUnit = deepFreeze({
        ...target,
        disableEffects: [...(target.disableEffects ?? []), effect],
        stealthed: false,
        warded: false,
      });
      const disabledState = deepFreeze({
        ...castState,
        realm: {
          ...castState.realm,
          units: castState.realm.units.map((unit, index) => index === targetIndex ? disabledUnit : unit),
        },
      });
      const settlement = settleRegionOccupancy(disabledState);
      const outcomes: readonly GameOutcome[] = [
        ...castOutcomes,
        {
          payload: {
            expiresAtSeat: seat,
            instanceId: target.instanceId,
            seat: target.controller,
            sourceInstanceId: card.instanceId,
            stealthRemoved: target.stealthed,
            wardRemoved: target.warded,
          },
          type: 'minion-disabled',
        },
        ...settlement.outcomes,
      ];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(settlement.state, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
      ];
    }
    if (definition.burrowAllMinionsAndArtifactsAtTargetLandSite === true) {
      if (!descriptor.targetLocation
        || descriptor.targetLocation.region !== 'surface'
        || !descriptor.targetSiteInstanceId) {
        throw new Error('unreachable Cave-In target');
      }
      const cell = descriptor.targetLocation.cell;
      const site = castState.realm.sites[cell];
      if (!site
        || site.instanceId !== descriptor.targetSiteInstanceId
        || isWaterSite(castState, cell)) {
        throw new Error('unreachable Cave-In Land Site');
      }
      const minions = castState.realm.units
        .filter((unit) => unit.region === 'surface'
          && unitOccupiedCells(unit).includes(cell)
          && unitOccupiedCells(unit).every((occupiedCell) =>
            castState.realm.sites[occupiedCell] !== undefined
            && !isWaterSite(castState, occupiedCell)))
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      const minionIds = new Set(minions.map(({ instanceId }) => instanceId));
      const artifacts = (castState.realm.artifacts ?? []).filter((artifact) => {
        const location = artifactLocation(castState, artifact);
        return location.cell === cell && location.region === 'surface';
      }).sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      const artifactIds = new Set(artifacts.map(({ instanceId }) => instanceId));
      const burrowedState = deepFreeze({
        ...castState,
        realm: {
          ...castState.realm,
          ...(castState.realm.artifacts
            ? {
              artifacts: castState.realm.artifacts.map((artifact) => {
                if (!artifactIds.has(artifact.instanceId)
                  || 'bearer' in artifact && minionIds.has(artifact.bearer.instanceId)) return artifact;
                return deepFreeze({
                  cardId: artifact.cardId,
                  instanceId: artifact.instanceId,
                  location: cell,
                  owner: artifact.owner,
                  region: 'underground' as const,
                  source: artifact.source,
                });
              }),
            }
            : {}),
          units: castState.realm.units.map((unit) => minionIds.has(unit.instanceId)
            ? deepFreeze({ ...unit, region: 'underground' as const })
            : unit),
        },
      });
      const burrowOutcomes: GameOutcome[] = [
        ...minions.map((unit) => ({
          instanceId: unit.instanceId,
          outcome: {
            payload: {
              cell,
              instanceId: unit.instanceId,
              seat: unit.controller,
              sourceInstanceId: card.instanceId,
            },
            type: 'minion-burrowed' as const,
          },
        })),
        ...artifacts.map((artifact) => ({
          instanceId: artifact.instanceId,
          outcome: {
            payload: {
              cell,
              instanceId: artifact.instanceId,
              owner: artifact.owner,
              sourceInstanceId: card.instanceId,
            },
            type: 'artifact-burrowed' as const,
          },
        })),
      ].sort((left, right) => left.instanceId.localeCompare(right.instanceId))
        .map(({ outcome }) => outcome);
      const settlement = settleRegionOccupancy(burrowedState);
      const outcomes = [...castOutcomes, ...burrowOutcomes, ...settlement.outcomes];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(settlement.state, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
      ];
    }
    if (definition.burrowTargetMinionOrArtifact === true
      && descriptor.targetArtifactInstanceId !== undefined) {
      const artifactIndex = (castState.realm.artifacts ?? []).findIndex(({ instanceId }) =>
        instanceId === descriptor.targetArtifactInstanceId);
      const artifact = castState.realm.artifacts?.[artifactIndex];
      if (!artifact) throw new Error('unreachable Bury Artifact target');
      const location = artifactLocation(castState, artifact);
      const canMove = location.region === 'surface'
        && castState.realm.sites[location.cell] !== undefined
        && !isWaterSite(castState, location.cell);
      if (!canMove) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const movedState = deepFreeze({
        ...castState,
        realm: {
          ...castState.realm,
          artifacts: castState.realm.artifacts!.map((candidate) =>
            candidate.instanceId === artifact.instanceId
              ? deepFreeze({
                cardId: candidate.cardId,
                instanceId: candidate.instanceId,
                location: location.cell,
                owner: candidate.owner,
                region: 'underground' as const,
                source: candidate.source,
              })
              : candidate),
        },
      });
      return [
        withStateVersion(movedState, {}),
        [
          ...castOutcomes,
          {
            payload: {
              cell: location.cell,
              instanceId: artifact.instanceId,
              owner: artifact.owner,
              sourceInstanceId: card.instanceId,
            },
            type: 'artifact-burrowed',
          },
          resolved,
        ],
        [],
      ];
    }
    if (definition.burrowTargetMinionOrArtifact === true
      || definition.submergeTargetMinion === true) {
      const submerge = definition.submergeTargetMinion === true;
      if (descriptor.target?.kind !== 'minion') throw new Error('unreachable subsurface Magic cast');
      const targetIndex = castState.realm.units.findIndex(({ instanceId, controller }) =>
        instanceId === descriptor.target!.instanceId && controller === descriptor.target!.seat);
      const target = castState.realm.units[targetIndex];
      if (!target) throw new Error('unreachable subsurface Magic target');
      if (target.warded && target.controller !== seat) {
        const wardedState = deepFreeze({
          ...castState,
          realm: {
            ...castState.realm,
            units: castState.realm.units.map((unit, index) => index === targetIndex
              ? deepFreeze({ ...unit, warded: false })
              : unit),
          },
        });
        return [
          withStateVersion(wardedState, {}),
          [
            ...castOutcomes,
            { payload: { instanceId: target.instanceId, seat: target.controller }, type: 'ward-broken' },
            resolved,
          ],
          [],
        ];
      }
      const targetCells = unitOccupiedCells(target);
      const canMove = target.region === 'surface'
        && targetCells.every((cell) =>
          castState.realm.sites[cell] !== undefined
          && isWaterSite(castState, cell) === submerge);
      if (!canMove) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const movedUnit = deepFreeze({
        ...target,
        region: submerge ? 'underwater' as const : 'underground' as const,
      });
      const movedState = deepFreeze({
        ...castState,
        realm: {
          ...castState.realm,
          units: castState.realm.units.map((unit, index) => index === targetIndex ? movedUnit : unit),
        },
      });
      const movedOutcome: GameOutcome = {
        payload: {
          cell: target.location,
          instanceId: target.instanceId,
          seat: target.controller,
          sourceInstanceId: card.instanceId,
        },
        type: submerge ? 'minion-submerged' : 'minion-burrowed',
      };
      const settlement = settleRegionOccupancy(movedState);
      const outcomes = [...castOutcomes, movedOutcome, ...settlement.outcomes];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(settlement.state, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
      ];
    }
    if (definition.teleportAllyToTargetSite === true
      || definition.teleportNearbyAllyThenDrawCard === true) {
      const blink = definition.teleportNearbyAllyThenDrawCard === true;
      if (!descriptor.ally
        || !descriptor.targetLocation
        || (!blink && !descriptor.targetSiteInstanceId)
        || (blink && !descriptor.drawZone)) {
        throw new Error('unreachable Teleport cast');
      }
      const status = unitStatus(castState, descriptor.ally);
      const from: GameLocation = { cell: status.location, region: status.region };
      const destination: GameLocation = descriptor.allyDestinationCells
        ? { cell: descriptor.allyDestinationCells[0], region: descriptor.targetLocation.region }
        : descriptor.targetLocation;
      const destinationCells = descriptor.allyDestinationCells ?? [destination.cell];
      const stays = status.region === destination.region
        && status.occupiedCells.length === destinationCells.length
        && status.occupiedCells.every((cell, index) => cell === destinationCells[index]);
      let effectState = castState;
      const effectOutcomes: GameOutcome[] = [...castOutcomes];
      if (!stays) {
        const moved = moveUnit(castState, descriptor.ally, destination, false);
        const teleportedState = deepFreeze({
          ...castState,
          players: moved.players,
          realm: moved.realm,
        });
        effectOutcomes.push({
          payload: {
            from,
            seat: descriptor.ally.seat,
            sourceInstanceId: card.instanceId,
            targetInstanceId: descriptor.ally.instanceId,
            ...(descriptor.targetSiteInstanceId
              ? { targetSiteInstanceId: descriptor.targetSiteInstanceId }
              : {}),
            ...(descriptor.allyDestinationCells
              ? { cells: descriptor.allyDestinationCells }
              : {}),
            to: destination,
          },
          type: 'unit-teleported',
        });
        const regionSettlement = settleRegionOccupancy(teleportedState);
        const powerSettlement = regionSettlement.state.pendingDeathrites
          ? { outcomes: [] as readonly GameOutcome[], state: regionSettlement.state }
          : settleStaticPowerDeaths(regionSettlement.state);
        effectState = powerSettlement.state;
        effectOutcomes.push(...regionSettlement.outcomes, ...powerSettlement.outcomes);
      }
      const terminalIndex = effectOutcomes.findIndex(({ type }) => type === 'game-ended');
      if (!blink || effectState.terminal.status === 'finished') {
        return [
          withStateVersion(effectState, {}),
          terminalIndex < 0
            ? [...effectOutcomes, resolved]
            : [
              ...effectOutcomes.slice(0, terminalIndex),
              resolved,
              ...effectOutcomes.slice(terminalIndex),
            ],
          [],
        ];
      }
      if (effectState.pendingDeathrites) {
        return [
          withStateVersion(effectState, {
            pendingDeathrites: deepFreeze({
              ...effectState.pendingDeathrites,
              blinkContinuation: {
                cardId: card.cardId,
                instanceId: card.instanceId,
                owner: card.owner,
                seat,
                zone: descriptor.drawZone!,
              },
            }),
          }),
          effectOutcomes,
          [],
        ];
      }
      const drawingPlayer = effectState.players[seat];
      const zone = descriptor.drawZone!;
      const [drawn, ...remaining] = drawingPlayer[zone];
      if (!drawn) {
        const winner = otherSeat(seat);
        return [
          withStateVersion(effectState, {
            pendingCombat: null,
            phase: 'terminal',
            terminal: { loser: seat, reason: 'deck_empty', status: 'finished', winner },
          }),
          [
            ...effectOutcomes,
            resolved,
            { payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' },
          ],
          [],
        ];
      }
      const updatedPlayer = deepFreeze({
        ...drawingPlayer,
        [zone]: remaining,
        hand: {
          ...drawingPlayer.hand,
          [zone]: [...drawingPlayer.hand[zone], drawn],
        },
      });
      return [
        withStateVersion(effectState, {
          players: replacePlayer(effectState, seat, updatedPlayer),
        }),
        [
          ...effectOutcomes,
          {
            payload: { seat, sourceInstanceId: card.instanceId },
            type: zone === 'atlas' ? 'site-drawn' : 'spell-drawn',
          },
          resolved,
        ],
        [],
      ];
    }
    if (definition.damageUnitsAboveAndBelowTargetSiteByManhattanDistance !== undefined) {
      if (!descriptor.targetLocation || !descriptor.targetSiteInstanceId) {
        throw new Error('unreachable Craterize target');
      }
      const cell = descriptor.targetLocation.cell;
      const targetSite = castState.realm.sites[cell];
      if (!targetSite || targetSite.instanceId !== descriptor.targetSiteInstanceId) {
        throw new Error('unreachable Craterize site');
      }
      const damageGrid = definition.damageUnitsAboveAndBelowTargetSiteByManhattanDistance;
      const targets = (['north', 'south'] as const)
        .flatMap((targetSeat) => unitRefs(castState, targetSeat))
        .map((target) => {
          const status = unitStatus(castState, target);
          const amount = status.region === 'void'
            ? 0
            : status.occupiedCells.reduce((total, occupiedCell) => {
              const fileDistance = Math.abs(occupiedCell.charCodeAt(0) - cell.charCodeAt(0));
              const rankDistance = Math.abs(Number(occupiedCell[1]) - Number(cell[1]));
              return fileDistance <= 2 && rankDistance <= 2
                ? total + damageGrid[fileDistance + rankDistance]!
                : total;
            }, 0);
          return { amount, target };
        })
        .filter(({ amount }) => amount > 0)
        .sort((left, right) => left.target.instanceId.localeCompare(right.target.instanceId));
      const targetProtected = !isRubble(targetSite)
        && siteCannotBeMovedDestroyedOrModified(castState, targetSite);
      const destroyed = isRubble(targetSite) || targetProtected
        ? []
        : [{ cell, site: targetSite }];
      const pending: PendingCombat = deepFreeze({
        allocations: targets.map(({ amount, target }) => ({
          amount,
          targetInstanceId: target.instanceId,
        })),
        attacker: caster,
        attackingSeat: seat,
        cell,
        combatants: targets.map(({ target }) => target),
        defenders: [],
        originalTarget: null,
        targetRemoved: false,
      });
      const [damaged, outcomes, randomDraws] = resolveFightWindow(
        castState,
        pending,
        [
          ...castOutcomes,
          {
            payload: {
              cell,
              instanceId: targetSite.instanceId,
              ...(!isRubble(targetSite) ? { owner: targetSite.owner } : {}),
              sourceInstanceId: card.instanceId,
            },
            type: targetProtected ? 'site-destruction-prevented' : 'site-destroyed',
          },
          ...targets.map(({ amount, target }) => ({
            payload: {
              amount,
              sourceInstanceId: card.instanceId,
              targetInstanceId: target.instanceId,
            },
            type: 'magic-damage-allocated',
          })),
        ],
        'non-unit',
        true,
        false,
        [caster],
        false,
        false,
        destroyed.length > 0 ? { destroyed, sourceInstanceId: card.instanceId } : undefined,
      );
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(damaged, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        randomDraws,
      ];
    }
    if (definition.damageChainNearbyUnits === true) {
      const targets = descriptor.targets;
      if (!targets?.length) throw new Error('unreachable chained Magic cast');
      return resolveChainMagicDamage(
        castState,
        seat,
        caster,
        targets,
        card.instanceId,
        castOutcomes,
        resolved,
      );
    }
    if (definition.damageEachAbovegroundMinion === 1) {
      const amount = definition.damageEachAbovegroundMinion;
      const targets = (['north', 'south'] as const)
        .flatMap((targetSeat) => unitRefs(castState, targetSeat))
        .filter((target) => target.kind === 'minion' && unitStatus(castState, target).region === 'surface')
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      if (targets.length === 0) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const pending: PendingCombat = deepFreeze({
        allocations: targets.map(({ instanceId }) => ({
          amount,
          targetInstanceId: instanceId,
        })),
        attacker: caster,
        attackingSeat: seat,
        cell: unitStatus(castState, caster).location,
        combatants: targets,
        defenders: [],
        originalTarget: null,
        targetRemoved: false,
      });
      const allocationOutcomes: readonly GameOutcome[] = targets.map(({ instanceId }) => ({
        payload: {
          amount,
          sourceInstanceId: card.instanceId,
          targetInstanceId: instanceId,
        },
        type: 'magic-damage-allocated',
      }));
      const [damaged, outcomes, randomDraws] = resolveFightWindow(
        castState,
        pending,
        [...castOutcomes, ...allocationOutcomes],
        'non-unit',
        true,
        false,
        [caster],
        false,
      );
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(damaged, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        randomDraws,
      ];
    }
    if (definition.damageEachUnitAtLocationWithinTwoSteps !== undefined) {
      if (!descriptor.targetLocation) throw new Error('unreachable area-location Magic cast');
      const targets = (['north', 'south'] as const)
        .flatMap((targetSeat) => unitRefs(castState, targetSeat))
        .filter((target) => {
          const status = unitStatus(castState, target);
          return status.occupiedCells.includes(descriptor.targetLocation!.cell)
            && status.region === descriptor.targetLocation!.region;
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      if (targets.length === 0) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const pending: PendingCombat = deepFreeze({
        allocations: targets.map(({ instanceId }) => ({
          amount: definition.damageEachUnitAtLocationWithinTwoSteps!,
          targetInstanceId: instanceId,
        })),
        attacker: caster,
        attackingSeat: seat,
        cell: descriptor.targetLocation.cell,
        combatants: targets,
        defenders: [],
        originalTarget: null,
        ...(descriptor.targetLocation.region === 'surface'
          ? {}
          : { region: descriptor.targetLocation.region }),
        targetRemoved: false,
      });
      const allocationOutcomes: readonly GameOutcome[] = targets.map(({ instanceId }) => ({
        payload: {
          amount: definition.damageEachUnitAtLocationWithinTwoSteps!,
          sourceInstanceId: card.instanceId,
          targetInstanceId: instanceId,
        },
        type: 'magic-damage-allocated',
      }));
      const [damaged, outcomes, randomDraws] = resolveFightWindow(
        castState,
        pending,
        [...castOutcomes, ...allocationOutcomes],
        'non-unit',
        true,
        false,
        [caster],
        false,
      );
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(damaged, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        randomDraws,
      ];
    }
    if (definition.damageRandomUnitAtLocation !== undefined) {
      if (!descriptor.targetLocation) throw new Error('unreachable random-location Magic cast');
      const candidates = randomUnitCandidatesAtLocation(castState, descriptor.targetLocation);
      if (candidates.length === 0) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const selected = resolveRandomOutcome(
        castState,
        candidates.map(({ instanceId }) => instanceId),
        'magic_random_unit_at_location',
        'unit_index_candidate',
        forcedRandomOutcomeInstanceId,
      );
      const targetRef = candidates.find(({ instanceId }) =>
        instanceId === selected.outcomeInstanceId)!;
      const randomizedState = deepFreeze({ ...castState, engine: selected.engine });
      const pending: PendingCombat = deepFreeze({
        allocations: [{ amount: definition.damageRandomUnitAtLocation, targetInstanceId: targetRef.instanceId }],
        attacker: caster,
        attackingSeat: seat,
        cell: descriptor.targetLocation.cell,
        combatants: [targetRef],
        defenders: [],
        originalTarget: targetRef,
        ...(descriptor.targetLocation.region === 'surface'
          ? {}
          : { region: descriptor.targetLocation.region }),
        targetRemoved: false,
      });
      const [damaged, outcomes, randomDraws] = resolveFightWindow(
        randomizedState,
        pending,
        [...castOutcomes, {
          payload: {
            amount: definition.damageRandomUnitAtLocation,
            sourceInstanceId: card.instanceId,
            targetInstanceId: targetRef.instanceId,
          },
          type: 'magic-damage-allocated',
        }],
        'non-unit',
        true,
        false,
        [caster],
        false,
      );
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(damaged, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [...selected.randomDraws, ...randomDraws],
      ];
    }
    if (definition.damageTargetUnit === undefined || descriptor.target === undefined) {
      throw new Error('unreachable targeted Magic cast');
    }
    const targetRef = descriptor.target;
    const target = unitStatus(castState, targetRef);
    const pending: PendingCombat = deepFreeze({
      allocations: [{ amount: definition.damageTargetUnit, targetInstanceId: targetRef.instanceId }],
      attacker: caster,
      attackingSeat: seat,
      cell: target.location,
      combatants: [targetRef],
      defenders: [],
      originalTarget: targetRef,
      ...(target.region === 'surface' ? {} : { region: target.region as 'underground' | 'underwater' | 'void' }),
      targetRemoved: false,
    });
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      castState,
      pending,
      [...castOutcomes, {
        payload: {
          amount: definition.damageTargetUnit,
          sourceInstanceId: card.instanceId,
          targetInstanceId: targetRef.instanceId,
          },
          type: 'magic-damage-allocated',
        }],
      'non-unit',
      true,
      false,
      [caster],
      false,
    );
    const survivingTarget = definition.untapTargetMinionAfterDamage === true
      && targetRef.kind === 'minion'
      ? damaged.realm.units.find(({ instanceId }) => instanceId === targetRef.instanceId)
      : undefined;
    const targetUntapped = Boolean(survivingTarget?.tapped);
    const resolvedState = targetUntapped
      ? deepFreeze({
        ...damaged,
        realm: {
          ...damaged.realm,
          units: damaged.realm.units.map((unit) => unit.instanceId === targetRef.instanceId
            ? deepFreeze({ ...unit, tapped: false })
            : unit),
        },
      })
      : damaged;
    const untapOutcomes: readonly GameOutcome[] = targetUntapped
      ? [{
        payload: {
          instanceId: targetRef.instanceId,
          seat: targetRef.seat,
          sourceInstanceId: card.instanceId,
        },
        type: 'minion-untapped',
      }]
      : [];
    const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
    return [
      withStateVersion(resolvedState, {}),
      terminalIndex < 0
        ? [...outcomes, ...untapOutcomes, resolved]
        : [
          ...outcomes.slice(0, terminalIndex),
          ...untapOutcomes,
          resolved,
          ...outcomes.slice(terminalIndex),
        ],
      randomDraws,
    ];
  }

  if (descriptor.kind === 'summon-minion') {
    const pendingCemeterySummon = state.phase === 'cemetery-summon'
      ? state.pendingCemeterySummon
      : undefined;
    const sourceMagic = pendingCemeterySummon
      ? (['north', 'south'] as const)
        .flatMap((owner) => state.players[owner].cemetery)
        .find(({ instanceId }) => instanceId === pendingCemeterySummon.sourceMagicInstanceId)
      : undefined;
    if (pendingCemeterySummon && !sourceMagic) {
      throw new Error('unreachable missing Raise Dead source Magic');
    }
    const completeSummon = (
      result: readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]],
    ): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] => {
      if (!sourceMagic) return result;
      const [summonedState, outcomes, randomDraws] = result;
      const resolved: GameOutcome = {
        payload: {
          cardId: sourceMagic.cardId,
          instanceId: sourceMagic.instanceId,
          owner: sourceMagic.owner,
        },
        type: 'magic-resolved',
      };
      const gameEndedIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        summonedState,
        gameEndedIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, gameEndedIndex), resolved, ...outcomes.slice(gameEndedIndex)],
        randomDraws,
      ];
    };
    const card = pendingCemeterySummon
      ? state.players[pendingCemeterySummon.cardOwner].cemetery.find(({ cardId, instanceId }) =>
        instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId)
      : player.hand.spellbook.find(({ cardId, instanceId }) =>
        instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    const definition = card && cardDefinition(state, card.cardId);
    const legal = (pendingCemeterySummon
      ? cemeterySummonDescriptors(state, seat)
      : summonDescriptors(state, seat)).some((candidate) => {
      if (candidate.kind !== 'summon-minion') return false;
      const candidateSacrifices = candidate.sacrificedMinionInstanceIds ?? [];
      const requestedSacrifices = descriptor.sacrificedMinionInstanceIds ?? [];
      return candidate.cardInstanceId === descriptor.cardInstanceId
        && candidate.casterInstanceId === descriptor.casterInstanceId
        && candidate.cell === descriptor.cell
        && ((candidate.cells === undefined && descriptor.cells === undefined)
          || candidate.cells !== undefined && descriptor.cells !== undefined
            && candidate.cells.every((cell, index) => cell === descriptor.cells![index]))
        && candidate.genesisDamageChoice === descriptor.genesisDamageChoice
        && candidate.genesisDamageTarget?.instanceId === descriptor.genesisDamageTarget?.instanceId
        && candidate.genesisDamageTarget?.kind === descriptor.genesisDamageTarget?.kind
        && candidate.genesisDamageTarget?.seat === descriptor.genesisDamageTarget?.seat
        && candidate.manaCost === descriptor.manaCost
        && candidate.paymentMode === descriptor.paymentMode
        && (candidate.region ?? 'surface') === (descriptor.region ?? 'surface')
        && candidateSacrifices.length === requestedSacrifices.length
        && candidateSacrifices.every((instanceId, index) =>
          instanceId === requestedSacrifices[index]);
    });
    const caster = spellcasterRefs(state, seat).find(({ instanceId }) =>
      instanceId === descriptor.casterInstanceId);
    if (!card
      || !definition
      || definition.cardType !== 'minion'
      || !legal
      || !pendingCemeterySummon && !caster) {
      throw new Error('unreachable illegal minion summon');
    }
    const discardCandidates = descriptor.paymentMode === 'random-card-discard'
      ? [
        ...player.hand.atlas.map((candidate) => ({ card: candidate, zone: 'atlas' as const })),
        ...player.hand.spellbook
          .filter(({ instanceId }) => instanceId !== card.instanceId)
          .map((candidate) => ({ card: candidate, zone: 'spellbook' as const })),
      ]
      : [];
    const randomCost = descriptor.paymentMode === 'random-card-discard'
      ? resolveRandomOutcome(
        state,
        discardCandidates.map(({ card: { instanceId } }) => instanceId),
        'summon_random_card_discard_cost',
        'card_index_candidate',
        forcedRandomOutcomeInstanceId,
      )
      : undefined;
    const discardedCard = randomCost
      ? discardCandidates.find(({ card: { instanceId } }) =>
        instanceId === randomCost.outcomeInstanceId)
      : undefined;
    if (descriptor.paymentMode === 'random-card-discard' && !discardedCard) {
      throw new Error('unreachable random card discard cost without another card');
    }
    const randomizedState = randomCost
      ? deepFreeze({ ...state, engine: randomCost.engine })
      : state;
    const discardOutcomes: readonly GameOutcome[] = discardedCard
      ? [{
        payload: {
          cardId: discardedCard.card.cardId,
          instanceId: discardedCard.card.instanceId,
          owner: discardedCard.card.owner,
          seat,
          sourceInstanceId: card.instanceId,
          zone: discardedCard.zone,
        },
        type: 'card-discarded',
      }]
      : [];
    const paymentRandomDraws = randomCost?.randomDraws ?? [];
    const unit: UnitInstance = deepFreeze({
      ...card,
      ...(definition.lanceCount ? { carriedLanceCount: definition.lanceCount } : {}),
      controller: seat,
      damage: 0,
      location: descriptor.cell,
      ...(descriptor.cells ? { occupiedCells: descriptor.cells } : {}),
      region: descriptor.region ?? 'surface',
      stealthed: definition.stealth === true,
      summoningSickness: true,
      tapped: false,
      warded: definition.ward === true,
    });
    let paidState: GameState;
    if (pendingCemeterySummon) {
      const cemeteryOwner = randomizedState.players[pendingCemeterySummon.cardOwner];
      const withoutPending = { ...randomizedState };
      delete withoutPending.pendingCemeterySummon;
      paidState = deepFreeze({
        ...withoutPending,
        phase: 'main',
        players: replacePlayer(
          randomizedState,
          pendingCemeterySummon.cardOwner,
          deepFreeze({
            ...cemeteryOwner,
            cemetery: cemeteryOwner.cemetery.filter(({ instanceId }) =>
              instanceId !== card.instanceId),
          }),
        ),
      });
    } else {
      const updatedPlayer = deepFreeze({
        ...player,
        ...(player.airThresholdsCastThisTurn !== undefined
          ? {
            airThresholdsCastThisTurn:
              player.airThresholdsCastThisTurn + definition.thresholds.air,
          }
          : {}),
        cemetery: discardedCard ? [...player.cemetery, discardedCard.card] : player.cemetery,
        hand: {
          ...player.hand,
          atlas: player.hand.atlas.filter(({ instanceId }) =>
            instanceId !== discardedCard?.card.instanceId),
          spellbook: player.hand.spellbook.filter(({ instanceId }) =>
            instanceId !== card.instanceId && instanceId !== discardedCard?.card.instanceId),
        },
        mana: player.mana - descriptor.manaCost,
      });
      paidState = deepFreeze({
        ...randomizedState,
        players: replacePlayer(randomizedState, seat, updatedPlayer),
      });
    }
    const sacrificedUnits = (descriptor.sacrificedMinionInstanceIds ?? []).map((instanceId) => {
      const sacrificed = paidState.realm.units.find((unit) => unit.instanceId === instanceId);
      if (!sacrificed) throw new Error('unreachable missing minion sacrifice cost');
      return sacrificed;
    });
    const sacrificedOutcomes: readonly GameOutcome[] = sacrificedUnits.map((sacrificed) => ({
      payload: {
        cardId: sacrificed.cardId,
        instanceId: sacrificed.instanceId,
        owner: sacrificed.owner,
        seat: sacrificed.controller,
        sourceInstanceId: card.instanceId,
      },
      type: 'minion-sacrificed',
    }));
    const deathResolution = sacrificedUnits.length > 0
      ? resolveMinionDeaths(
        paidState,
        paidState.players,
        paidState.realm.units,
        sacrificedUnits,
        new Set(),
      )
      : undefined;
    // A Deathrite ordering pause during payment must interrupt the summon itself;
    // never continue the summon with an authoritative choice still pending.
    if (deathResolution?.pendingDeathrites) {
      throw new Error('unsupported Deathrite ordering during summon payment continuation');
    }
    const resolvedPaymentState = deathResolution
      ? deepFreeze({
        ...paidState,
        ...(deathResolution.terminal.status === 'finished' ? { phase: 'terminal' as const } : {}),
        ...(deathResolution.pendingDeathrites
          ? { pendingDeathrites: deathResolution.pendingDeathrites }
          : {}),
        players: deathResolution.players,
        realm: {
          ...paidState.realm,
          ...(deathResolution.artifacts ? { artifacts: deathResolution.artifacts } : {}),
          units: deathResolution.units,
        },
        terminal: deathResolution.terminal,
      })
      : paidState;
    const paymentOutcomes = [
      ...discardOutcomes,
      ...sacrificedOutcomes,
      ...(deathResolution?.outcomes ?? []),
    ];
    if (resolvedPaymentState.terminal.status === 'finished') {
      return completeSummon([
        withStateVersion(resolvedPaymentState, {}),
        paymentOutcomes,
        paymentRandomDraws,
      ]);
    }
    const interaction = pendingCemeterySummon
      ? {
        outcomes: [] as readonly GameOutcome[],
        players: resolvedPaymentState.players,
        units: resolvedPaymentState.realm.units,
      }
      : recordInteraction(resolvedPaymentState, [caster!]);
    const realm = { ...resolvedPaymentState.realm, units: [...interaction.units, unit] };
    const summoned: GameOutcome = {
      payload: {
        cardId: card.cardId,
        casterInstanceId: descriptor.casterInstanceId,
        cell: descriptor.cell,
        ...(descriptor.cells ? { cells: descriptor.cells } : {}),
        instanceId: card.instanceId,
        manaPaid: descriptor.manaCost,
        ...(pendingCemeterySummon
          ? { owner: card.owner, sourceInstanceId: pendingCemeterySummon.sourceMagicInstanceId }
          : {}),
        ...(descriptor.region ? { region: descriptor.region } : {}),
        seat,
      },
      type: 'minion-summoned',
    };
    const summonOutcomes = [
      ...paymentOutcomes,
      ...interaction.outcomes,
      summoned,
      ...(definition.lanceCount
        ? [{
          payload: {
            bearerInstanceId: unit.instanceId,
            count: definition.lanceCount,
            sourceInstanceId: unit.instanceId,
          },
          type: 'lance-gained',
        }]
        : []),
    ];
    const summonedState = deepFreeze({
      ...resolvedPaymentState,
      players: interaction.players,
      realm,
    });
    const settlement = settleRegionOccupancy(summonedState);
    const summonedUnitSurvived = settlement.state.realm.units
      .some(({ instanceId }) => instanceId === unit.instanceId);
    if (!summonedUnitSurvived || settlement.state.terminal.status === 'finished') {
      return completeSummon([
        withStateVersion(settlement.state, {}),
        [...summonOutcomes, ...settlement.outcomes],
        paymentRandomDraws,
      ]);
    }
    const settledPlayer = settlement.state.players[seat];
    const genesisDrawCount = definition.genesisDrawSite
      ? 1
      : definition.genesisDrawSpells ?? 0;
    const genesisDrawZone = definition.genesisDrawSite
      ? 'atlas'
      : definition.genesisDrawSpells !== undefined ? 'spellbook' : undefined;
    if (definition.genesisDisableSelfUntilDamaged === true) {
      const disabled = withStateVersion(settlement.state, {
        realm: {
          ...settlement.state.realm,
          units: settlement.state.realm.units.map((candidate) => candidate.instanceId === unit.instanceId
            ? deepFreeze({ ...candidate, disabledUntilDamaged: true as const })
            : candidate),
        },
      });
      return completeSummon([
        disabled,
        [
          ...summonOutcomes,
          ...settlement.outcomes,
          {
            payload: { instanceId: unit.instanceId, seat, sourceInstanceId: unit.instanceId },
            type: 'minion-disabled',
          },
        ],
        paymentRandomDraws,
      ]);
    }
    if (definition.genesisDamageEachOtherUnitHere === 1
      || definition.genesisStrikeEachEnemyHere === true) {
      const source: GameUnitRef = {
        instanceId: unit.instanceId,
        kind: 'minion',
        seat,
      };
      const sourceUnit = settlement.state.realm.units.find(({ instanceId }) =>
        instanceId === unit.instanceId);
      if (!sourceUnit || minionDisabled(settlement.state, sourceUnit)) {
        return completeSummon([
          withStateVersion(settlement.state, {}),
          [...summonOutcomes, ...settlement.outcomes],
          paymentRandomDraws,
        ]);
      }
      const isStrike = definition.genesisStrikeEachEnemyHere === true;
      const targets = (isStrike ? [otherSeat(seat)] : ['north', 'south'] as const)
        .flatMap((targetSeat) => unitRefs(settlement.state, targetSeat))
        .filter((target) => {
          if (target.instanceId === source.instanceId) return false;
          const status = unitStatus(settlement.state, target);
          return status.region === sourceUnit.region
            && status.occupiedCells.some((cell) => unitOccupiedCells(sourceUnit).includes(cell));
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      if (targets.length === 0) {
        return completeSummon([
          withStateVersion(settlement.state, {}),
          [...summonOutcomes, ...settlement.outcomes],
          paymentRandomDraws,
        ]);
      }
      const amount = isStrike
        ? strikeDamage(settlement.state, source)
        : definition.genesisDamageEachOtherUnitHere!;
      const pending: PendingCombat = deepFreeze({
        allocations: targets.map(({ instanceId }) => ({
          amount,
          targetInstanceId: instanceId,
        })),
        attacker: source,
        attackingSeat: seat,
        cell: sourceUnit.location,
        combatants: targets,
        defenders: [],
        originalTarget: null,
        ...(sourceUnit.region === 'surface' ? {} : { region: sourceUnit.region }),
        targetRemoved: false,
      });
      const allocationOutcomes: readonly GameOutcome[] = targets.map(({ instanceId }) => ({
        payload: {
          amount,
          ...(isStrike
            ? { strikerInstanceId: source.instanceId }
            : { sourceInstanceId: source.instanceId }),
          targetInstanceId: instanceId,
        },
        type: isStrike ? 'strike-damage-allocated' : 'genesis-damage-allocated',
      }));
      const [damaged, outcomes, randomDraws] = resolveFightWindow(
        settlement.state,
        pending,
        [...summonOutcomes, ...settlement.outcomes, ...allocationOutcomes],
        'attacker-unit',
        true,
        false,
        [source],
        isStrike,
        true,
      );
      return completeSummon([
        withStateVersion(damaged, {}),
        outcomes,
        [...paymentRandomDraws, ...randomDraws],
      ]);
    }
    if (definition.genesisMayDamageTargetAdjacentUnit === 2
      && descriptor.genesisDamageChoice === 'target'
      && descriptor.genesisDamageTarget) {
      const source: GameUnitRef = {
        instanceId: unit.instanceId,
        kind: 'minion',
        seat,
      };
      const sourceUnit = settlement.state.realm.units.find(({ instanceId }) =>
        instanceId === unit.instanceId);
      const target = unitRefs(settlement.state, descriptor.genesisDamageTarget.seat)
        .find((candidate) => candidate.kind === descriptor.genesisDamageTarget!.kind
          && candidate.instanceId === descriptor.genesisDamageTarget!.instanceId);
      const targetStatus = target ? unitStatus(settlement.state, target) : undefined;
      if (!sourceUnit
        || minionDisabled(settlement.state, sourceUnit)
        || !target
        || !targetStatus
        || targetStatus.region !== sourceUnit.region
        || target.seat !== seat && targetStatus.stealthed
        || !footprintsHereOrBordering(
          unitOccupiedCells(sourceUnit),
          targetStatus.occupiedCells,
        )) {
        return completeSummon([
          withStateVersion(settlement.state, {}),
          [...summonOutcomes, ...settlement.outcomes],
          paymentRandomDraws,
        ]);
      }
      const pending: PendingCombat = deepFreeze({
        allocations: [{
          amount: definition.genesisMayDamageTargetAdjacentUnit,
          targetInstanceId: target.instanceId,
        }],
        attacker: source,
        attackingSeat: seat,
        cell: sourceUnit.location,
        combatants: [target],
        defenders: [],
        originalTarget: null,
        ...(sourceUnit.region === 'surface' ? {} : { region: sourceUnit.region }),
        targetRemoved: false,
      });
      const [damaged, outcomes, randomDraws] = resolveFightWindow(
        settlement.state,
        pending,
        [
          ...summonOutcomes,
          ...settlement.outcomes,
          {
            payload: {
              amount: definition.genesisMayDamageTargetAdjacentUnit,
              sourceInstanceId: source.instanceId,
              targetInstanceId: target.instanceId,
            },
            type: 'genesis-damage-allocated',
          },
        ],
        'attacker-unit',
        true,
        false,
        [source],
        false,
        true,
      );
      return completeSummon([
        withStateVersion(damaged, {}),
        outcomes,
        [...paymentRandomDraws, ...randomDraws],
      ]);
    }
    if (definition.genesisHealController === 2) {
      const avatarDefinition = cardDefinition(settlement.state, settledPlayer.avatar.card.cardId);
      if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
      const [healed, amount] = healAvatar(
        settledPlayer,
        avatarDefinition.life,
        definition.genesisHealController,
      );
      return completeSummon([
        withStateVersion(settlement.state, {
          players: replacePlayer(settlement.state, seat, healed),
        }),
        [
          ...summonOutcomes,
          ...settlement.outcomes,
          ...(amount > 0
            ? [{
              payload: {
                amount,
                attemptedAmount: definition.genesisHealController,
                life: healed.avatar.life,
                seat,
                sourceInstanceId: card.instanceId,
              },
              type: 'avatar-healed' as const,
            }]
            : []),
        ],
        paymentRandomDraws,
      ]);
    }
    if (definition.genesisLoseControllerLife === 2) {
      const [lifePlayer, amount, reachedDeathsDoor] = loseAvatarLife(
        settledPlayer,
        definition.genesisLoseControllerLife,
        state.turnNumber,
      );
      return completeSummon([
        withStateVersion(settlement.state, {
          players: replacePlayer(settlement.state, seat, lifePlayer),
        }),
        [
          ...summonOutcomes,
          ...settlement.outcomes,
          ...(amount > 0
            ? [{
              payload: {
                amount,
                life: lifePlayer.avatar.life,
                seat,
                sourceInstanceId: card.instanceId,
              },
              type: 'avatar-life-lost' as const,
            }]
            : []),
          ...(reachedDeathsDoor
            ? [{
              payload: { seat, sourceInstanceId: card.instanceId, turnNumber: state.turnNumber },
              type: 'avatar-reached-deaths-door' as const,
            }]
            : []),
        ],
        paymentRandomDraws,
      ]);
    }
    if (genesisDrawZone) {
      const drawn = settledPlayer[genesisDrawZone].slice(0, genesisDrawCount);
      const remaining = settledPlayer[genesisDrawZone].slice(drawn.length);
      const drawFailed = drawn.length < genesisDrawCount;
      const drawingPlayer = deepFreeze({
        ...settledPlayer,
        [genesisDrawZone]: remaining,
        hand: {
          ...settledPlayer.hand,
          [genesisDrawZone]: [...settledPlayer.hand[genesisDrawZone], ...drawn],
        },
      });
      const winner = otherSeat(seat);
      return completeSummon([
        withStateVersion(settlement.state, {
          ...(drawFailed
            ? {
              pendingCombat: null,
              phase: 'terminal' as const,
              terminal: {
                loser: seat,
                reason: 'deck_empty' as const,
                status: 'finished' as const,
                winner,
              },
            }
            : {}),
          players: replacePlayer(settlement.state, seat, drawingPlayer),
        }),
        [
          ...summonOutcomes,
          ...settlement.outcomes,
          ...drawn.map(() => ({
            payload: { seat, sourceInstanceId: card.instanceId },
            type: genesisDrawZone === 'atlas' ? 'site-drawn' : 'spell-drawn',
          })),
          ...(drawFailed
            ? [{ payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' }]
            : []),
        ],
        paymentRandomDraws,
      ]);
    }
    return completeSummon([
      withStateVersion(settlement.state, {}),
      [...summonOutcomes, ...settlement.outcomes],
      paymentRandomDraws,
    ]);
  }

  if (descriptor.kind === 'activate-artifact-damage') {
    const legal = artifactDamageAbilityDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-artifact-damage'
        && candidate.artifactInstanceId === descriptor.artifactInstanceId
        && candidate.helper.instanceId === descriptor.helper.instanceId
        && candidate.helper.kind === descriptor.helper.kind
        && candidate.helper.seat === descriptor.helper.seat
        && candidate.target.instanceId === descriptor.target.instanceId
        && candidate.target.kind === descriptor.target.kind
        && candidate.target.seat === descriptor.target.seat);
    const artifact = state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === descriptor.artifactInstanceId);
    if (!legal || !artifact || !('bearer' in artifact)) {
      throw new Error('unreachable illegal Artifact damage activation');
    }
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps !== 3) {
      throw new Error('unreachable Artifact damage definition');
    }
    const costIds = new Set([artifact.bearer.instanceId, descriptor.helper.instanceId]);
    const tappedPlayer = costIds.has(player.avatar.card.instanceId)
      ? deepFreeze({ ...player, avatar: { ...player.avatar, tapped: true } })
      : player;
    const tappedState = deepFreeze({
      ...state,
      players: replacePlayer(state, seat, tappedPlayer),
      realm: {
        ...state.realm,
        units: state.realm.units.map((unit) => costIds.has(unit.instanceId)
          ? deepFreeze({ ...unit, tapped: true })
          : unit),
      },
    });
    const target = unitStatus(tappedState, descriptor.target);
    const pending: PendingCombat = deepFreeze({
      allocations: [{
        amount: definition.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps,
        targetInstanceId: descriptor.target.instanceId,
      }],
      attacker: artifact.bearer,
      attackingSeat: seat,
      cell: target.location,
      combatants: [descriptor.target],
      defenders: [],
      originalTarget: descriptor.target,
      ...(target.region === 'surface'
        ? {}
        : { region: target.region as 'underground' | 'underwater' | 'void' }),
      targetRemoved: false,
    });
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      tappedState,
      pending,
      [{
        payload: {
          bearerInstanceId: artifact.bearer.instanceId,
          helperInstanceId: descriptor.helper.instanceId,
          seat,
          sourceInstanceId: artifact.instanceId,
          targetInstanceId: descriptor.target.instanceId,
        },
        type: 'artifact-damage-activated',
      }, {
        payload: {
          amount: definition.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps,
          sourceInstanceId: artifact.instanceId,
          targetInstanceId: descriptor.target.instanceId,
        },
        type: 'artifact-damage-allocated',
      }],
      'non-unit',
      true,
      false,
      [],
      false,
      false,
    );
    return [withStateVersion(damaged, {}), outcomes, randomDraws];
  }

  if (descriptor.kind === 'activate-artifact-discard-area-damage') {
    const legal = artifactDiscardAreaDamageAbilityDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-artifact-discard-area-damage'
        && candidate.artifactInstanceId === descriptor.artifactInstanceId
        && candidate.discardCardInstanceId === descriptor.discardCardInstanceId
        && candidate.discardZone === descriptor.discardZone
        && candidate.helper.instanceId === descriptor.helper.instanceId
        && candidate.helper.kind === descriptor.helper.kind
        && candidate.helper.seat === descriptor.helper.seat
        && sameLocation(candidate.targetLocation, descriptor.targetLocation));
    const artifact = state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === descriptor.artifactInstanceId);
    const discardedCard = player.hand[descriptor.discardZone].find(({ instanceId }) =>
      instanceId === descriptor.discardCardInstanceId);
    if (!legal || !artifact || !('bearer' in artifact) || !discardedCard) {
      throw new Error('unreachable illegal Artifact discard-area-damage activation');
    }
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition
        .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps
          !== true) {
      throw new Error('unreachable Artifact discard-area-damage definition');
    }
    const discardedDefinition = cardDefinition(state, discardedCard.cardId);
    if (discardedDefinition.cardType === 'avatar') {
      throw new Error('unreachable discarded Avatar');
    }
    const amount = discardedDefinition.cardType === 'site' ? 0 : discardedDefinition.manaCost;
    const paidPlayer = deepFreeze({
      ...player,
      cemetery: [...player.cemetery, discardedCard],
      hand: {
        ...player.hand,
        [descriptor.discardZone]: player.hand[descriptor.discardZone].filter(({ instanceId }) =>
          instanceId !== descriptor.discardCardInstanceId),
      },
    });
    const costIds = new Set([artifact.bearer.instanceId, descriptor.helper.instanceId]);
    const tappedPlayer = costIds.has(player.avatar.card.instanceId)
      ? deepFreeze({ ...paidPlayer, avatar: { ...paidPlayer.avatar, tapped: true } })
      : paidPlayer;
    const costState = deepFreeze({
      ...state,
      players: replacePlayer(state, seat, tappedPlayer),
      realm: {
        ...state.realm,
        units: state.realm.units.map((unit) => costIds.has(unit.instanceId)
          ? deepFreeze({ ...unit, tapped: true })
          : unit),
      },
    });
    const targets = (['north', 'south'] as const)
      .flatMap((targetSeat) => unitRefs(costState, targetSeat))
      .filter((target) => {
        const status = unitStatus(costState, target);
        return status.occupiedCells.includes(descriptor.targetLocation.cell)
          && status.region === descriptor.targetLocation.region;
      })
      .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
    const events: readonly GameOutcome[] = [{
      payload: {
        cardId: discardedCard.cardId,
        instanceId: discardedCard.instanceId,
        owner: discardedCard.owner,
        seat,
        sourceInstanceId: artifact.instanceId,
        zone: descriptor.discardZone,
      },
      type: 'card-discarded',
    }, {
      payload: {
        bearerInstanceId: artifact.bearer.instanceId,
        discardCardInstanceId: discardedCard.instanceId,
        helperInstanceId: descriptor.helper.instanceId,
        seat,
        sourceInstanceId: artifact.instanceId,
        targetCell: descriptor.targetLocation.cell,
        targetRegion: descriptor.targetLocation.region,
      },
      type: 'artifact-discard-area-damage-activated',
    }, ...targets.map(({ instanceId }) => ({
      payload: {
        amount,
        sourceInstanceId: artifact.instanceId,
        targetInstanceId: instanceId,
      },
      type: 'artifact-discard-area-damage-allocated',
    }))];
    if (targets.length === 0) return [withStateVersion(costState, {}), events, []];
    const pending: PendingCombat = deepFreeze({
      allocations: targets.map(({ instanceId }) => ({ amount, targetInstanceId: instanceId })),
      attacker: artifact.bearer,
      attackingSeat: seat,
      cell: descriptor.targetLocation.cell,
      combatants: targets,
      defenders: [],
      originalTarget: null,
      ...(descriptor.targetLocation.region === 'surface'
        ? {}
        : { region: descriptor.targetLocation.region }),
      targetRemoved: false,
    });
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      costState,
      pending,
      events,
      'non-unit',
      true,
      false,
      [],
      false,
      false,
    );
    return [withStateVersion(damaged, {}), outcomes, randomDraws];
  }

  if (descriptor.kind === 'activate-artifact-roll-damage') {
    const legal = artifactRollDamageAbilityDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-artifact-roll-damage'
        && candidate.artifactInstanceId === descriptor.artifactInstanceId
        && candidate.direction === descriptor.direction
        && candidate.pusher.instanceId === descriptor.pusher.instanceId
        && candidate.pusher.kind === descriptor.pusher.kind
        && candidate.pusher.seat === descriptor.pusher.seat
        && candidate.path.length === descriptor.path.length
        && candidate.path.every((location, index) =>
          sameLocation(location, descriptor.path[index]!)));
    const artifact = state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === descriptor.artifactInstanceId);
    const destination = descriptor.path.at(-1);
    if (!legal || !artifact || !destination) {
      throw new Error('unreachable illegal Artifact roll-damage activation');
    }
    const definition = cardDefinition(state, artifact.cardId);
    if (definition.cardType !== 'artifact'
      || definition.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath !== 4) {
      throw new Error('unreachable Artifact roll-damage definition');
    }
    const tappedPlayers = descriptor.pusher.kind === 'avatar'
      ? replacePlayer(state, seat, deepFreeze({
        ...player,
        avatar: { ...player.avatar, tapped: true },
      }))
      : state.players;
    const tappedUnits = descriptor.pusher.kind === 'minion'
      ? state.realm.units.map((unit) => unit.instanceId === descriptor.pusher.instanceId
        ? deepFreeze({ ...unit, tapped: true })
        : unit)
      : state.realm.units;
    const movedArtifacts = state.realm.artifacts!.map((candidate): ArtifactInstance =>
      candidate.instanceId === artifact.instanceId
        ? deepFreeze({
          cardId: candidate.cardId,
          instanceId: candidate.instanceId,
          location: destination.cell,
          owner: candidate.owner,
          region: destination.region,
          source: candidate.source,
        })
        : candidate);
    const costState = deepFreeze({
      ...state,
      players: tappedPlayers,
      realm: { ...state.realm, artifacts: movedArtifacts, units: tappedUnits },
    });
    const pathCells = descriptor.path.length > 1
      ? new Set(descriptor.path.map(({ cell }) => cell))
      : new Set<RealmCell>();
    const damagedTargets = (['north', 'south'] as const)
      .flatMap((targetSeat) => unitRefs(costState, targetSeat))
      .flatMap((target) => {
        if (target.instanceId === descriptor.pusher.instanceId) return [];
        const status = unitStatus(costState, target);
        if (status.region !== destination.region) return [];
        const coveredCells = status.occupiedCells.filter((cell) => pathCells.has(cell)).length;
        return coveredCells > 0
          ? [{
            amount:
              definition.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath
              * coveredCells,
            target,
          }]
          : [];
      })
      .sort((left, right) => left.target.instanceId.localeCompare(right.target.instanceId));
    const events: readonly GameOutcome[] = [{
      payload: {
        direction: descriptor.direction,
        fromCell: descriptor.path[0]!.cell,
        fromRegion: descriptor.path[0]!.region,
        path: descriptor.path,
        pusherInstanceId: descriptor.pusher.instanceId,
        pusherKind: descriptor.pusher.kind,
        pusherSeat: descriptor.pusher.seat,
        seat,
        sourceInstanceId: artifact.instanceId,
        toCell: destination.cell,
        toRegion: destination.region,
      },
      type: 'artifact-roll-damage-activated',
    }, ...damagedTargets.map(({ amount, target }) => ({
      payload: {
        amount,
        sourceInstanceId: artifact.instanceId,
        targetInstanceId: target.instanceId,
      },
      type: 'artifact-roll-damage-allocated',
    }))];
    if (damagedTargets.length === 0) return [withStateVersion(costState, {}), events, []];
    const pending: PendingCombat = deepFreeze({
      allocations: damagedTargets.map(({ amount, target }) => ({
        amount,
        targetInstanceId: target.instanceId,
      })),
      attacker: descriptor.pusher,
      attackingSeat: seat,
      cell: destination.cell,
      combatants: damagedTargets.map(({ target }) => target),
      defenders: [],
      originalTarget: null,
      ...(destination.region === 'surface' ? {} : { region: destination.region }),
      targetRemoved: false,
    });
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      costState,
      pending,
      events,
      'non-unit',
      true,
      false,
      [],
      false,
      false,
    );
    return [withStateVersion(damaged, {}), outcomes, randomDraws];
  }

  if (descriptor.kind === 'activate-area-damage') {
    const legal = areaDamageAbilityDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-area-damage'
        && candidate.sourceInstanceId === descriptor.sourceInstanceId
        && sameLocation(candidate.targetLocation, descriptor.targetLocation));
    const source = state.realm.units.find(({ instanceId }) => instanceId === descriptor.sourceInstanceId);
    if (!legal || !source) throw new Error('unreachable illegal area-damage activation');
    const definition = cardDefinition(state, source.cardId);
    if (definition.cardType !== 'minion'
      || definition.tapToDamageEachUnitAtAdjacentLocation !== 2) {
      throw new Error('unreachable area-damage source definition');
    }
    const amount = definition.tapToDamageEachUnitAtAdjacentLocation;
    const sourceRef: GameUnitRef = { instanceId: source.instanceId, kind: 'minion', seat };
    const tappedState = deepFreeze({
      ...state,
      realm: {
        ...state.realm,
        units: state.realm.units.map((candidate) => candidate.instanceId === source.instanceId
          ? deepFreeze({ ...candidate, tapped: true })
          : candidate),
      },
    });
    const interaction = recordInteraction(tappedState, [sourceRef]);
    const activatedState = deepFreeze({
      ...tappedState,
      players: interaction.players,
      realm: { ...tappedState.realm, units: interaction.units },
    });
    const activated: GameOutcome = {
      payload: {
        cell: descriptor.targetLocation.cell,
        region: descriptor.targetLocation.region,
        seat,
        sourceInstanceId: source.instanceId,
      },
      type: 'area-damage-activated',
    };
    const targets = (['north', 'south'] as const)
      .flatMap((targetSeat) => unitRefs(activatedState, targetSeat))
      .filter((target) => {
        const status = unitStatus(activatedState, target);
        return status.occupiedCells.includes(descriptor.targetLocation.cell)
          && status.region === descriptor.targetLocation.region;
      })
      .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
    if (targets.length === 0) {
      return [withStateVersion(activatedState, {}), [activated, ...interaction.outcomes], []];
    }
    const pending: PendingCombat = deepFreeze({
      allocations: targets.map(({ instanceId }) => ({
        amount,
        targetInstanceId: instanceId,
      })),
      attacker: sourceRef,
      attackingSeat: seat,
      cell: descriptor.targetLocation.cell,
      combatants: targets,
      defenders: [],
      originalTarget: null,
      ...(descriptor.targetLocation.region === 'surface'
        ? {}
        : { region: descriptor.targetLocation.region }),
      targetRemoved: false,
    });
    const allocationOutcomes: readonly GameOutcome[] = targets.map(({ instanceId }) => ({
      payload: {
        amount,
        sourceInstanceId: source.instanceId,
        targetInstanceId: instanceId,
      },
      type: 'area-damage-allocated',
    }));
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      activatedState,
      pending,
      [activated, ...interaction.outcomes, ...allocationOutcomes],
      'attacker-unit',
      true,
      false,
      [],
      false,
      true,
    );
    return [withStateVersion(damaged, {}), outcomes, randomDraws];
  }

  if (descriptor.kind === 'activate-discard-random-damage') {
    const legal = discardRandomDamageDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-discard-random-damage'
        && candidate.sourceInstanceId === descriptor.sourceInstanceId
        && candidate.discardCardInstanceId === descriptor.discardCardInstanceId);
    const source = state.realm.units.find(({ instanceId }) =>
      instanceId === descriptor.sourceInstanceId);
    const discardedCard = player.hand.spellbook.find(({ instanceId }) =>
      instanceId === descriptor.discardCardInstanceId);
    if (!legal || !source || !discardedCard) {
      throw new Error('unreachable illegal discard-random-damage activation');
    }
    const definition = cardDefinition(state, source.cardId);
    if (definition.cardType !== 'minion'
      || definition.discardSpellToDamageRandomOtherUnitHere === undefined) {
      throw new Error('unreachable discard-random-damage source definition');
    }
    const amount = definition.discardSpellToDamageRandomOtherUnitHere;
    const sourceRef: GameUnitRef = { instanceId: source.instanceId, kind: 'minion', seat };
    const sourceStatus = unitStatus(state, sourceRef);
    const paidPlayer = deepFreeze({
      ...player,
      cemetery: [...player.cemetery, discardedCard],
      hand: {
        ...player.hand,
        spellbook: player.hand.spellbook.filter(({ instanceId }) =>
          instanceId !== descriptor.discardCardInstanceId),
      },
    });
    const paidState = deepFreeze({
      ...state,
      players: replacePlayer(state, seat, paidPlayer),
    });
    const interaction = recordInteraction(paidState, [sourceRef]);
    const activatedState = deepFreeze({
      ...paidState,
      players: interaction.players,
      realm: { ...paidState.realm, units: interaction.units },
    });
    const candidates = randomUnitCandidatesAtLocation(
      activatedState,
      { cell: sourceStatus.location, region: sourceStatus.region },
      source.instanceId,
    );
    const discarded: GameOutcome = {
      payload: {
        cardId: discardedCard.cardId,
        instanceId: discardedCard.instanceId,
        owner: discardedCard.owner,
        seat,
        sourceInstanceId: source.instanceId,
        zone: 'spellbook',
      },
      type: 'card-discarded',
    };
    if (candidates.length === 0) {
      return [withStateVersion(activatedState, {}), [discarded, {
        payload: {
          amount,
          discardCardInstanceId: discardedCard.instanceId,
          seat,
          sourceInstanceId: source.instanceId,
          sourceLocation: { cell: sourceStatus.location, region: sourceStatus.region },
        },
        type: 'discard-random-damage-activated',
      }, ...interaction.outcomes], []];
    }
    const selected = resolveRandomOutcome(
      activatedState,
      candidates.map(({ instanceId }) => instanceId),
      'discard_spell_random_other_unit_here',
      'unit_index_candidate',
      forcedRandomOutcomeInstanceId,
    );
    const targetRef = candidates.find(({ instanceId }) =>
      instanceId === selected.outcomeInstanceId)!;
    const randomizedState = deepFreeze({ ...activatedState, engine: selected.engine });
    const activated: GameOutcome = {
      payload: {
        amount,
        discardCardInstanceId: discardedCard.instanceId,
        seat,
        sourceInstanceId: source.instanceId,
        sourceLocation: { cell: sourceStatus.location, region: sourceStatus.region },
        targetInstanceId: targetRef.instanceId,
        targetKind: targetRef.kind,
        targetSeat: targetRef.seat,
      },
      type: 'discard-random-damage-activated',
    };
    const allocated: GameOutcome = {
      payload: {
        amount,
        sourceInstanceId: source.instanceId,
        targetInstanceId: targetRef.instanceId,
      },
      type: 'discard-random-damage-allocated',
    };
    const pending: PendingCombat = deepFreeze({
      allocations: [{ amount, targetInstanceId: targetRef.instanceId }],
      attacker: sourceRef,
      attackingSeat: seat,
      cell: sourceStatus.location,
      combatants: [targetRef],
      defenders: [],
      originalTarget: targetRef,
      ...(sourceStatus.region === 'surface' ? {} : { region: sourceStatus.region }),
      targetRemoved: false,
    });
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      randomizedState,
      pending,
      [discarded, activated, ...interaction.outcomes, allocated],
      'attacker-unit',
      true,
      false,
      [],
      false,
      true,
    );
    return [
      withStateVersion(damaged, {}),
      outcomes,
      [...selected.randomDraws, ...randomDraws],
    ];
  }

  if (descriptor.kind === 'activate-sparkmage') {
    const legal = sparkmageDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-sparkmage'
        && candidate.sourceInstanceId === descriptor.sourceInstanceId
        && sameLocation(candidate.targetLocation, descriptor.targetLocation));
    const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
    if (!legal
      || descriptor.sourceInstanceId !== player.avatar.card.instanceId
      || avatarDefinition.cardType !== 'avatar'
      || avatarDefinition.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn !== true) {
      throw new Error('unreachable illegal Sparkmage activation');
    }
    const amount = player.airThresholdsCastThisTurn ?? 0;
    const sourceRef: GameUnitRef = {
      instanceId: player.avatar.card.instanceId,
      kind: 'avatar',
      seat,
    };
    const interaction = recordInteraction(state, [sourceRef]);
    const interactedPlayer = interaction.players[seat];
    const activatedState = deepFreeze({
      ...state,
      players: deepFreeze({
        ...interaction.players,
        [seat]: deepFreeze({
          ...interactedPlayer,
          avatar: { ...interactedPlayer.avatar, tapped: true },
        }),
      }),
      realm: { ...state.realm, units: interaction.units },
    });
    const candidates = randomUnitCandidatesAtLocation(
      activatedState,
      descriptor.targetLocation,
      sourceRef.instanceId,
    );
    if (candidates.length === 0) {
      return [
        withStateVersion(activatedState, {}),
        [{
          payload: {
            amount,
            seat,
            sourceInstanceId: sourceRef.instanceId,
            targetLocation: descriptor.targetLocation,
          },
          type: 'sparkmage-activated',
        }, ...interaction.outcomes],
        [],
      ];
    }
    const selected = resolveRandomOutcome(
      activatedState,
      candidates.map(({ instanceId }) => instanceId),
      'sparkmage_random_other_unit_at_nearby_location',
      'unit_index_candidate',
      forcedRandomOutcomeInstanceId,
    );
    const targetRef = candidates.find(({ instanceId }) =>
      instanceId === selected.outcomeInstanceId)!;
    const randomizedState = deepFreeze({ ...activatedState, engine: selected.engine });
    const activated: GameOutcome = {
      payload: {
        amount,
        seat,
        sourceInstanceId: sourceRef.instanceId,
        targetInstanceId: targetRef.instanceId,
        targetKind: targetRef.kind,
        targetLocation: descriptor.targetLocation,
        targetSeat: targetRef.seat,
      },
      type: 'sparkmage-activated',
    };
    if (amount === 0) {
      return [
        withStateVersion(randomizedState, {}),
        [activated, ...interaction.outcomes],
        selected.randomDraws,
      ];
    }
    const pending: PendingCombat = deepFreeze({
      allocations: [{
        amount,
        targetInstanceId: targetRef.instanceId,
      }],
      attacker: sourceRef,
      attackingSeat: seat,
      cell: descriptor.targetLocation.cell,
      combatants: [targetRef],
      defenders: [],
      originalTarget: targetRef,
      ...(descriptor.targetLocation.region === 'surface'
        ? {}
        : { region: descriptor.targetLocation.region }),
      targetRemoved: false,
    });
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      randomizedState,
      pending,
      [activated, ...interaction.outcomes],
      'attacker-unit',
      true,
      false,
      [],
      false,
      true,
    );
    return [
      withStateVersion(damaged, {}),
      outcomes,
      [...selected.randomDraws, ...randomDraws],
    ];
  }

  if (descriptor.kind === 'activate-mana') {
    const legal = manaAbilityDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-mana'
        && candidate.amount === descriptor.amount
        && candidate.unitInstanceId === descriptor.unitInstanceId);
    const unit = state.realm.units.find(({ instanceId }) => instanceId === descriptor.unitInstanceId);
    if (!legal || !unit) throw new Error('unreachable illegal mana activation');
    const tappedState = deepFreeze({
      ...state,
      realm: {
        ...state.realm,
        units: state.realm.units.map((candidate) => candidate.instanceId === unit.instanceId
          ? deepFreeze({ ...candidate, tapped: true })
          : candidate),
      },
    });
    const interaction = recordInteraction(
      tappedState,
      [{ instanceId: unit.instanceId, kind: 'minion', seat }],
    );
    return [
      withStateVersion(state, {
        players: deepFreeze({
          ...interaction.players,
          [seat]: deepFreeze({
            ...interaction.players[seat],
            mana: player.mana + descriptor.amount,
          }),
        }),
        realm: { ...state.realm, units: interaction.units },
      }),
      [
        {
          payload: {
            amount: descriptor.amount,
            seat,
            unitInstanceId: descriptor.unitInstanceId,
          },
          type: 'mana-activated',
        },
        ...interaction.outcomes,
      ],
      [],
    ];
  }

  if (descriptor.kind === 'shoot-projectile') {
    const movement = state.phase === 'movement' ? state.pendingBasicMovement : null;
    const legal = (movement ? basicMovementDescriptors(state, seat) : rangedDescriptors(state, seat))
      .some((candidate) =>
      candidate.kind === 'shoot-projectile'
        && candidate.shooterInstanceId === descriptor.shooterInstanceId
        && candidate.direction === descriptor.direction
        && samePath(candidate.path, descriptor.path)
        && (candidate.hit === null && descriptor.hit === null
          || candidate.hit !== null && descriptor.hit !== null
            && candidate.hit.instanceId === descriptor.hit.instanceId
            && candidate.hit.kind === descriptor.hit.kind
            && candidate.hit.seat === descriptor.hit.seat));
    const shooter = unitRefs(state, seat)
      .find(({ instanceId }) => instanceId === descriptor.shooterInstanceId);
    if (!legal || !shooter) throw new Error('unreachable illegal Ranged projectile');
    const shooterStatus = unitStatus(state, shooter);
    const tapped = moveAndTapUnit(state, shooter, {
      cell: shooterStatus.location,
      region: shooterStatus.region,
    });
    const interaction = recordInteraction(deepFreeze({
      ...state,
      players: tapped.players,
      realm: tapped.realm,
    }), [shooter]);
    const shotState = deepFreeze({
      ...state,
      ...(movement
        ? { pendingBasicMovement: { ...movement, rangedStrikeUsed: true } }
        : {}),
      players: interaction.players,
      realm: { ...tapped.realm, units: interaction.units },
    });
    const shot: GameOutcome = {
      payload: {
        direction: descriptor.direction,
        hit: descriptor.hit,
        path: descriptor.path,
        seat,
        shooterInstanceId: shooter.instanceId,
      },
      type: 'projectile-shot',
    };
    if (!descriptor.hit) {
      return [withStateVersion(shotState, {}), [shot, ...interaction.outcomes], []];
    }
    const amount = strikeDamage(shotState, shooter);
    const strike: GameOutcome = {
      payload: {
        amount,
        strikerInstanceId: shooter.instanceId,
        targetInstanceId: descriptor.hit.instanceId,
      },
      type: 'strike-damage-allocated',
    };
    const result = finishFight(shotState, deepFreeze({
      allocations: [{ amount, targetInstanceId: descriptor.hit.instanceId }],
      attacker: shooter,
      attackingSeat: seat,
      cell: descriptor.path.at(-1)!.cell,
      combatants: [descriptor.hit],
      defenders: [],
      originalTarget: descriptor.hit,
      ...(unitStatus(state, descriptor.hit).region === 'surface'
        ? {}
        : { region: unitStatus(state, descriptor.hit).region as 'underground' | 'underwater' | 'void' }),
      targetRemoved: false,
    }), [shot, ...interaction.outcomes, strike], false, movement === null);
    if (!movement) return result;
    const [resolved, outcomes, draws] = result;
    const pendingCombat = movement.purpose === 'defend' && state.pendingCombat
      ? reconcilePendingCombat(resolved, state.pendingCombat)
      : null;
    const sourceRemains = resolved.realm.units.some(({ controller, instanceId }) =>
      controller === seat && instanceId === movement.sourceInstanceId);
    if (resolved.terminal.status === 'finished') {
      return [withStateVersion(resolved, {
        pendingBasicMovement: null,
        pendingCombat: null,
        phase: 'terminal',
      }), outcomes, draws];
    }
    if (!sourceRemains || (movement.purpose === 'defend' && pendingCombat === null)) {
      return [withStateVersion(resolved, {
        decisionSeat: pendingCombat ? movement.seat : resolved.activeSeat,
        pendingBasicMovement: null,
        pendingCombat,
        phase: pendingCombat ? 'defend' : 'main',
      }), outcomes, draws];
    }
    return [withStateVersion(resolved, {
      decisionSeat: movement.seat,
      pendingBasicMovement: { ...movement, rangedStrikeUsed: true },
      ...(movement.purpose === 'defend' ? { pendingCombat } : {}),
      phase: 'movement',
    }), outcomes, draws];
  }

  if (descriptor.kind === 'shoot-drag-projectile') {
    const legal = dragProjectileDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'shoot-drag-projectile'
        && candidate.shooterInstanceId === descriptor.shooterInstanceId
        && candidate.direction === descriptor.direction
        && candidate.fightOnArrival === descriptor.fightOnArrival
        && samePath(candidate.path, descriptor.path)
        && (candidate.hit === null && descriptor.hit === null
          || candidate.hit !== null && descriptor.hit !== null
            && candidate.hit.instanceId === descriptor.hit.instanceId
            && candidate.hit.kind === descriptor.hit.kind
            && candidate.hit.seat === descriptor.hit.seat));
    const shooter = unitRefs(state, seat)
      .find(({ instanceId }) => instanceId === descriptor.shooterInstanceId);
    if (!legal || !shooter) throw new Error('unreachable illegal drag projectile');
    const shooterStatus = unitStatus(state, shooter);
    const tapped = moveAndTapUnit(state, shooter, {
      cell: shooterStatus.location,
      region: shooterStatus.region,
    });
    const interaction = recordInteraction(deepFreeze({
      ...state,
      players: tapped.players,
      realm: tapped.realm,
    }), [shooter]);
    const shotState = deepFreeze({
      ...state,
      players: interaction.players,
      realm: { ...tapped.realm, units: interaction.units },
    });
    const shot: GameOutcome = {
      payload: {
        direction: descriptor.direction,
        hit: descriptor.hit,
        path: descriptor.path,
        seat,
        shooterInstanceId: shooter.instanceId,
      },
      type: 'projectile-shot',
    };
    if (!descriptor.hit) {
      return [withStateVersion(shotState, {}), [shot, ...interaction.outcomes], []];
    }
    const targetStatus = unitStatus(shotState, descriptor.hit);
    const from: GameLocation = { cell: targetStatus.location, region: targetStatus.region };
    const to: GameLocation = { cell: shooterStatus.location, region: shooterStatus.region };
    const contacted = descriptor.path.at(-1)!;
    const dragPath = [...descriptor.path].reverse().map((location): GameLocation => ({
      cell: translatedFootprint(
        [targetStatus.location],
        contacted.cell,
        location.cell,
      )![0]!,
      region: location.region,
    }));
    const path = resolveDeclaredPath(shotState, descriptor.hit, dragPath, false);
    const actualTo = path.path.at(-1) ?? from;
    const dragged: GameOutcome = {
      payload: {
        from,
        path: path.path,
        seat,
        sourceInstanceId: shooter.instanceId,
        steps: path.path.length - 1,
        targetInstanceId: descriptor.hit.instanceId,
        to: actualTo,
      },
      type: 'unit-dragged',
    };
    const outcomes = [shot, ...interaction.outcomes, dragged, ...path.outcomes];
    if (path.state.pendingDeathrites) {
      // ponytail: serialize the optional arrival fight only if this rare ordered
      // drag interaction becomes an exercised actual-card path.
      throw new Error('unsupported Deathrite ordering during drag-fight continuation');
    }
    const hitArrived = unitRefs(path.state, descriptor.hit.seat).some((candidate) =>
      candidate.instanceId === descriptor.hit!.instanceId
      && candidate.kind === descriptor.hit!.kind
      && unitOccupiesLocation(path.state, candidate, to));
    const shooterRemains = path.state.realm.units.some(({ instanceId }) => instanceId === shooter.instanceId);
    if (!descriptor.fightOnArrival || !hitArrived || !shooterRemains
      || path.state.terminal.status === 'finished') {
      return [withStateVersion(path.state, {}), outcomes, []];
    }
    const pending: PendingCombat = deepFreeze({
      allocations: [],
      attacker: shooter,
      attackingSeat: seat,
      cell: to.cell,
      combatants: [],
      defenders: [],
      originalTarget: descriptor.hit,
      ...(to.region === 'surface' ? {} : { region: to.region }),
      targetRemoved: false,
    });
    return beginFight(path.state, pending, [descriptor.hit], outcomes);
  }

  if (descriptor.kind === 'continue-basic-movement') {
    const pending = state.pendingBasicMovement;
    const legal = state.phase === 'movement'
      && pending?.sourceInstanceId === descriptor.unitInstanceId
      && basicMovementDescriptors(state, seat).some((candidate) =>
        candidate.kind === 'continue-basic-movement'
        && candidate.unitInstanceId === descriptor.unitInstanceId);
    if (!legal || !pending) throw new Error('unreachable illegal basic movement continuation');
    const unit = state.realm.units.find(({ controller, instanceId }) =>
      controller === seat && instanceId === pending.sourceInstanceId);
    if (!unit) {
      return [
        withStateVersion(state, {
          decisionSeat: pending.purpose === 'defend' && state.pendingCombat !== null
            ? seat
            : state.activeSeat,
          pendingBasicMovement: null,
          phase: pending.purpose === 'defend' && state.pendingCombat !== null ? 'defend' : 'main',
        }),
        [],
        [],
      ];
    }
    const ref: GameUnitRef = { instanceId: unit.instanceId, kind: 'minion', seat };
    const reachedPath = pending.path.slice(0, pending.pathIndex + 1);
    const next = pending.path[pending.pathIndex + 1];
    if (!next) {
      return pending.purpose === 'move-and-attack'
        ? finishMoveAndAttackMovement(state, ref, pending.path.at(-1)!, reachedPath, [])
        : finishDefendMovement(state, ref, reachedPath, []);
    }
    const edge = resolveDeclaredPath(
      state,
      ref,
      [pending.path[pending.pathIndex]!, next],
      false,
    );
    const path = [...reachedPath, ...edge.path.slice(1)];
    const sourceRemains = edge.state.realm.units.some(({ controller, instanceId, location, region }) =>
      controller === seat && instanceId === ref.instanceId
      && location === next.cell && region === next.region);
    const combatRemains = pending.purpose === 'move-and-attack' || edge.state.pendingCombat !== null;
    if (!sourceRemains || !combatRemains || edge.state.terminal.status === 'finished') {
      return pending.purpose === 'move-and-attack'
        ? finishMoveAndAttackMovement(edge.state, ref, pending.path.at(-1)!, path, edge.outcomes)
        : finishDefendMovement(edge.state, ref, path, edge.outcomes);
    }
    return [
      withStateVersion(edge.state, {
        decisionSeat: seat,
        pendingBasicMovement: { ...pending, pathIndex: pending.pathIndex + 1 },
        phase: 'movement',
      }),
      [{
        payload: {
          from: edge.path[0]!,
          purpose: pending.purpose,
          seat,
          sourceInstanceId: ref.instanceId,
          to: edge.path.at(-1)!,
        },
        type: 'basic-movement-continued',
      }, ...edge.outcomes],
      [],
    ];
  }

  if (descriptor.kind === 'move-and-attack') {
    const legal = movementDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'move-and-attack'
        && candidate.unitInstanceId === descriptor.unitInstanceId
        && candidate.from.cell === descriptor.from.cell
        && candidate.from.region === descriptor.from.region
        && samePath(candidate.path, descriptor.path)
        && candidate.to.cell === descriptor.to.cell
        && candidate.to.region === descriptor.to.region);
    const ref = unitRefs(state, seat).find(({ instanceId }) => instanceId === descriptor.unitInstanceId);
    if (!legal || !ref) throw new Error('unreachable illegal Move and Attack');
    const definition = ref.kind === 'minion'
      ? cardDefinition(state, state.realm.units.find(({ instanceId }) =>
        instanceId === ref.instanceId)!.cardId)
      : undefined;
    if (definition?.cardType === 'minion'
      && definition.mayRangedStrikeOnceDuringBasicMovement === true) {
      return beginBasicMovement(state, ref, descriptor.path, 'move-and-attack');
    }
    const path = resolveDeclaredPath(state, ref, descriptor.path, true);
    if (path.state.pendingDeathrites) {
      const activated: GameOutcome = {
        payload: {
          from: path.path[0]!,
          path: path.path,
          seat: ref.seat,
          steps: path.path.length - 1,
          to: path.path.at(-1)!,
          unitInstanceId: ref.instanceId,
        },
        type: 'move-and-attack-activated',
      };
      return [
        withStateVersion(path.state, {
          decisionSeat: seat,
          pendingBasicMovement: {
            activationEmitted: true,
            path: descriptor.path,
            pathIndex: path.path.length - 1,
            purpose: 'move-and-attack',
            rangedStrikeUsed: false,
            seat,
            sourceInstanceId: ref.instanceId,
          },
          phase: 'movement',
        }),
        [activated, ...path.outcomes],
        [],
      ];
    }
    return finishMoveAndAttackMovement(path.state, ref, descriptor.to, path.path, path.outcomes);
  }

  if (descriptor.kind === 'decline-attack') {
    const pending = pendingCombat(state);
    const interceptors = responseUnitRefs(state, pending, true);
    return [
      withStateVersion(state, interceptors.length > 0
        ? {
          decisionSeat: otherSeat(pending.attackingSeat),
          phase: 'intercept',
        }
        : {
          decisionSeat: pending.attackingSeat,
          pendingCombat: null,
          phase: 'main',
        }),
      [{
        payload: {
          interceptWindowOpened: interceptors.length > 0,
          seat: pending.attackingSeat,
          unitInstanceId: pending.attacker.instanceId,
        },
        type: 'attack-declined',
      }],
      [],
    ];
  }

  if (descriptor.kind === 'declare-attack') {
    const pending = pendingCombat(state);
    const legal = attackTargets(state, pending).some((target) =>
      target.instanceId === descriptor.target.instanceId
        && target.kind === descriptor.target.kind
        && target.seat === descriptor.target.seat);
    if (!legal) throw new Error('unreachable illegal attack target');
    const attackerCells = new Set(unitRefOccupiedCells(state, pending.attacker));
    const contestedCell = descriptor.target.kind === 'site'
      ? (Object.entries(state.realm.sites)
        .find(([, site]) => site.instanceId === descriptor.target.instanceId)?.[0] as RealmCell | undefined)
      : unitRefOccupiedCells(state, descriptor.target)
        .filter((cell) => attackerCells.has(cell))
        .sort()[0];
    if (!contestedCell) throw new Error('unreachable attack without contested location');
    const declaredPending = deepFreeze({
      ...pending,
      cell: contestedCell,
      originalTarget: descriptor.target,
    });
    const declared: GameOutcome = {
      payload: {
        attackerInstanceId: pending.attacker.instanceId,
        cell: contestedCell,
        ...(pending.region ? { region: pending.region } : {}),
        seat: pending.attackingSeat,
        target: descriptor.target,
      },
      type: 'attack-declared',
    };
    if (unitStatus(state, pending.attacker).stealthed) {
      return descriptor.target.kind === 'site'
        ? strikeUndefendedSite(state, declaredPending, [declared])
        : beginFight(state, declaredPending, [descriptor.target], [declared]);
    }
    return [
      withStateVersion(state, {
        decisionSeat: otherSeat(pending.attackingSeat),
        pendingCombat: declaredPending,
        phase: 'defend',
      }),
      [declared],
      [],
    ];
  }

  if (descriptor.kind === 'defend') {
    const legal = actionDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'defend'
        && candidate.unitInstanceId === descriptor.unitInstanceId
        && candidate.from.cell === descriptor.from.cell
        && candidate.from.region === descriptor.from.region
        && samePath(candidate.path, descriptor.path)
        && candidate.to.cell === descriptor.to.cell
        && candidate.to.region === descriptor.to.region);
    const ref = unitRefs(state, seat).find(({ instanceId }) => instanceId === descriptor.unitInstanceId);
    if (!legal || !ref) {
      throw new Error('unreachable illegal defender');
    }
    const definition = ref.kind === 'minion'
      ? cardDefinition(state, state.realm.units.find(({ instanceId }) =>
        instanceId === ref.instanceId)!.cardId)
      : undefined;
    if (definition?.cardType === 'minion'
      && definition.mayRangedStrikeOnceDuringBasicMovement === true) {
      return beginBasicMovement(state, ref, descriptor.path, 'defend');
    }
    const path = resolveDeclaredPath(state, ref, descriptor.path, true);
    if (path.state.pendingDeathrites) {
      const destination = state.pendingCombat
        ? {
          cell: state.pendingCombat.cell,
          region: state.pendingCombat.region ?? 'surface' as const,
        }
        : path.path.at(-1)!;
      const defenderArrived = state.pendingCombat !== null
        && unitRefs(path.state, ref.seat).some((candidate) =>
          candidate.instanceId === ref.instanceId
            && candidate.kind === ref.kind
            && unitOccupiesLocation(path.state, candidate, destination));
      const movement: GameOutcome = {
        payload: {
          from: path.path[0]!,
          instanceId: ref.instanceId,
          path: path.path,
          seat: ref.seat,
          steps: path.path.length - 1,
          to: path.path.at(-1)!,
        },
        type: defenderArrived ? 'defender-joined' : 'defender-moved',
      };
      return [
        withStateVersion(path.state, {
          decisionSeat: seat,
          pendingBasicMovement: {
            activationEmitted: true,
            path: descriptor.path,
            pathIndex: path.path.length - 1,
            purpose: 'defend',
            rangedStrikeUsed: false,
            seat,
            sourceInstanceId: ref.instanceId,
          },
          phase: 'movement',
        }),
        [movement, ...path.outcomes],
        [],
      ];
    }
    return finishDefendMovement(path.state, ref, path.path, path.outcomes);
  }

  if (descriptor.kind === 'intercept') {
    const pending = pendingCombat(state);
    const ref = responseUnitRefs(state, pending, true)
      .find(({ instanceId }) => instanceId === descriptor.unitInstanceId);
    if (!ref) throw new Error('unreachable illegal interceptor');
    const interceptor = unitStatus(state, ref);
    const destination: GameLocation = { cell: interceptor.location, region: interceptor.region };
    const tapped = moveAndTapUnit(state, ref, destination);
    return [
      withStateVersion(state, {
        pendingCombat: deepFreeze({
          ...pending,
          defenders: [...pending.defenders, ref],
        }),
        players: tapped.players,
        realm: tapped.realm,
      }),
      [{
        payload: {
          cell: pending.cell,
          instanceId: ref.instanceId,
          ...(pending.region ? { region: pending.region } : {}),
          seat,
        },
        type: 'interceptor-joined',
      }],
      [],
    ];
  }

  if (descriptor.kind === 'close-defend') {
    const pending = pendingCombat(state);
    const legal = actionDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'close-defend'
        && candidate.originalTargetParticipates === descriptor.originalTargetParticipates);
    if (!legal || !pending.originalTarget) throw new Error('unreachable illegal defend close');
    const outcomes: GameOutcome[] = [{
      payload: {
        defenderCount: pending.defenders.length,
        originalTargetParticipates: descriptor.originalTargetParticipates,
      },
      type: 'defend-window-closed',
    }];
    if (pending.originalTarget.kind === 'site') {
      return pending.defenders.length === 0
        ? strikeUndefendedSite(state, pending, outcomes)
        : beginFight(state, pending, pending.defenders, outcomes);
    }
    if (!descriptor.originalTargetParticipates) {
      outcomes.push({
        payload: { instanceId: pending.originalTarget.instanceId, kind: pending.originalTarget.kind },
        type: 'original-target-removed',
      });
    }
    return beginFight(
      state,
      deepFreeze({ ...pending, targetRemoved: !descriptor.originalTargetParticipates }),
      [
        ...pending.defenders,
        ...(descriptor.originalTargetParticipates ? [pending.originalTarget] : []),
      ],
      outcomes,
    );
  }

  if (descriptor.kind === 'close-intercept') {
    const pending = pendingCombat(state);
    const closed: GameOutcome = {
      payload: { interceptorCount: pending.defenders.length },
      type: 'intercept-window-closed',
    };
    if (pending.defenders.length === 0) {
      return [
        withStateVersion(state, {
          decisionSeat: pending.attackingSeat,
          pendingCombat: null,
          phase: 'main',
        }),
        [closed],
        [],
      ];
    }
    return beginFight(state, pending, pending.defenders, [closed]);
  }

  if (descriptor.kind === 'allocate-strike') {
    const pending = pendingCombat(state);
    const legal = actionDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'allocate-strike'
        && candidate.amount === descriptor.amount
        && candidate.targetInstanceId === descriptor.targetInstanceId);
    if (!legal) throw new Error('unreachable illegal strike allocation');
    const updated = deepFreeze({
      ...pending,
      allocations: [...pending.allocations, {
        amount: descriptor.amount,
        targetInstanceId: descriptor.targetInstanceId,
      }],
    });
    const allocated: GameOutcome = {
      payload: {
        amount: descriptor.amount,
        strikerInstanceId: pending.attacker.instanceId,
        targetInstanceId: descriptor.targetInstanceId,
      },
      type: 'strike-damage-allocated',
    };
    if (updated.allocations.length === updated.combatants.length) {
      return finishFight(state, updated, [allocated]);
    }
    return [
      withStateVersion(state, { pendingCombat: updated }),
      [allocated],
      [],
    ];
  }

  if (descriptor.kind === 'draw' || descriptor.kind === 'draw-site' || descriptor.kind === 'draw-spell') {
    const avatarDraw = descriptor.kind !== 'draw';
    const zone = descriptor.kind === 'draw-site'
      ? 'atlas'
      : descriptor.kind === 'draw-spell'
        ? 'spellbook'
        : descriptor.zone;
    const deck = player[zone];
    const activatedAvatar = {
      ...player.avatar,
      ...(descriptor.kind === 'draw-spell' ? { lastInteractedTurn: state.turnNumber } : {}),
      tapped: true,
    };
    if (deck.length === 0) {
      const winner = otherSeat(seat);
      const players = avatarDraw
        ? replacePlayer(state, seat, deepFreeze({
          ...player,
          avatar: activatedAvatar,
        }))
        : state.players;
      return [
        withStateVersion(state, {
          phase: 'terminal',
          players,
          terminal: { loser: seat, reason: 'deck_empty', status: 'finished', winner },
        }),
        [{ payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' }],
        [],
      ];
    }
    const [drawn, ...remaining] = deck;
    const updatedPlayer = deepFreeze({
      ...player,
      ...(avatarDraw ? { avatar: activatedAvatar } : {}),
      [zone]: remaining,
      hand: { ...player.hand, [zone]: [...player.hand[zone], drawn!] },
    });
    return [
      withStateVersion(state, {
        phase: 'main',
        players: replacePlayer(state, seat, updatedPlayer),
      }),
      [descriptor.kind === 'draw-site'
        ? { payload: { seat }, type: 'site-drawn' }
        : descriptor.kind === 'draw-spell'
          ? { payload: { seat }, type: 'spell-drawn' }
          : { payload: { seat, zone }, type: 'card-drawn' }],
      [],
    ];
  }

  if (descriptor.kind !== 'end-turn') throw new Error('unreachable unsupported action');
  const endOfTurnLifeLoss = resumeEndTurn
    ? { outcomes: [] as readonly GameOutcome[], state }
    : resolveEndOfEachTurnSiteControllerLifeLoss(state, seat);
  const endOfTurnDeaths = resumeEndTurn
    ? { outcomes: [] as readonly GameOutcome[], state: endOfTurnLifeLoss.state }
    : resolveEndOfTurnDeaths(endOfTurnLifeLoss.state, seat);
  if (endOfTurnDeaths.state.pendingDeathrites) {
    return [
      withStateVersion(endOfTurnDeaths.state, {}),
      [...endOfTurnLifeLoss.outcomes, ...endOfTurnDeaths.outcomes],
      [],
    ];
  }
  const endOfTurnPowerDeaths = resumeEndTurn
    ? { outcomes: [] as readonly GameOutcome[], state: endOfTurnDeaths.state }
    : settleStaticPowerDeaths(endOfTurnDeaths.state);
  const endState = endOfTurnPowerDeaths.state;
  if (endState.terminal.status === 'finished') {
    return [
      withStateVersion(endState, { pendingCombat: null, phase: 'terminal' }),
      [
        ...endOfTurnLifeLoss.outcomes,
        ...endOfTurnDeaths.outcomes,
        ...endOfTurnPowerDeaths.outcomes,
      ],
      [],
    ];
  }
  if (resumeEndTurn !== 'after-auras') {
    const endTurnAura = beginEndTurnAura(endState, seat, endTurnDamageAuraIds(endState, seat));
    if (endTurnAura) {
      return [
        withStateVersion(endTurnAura[0], {}),
        [
          ...endOfTurnLifeLoss.outcomes,
          ...endOfTurnDeaths.outcomes,
          ...endOfTurnPowerDeaths.outcomes,
          ...endTurnAura[1],
        ],
        endTurnAura[2],
      ];
    }
  }
  const nextSeat = otherSeat(seat);
  const {
    temporaryPowerSources: avatarPowerSources,
    ...endingAvatar
  } = endState.players[seat].avatar;
  const powerExpired: GameOutcome[] = (avatarPowerSources ?? []).map((sourceInstanceId) => ({
    payload: {
      amount: 2,
      instanceId: endingAvatar.card.instanceId,
      seat,
      sourceInstanceId,
    },
    type: 'power-expired',
  }));
  const endingPlayer = deepFreeze({
    ...endState.players[seat],
    ...(endState.players[seat].airThresholdsCastThisTurn !== undefined
      ? { airThresholdsCastThisTurn: 0 }
      : {}),
    avatar: deepFreeze(endingAvatar),
    mana: 0,
  });
  const nextPlayer = endState.players[nextSeat];
  const startingPlayer = deepFreeze({
    ...nextPlayer,
    ...(nextPlayer.airThresholdsCastThisTurn !== undefined
      ? { airThresholdsCastThisTurn: 0 }
      : {}),
    avatar: { ...nextPlayer.avatar, tapped: false },
    mana: siteCount(endState, nextSeat),
  });
  const {
    auras: previousAuras,
    immobileAreas: previousImmobileAreas,
    ...endingRealm
  } = endState.realm;
  let players = deepFreeze({ ...endState.players, [seat]: endingPlayer, [nextSeat]: startingPlayer });
  const countedAuras = (previousAuras ?? [])
    .filter(({ controller }) => controller === seat);
  const expiredAuras = countedAuras.filter(({ turnCounters }) => turnCounters >= 2);
  const expiredAuraIds = new Set(expiredAuras.map(({ instanceId }) => instanceId));
  const activeAuras = (previousAuras ?? []).flatMap((aura): readonly AuraInstance[] => {
    if (aura.controller !== seat) return [aura];
    const turnCounters = aura.turnCounters + 1;
    return turnCounters < 3 ? [deepFreeze({ ...aura, turnCounters })] : [];
  });
  for (const aura of expiredAuras) {
    const owner = players[aura.owner];
    const card: CardInstance = deepFreeze({
      cardId: aura.cardId,
      instanceId: aura.instanceId,
      owner: aura.owner,
      source: aura.source,
    });
    players = deepFreeze({
      ...players,
      [aura.owner]: deepFreeze({ ...owner, cemetery: [...owner.cemetery, card] }),
    });
  }
  const stealthGained = endState.realm.units.filter((unit) => {
    if (unit.controller !== seat || minionDisabled(endState, unit) || unit.stealthed) return false;
    const definition = cardDefinition(endState, unit.cardId);
    return definition.cardType === 'minion' && definition.gainsStealthAtEndOfTurn === true;
  });
  const stealthGainedIds = new Set(stealthGained.map(({ instanceId }) => instanceId));
  const endPhaseUntapped = endState.realm.units.filter((unit) => {
    if (unit.controller !== seat || !unit.tapped || minionDisabled(endState, unit)) return false;
    const definition = cardDefinition(endState, unit.cardId);
    return definition.cardType === 'minion'
      && definition.untapsAtEndOfControllerTurn === true;
  });
  const endPhaseUntappedIds = new Set(endPhaseUntapped.map(({ instanceId }) => instanceId));
  const expiredDisableEffects = endState.realm.units.flatMap((unit) =>
    (unit.disableEffects ?? [])
      .filter(({ expiresAtSeat }) => expiresAtSeat === nextSeat)
      .map((effect) => ({ effect, unit })));
  const immobileAreas = (previousImmobileAreas ?? [])
    .filter(({ expiresAtSeat, sourceInstanceId }) =>
      expiresAtSeat !== nextSeat && !expiredAuraIds.has(sourceInstanceId));
  const chargeExpired: GameOutcome[] = [];
  const units = endState.realm.units.map((unit) => {
    const {
      disableEffects: previousDisableEffects,
      temporaryChargeSources,
      temporaryPowerSources,
      ...baseUnit
    } = unit;
    const disableEffects = (previousDisableEffects ?? [])
      .filter(({ expiresAtSeat }) => expiresAtSeat !== nextSeat);
    for (const sourceInstanceId of temporaryChargeSources ?? []) {
      chargeExpired.push({
        payload: { instanceId: unit.instanceId, seat: unit.controller, sourceInstanceId },
        type: 'charge-expired',
      });
    }
    for (const sourceInstanceId of temporaryPowerSources ?? []) {
      powerExpired.push({
        payload: {
          amount: 2,
          instanceId: unit.instanceId,
          seat: unit.controller,
          sourceInstanceId,
        },
        type: 'power-expired',
      });
    }
    return deepFreeze({
      ...baseUnit,
      ...(disableEffects.length > 0 ? { disableEffects } : {}),
      damage: 0,
      ...(stealthGainedIds.has(unit.instanceId) ? { stealthed: true } : {}),
      ...(unit.controller === seat ? { summoningSickness: false } : {}),
      ...(unit.controller === nextSeat || endPhaseUntappedIds.has(unit.instanceId)
        ? { tapped: false }
        : {}),
    });
  });
  const turnNumber = endState.turnNumber + 1;
  const nextTurnState = deepFreeze({
    ...endState,
    activeSeat: nextSeat,
    decisionSeat: nextSeat,
    pendingCombat: null,
    phase: 'draw' as const,
    players,
    realm: {
      ...endingRealm,
      ...(activeAuras.length > 0 ? { auras: activeAuras } : {}),
      ...(immobileAreas.length > 0 ? { immobileAreas } : {}),
      units,
    },
    turnNumber,
  });
  const startTurnTriggerIds = startTurnTriggerInstanceIds(nextTurnState, nextSeat);
  const startedState: GameState = startTurnTriggerIds.length === 0
    ? nextTurnState
    : deepFreeze({
      ...nextTurnState,
      pendingStartTurn: {
        remainingTriggerInstanceIds: startTurnTriggerIds,
        seat: nextSeat,
      },
      phase: 'start-turn',
    });
  return [
    withStateVersion(startedState, {}),
    [
      ...endOfTurnLifeLoss.outcomes,
      ...endOfTurnDeaths.outcomes,
      ...endOfTurnPowerDeaths.outcomes,
      ...endPhaseUntapped.map(({ controller, instanceId }) => ({
        payload: { instanceId, seat: controller, sourceInstanceId: instanceId },
        type: 'minion-untapped',
      })),
      ...stealthGained.map(({ controller, instanceId }) => ({
        payload: { instanceId, seat: controller },
        type: 'stealth-gained',
      })),
      ...chargeExpired,
      ...powerExpired,
      ...countedAuras.map(({ controller, instanceId, turnCounters }) => ({
        payload: {
          count: turnCounters + 1,
          instanceId,
          seat: controller,
          sourceInstanceId: instanceId,
        },
        type: 'aura-turn-counted',
      })),
      ...expiredAuras.map(({ controller, instanceId, owner }) => ({
        payload: { instanceId, owner, seat: controller, sourceInstanceId: instanceId },
        type: 'aura-dispelled',
      })),
      { payload: { seat, turnNumber: endState.turnNumber }, type: 'turn-ended' },
      ...expiredDisableEffects.map(({ effect, unit }) => ({
        payload: {
          instanceId: unit.instanceId,
          seat: unit.controller,
          sourceInstanceId: effect.sourceInstanceId,
        },
        type: 'minion-disable-expired',
      })),
      { payload: { drawSkipped: false, seat: nextSeat, turnNumber }, type: 'turn-started' },
    ],
    [],
  ];
}

export function stepGame(session: GameSession, request: GameActionRequest): GameStepResult {
  const state = session.state;
  const command: GameActionRequest = deepFreeze({
    actionId: request.actionId,
    seat: request.seat,
    stateVersion: request.stateVersion,
  });
  const stateHash = hashGameState(state);
  const reject = (code: EngineRejection['code']): GameStepResult => {
    const reason = createRejection(code, state.stateVersion, stateHash);
    const attempt = createAttempt(
      session.attempts.length + 1,
      command,
      state.stateVersion,
      stateHash,
      { reasonCode: code },
    );
    return deepFreeze({
      accepted: false,
      reason,
      session: { ...session, attempts: [...session.attempts, attempt] },
    });
  };

  if (state.terminal.status === 'finished') return reject('terminal_state');
  if (command.stateVersion !== state.stateVersion) return reject('stale_version');
  if (command.seat !== state.decisionSeat) return reject('wrong_seat');
  const action = legalGameActions(state, command.seat).find(({ actionId }) => actionId === command.actionId);
  if (!action) return reject('unknown_action');

  const receiptSequence = session.transcript.length + 1;
  const firstEventSequence = session.transcript.reduce((count, receipt) => count + receipt.events.length, 0) + 1;
  const [appliedState, appliedOutcomes, randomDraws] = applyDescriptor(
    state,
    action.descriptor,
    session.manifest,
  );
  const stealthSettlement = settleNearbyEnemyStealth(appliedState);
  const powerSettlement = stealthSettlement.state.pendingDeathrites
    ? { outcomes: [] as readonly GameOutcome[], state: stealthSettlement.state }
    : settleStaticPowerDeaths(stealthSettlement.state);
  const settlementOutcomes = [...stealthSettlement.outcomes, ...powerSettlement.outcomes];
  const completionIndex = appliedOutcomes.findIndex(({ type }) =>
    type === 'game-ended' || type === 'magic-resolved' || type === 'turn-ended');
  const settlementEndIndex = settlementOutcomes.findIndex(({ type }) =>
    type === 'game-ended');
  const settlementBeforeCompletion = settlementEndIndex < 0
    ? settlementOutcomes
    : settlementOutcomes.slice(0, settlementEndIndex);
  const settlementAfterCompletion = settlementEndIndex < 0
    ? []
    : settlementOutcomes.slice(settlementEndIndex);
  const orderedOutcomes = settlementOutcomes.length === 0
    ? appliedOutcomes
    : completionIndex < 0
      ? [...appliedOutcomes, ...settlementOutcomes]
      : [
        ...appliedOutcomes.slice(0, completionIndex),
        ...settlementBeforeCompletion,
        ...appliedOutcomes.slice(completionIndex),
        ...settlementAfterCompletion,
      ];
  const deferredIndex = powerSettlement.state.pendingDeathrites
    ? orderedOutcomes.findIndex(({ type }) => type === 'magic-resolved' || type === 'turn-ended')
    : -1;
  const completionState = deferredIndex < 0
    ? powerSettlement.state
    : deepFreeze({
      ...powerSettlement.state,
      pendingDeathrites: {
        ...powerSettlement.state.pendingDeathrites!,
        deferredOutcomes: [
          ...(powerSettlement.state.pendingDeathrites!.deferredOutcomes ?? []),
          orderedOutcomes[deferredIndex]!,
        ],
      },
    });
  const outcomes = deferredIndex < 0
    ? orderedOutcomes
    : orderedOutcomes.filter((_, index) => index !== deferredIndex);
  const rangedState = state.phase !== 'movement'
    && action.descriptor.kind === 'shoot-projectile' && action.descriptor.hit
    ? queueRangedStep(completionState, action.descriptor.shooterInstanceId)
    : completionState;
  const nextState = exposeDeathriteOrder(rangedState);
  const events: readonly EngineEvent[] = createEvents(
    command.actionId,
    receiptSequence,
    firstEventSequence,
    outcomes,
  );
  const receipt = createReceipt({
    actionId: command.actionId,
    events,
    nextStateVersion: nextState.stateVersion,
    postStateHash: hashGameState(nextState),
    preStateHash: stateHash,
    randomDraws,
    receiptSequence,
    seat: command.seat,
    stateVersion: state.stateVersion,
  });
  const attempt = createAttempt(
    session.attempts.length + 1,
    command,
    state.stateVersion,
    stateHash,
    { receiptId: receipt.receiptId },
  );
  return deepFreeze({
    accepted: true,
    receipt,
    session: {
      ...session,
      attempts: [...session.attempts, attempt],
      state: nextState,
      transcript: [...session.transcript, receipt],
    },
  });
}

export function replayGame(manifest: GameManifest, actionIds: readonly string[]): GameSession {
  let session = createGameSession(manifest);
  for (const actionId of actionIds) {
    const result = stepGame(session, {
      actionId,
      seat: session.state.decisionSeat,
      stateVersion: session.state.stateVersion,
    });
    if (!result.accepted) throw new Error(`game replay rejected action: ${result.reason.code}`);
    session = result.session;
  }
  return session;
}

export function verifyGameReplay(expected: GameSession): boolean {
  try {
    const replayed = replayGame(expected.manifest, expected.transcript.map(({ actionId }) => actionId));
    return canonicalJson({
      initialRandomDraws: replayed.initialRandomDraws,
      state: replayed.state,
      transcript: replayed.transcript,
    }) === canonicalJson({
      initialRandomDraws: expected.initialRandomDraws,
      state: expected.state,
      transcript: expected.transcript,
    });
  } catch {
    return false;
  }
}
