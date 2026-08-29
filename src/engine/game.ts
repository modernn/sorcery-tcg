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
const NORTH_START = 'C4';
const SOUTH_START = 'C1';

export type GameSeat = EngineSeat;
export type DeckZone = 'atlas' | 'spellbook';
export type RealmCell = `${'A' | 'B' | 'C' | 'D' | 'E'}${1 | 2 | 3 | 4}`;
export type GameElement = 'air' | 'earth' | 'fire' | 'water';
export type GameThresholds = Readonly<Record<GameElement, number>>;
export type GameRegion = 'surface' | 'underground' | 'underwater' | 'void';
type MovementPurpose = 'defend' | 'effect' | 'move-and-attack';

const REALM_CELLS = (['A', 'B', 'C', 'D', 'E'] as const)
  .flatMap((file) => ([1, 2, 3, 4] as const).map((rank) => `${file}${rank}` as RealmCell));

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
    cardType: 'artifact';
    grantsBearerLethal?: never;
    grantsBearerPower: 2;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    cardType: 'artifact';
    grantsBearerLethal: true;
    grantsBearerPower?: never;
    manaCost: number;
    thresholds: GameThresholds;
  }>
  | Readonly<{
    airborneMinionsAtopMoveFreelyAway?: true;
    blocksGroundMinionEntryWhileMinionAtop?: true;
    cardType: 'site';
    connectsBurrowedAllies?: boolean;
    elements: readonly GameElement[];
    genesisDiscardTopSpells?: 2;
    genesisDrawSpellPerAdjacentSameCard?: boolean;
    genesisEnemiesLoseStealth?: true;
    genesisGainMana?: number;
    genesisGainManaIfOnlyControlledCopy?: 1;
    genesisImmobilizeNearbyUntilNextTurn?: true;
    genesisMayBottomNextSpell?: true;
    genesisPayOneManaToSummonToken?: string;
    ordinaryMinionManaDiscount?: 1;
    rangedUnitsHereRangeBonus?: 1;
    sacrificeToDestroyNearbySite?: true;
  }>
  | Readonly<{
    burrowTargetMinionOrArtifact?: true;
    cardType: 'magic';
    damageEachAbovegroundMinion?: 1;
    damageEachUnitAtLocationWithinTwoSteps?: number;
    damageRandomUnitAtLocation?: number;
    damageTargetUnit?: number;
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
    summonTokenToEachControlledSiteBorderingEnemySite?: string;
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
    deathriteHeal?: number;
    deathriteDrawSite?: boolean;
    deathriteLoseLifePerNearbySiteControlled?: 1;
    defense: number;
    discardRandomCardInsteadOfMana?: true;
    diesAtEndOfControllerTurn?: true;
    genesisDamageEachOtherUnitHere?: 1;
    genesisMayDamageTargetAdjacentUnit?: 2;
    genesisDrawSpell?: boolean;
    genesisDrawSite?: boolean;
    genesisHealController?: 2;
    genesisLoseControllerLife?: 2;
    gainsStealthAtEndOfTurn?: boolean;
    immobile?: boolean;
    lanceCount?: 1 | 2 | 3;
    lethal?: boolean;
    manaCost: number;
    movementBonus?: 1 | 2;
    movesOnlyForward?: boolean;
    movesOnlySideways?: boolean;
    mustBeCastBurrowed?: boolean;
    mustBeCastSubmerged?: boolean;
    mustBeCastToWaterSite?: boolean;
    nearbyEnemiesPermanentlyLoseStealth?: true;
    ordinary?: true;
    otherNearbyAlliesPowerBonus?: 1;
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

type SiteInstance = Readonly<CardInstance & { controller: GameSeat }>;

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
  expiresAtSeat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type UnitInstance = Readonly<CardInstance & {
  // ponytail: intrinsic Lance marks omit Artifact transfer/drop; promote them to realm Artifacts when a supported card needs it.
  carriedLanceCount?: number;
  controller: GameSeat;
  damage: number;
  disableEffects?: readonly DisableEffect[];
  lastDroppedArtifactsTurn?: number;
  lastInteractedTurn?: number;
  lastPickedUpArtifactsTurn?: number;
  location: RealmCell;
  region: GameRegion;
  stealthed: boolean;
  summoningSickness: boolean;
  tapped: boolean;
  temporaryChargeSources?: readonly StateHash[];
  temporaryPowerSources?: readonly StateHash[];
  warded: boolean;
}>;

type ArtifactInstance = Readonly<CardInstance & (
  | Readonly<{ bearer: GameUnitRef }>
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

type PendingGenesisSpell = Readonly<{
  seat: GameSeat;
  sourceInstanceId: StateHash;
}>;

type PendingGenesisToken = Readonly<{
  cell: RealmCell;
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
  pendingCombat: PendingCombat | null;
  pendingGenesisSpell?: PendingGenesisSpell | null;
  pendingGenesisToken?: PendingGenesisToken | null;
  phase: 'allocate' | 'attack' | 'defend' | 'draw' | 'genesis' | 'intercept' | 'main' | 'mulligan' | 'terminal';
  players: Readonly<Record<GameSeat, PlayerState>>;
  realm: Readonly<{
    artifacts?: readonly ArtifactInstance[];
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
    kind: 'activate-site-destruction';
    sourceSiteInstanceId: StateHash;
    targetCell: RealmCell;
    targetSiteInstanceId: StateHash;
  }>
  | Readonly<{
    bearer?: GameUnitRef;
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
    cemeteryMinionInstanceId?: StateHash;
    drawZone?: DeckZone;
    kind: 'cast-magic';
    ally?: GameUnitRef;
    allyDestination?: GameLocation;
    target?: GameUnitRef;
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
    direction: ProjectileDirection;
    hit: GameUnitRef | null;
    kind: 'shoot-projectile';
    path: readonly GameLocation[];
    shooterInstanceId: StateHash;
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
    kind: 'pick-up-artifacts';
    unit: GameUnitRef;
  }>
  | Readonly<{
    artifactInstanceIds: readonly StateHash[];
    kind: 'drop-artifacts';
    unit: GameUnitRef;
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
    kind: 'activate-sparkmage';
    sourceInstanceId: StateHash;
    targetLocation: GameLocation;
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

function summonDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const casters = spellcasterRefs(state, seat);
  const controlledCells = controlledSiteCells(state, seat);
  const siteCells = Object.keys(state.realm.sites).sort() as RealmCell[];
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'minion'
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    const summonCells = (definition.summonToAnySite ? siteCells : controlledCells)
      .filter((cell) => !definition.mustBeCastToOuterColumn || cell[0] === 'A' || cell[0] === 'E')
      .filter((cell) => !definition.mustBeCastToWaterSite || isWaterSite(state, cell));
    const summonLocations: readonly Readonly<{
      cell: RealmCell;
      region?: 'underground' | 'underwater' | 'void';
    }>[] = [
      ...(!definition.mustBeCastBurrowed && !definition.mustBeCastSubmerged
        ? summonCells.map((cell) => ({ cell }))
        : []),
      ...(definition.burrowing && !definition.mustBeCastSubmerged
        ? summonCells.filter((cell) => !isWaterSite(state, cell))
          .map((cell) => ({ cell, region: 'underground' as const }))
        : []),
      ...(definition.submerge && !definition.mustBeCastBurrowed
        ? summonCells.filter((cell) => isWaterSite(state, cell))
          .map((cell) => ({ cell, region: 'underwater' as const }))
        : []),
      ...(definition.voidwalk && !definition.mustBeCastBurrowed && !definition.mustBeCastSubmerged
        && !definition.mustBeCastToWaterSite
        ? REALM_CELLS.filter((cell) => !state.realm.sites[cell]
          && (!definition.mustBeCastToOuterColumn || cell[0] === 'A' || cell[0] === 'E'))
          .map((cell) => ({ cell, region: 'void' as const }))
        : []),
    ];
    return casters.flatMap(({ instanceId: casterInstanceId }) =>
      summonLocations.flatMap(({ cell, region }) => {
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
            && unit.location === cell
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
          .filter(({ manaCost }) => player.mana >= manaCost);
        const genesisChoices = definition.genesisMayDamageTargetAdjacentUnit === 2
          ? [
            { genesisDamageChoice: 'decline' as const },
            ...[
              { instanceId, kind: 'minion' as const, seat },
              ...(['north', 'south'] as const)
                .flatMap((targetSeat) => unitRefs(state, targetSeat))
                .filter((target) => {
                  const status = unitStatus(state, target);
                  return status.region === exactRegion
                    && (status.location === cell
                      || borderingCells(cell).includes(status.location))
                    && (target.seat === seat || !status.stealthed);
                }),
            ].sort((left, right) => left.instanceId.localeCompare(right.instanceId))
              .map((genesisDamageTarget) => ({
                genesisDamageChoice: 'target' as const,
                genesisDamageTarget,
              })),
          ]
          : [{}];
        return [...basePaymentOptions, ...sacrificePayments].flatMap((payment) =>
          genesisChoices.map((choice) => ({
            cardId,
            cardInstanceId: instanceId,
            casterInstanceId,
            cell,
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
      ...bearers.map((bearer) => ({
        bearer,
        cardId,
        cardInstanceId: instanceId,
        casterInstanceId,
        kind: 'cast-artifact' as const,
        manaCost: definition.manaCost,
      })),
    ]);
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
    const artifactInstanceIds = uncarried
      .filter(({ location, region }) =>
        location === status.location && region === status.region)
      .map(({ instanceId }) => instanceId)
      .sort();
    // ponytail: all subsets are exponential; use staged selection if supported local Artifact counts grow large.
    return nonemptyCombinations(artifactInstanceIds, artifactInstanceIds.length)
      .map((ids) => ({
        artifactInstanceIds: ids,
        kind: 'pick-up-artifacts' as const,
        unit,
      }));
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

function magicDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const casters = spellcasterRefs(state, seat);
  const targets = (['north', 'south'] as const).flatMap((targetSeat) => unitRefs(state, targetSeat));
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'magic'
      || player.mana < definition.manaCost
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    return casters.flatMap((casterRef) => {
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
          status.voidwalk,
          status.connectsTopBottom,
          status.immobile,
          ally.kind === 'minion',
          true,
        ).map((path) => path.at(-1)!);
        return [...new Map(destinations.map((allyDestination) => [
          `${allyDestination.cell}:${allyDestination.region}`,
          allyDestination,
        ])).values()].map((allyDestination) => ({ ...cast, ally, allyDestination }));
      });
    }
    if (definition.fightAllyWithAdjacentEnemy === true) {
      return unitRefs(state, seat).flatMap((ally) => {
        const allyStatus = unitStatus(state, ally);
        return unitRefs(state, otherSeat(seat)).filter((target) => {
          const targetStatus = unitStatus(state, target);
          return targetStatus.region === allyStatus.region
            && !targetStatus.stealthed
            && (targetStatus.location === allyStatus.location
              || borderingCells(allyStatus.location).includes(targetStatus.location));
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
          && nearby.has(status.location)
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
        const nearbySiteCells = new Set([
          allyStatus.location,
          ...borderingCells(allyStatus.location),
          ...diagonalCells(allyStatus.location),
        ]);
        return unitRefs(state, otherSeat(seat)).flatMap((temptedEnemy) => {
          if (temptedEnemy.kind !== 'minion') return [];
          const enemyStatus = unitStatus(state, temptedEnemy);
          if (enemyStatus.disabled
            || enemyStatus.region === 'void'
            || !state.realm.sites[enemyStatus.location]
            || !nearbySiteCells.has(enemyStatus.location)) return [];
          const from: GameLocation = { cell: enemyStatus.location, region: enemyStatus.region };
          const startingDistance = cardinalCellDistance(enemyStatus.location, allyStatus.location);
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
            enemyStatus.voidwalk,
            enemyStatus.connectsTopBottom,
            enemyStatus.immobile,
            true,
            true,
          ).flatMap((path) => path.length === 2 ? [path[1]!] : [])
            .filter(({ cell }) => cardinalCellDistance(cell, allyStatus.location) < startingDistance);
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
    if (definition.teleportNearbyAllyThenDrawCard === true) {
      return unitRefs(state, seat).flatMap((ally) => {
        const status = unitStatus(state, ally);
        return [
          status.location,
          ...borderingCells(status.location),
          ...diagonalCells(status.location),
        ].flatMap((cell) => {
          const targetLocation = { cell, region: status.region };
          if (!locationExists(state, targetLocation)) return [];
          const site = state.realm.sites[cell];
          return (['atlas', 'spellbook'] as const).map((drawZone) => ({
            ...cast,
            ally,
            drawZone,
            targetLocation,
            ...(site ? { targetSiteInstanceId: site.instanceId } : {}),
          }));
        });
      });
    }
    if (definition.teleportAllyToTargetSite === true) {
      if (caster.region !== 'surface') return [];
      const targetSites = REALM_CELLS.flatMap((cell) => {
        const site = state.realm.sites[cell];
        return site ? [{ site, targetLocation: { cell, region: 'surface' as const } }] : [];
      });
      return unitRefs(state, seat).flatMap((ally) => targetSites.map(({ site, targetLocation }) => ({
        ...cast,
        ally,
        targetLocation,
        targetSiteInstanceId: site.instanceId,
      })));
    }
    if (definition.damageEachAbovegroundMinion === 1) return [cast];
    if (definition.damageEachUnitAtLocationWithinTwoSteps !== undefined) {
      const endpoints = movementPaths(
        state,
        { cell: caster.location, region: caster.region },
        2,
        seat,
      ).map((path) => path.at(-1)!);
      return [...new Map(endpoints.map((location) => [
        `${location.cell}:${location.region}`,
        location,
      ])).values()]
        .sort((left, right) => left.cell.localeCompare(right.cell))
        .map((targetLocation) => ({ ...cast, targetLocation }));
    }
    if (definition.damageRandomUnitAtLocation !== undefined) {
      return REALM_CELLS
        .map((cell): GameLocation => ({ cell, region: caster.region }))
        .filter((location) => locationExists(state, location))
        .map((targetLocation) => ({ ...cast, targetLocation }));
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
          || status.location === caster.location
          || borderingCells(caster.location).includes(status.location)
          || diagonalCells(caster.location).includes(status.location));
      }).map((target) => ({ ...cast, target }));
    });
  });
}

function requireCardId(value: string, path: string): void {
  if (!value.trim() || value.length > 256) throw new RangeError(`${path} must be 1-256 characters`);
}

function validateCardDefinition(card: GameCardDefinition, path: string): void {
  const elements: readonly GameElement[] = ['earth', 'fire', 'water', 'air'];
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
    if (card.genesisImmobilizeNearbyUntilNextTurn !== undefined
      && card.genesisImmobilizeNearbyUntilNextTurn !== true) {
      throw new RangeError(
        `${path}.genesisImmobilizeNearbyUntilNextTurn must be true when defined`,
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
        || card.genesisImmobilizeNearbyUntilNextTurn !== undefined
        || card.genesisMayBottomNextSpell !== undefined)) {
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
        || card.genesisImmobilizeNearbyUntilNextTurn !== undefined
        || card.genesisPayOneManaToSummonToken !== undefined)) {
      throw new RangeError(`${path} simultaneous next-spell and another site Genesis are unsupported`);
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
    if (card.grantsBearerPower !== undefined && card.grantsBearerPower !== 2) {
      throw new RangeError(`${path}.grantsBearerPower must be 2`);
    }
    if (card.grantsBearerLethal !== undefined && card.grantsBearerLethal !== true) {
      throw new RangeError(`${path}.grantsBearerLethal must be true`);
    }
    if (Number(card.grantsBearerPower === 2) + Number(card.grantsBearerLethal === true) !== 1) {
      throw new RangeError(`${path} must define exactly one supported bearer grant`);
    }
    if (!Number.isSafeInteger(card.manaCost) || card.manaCost < 0) {
      throw new RangeError(`${path}.manaCost must be a supported nonnegative safe integer`);
    }
    for (const element of elements) {
      if (!Number.isSafeInteger(card.thresholds[element]) || card.thresholds[element] < 0) {
        throw new RangeError(`${path}.thresholds.${element} must be a nonnegative safe integer`);
      }
    }
    return;
  }
  if (card.cardType === 'magic') {
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
    const effectCount = Number(card.burrowTargetMinionOrArtifact === true)
      + Number(card.submergeTargetMinion === true)
      + Number(card.damageEachAbovegroundMinion === 1)
      + Number(card.damageEachUnitAtLocationWithinTwoSteps !== undefined)
      + Number(card.damageRandomUnitAtLocation !== undefined)
      + Number(card.damageTargetUnit !== undefined)
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
  if (card.genesisDrawSpell !== undefined && typeof card.genesisDrawSpell !== 'boolean') {
    throw new RangeError(`${path}.genesisDrawSpell must be boolean`);
  }
  if (card.genesisDamageEachOtherUnitHere !== undefined
    && card.genesisDamageEachOtherUnitHere !== 1) {
    throw new RangeError(`${path}.genesisDamageEachOtherUnitHere must be 1`);
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
  if (card.diesAtEndOfControllerTurn !== undefined
    && card.diesAtEndOfControllerTurn !== true) {
    throw new RangeError(`${path}.diesAtEndOfControllerTurn must be true when defined`);
  }
  if (card.genesisDrawSite && card.genesisDrawSpell) {
    throw new RangeError(`${path} simultaneous Genesis site and spell draws are unsupported`);
  }
  if (card.genesisLoseControllerLife !== undefined
    && (card.genesisDrawSite || card.genesisDrawSpell)) {
    throw new RangeError(`${path} simultaneous Genesis life loss and draw are unsupported`);
  }
  if (card.genesisHealController !== undefined
    && (card.genesisDrawSite || card.genesisDrawSpell
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis healing and another effect are unsupported`);
  }
  if (card.genesisDamageEachOtherUnitHere === 1
    && (card.genesisDrawSite || card.genesisDrawSpell
      || card.genesisMayDamageTargetAdjacentUnit !== undefined
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis damage and another effect are unsupported`);
  }
  if (card.genesisMayDamageTargetAdjacentUnit === 2
    && (card.genesisDrawSite || card.genesisDrawSpell
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} simultaneous Genesis damage and another effect are unsupported`);
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
  if (card.movementBonus !== undefined
    && (!Number.isSafeInteger(card.movementBonus)
      || card.movementBonus < 1
      || card.movementBonus > 2)) {
    throw new RangeError(`${path}.movementBonus must be a safe integer between 1 and 2`);
  }
  if (card.nearbyEnemiesPermanentlyLoseStealth !== undefined
    && card.nearbyEnemiesPermanentlyLoseStealth !== true) {
    throw new RangeError(`${path}.nearbyEnemiesPermanentlyLoseStealth must be true when defined`);
  }
  if (card.otherNearbyAlliesPowerBonus !== undefined
    && card.otherNearbyAlliesPowerBonus !== 1) {
    throw new RangeError(`${path}.otherNearbyAlliesPowerBonus must be 1`);
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
    && (card.genesisDrawSite || card.genesisDrawSpell
      || card.genesisDamageEachOtherUnitHere === 1
      || card.genesisMayDamageTargetAdjacentUnit === 2
      || card.genesisHealController !== undefined
      || card.genesisLoseControllerLife !== undefined)) {
    throw new RangeError(`${path} token Genesis effects are unsupported`);
  }
  if (card.ward !== undefined && typeof card.ward !== 'boolean') {
    throw new RangeError(`${path}.ward must be boolean`);
  }
  if (card.takesLessDamage === 1 && card.ward) {
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
    && (card.genesisDrawSite || card.genesisDrawSpell
      || card.genesisDamageEachOtherUnitHere === 1
      || card.genesisMayDamageTargetAdjacentUnit === 2
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
      && definition?.cardType !== 'minion'
      && definition?.cardType !== 'magic')
      || definition?.cardType === 'minion' && definition.token === true) {
      throw new RangeError(`${path}.spellbook[${index}] references an unsupported spell`);
    }
  });
}

export function createGameManifest(input: GameManifestInput): GameManifest {
  createEngineState(input.seed);
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
            ...(card.grantsBearerPower === 2
              ? { grantsBearerPower: 2 as const }
              : { grantsBearerLethal: true as const }),
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
            ...(card.genesisDiscardTopSpells === 2 ? { genesisDiscardTopSpells: 2 as const } : {}),
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
            ...(card.genesisImmobilizeNearbyUntilNextTurn === true
              ? { genesisImmobilizeNearbyUntilNextTurn: true as const }
              : {}),
            ...(card.genesisMayBottomNextSpell === true
              ? { genesisMayBottomNextSpell: true as const }
              : {}),
            ...(card.genesisPayOneManaToSummonToken
              ? { genesisPayOneManaToSummonToken: card.genesisPayOneManaToSummonToken }
              : {}),
            ...(card.ordinaryMinionManaDiscount === 1
              ? { ordinaryMinionManaDiscount: 1 as const }
              : {}),
            ...(card.rangedUnitsHereRangeBonus === 1
              ? { rangedUnitsHereRangeBonus: 1 as const }
              : {}),
            ...(card.sacrificeToDestroyNearbySite === true
              ? { sacrificeToDestroyNearbySite: true as const }
              : {}),
          }
          : card.cardType === 'magic'
            ? {
              cardType: 'magic' as const,
              ...(card.burrowTargetMinionOrArtifact === true
                ? { burrowTargetMinionOrArtifact: true as const }
                : card.submergeTargetMinion === true
                  ? { submergeTargetMinion: true as const }
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
            attack: card.attack,
            ...(card.burrowing === true ? { burrowing: true } : {}),
            cardType: 'minion' as const,
            ...(card.cannotAttackSites === true ? { cannotAttackSites: true } : {}),
            ...(card.charge === true ? { charge: true } : {}),
            ...(card.cannotDefend === true ? { cannotDefend: true } : {}),
            ...(card.cannotDefendOrIntercept === true ? { cannotDefendOrIntercept: true } : {}),
            ...(card.connectsTopBottom === true ? { connectsTopBottom: true } : {}),
            ...(card.deathriteDrawSite === true ? { deathriteDrawSite: true } : {}),
            ...(card.deathriteHeal ? { deathriteHeal: card.deathriteHeal } : {}),
            ...(card.deathriteLoseLifePerNearbySiteControlled === 1
              ? { deathriteLoseLifePerNearbySiteControlled: 1 as const }
              : {}),
            defense: card.defense,
            ...(card.diesAtEndOfControllerTurn === true
              ? { diesAtEndOfControllerTurn: true as const }
              : {}),
            ...(card.discardRandomCardInsteadOfMana === true
              ? { discardRandomCardInsteadOfMana: true as const }
              : {}),
            ...(card.genesisDamageEachOtherUnitHere === 1
              ? { genesisDamageEachOtherUnitHere: 1 as const }
              : {}),
            ...(card.genesisMayDamageTargetAdjacentUnit === 2
              ? { genesisMayDamageTargetAdjacentUnit: 2 as const }
              : {}),
            ...(card.genesisDrawSpell === true ? { genesisDrawSpell: true } : {}),
            ...(card.genesisDrawSite === true ? { genesisDrawSite: true } : {}),
            ...(card.genesisHealController === 2 ? { genesisHealController: 2 as const } : {}),
            ...(card.genesisLoseControllerLife === 2 ? { genesisLoseControllerLife: 2 as const } : {}),
            ...(card.gainsStealthAtEndOfTurn === true ? { gainsStealthAtEndOfTurn: true } : {}),
            ...(card.immobile === true ? { immobile: true } : {}),
            ...(card.lanceCount !== undefined ? { lanceCount: card.lanceCount } : {}),
            ...(card.lethal === true ? { lethal: true } : {}),
            manaCost: card.manaCost,
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
            ...(card.otherNearbyAlliesPowerBonus === 1
              ? { otherNearbyAlliesPowerBonus: 1 as const }
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
  return Boolean(unit.disableEffects?.length)
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
      if (sitesProvidingNoThreshold.has(cell as RealmCell)) return;
      const definition = cardDefinition(state, site.cardId);
      if (definition.cardType !== 'site') throw new Error('realm site lacks site definition');
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
  return state.realm.units.filter((source) => {
    if (source.controller !== ref.seat
      || source.instanceId === ref.instanceId
      || source.region !== target.region
      || minionDisabled(state, source)) return false;
    const definition = cardDefinition(state, source.cardId);
    return definition.cardType === 'minion'
      && definition.otherNearbyAlliesPowerBonus === 1
      && (source.location === target.location
        || borderingCells(source.location).includes(target.location)
        || diagonalCells(source.location).includes(target.location));
  }).length;
}

function locationInImmobileArea(state: GameState, location: RealmCell): boolean {
  return state.realm.immobileAreas?.some(({ cells }) => cells.includes(location)) ?? false;
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
      const bearer = unitStatus(state, artifact.bearer);
      return {
        bearer: artifact.bearer,
        cardId: artifact.cardId,
        controller: artifact.bearer.seat,
        instanceId: artifact.instanceId,
        location: bearer.location,
        owner: artifact.owner,
        region: bearer.region,
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
  return unitRefs(state, seat).filter((ref) => {
    if (ref.kind === 'avatar') return true;
    const unit = state.realm.units.find(({ instanceId }) => instanceId === ref.instanceId);
    if (!unit || minionDisabled(state, unit)) return false;
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion' && definition.spellcaster === true;
  });
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
  lethal: boolean;
  location: RealmCell;
  movementSteps: number;
  movesOnlyForward: boolean;
  movesOnlySideways: boolean;
  ranged: boolean;
  region: GameRegion;
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
      immobile: locationInImmobileArea(state, avatar.location),
      lethal: bearerHasLethal(state, ref),
      location: avatar.location,
      movementSteps: 1,
      movesOnlyForward: false,
      movesOnlySideways: false,
      ranged: false,
      region: avatar.region,
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
  const powerBonus = temporaryPowerBonus(unit.temporaryPowerSources)
    + bearerPowerBonus(state, ref)
    + nearbyAlliesPowerBonus(state, ref);
  return {
    airborne: !disabled && definition.airborne === true && unit.region === 'surface',
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
    immobile: locationInImmobileArea(state, unit.location)
      || (!disabled && definition.immobile === true),
    lethal: !disabled && (definition.lethal === true || bearerHasLethal(state, ref)),
    location: unit.location,
    movementSteps: disabled ? 0 : 1 + (definition.movementBonus ?? 0),
    movesOnlyForward: !disabled && definition.movesOnlyForward === true,
    movesOnlySideways: !disabled && definition.movesOnlySideways === true,
    ranged: !disabled && definition.ranged === true,
    region: unit.region,
    stealthed: !disabled && unit.stealthed,
    strikesFirstWhileAttacking: !disabled && definition.strikesFirstWhileAttacking === true,
    submerge: !disabled && definition.submerge === true,
    summoningSickness: unit.summoningSickness,
    tapped: unit.tapped,
    takesLessDamage: disabled ? 0 : (definition.takesLessDamage ?? 0),
    voidwalk: !disabled && definition.voidwalk === true,
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

function locationExists(state: GameState, location: GameLocation): boolean {
  if (location.region === 'void') return state.realm.sites[location.cell] === undefined;
  return state.realm.sites[location.cell] !== undefined
    && (location.region === 'surface'
      || location.region === (isWaterSite(state, location.cell) ? 'underwater' : 'underground'));
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

function groundMinionEntryAllowed(
  state: GameState,
  current: GameLocation,
  candidate: GameLocation,
  airborne: boolean,
  movingMinion: boolean,
): boolean {
  if (!movingMinion
    || airborne
    || current.region !== 'surface'
    || candidate.region !== 'surface'
    || current.cell === candidate.cell) return true;
  const site = state.realm.sites[candidate.cell];
  if (!site || isRubble(site)) return true;
  const definition = cardDefinition(state, site.cardId);
  return definition.cardType !== 'site'
    || definition.blocksGroundMinionEntryWhileMinionAtop !== true
    || !state.realm.units.some((unit) =>
      unit.location === candidate.cell && unit.region === 'surface');
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
  connectsTopBottom = false,
  immobile = false,
  movingMinion = false,
  movingUnit = false,
  purpose: MovementPurpose = 'effect',
): readonly (readonly GameLocation[])[] {
  if (!locationExists(state, start)) return [];
  if (immobile) return [[start]];
  const paths: GameLocation[][] = [[start]];
  let frontier: Array<Readonly<{ cost: number; path: GameLocation[] }>> = [{ cost: 0, path: [start] }];
  while (frontier.length > 0) {
    frontier = frontier.flatMap(({ cost, path }) => {
      const current = path.at(-1)!;
      if (movingUnit && locationInImmobileArea(state, current.cell)) return [];
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
          ...(voidwalk
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
            ...(voidwalk
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
            ...(voidwalk
              ? borderingCells(current.cell, connectsTopBottom).map((cell) => ({ cell, region: 'void' as const }))
              : []),
          ]
          : current.region === 'void' && voidwalk
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
          // ponytail: tunnel-hop direction stays implicit until direction-sensitive effects need path metadata.
          return locationExists(state, candidate)
            && groundMinionEntryAllowed(state, current, candidate, airborne, movingMinion)
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
    unit.voidwalk,
    unit.connectsTopBottom,
    unit.immobile,
    ref.kind === 'minion',
    true,
    unit.canMoveToDefend ? 'defend' : 'effect',
  ).filter((path) => sameLocation(path.at(-1)!, destination));
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
      unit.voidwalk,
      unit.connectsTopBottom,
      unit.immobile,
      ref.kind === 'minion',
      true,
      'move-and-attack',
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

function rangedDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const directions = ['east', 'north', 'south', 'west'] as const;
  const allUnits = [...unitRefs(state, 'north'), ...unitRefs(state, 'south')];
  return unitRefs(state, seat).flatMap((shooter) => {
    const status = unitStatus(state, shooter);
    if (!status.ranged || status.tapped || status.summoningSickness) return [];
    const range = rangedProjectileRange(state, status.location, status.region);
    const startingEnemies = allUnits
      .filter((ref) => {
        const target = unitStatus(state, ref);
        return ref.seat !== seat
          && !target.stealthed
          && target.location === status.location
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
              && target.location === next
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
  });
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
            && target.location === location.cell
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
    return borderingCells(unit.location)
      .map((cell): GameLocation => ({ cell, region: unit.region }))
      .filter((location) => locationExists(state, location))
      .map((targetLocation) => ({
        kind: 'activate-area-damage' as const,
        sourceInstanceId: unit.instanceId,
        targetLocation,
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

function attackTargets(state: GameState, pending: PendingCombat): readonly CombatTarget[] {
  const defendingSeat = otherSeat(pending.attackingSeat);
  const attackerAirborne = unitStatus(state, pending.attacker).airborne;
  const region = pending.region ?? 'surface';
  const targets: CombatTarget[] = unitRefs(state, defendingSeat)
    .filter((ref) => {
      const target = unitStatus(state, ref);
      return target.location === pending.cell
        && target.region === region
        && !target.stealthed
        && (!target.airborne || attackerAirborne);
    });
  const site = state.realm.sites[pending.cell];
  if (region === 'surface'
    && site?.controller === defendingSeat
    && unitStatus(state, pending.attacker).canAttackSites) {
    targets.push({ instanceId: site.instanceId, kind: 'site', seat: defendingSeat });
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
      ? sameLocation({ cell: unit.location, region: unit.region }, pendingLocation)
      : defendPaths(state, ref, pendingLocation).length > 0;
  });
}

function pendingCombat(state: GameState): PendingCombat {
  if (!state.pendingCombat) throw new Error('unreachable missing pending combat');
  return state.pendingCombat;
}

function actionDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  if (state.terminal.status === 'finished' || state.phase === 'terminal' || seat !== state.decisionSeat) return [];
  const player = state.players[seat];
  if (state.phase === 'mulligan') return mulliganDescriptors(player);
  if (state.phase === 'draw') return [{ kind: 'draw', zone: 'atlas' }, { kind: 'draw', zone: 'spellbook' }];
  if (state.phase === 'genesis') {
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
    player.hand.atlas.flatMap(({ cardId, instanceId }) => cells.flatMap((cell) => {
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
      return site && isRubble(site)
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
    ...magicDescriptors(state, seat),
    ...pickUpArtifactDescriptors(state, seat),
    ...dropArtifactDescriptors(state, seat),
    ...siteDestructionDescriptors(state, seat),
    ...areaDamageAbilityDescriptors(state, seat),
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
  if (descriptor.kind === 'cast-artifact') {
    const destination = descriptor.bearer
      ? `carried by ${descriptor.bearer.kind} ${descriptor.bearer.instanceId.slice(0, 15)}…`
      : `uncarried at ${descriptor.cell}`;
    return `Cast ${descriptor.cardId} ${destination} (${descriptor.manaCost} mana)`;
  }
  if (descriptor.kind === 'activate-site-destruction') {
    return `Sacrifice site to destroy ${descriptor.targetCell}`;
  }
  if (descriptor.kind === 'summon-minion') {
    const caster = state.realm.units.find(({ instanceId }) =>
      instanceId === descriptor.casterInstanceId);
    const payment = descriptor.paymentMode === 'random-card-discard'
      ? 'discard random card'
      : descriptor.sacrificedMinionInstanceIds
        ? `${descriptor.manaCost} mana + sacrifice ${descriptor.sacrificedMinionInstanceIds.length} minion${descriptor.sacrificedMinionInstanceIds.length === 1 ? '' : 's'}`
      : `${descriptor.manaCost} mana`;
    const genesis = descriptor.genesisDamageChoice === 'target' && descriptor.genesisDamageTarget
      ? `; Genesis targets ${descriptor.genesisDamageTarget.kind} ${descriptor.genesisDamageTarget.instanceId.slice(0, 15)}…`
      : descriptor.genesisDamageChoice === 'decline' ? '; decline Genesis' : '';
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
      return withCaster(
        `Cast ${descriptor.cardId}: ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}… `
          + (stays
            ? `stays at ${descriptor.allyDestination.cell} and strikes enemies there`
            : `steps to ${descriptor.allyDestination.cell} and strikes enemies there`),
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
  if (descriptor.kind === 'shoot-projectile') {
    const target = descriptor.hit
      ? `${descriptor.hit.kind} ${descriptor.hit.instanceId.slice(0, 15)}…`
      : 'nothing';
    return `Shoot ${descriptor.direction} at ${target}`;
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
  if (descriptor.kind === 'activate-sparkmage') {
    const amount = state.players[state.decisionSeat].airThresholdsCastThisTurn ?? 0;
    return `Tap Sparkmage to deal ${amount} to a random other unit at ${descriptor.targetLocation.cell}`;
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
  return {
    players: state.players,
    realm: {
      ...state.realm,
      units: state.realm.units.map((unit) => unit.instanceId === ref.instanceId
        ? deepFreeze({ ...unit, location: location.cell, region: location.region, tapped: tap || unit.tapped })
        : unit),
    },
  };
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
    const nearby = new Set([
      source.location,
      ...borderingCells(source.location),
      ...diagonalCells(source.location),
    ]);
    const refs = units.filter((unit) => unit.controller !== source.controller
      && unit.region === source.region
      && unit.stealthed
      && nearby.has(unit.location))
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
          location: bearer.location,
          owner: artifact.owner,
          region: bearer.region,
          source: artifact.source,
        })
        : artifact),
    outcomes: dropped.map((artifact) => ({
      payload: {
        bearerInstanceId: bearer.instanceId,
        cardId: artifact.cardId,
        cell: bearer.location,
        instanceId: artifact.instanceId,
        owner: artifact.owner,
        region: bearer.region,
      },
      type: 'artifact-dropped',
    })),
  };
}

function resolveMinionDeaths(
  state: GameState,
  startingPlayers: GameState['players'],
  units: readonly UnitInstance[],
  deaths: readonly UnitInstance[],
  defeatedAvatars: ReadonlySet<GameSeat>,
): Readonly<{
  artifacts: readonly ArtifactInstance[] | undefined;
  outcomes: readonly GameOutcome[];
  players: GameState['players'];
  terminal: GameTerminal;
  units: readonly UnitInstance[];
}> {
  const players: Record<GameSeat, PlayerState> = {
    north: startingPlayers.north,
    south: startingPlayers.south,
  };
  const deathOutcomes: GameOutcome[] = [];
  let artifacts = state.realm.artifacts;
  const resolvedDeaths = [...deaths];
  const deadIds = new Set(resolvedDeaths.map(({ instanceId }) => instanceId));
  let survivingUnits = units.filter(({ instanceId }) => !deadIds.has(instanceId));
  while (true) {
    const projectedState = deepFreeze({
      ...state,
      players: startingPlayers,
      realm: { ...state.realm, units: survivingUnits },
    });
    const newlyLethal = survivingUnits.filter((unit) => unit.damage > 0
      && unit.damage >= unitStatus(projectedState, {
        instanceId: unit.instanceId,
        kind: 'minion',
        seat: unit.controller,
      }).defense);
    if (newlyLethal.length === 0) break;
    newlyLethal.forEach((unit) => {
      deadIds.add(unit.instanceId);
      resolvedDeaths.push(unit);
    });
    survivingUnits = survivingUnits.filter(({ instanceId }) => !deadIds.has(instanceId));
  }
  const deckLosers = new Set<GameSeat>();
  for (const dead of resolvedDeaths) {
    const definition = cardDefinition(state, dead.cardId);
    if (definition.cardType !== 'minion' || minionDisabled(state, dead)) continue;
    if (definition.deathriteHeal) {
      const controller = players[dead.controller];
      const avatarDefinition = cardDefinition(state, controller.avatar.card.cardId);
      if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
      const [healed, amount] = healAvatar(controller, avatarDefinition.life, definition.deathriteHeal);
      players[dead.controller] = healed;
      if (amount > 0) {
        deathOutcomes.push({
          payload: {
            amount,
            attemptedAmount: definition.deathriteHeal,
            life: healed.avatar.life,
            seat: dead.controller,
            sourceInstanceId: dead.instanceId,
          },
          type: 'avatar-healed',
        });
      }
    }
    if (definition.deathriteLoseLifePerNearbySiteControlled === 1) {
      const nearbyCells = new Set([
        dead.location,
        ...borderingCells(dead.location),
        ...diagonalCells(dead.location),
      ]);
      for (const seat of ['north', 'south'] as const) {
        const amount = Object.entries(state.realm.sites).filter(([cell, site]) =>
          site.controller === seat
            && nearbyCells.has(cell as RealmCell)
            && locationExists(state, { cell: cell as RealmCell, region: dead.region })).length;
        const [lifePlayer, lost, reachedDeathsDoor] = loseAvatarLife(
          players[seat],
          amount,
          state.turnNumber,
        );
        players[seat] = lifePlayer;
        if (lost > 0) {
          deathOutcomes.push({
            payload: {
              amount: lost,
              life: lifePlayer.avatar.life,
              seat,
              sourceInstanceId: dead.instanceId,
            },
            type: 'avatar-life-lost',
          });
        }
        if (reachedDeathsDoor) {
          deathOutcomes.push({
            payload: { seat, sourceInstanceId: dead.instanceId, turnNumber: state.turnNumber },
            type: 'avatar-reached-deaths-door',
          });
        }
      }
    }
    if (!definition.deathriteDrawSite) continue;
    const controller = players[dead.controller];
    const [drawn, ...atlas] = controller.atlas;
    if (!drawn) {
      deckLosers.add(dead.controller);
      continue;
    }
    players[dead.controller] = deepFreeze({
      ...controller,
      atlas,
      hand: { ...controller.hand, atlas: [...controller.hand.atlas, drawn] },
    });
    deathOutcomes.push({
      payload: { seat: dead.controller, sourceInstanceId: dead.instanceId },
      type: 'site-drawn',
    });
  }
  for (const dead of resolvedDeaths) {
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
    deathOutcomes.push(...drop.outcomes, {
      payload: { cardId: dead.cardId, instanceId: dead.instanceId, owner: dead.owner },
      type: 'minion-died',
    });
    if (token) {
      deathOutcomes.push({
        payload: { cardId: dead.cardId, instanceId: dead.instanceId, owner: dead.owner },
        type: 'minion-banished',
      });
    }
  }

  let terminal: GameTerminal = { status: 'active' };
  const losers = new Set([...defeatedAvatars, ...deckLosers]);
  if (losers.size === 2) {
    const reason = defeatedAvatars.size === 2 && deckLosers.size === 0
      ? 'simultaneous_avatar_defeat'
      : 'simultaneous_defeat';
    terminal = { reason, result: 'draw', status: 'finished' };
    deathOutcomes.push({ payload: { reason, result: 'draw' }, type: 'game-ended' });
  } else if (losers.size === 1) {
    const loser = [...losers][0]!;
    const winner = otherSeat(loser);
    const reason = defeatedAvatars.has(loser) ? 'avatar_defeated' : 'deck_empty';
    terminal = { loser, reason, status: 'finished', winner };
    deathOutcomes.push({ payload: { loser, reason, winner }, type: 'game-ended' });
  }
  return {
    artifacts,
    outcomes: deathOutcomes,
    players: deepFreeze(players),
    terminal,
    units: survivingUnits,
  };
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
  const survivingIds = new Set(resolution.units.map(({ instanceId }) => instanceId));
  const unitSurvives = (ref: GameUnitRef): boolean =>
    ref.kind === 'avatar' || survivingIds.has(ref.instanceId);
  const pendingInvalid = state.pendingCombat !== null
    && (!unitSurvives(state.pendingCombat.attacker)
      || (state.pendingCombat.originalTarget?.kind === 'minion'
        && !unitSurvives(state.pendingCombat.originalTarget)));
  const pendingCombat = state.pendingCombat === null || pendingInvalid
    ? null
    : deepFreeze({
      ...state.pendingCombat,
      combatants: state.pendingCombat.combatants.filter(unitSurvives),
      defenders: state.pendingCombat.defenders.filter(unitSurvives),
    });
  const terminal = state.terminal.status === 'finished'
    ? state.terminal
    : resolution.terminal;
  return {
    outcomes: resolution.outcomes,
    state: deepFreeze({
      ...state,
      ...(terminal.status === 'finished'
        ? { pendingCombat: null, phase: 'terminal' as const }
        : pendingInvalid
          ? { decisionSeat: state.activeSeat, pendingCombat: null, phase: 'main' as const }
          : { pendingCombat }),
      players: resolution.players,
      realm: {
        ...state.realm,
        ...(resolution.artifacts ? { artifacts: resolution.artifacts } : {}),
        units: resolution.units,
      },
      terminal,
    }),
  };
}

function resolveEndOfTurnDeaths(
  state: GameState,
  seat: GameSeat,
): Readonly<{ outcomes: readonly GameOutcome[]; state: GameState }> {
  const triggeredIds = state.realm.units.flatMap((unit) => {
    if (unit.controller !== seat || minionDisabled(state, unit)) return [];
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion'
      && definition.diesAtEndOfControllerTurn === true
      ? [unit.instanceId]
      : [];
  });
  let current = state;
  const outcomes: GameOutcome[] = [];
  for (const instanceId of triggeredIds) {
    const dead = current.realm.units.find((unit) => unit.instanceId === instanceId);
    if (!dead) continue;
    const resolution = resolveMinionDeaths(
      current,
      current.players,
      current.realm.units,
      [dead],
      new Set<GameSeat>(),
    );
    current = deepFreeze({
      ...current,
      ...(resolution.terminal.status === 'finished'
        ? { pendingCombat: null, phase: 'terminal' as const }
        : {}),
      players: resolution.players,
      realm: {
        ...current.realm,
        ...(resolution.artifacts ? { artifacts: resolution.artifacts } : {}),
        units: resolution.units,
      },
      terminal: resolution.terminal,
    });
    outcomes.push(...resolution.outcomes);
    if (resolution.terminal.status === 'finished') break;
  }
  return { outcomes, state: current };
}

type MinionRegionDisposition = 'banished' | 'dies' | 'survives';

function minionRegionDisposition(state: GameState, unit: UnitInstance): MinionRegionDisposition {
  if (unit.region === 'surface') return 'survives';
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion') throw new Error('realm minion lacks minion definition');
  const disabled = minionDisabled(state, unit);
  if (unit.region === 'void') return !disabled && definition.voidwalk === true ? 'survives' : 'banished';
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
    if (!groundMinionEntryAllowed(
      current,
      expectedFrom,
      next,
      currentStatus.airborne,
      ref.kind === 'minion',
    )) break;
    const moved = moveUnit(current, ref, next, false);
    current = deepFreeze({ ...current, players: moved.players, realm: moved.realm });
    actualPath.push(next);
    const settlement = settleRegionOccupancy(current);
    current = settlement.state;
    outcomes.push(...settlement.outcomes);
    removals.push(...settlement.removals);
    const stealthSettlement = settleNearbyEnemyStealth(current);
    current = stealthSettlement.state;
    outcomes.push(...stealthSettlement.outcomes);
    const powerSettlement = settleStaticPowerDeaths(current);
    current = powerSettlement.state;
    outcomes.push(...powerSettlement.outcomes);
    if (settlement.removals.some(({ instanceId }) => instanceId === ref.instanceId)
      || current.terminal.status === 'finished') break;
  }
  return { outcomes, path: actualPath, removals, state: current };
}

function resolveSiteDeaths(
  state: GameState,
  destroyed: readonly Readonly<{ cell: RealmCell; site: SiteInstance }>[],
  sourceInstanceId: StateHash,
): Readonly<{
  outcomes: readonly GameOutcome[];
  players: GameState['players'];
  realm: GameState['realm'];
  terminal: GameTerminal;
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
    floodedCells.has(unit.location) && unit.region === 'underwater'
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
  const settlement = settleRegionOccupancy(terrainState);
  const players: Record<GameSeat, PlayerState> = {
    north: settlement.state.players.north,
    south: settlement.state.players.south,
  };
  for (const { site } of unique) {
    const owner = players[site.owner];
    players[site.owner] = deepFreeze({
      ...owner,
      cemetery: [...owner.cemetery, {
        cardId: site.cardId,
        instanceId: site.instanceId,
        owner: site.owner,
        source: site.source,
      }],
    });
  }
  const terminalIndex = settlement.outcomes.findIndex(({ type }) => type === 'game-ended');
  return {
    outcomes: terminalIndex < 0
      ? [...settlement.outcomes, ...rubbleOutcomes]
      : [
        ...settlement.outcomes.slice(0, terminalIndex),
        ...rubbleOutcomes,
        ...settlement.outcomes.slice(terminalIndex),
      ],
    players: deepFreeze(players),
    realm: settlement.state.realm,
    terminal: settlement.state.terminal,
  };
}

function resolveFightWindow(
  state: GameState,
  pending: PendingCombat,
  outcomes: readonly GameOutcome[],
  attackerStrikes: boolean,
  combatantsStrike: boolean | readonly GameUnitRef[],
  interactingRefs?: readonly GameUnitRef[],
  allocationsAreStrikes = true,
  allocationsUseAttackerLethal = allocationsAreStrikes,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const allocations = new Map(pending.allocations.map(({ amount, targetInstanceId }) =>
    [targetInstanceId, amount]));
  const damageSources = new Map<StateHash, readonly number[]>();
  const lethalDamage = new Set<StateHash>();
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
      strikingCombatants.map((ref) => strikeDamage(state, ref)),
    );
  }
  if (attackerCanStrike) {
    pending.combatants.forEach((ref) =>
      damageSources.set(ref.instanceId, [allocations.get(ref.instanceId) ?? 0]));
  }
  pending.combatants.forEach((ref) => {
    const striker = unitStatus(state, ref);
    if (strikingCombatants.some(({ instanceId }) => instanceId === ref.instanceId)
      && striker.lethal
      && strikeDamage(state, ref) > attackerStatus.takesLessDamage) {
      lethalDamage.add(pending.attacker.instanceId);
    }
    if (attackerCanStrike
      && allocationsUseAttackerLethal
      && attackerStatus.lethal
      && (allocations.get(ref.instanceId) ?? 0) > unitStatus(state, ref).takesLessDamage) {
      lethalDamage.add(ref.instanceId);
    }
  });

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
    const amount = sources.reduce((total, source) => total + source, 0);
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
    if (amount > 0 && unit.warded) {
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
    const reduction = unitStatus(state, ref).takesLessDamage;
    const dealt = sources.reduce((total, source) => total + Math.max(0, source - reduction), 0);
    const accumulated = unit.damage + dealt;
    units[index] = deepFreeze({ ...unit, damage: accumulated });
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
    if (accumulated > 0
      && (accumulated >= unitStatus(state, ref).defense
        || (dealt > 0 && lethalDamage.has(ref.instanceId)))) {
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

  const deathResolution = resolveMinionDeaths(state, players, units, deaths, defeatedAvatars);

  return [
    deepFreeze({
      ...state,
      decisionSeat: state.activeSeat,
      pendingCombat: null,
      phase: deathResolution.terminal.status === 'finished' ? 'terminal' : 'main',
      players: deathResolution.players,
      realm: {
        ...state.realm,
        ...(deathResolution.artifacts ? { artifacts: deathResolution.artifacts } : {}),
        units: deathResolution.units,
      },
      terminal: deathResolution.terminal,
    }),
    [...outcomes, ...damageOutcomes, ...lanceOutcomes, ...deathResolution.outcomes],
    [],
  ];
}

function finishFight(
  state: GameState,
  pending: PendingCombat,
  outcomes: readonly GameOutcome[],
  returnStrikes = true,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
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
      attackerStrikesFirst,
      firstCombatants,
    );
    const survivors = pending.combatants.filter((ref) =>
      ref.kind === 'avatar'
        || earlyState.realm.units.some(({ instanceId }) => instanceId === ref.instanceId));
    const attackerSurvived = pending.attacker.kind === 'avatar'
      || earlyState.realm.units.some(({ instanceId }) => instanceId === pending.attacker.instanceId);
    if (earlyState.terminal.status === 'finished' || !attackerSurvived || survivors.length === 0) {
      return [withStateVersion(earlyState, {}), earlyOutcomes, earlyDraws];
    }
    const firstIds = new Set(firstCombatants.map(({ instanceId }) => instanceId));
    const [resolved, resolvedOutcomes, normalDraws] = resolveFightWindow(
      earlyState,
      deepFreeze({ ...pending, combatants: survivors }),
      earlyOutcomes,
      !attackerStrikesFirst,
      survivors.filter(({ instanceId }) => !firstIds.has(instanceId)),
    );
    return [withStateVersion(resolved, {}), resolvedOutcomes, [...earlyDraws, ...normalDraws]];
  }
  const [resolved, resolvedOutcomes, draws] = resolveFightWindow(
    state,
    pending,
    outcomes,
    true,
    returnStrikes,
  );
  return [withStateVersion(resolved, {}), resolvedOutcomes, draws];
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
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const seat = state.decisionSeat;
  const player = state.players[seat];
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
    const resolved = resolveSiteDeaths(
      state,
      [
        { cell: sourceEntry.cell, site: source },
        ...(isRubble(target) ? [] : [{ cell: descriptor.targetCell, site: target }]),
      ],
      source.instanceId,
    );
    return [
      withStateVersion(state, {
        ...(resolved.terminal.status === 'finished' ? { phase: 'terminal' as const } : {}),
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
          type: 'site-destroyed',
        },
        ...resolved.outcomes,
      ],
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
    const nearbyCells = new Set([
      descriptor.cell,
      ...borderingCells(descriptor.cell),
      ...diagonalCells(descriptor.cell),
    ]);
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
    const pendingGenesisToken = terminal.status === 'active'
      && descriptor.genesisTokenChoice === 'defer'
      && definition.genesisPayOneManaToSummonToken !== undefined
      ? deepFreeze({ cell: descriptor.cell, seat, sourceInstanceId: card.instanceId })
      : undefined;
    const resolvedState = deepFreeze({
        ...(terminal.status === 'finished'
          ? { phase: 'terminal' as const }
          : pendingGenesisToken || pendingGenesisSpell
            ? {
              ...(pendingGenesisSpell ? { pendingGenesisSpell } : {}),
              ...(pendingGenesisToken ? { pendingGenesisToken } : {}),
              phase: 'genesis' as const,
            }
            : {}),
        players: settlement.state.players,
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
      ? deepFreeze({ ...card, bearer: descriptor.bearer })
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

  if (descriptor.kind === 'cast-magic') {
    const card = player.hand.spellbook.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    const definition = card && cardDefinition(state, card.cardId);
    const legal = magicDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'cast-magic'
        && candidate.cardInstanceId === descriptor.cardInstanceId
        && candidate.casterInstanceId === descriptor.casterInstanceId
        && candidate.cemeteryMinionInstanceId === descriptor.cemeteryMinionInstanceId
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
        && (candidate.target === undefined && descriptor.target === undefined
          || candidate.target !== undefined
            && descriptor.target !== undefined
            && candidate.target.instanceId === descriptor.target.instanceId
            && candidate.target.kind === descriptor.target.kind
            && candidate.target.seat === descriptor.target.seat)
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
        manaPaid: definition.manaCost,
        seat,
        ...(descriptor.target
          ? {
            targetInstanceId: descriptor.target.instanceId,
            targetSeat: descriptor.target.seat,
          }
          : {}),
        ...(descriptor.targetArtifactInstanceId
          ? { targetArtifactInstanceId: descriptor.targetArtifactInstanceId }
          : {}),
        ...(descriptor.targetLocation ? { targetLocation: descriptor.targetLocation } : {}),
        ...(descriptor.ally
          ? { allyInstanceId: descriptor.ally.instanceId, allySeat: descriptor.ally.seat }
          : {}),
        ...(descriptor.allyDestination ? { allyDestination: descriptor.allyDestination } : {}),
        ...(descriptor.targetSiteInstanceId
          ? { targetSiteInstanceId: descriptor.targetSiteInstanceId }
          : {}),
        ...(descriptor.cemeteryMinionInstanceId
          ? { cemeteryMinionInstanceId: descriptor.cemeteryMinionInstanceId }
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
    const castOutcomes: readonly GameOutcome[] = [castOutcome, ...interaction.outcomes];
    const resolved = {
      payload: { cardId: card.cardId, instanceId: card.instanceId, owner: card.owner },
      type: 'magic-resolved',
    } as const;
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
      const enemies = unitRefs(path.state, otherSeat(seat))
        .filter((enemy) => {
          const status = unitStatus(path.state, enemy);
          return status.location === striker.location && status.region === striker.region;
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
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
        cell: striker.location,
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
      return [
        withStateVersion(controlledState, {}),
        [
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
          resolved,
        ],
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
    if (definition.burrowTargetMinionOrArtifact === true
      && descriptor.targetArtifactInstanceId !== undefined) {
      const artifactIndex = (castState.realm.artifacts ?? []).findIndex(({ instanceId }) =>
        instanceId === descriptor.targetArtifactInstanceId);
      const artifact = castState.realm.artifacts?.[artifactIndex];
      if (!artifact) throw new Error('unreachable Bury Artifact target');
      const location = 'bearer' in artifact
        ? unitStatus(castState, artifact.bearer)
        : artifact;
      const canMove = location.region === 'surface'
        && castState.realm.sites[location.location] !== undefined
        && !isWaterSite(castState, location.location);
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
                location: location.location,
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
              cell: location.location,
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
      const canMove = target.region === 'surface'
        && castState.realm.sites[target.location] !== undefined
        && isWaterSite(castState, target.location) === submerge;
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
      let effectState = castState;
      const effectOutcomes: GameOutcome[] = [...castOutcomes];
      if (!sameLocation(from, descriptor.targetLocation)) {
        const moved = moveUnit(castState, descriptor.ally, descriptor.targetLocation, false);
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
            to: descriptor.targetLocation,
          },
          type: 'unit-teleported',
        });
        const regionSettlement = settleRegionOccupancy(teleportedState);
        const powerSettlement = settleStaticPowerDeaths(regionSettlement.state);
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
          return status.location === descriptor.targetLocation!.cell
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
      const candidates = (['north', 'south'] as const)
        .flatMap((targetSeat) => unitRefs(castState, targetSeat))
        .filter((target) => {
          const status = unitStatus(castState, target);
          return status.location === descriptor.targetLocation!.cell
            && status.region === descriptor.targetLocation!.region;
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      if (candidates.length === 0) {
        return [withStateVersion(castState, {}), [...castOutcomes, resolved], []];
      }
      const selected = drawCandidate(
        castState.engine,
        candidates.length,
        'magic_random_unit_at_location',
        'unit_index_candidate',
      );
      const targetRef = candidates[selected.index]!;
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
    const card = player.hand.spellbook.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    const definition = card && cardDefinition(state, card.cardId);
    const legal = summonDescriptors(state, seat).some((candidate) => {
      if (candidate.kind !== 'summon-minion') return false;
      const candidateSacrifices = candidate.sacrificedMinionInstanceIds ?? [];
      const requestedSacrifices = descriptor.sacrificedMinionInstanceIds ?? [];
      return candidate.cardInstanceId === descriptor.cardInstanceId
        && candidate.casterInstanceId === descriptor.casterInstanceId
        && candidate.cell === descriptor.cell
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
    if (!card || !definition || definition.cardType !== 'minion' || !legal || !caster) {
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
      ? drawCandidate(
        state.engine,
        discardCandidates.length,
        'summon_random_card_discard_cost',
        'card_index_candidate',
      )
      : undefined;
    const discardedCard = randomCost ? discardCandidates[randomCost.index] : undefined;
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
      region: descriptor.region ?? 'surface',
      stealthed: definition.stealth === true,
      summoningSickness: true,
      tapped: false,
      warded: definition.ward === true,
    });
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
    const paidState = deepFreeze({
      ...randomizedState,
      players: replacePlayer(randomizedState, seat, updatedPlayer),
    });
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
    const resolvedPaymentState = deathResolution
      ? deepFreeze({
        ...paidState,
        ...(deathResolution.terminal.status === 'finished' ? { phase: 'terminal' as const } : {}),
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
      return [
        withStateVersion(resolvedPaymentState, {}),
        paymentOutcomes,
        paymentRandomDraws,
      ];
    }
    const interaction = recordInteraction(resolvedPaymentState, [caster]);
    const realm = { ...resolvedPaymentState.realm, units: [...interaction.units, unit] };
    const summoned: GameOutcome = {
      payload: {
        cardId: card.cardId,
        casterInstanceId: descriptor.casterInstanceId,
        cell: descriptor.cell,
        instanceId: card.instanceId,
        manaPaid: descriptor.manaCost,
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
      return [
        withStateVersion(settlement.state, {}),
        [...summonOutcomes, ...settlement.outcomes],
        paymentRandomDraws,
      ];
    }
    const settledPlayer = settlement.state.players[seat];
    const genesisDrawZone = definition.genesisDrawSite
      ? 'atlas'
      : definition.genesisDrawSpell ? 'spellbook' : undefined;
    if (definition.genesisDamageEachOtherUnitHere === 1) {
      const source: GameUnitRef = {
        instanceId: unit.instanceId,
        kind: 'minion',
        seat,
      };
      const sourceUnit = settlement.state.realm.units.find(({ instanceId }) =>
        instanceId === unit.instanceId);
      if (!sourceUnit || minionDisabled(settlement.state, sourceUnit)) {
        return [
          withStateVersion(settlement.state, {}),
          [...summonOutcomes, ...settlement.outcomes],
          paymentRandomDraws,
        ];
      }
      const targets = (['north', 'south'] as const)
        .flatMap((targetSeat) => unitRefs(settlement.state, targetSeat))
        .filter((target) => {
          if (target.instanceId === source.instanceId) return false;
          const status = unitStatus(settlement.state, target);
          return status.location === sourceUnit.location && status.region === sourceUnit.region;
        })
        .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
      if (targets.length === 0) {
        return [
          withStateVersion(settlement.state, {}),
          [...summonOutcomes, ...settlement.outcomes],
          paymentRandomDraws,
        ];
      }
      const pending: PendingCombat = deepFreeze({
        allocations: targets.map(({ instanceId }) => ({
          amount: definition.genesisDamageEachOtherUnitHere!,
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
          amount: definition.genesisDamageEachOtherUnitHere!,
          sourceInstanceId: source.instanceId,
          targetInstanceId: instanceId,
        },
        type: 'genesis-damage-allocated',
      }));
      const [damaged, outcomes, randomDraws] = resolveFightWindow(
        settlement.state,
        pending,
        [...summonOutcomes, ...settlement.outcomes, ...allocationOutcomes],
        true,
        false,
        [source],
        false,
        true,
      );
      return [
        withStateVersion(damaged, {}),
        outcomes,
        [...paymentRandomDraws, ...randomDraws],
      ];
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
        || targetStatus.location !== sourceUnit.location
          && !borderingCells(sourceUnit.location).includes(targetStatus.location)) {
        return [
          withStateVersion(settlement.state, {}),
          [...summonOutcomes, ...settlement.outcomes],
          paymentRandomDraws,
        ];
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
        true,
        false,
        [source],
        false,
        true,
      );
      return [
        withStateVersion(damaged, {}),
        outcomes,
        [...paymentRandomDraws, ...randomDraws],
      ];
    }
    if (definition.genesisHealController === 2) {
      const avatarDefinition = cardDefinition(settlement.state, settledPlayer.avatar.card.cardId);
      if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
      const [healed, amount] = healAvatar(
        settledPlayer,
        avatarDefinition.life,
        definition.genesisHealController,
      );
      return [
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
      ];
    }
    if (definition.genesisLoseControllerLife === 2) {
      const [lifePlayer, amount, reachedDeathsDoor] = loseAvatarLife(
        settledPlayer,
        definition.genesisLoseControllerLife,
        state.turnNumber,
      );
      return [
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
      ];
    }
    if (genesisDrawZone) {
      const [drawn, ...remaining] = settledPlayer[genesisDrawZone];
      if (!drawn) {
        const winner = otherSeat(seat);
        return [
          withStateVersion(settlement.state, {
            phase: 'terminal',
            pendingCombat: null,
            terminal: { loser: seat, reason: 'deck_empty', status: 'finished', winner },
          }),
          [
            ...summonOutcomes,
            ...settlement.outcomes,
            { payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' },
          ],
          paymentRandomDraws,
        ];
      }
      const drawingPlayer = deepFreeze({
        ...settledPlayer,
        [genesisDrawZone]: remaining,
        hand: {
          ...settledPlayer.hand,
          [genesisDrawZone]: [...settledPlayer.hand[genesisDrawZone], drawn],
        },
      });
      return [
        withStateVersion(settlement.state, {
          players: replacePlayer(settlement.state, seat, drawingPlayer),
        }),
        [
          ...summonOutcomes,
          ...settlement.outcomes,
          {
            payload: { seat, sourceInstanceId: card.instanceId },
            type: genesisDrawZone === 'atlas' ? 'site-drawn' : 'spell-drawn',
          },
        ],
        paymentRandomDraws,
      ];
    }
    return [
      withStateVersion(settlement.state, {}),
      [...summonOutcomes, ...settlement.outcomes],
      paymentRandomDraws,
    ];
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
        return status.location === descriptor.targetLocation.cell
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
      true,
      false,
      [],
      false,
      true,
    );
    return [withStateVersion(damaged, {}), outcomes, randomDraws];
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
    const candidates = (['north', 'south'] as const)
      .flatMap((targetSeat) => unitRefs(activatedState, targetSeat))
      .filter((target) => {
        if (target.instanceId === sourceRef.instanceId) return false;
        const status = unitStatus(activatedState, target);
        return status.location === descriptor.targetLocation.cell
          && status.region === descriptor.targetLocation.region;
      })
      .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
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
    const selected = drawCandidate(
      activatedState.engine,
      candidates.length,
      'sparkmage_random_other_unit_at_nearby_location',
      'unit_index_candidate',
    );
    const targetRef = candidates[selected.index]!;
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
    const legal = rangedDescriptors(state, seat).some((candidate) =>
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
    return finishFight(shotState, deepFreeze({
      allocations: [{ amount, targetInstanceId: descriptor.hit.instanceId }],
      attacker: shooter,
      attackingSeat: seat,
      cell: unitStatus(state, descriptor.hit).location,
      combatants: [descriptor.hit],
      defenders: [],
      originalTarget: descriptor.hit,
      ...(unitStatus(state, descriptor.hit).region === 'surface'
        ? {}
        : { region: unitStatus(state, descriptor.hit).region as 'underground' | 'underwater' | 'void' }),
      targetRemoved: false,
    }), [shot, ...interaction.outcomes, strike], false);
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
    const dragPath = [...descriptor.path].reverse();
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
    const hitArrived = path.state.realm.units.some(({ instanceId, location, region }) =>
      instanceId === descriptor.hit!.instanceId && location === to.cell && region === to.region);
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
    const path = resolveDeclaredPath(state, ref, descriptor.path, true);
    const actualTo = path.path.at(-1) ?? descriptor.from;
    const activated: GameOutcome = {
      payload: {
        from: descriptor.from,
        path: path.path,
        seat,
        steps: path.path.length - 1,
        to: actualTo,
        unitInstanceId: descriptor.unitInstanceId,
      },
      type: 'move-and-attack-activated',
    };
    const moverArrived = ref.kind === 'avatar'
      ? path.state.players[ref.seat].avatar.card.instanceId === ref.instanceId
        && path.state.players[ref.seat].avatar.location === descriptor.to.cell
        && path.state.players[ref.seat].avatar.region === descriptor.to.region
      : path.state.realm.units.some(({ instanceId, location, region }) =>
        instanceId === ref.instanceId && location === descriptor.to.cell && region === descriptor.to.region);
    if (!moverArrived || path.state.terminal.status === 'finished') {
      return [withStateVersion(path.state, {}), [activated, ...path.outcomes], []];
    }
    const pending: PendingCombat = deepFreeze({
      allocations: [],
      attacker: ref,
      attackingSeat: seat,
      cell: descriptor.to.cell,
      combatants: [],
      defenders: [],
      originalTarget: null,
      ...(descriptor.to.region === 'surface' ? {} : { region: descriptor.to.region }),
      targetRemoved: false,
    });
    return [
      withStateVersion(path.state, {
        pendingCombat: pending,
        phase: 'attack',
      }),
      [activated, ...path.outcomes],
      [],
    ];
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
    const declaredPending = deepFreeze({ ...pending, originalTarget: descriptor.target });
    const declared: GameOutcome = {
      payload: {
        attackerInstanceId: pending.attacker.instanceId,
        cell: pending.cell,
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
    const pending = pendingCombat(state);
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
    const destination: GameLocation = { cell: pending.cell, region: pending.region ?? 'surface' };
    const path = resolveDeclaredPath(state, ref, descriptor.path, true);
    const defenderArrived = ref.kind === 'avatar'
      ? path.state.players[ref.seat].avatar.card.instanceId === ref.instanceId
        && path.state.players[ref.seat].avatar.location === destination.cell
        && path.state.players[ref.seat].avatar.region === destination.region
      : path.state.realm.units.some(({ instanceId, location, region }) =>
        instanceId === ref.instanceId && location === destination.cell && region === destination.region);
    const reconciledPending = path.state.pendingCombat;
    if (!defenderArrived
      || path.state.terminal.status === 'finished'
      || reconciledPending === null) {
      return [
        withStateVersion(path.state, {}),
        [{
          payload: {
            from: descriptor.from,
            instanceId: ref.instanceId,
            path: path.path,
            seat,
            steps: path.path.length - 1,
            to: path.path.at(-1) ?? descriptor.from,
          },
          type: 'defender-moved',
        }, ...path.outcomes],
        [],
      ];
    }
    const removesSite = reconciledPending.originalTarget?.kind === 'site'
      && !reconciledPending.targetRemoved;
    return [
      withStateVersion(path.state, {
        pendingCombat: deepFreeze({
          ...reconciledPending,
          defenders: [...reconciledPending.defenders, ref],
          targetRemoved: reconciledPending.targetRemoved || removesSite,
        }),
      }),
      [
        {
          payload: {
            from: descriptor.from,
            instanceId: ref.instanceId,
            path: path.path,
            seat,
            steps: path.path.length - 1,
            to: path.path.at(-1) ?? descriptor.from,
          },
          type: 'defender-joined',
        },
        ...path.outcomes,
        ...(removesSite
          ? [{
            payload: { instanceId: reconciledPending.originalTarget!.instanceId, kind: 'site' },
            type: 'original-target-removed',
          }]
          : []),
      ],
      [],
    ];
  }

  if (descriptor.kind === 'intercept') {
    const pending = pendingCombat(state);
    const ref = responseUnitRefs(state, pending, true)
      .find(({ instanceId }) => instanceId === descriptor.unitInstanceId);
    if (!ref) throw new Error('unreachable illegal interceptor');
    const destination: GameLocation = { cell: pending.cell, region: pending.region ?? 'surface' };
    const tapped = moveAndTapUnit(state, ref, destination);
    return [
      withStateVersion(state, {
        pendingCombat: deepFreeze({ ...pending, defenders: [...pending.defenders, ref] }),
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
  const endOfTurnDeaths = resolveEndOfTurnDeaths(state, seat);
  const endState = endOfTurnDeaths.state;
  if (endState.terminal.status === 'finished') {
    return [
      withStateVersion(endState, { pendingCombat: null, phase: 'terminal' }),
      endOfTurnDeaths.outcomes,
      [],
    ];
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
  const players = deepFreeze({ ...endState.players, [seat]: endingPlayer, [nextSeat]: startingPlayer });
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
  const {
    immobileAreas: previousImmobileAreas,
    ...endingRealm
  } = endState.realm;
  const immobileAreas = (previousImmobileAreas ?? [])
    .filter(({ expiresAtSeat }) => expiresAtSeat !== nextSeat);
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
  return [
    withStateVersion(endState, {
      activeSeat: nextSeat,
      decisionSeat: nextSeat,
      pendingCombat: null,
      phase: 'draw',
      players,
      realm: {
        ...endingRealm,
        ...(immobileAreas.length > 0 ? { immobileAreas } : {}),
        units,
      },
      turnNumber,
    }),
    [
      ...endOfTurnDeaths.outcomes,
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
  const powerSettlement = settleStaticPowerDeaths(stealthSettlement.state);
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
  const outcomes = settlementOutcomes.length === 0
    ? appliedOutcomes
    : completionIndex < 0
      ? [...appliedOutcomes, ...settlementOutcomes]
      : [
        ...appliedOutcomes.slice(0, completionIndex),
        ...settlementBeforeCompletion,
        ...appliedOutcomes.slice(completionIndex),
        ...settlementAfterCompletion,
      ];
  const nextState = powerSettlement.state;
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
