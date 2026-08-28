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

const REALM_CELLS = (['A', 'B', 'C', 'D', 'E'] as const)
  .flatMap((file) => ([1, 2, 3, 4] as const).map((rank) => `${file}${rank}` as RealmCell));

export type GameCardDefinition =
  | Readonly<{ attack: number; cardType: 'avatar'; defense: number; drawSpell: boolean; life: number }>
  | Readonly<{
    cardType: 'site';
    connectsBurrowedAllies?: boolean;
    elements: readonly GameElement[];
    genesisDiscardTopSpells?: 2;
    genesisDrawSpellPerAdjacentSameCard?: boolean;
    genesisGainMana?: number;
    sacrificeToDestroyNearbySite?: true;
  }>
  | Readonly<{
    burrowTargetMinion?: boolean;
    cardType: 'magic';
    damageEachUnitAtLocationWithinTwoSteps?: number;
    damageRandomUnitAtLocation?: number;
    damageTargetUnit?: number;
    disableTargetNearbyMinionUntilNextTurn?: true;
    grantChargeToAllyThisTurn?: true;
    healController?: number;
    lureEnemyMinionOneStepCloser?: true;
    manaCost: number;
    returnMinionFromOwnCemetery?: true;
    submergeTargetMinion?: true;
    targetNearby?: boolean;
    teleportAllyToTargetSite?: true;
    thresholds: GameThresholds;
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
    defense: number;
    diesAtEndOfControllerTurn?: true;
    genesisDrawSpell?: boolean;
    genesisDrawSite?: boolean;
    genesisLoseControllerLife?: 2;
    gainsStealthAtEndOfTurn?: boolean;
    immobile?: boolean;
    lethal?: boolean;
    manaCost: number;
    movementBonus?: 1 | 2;
    movesOnlyForward?: boolean;
    movesOnlySideways?: boolean;
    mustBeCastBurrowed?: boolean;
    mustBeCastSubmerged?: boolean;
    mustBeCastToWaterSite?: boolean;
    provides?: GameElement;
    ranged?: boolean;
    shootsDragProjectile?: boolean;
    stealth?: boolean;
    strikesFirstWhileAttacking?: boolean;
    submerge?: boolean;
    summonToAnySite?: boolean;
    mustBeCastToOuterColumn?: boolean;
    tapForMana?: number;
    thresholds: GameThresholds;
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
  source: 'atlas' | 'avatar' | 'spellbook';
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

type UnitInstance = Readonly<CardInstance & {
  controller: GameSeat;
  damage: number;
  disableEffects?: readonly DisableEffect[];
  location: RealmCell;
  region: GameRegion;
  stealthed: boolean;
  summoningSickness: boolean;
  tapped: boolean;
  temporaryChargeSources?: readonly StateHash[];
  warded: boolean;
}>;

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

type PlayerState = Readonly<{
  atlas: readonly CardInstance[];
  avatar: Readonly<{
    card: CardInstance;
    deathDoorTurn: number | null;
    life: number;
    location: RealmCell;
    region: GameRegion;
    tapped: boolean;
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
  phase: 'allocate' | 'attack' | 'defend' | 'draw' | 'intercept' | 'main' | 'mulligan' | 'terminal';
  players: Readonly<Record<GameSeat, PlayerState>>;
  realm: Readonly<{
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
  atlasCount: number;
  avatar: Readonly<{
    attack: number;
    cardId: string;
    deathDoorTurn: number | null;
    defense: number;
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
      controller: GameSeat;
      damage: number;
      defense: number;
      disabled: boolean;
      instanceId: StateHash;
      location: RealmCell;
      owner: GameSeat;
      region: GameRegion;
      stealthed: boolean;
      summoningSickness: boolean;
      tapped: boolean;
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
  | Readonly<{ cardId: string; cardInstanceId: string; cell: RealmCell; kind: 'play-site' }>
  | Readonly<{
    kind: 'activate-site-destruction';
    sourceSiteInstanceId: StateHash;
    targetCell: RealmCell;
    targetSiteInstanceId: StateHash;
  }>
  | Readonly<{
    cardId: string;
    cardInstanceId: string;
    casterInstanceId: string;
    cell: RealmCell;
    kind: 'summon-minion';
    manaCost: number;
    region?: 'underground' | 'underwater' | 'void';
  }>
  | Readonly<{
    cardId: string;
    cardInstanceId: string;
    casterInstanceId: StateHash;
    cemeteryMinionInstanceId?: StateHash;
    kind: 'cast-magic';
    ally?: GameUnitRef;
    target?: GameUnitRef;
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
  return [...new Set(Object.entries(state.realm.sites)
    .filter(([, site]) => site.controller === seat)
    .flatMap(([cell]) => borderingCells(cell as RealmCell)))]
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

function summonDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const controlledCells = controlledSiteCells(state, seat);
  const siteCells = Object.keys(state.realm.sites).sort() as RealmCell[];
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'minion'
      || player.mana < definition.manaCost
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    const summonCells = (definition.summonToAnySite ? siteCells : controlledCells)
      .filter((cell) => !definition.mustBeCastToOuterColumn || cell[0] === 'A' || cell[0] === 'E')
      .filter((cell) => !definition.mustBeCastToWaterSite || isWaterSite(state, cell));
    return [
      ...summonCells.flatMap((cell) => [
      ...(!definition.mustBeCastBurrowed && !definition.mustBeCastSubmerged
        ? [{
          cardId,
          cardInstanceId: instanceId,
          casterInstanceId: player.avatar.card.instanceId,
          cell,
          kind: 'summon-minion' as const,
          manaCost: definition.manaCost,
        }]
        : []),
      ...(definition.burrowing && !definition.mustBeCastSubmerged && !isWaterSite(state, cell)
        ? [{
          cardId,
          cardInstanceId: instanceId,
          casterInstanceId: player.avatar.card.instanceId,
          cell,
          kind: 'summon-minion' as const,
          manaCost: definition.manaCost,
          region: 'underground' as const,
        }]
        : []),
      ...(definition.submerge && !definition.mustBeCastBurrowed && isWaterSite(state, cell)
        ? [{
          cardId,
          cardInstanceId: instanceId,
          casterInstanceId: player.avatar.card.instanceId,
          cell,
          kind: 'summon-minion' as const,
          manaCost: definition.manaCost,
          region: 'underwater' as const,
        }]
        : []),
      ]),
      ...(definition.voidwalk && !definition.mustBeCastBurrowed && !definition.mustBeCastSubmerged
        && !definition.mustBeCastToWaterSite
        ? REALM_CELLS.filter((cell) => !state.realm.sites[cell]
          && (!definition.mustBeCastToOuterColumn || cell[0] === 'A' || cell[0] === 'E'))
          .map((cell) => ({
          cardId,
          cardInstanceId: instanceId,
          casterInstanceId: player.avatar.card.instanceId,
          cell,
          kind: 'summon-minion' as const,
          manaCost: definition.manaCost,
          region: 'void' as const,
        }))
        : []),
    ];
  });
}

function magicDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const player = state.players[seat];
  const caster = unitStatus(state, {
    instanceId: player.avatar.card.instanceId,
    kind: 'avatar',
    seat,
  });
  const targets = (['north', 'south'] as const).flatMap((targetSeat) => unitRefs(state, targetSeat));
  return player.hand.spellbook.flatMap(({ cardId, instanceId }) => {
    const definition = cardDefinition(state, cardId);
    if (definition.cardType !== 'magic'
      || player.mana < definition.manaCost
      || !meetsThresholds(state, seat, definition.thresholds)) return [];
    const cast = {
      cardId,
      cardInstanceId: instanceId,
      casterInstanceId: player.avatar.card.instanceId,
      kind: 'cast-magic' as const,
    };
    if (definition.healController !== undefined) return [cast];
    if (definition.grantChargeToAllyThisTurn === true) {
      return unitRefs(state, seat).map((ally) => ({ ...cast, ally }));
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
    return targets.filter((target) => {
      const status = unitStatus(state, target);
      return (!definition.burrowTargetMinion && !definition.submergeTargetMinion || target.kind === 'minion')
        && (!definition.disableTargetNearbyMinionUntilNextTurn || target.kind === 'minion')
        && status.region === caster.region
        && (target.seat === seat || !status.stealthed)
        && (!definition.targetNearby && !definition.disableTargetNearbyMinionUntilNextTurn
          || status.location === caster.location
          || borderingCells(caster.location).includes(status.location)
          || diagonalCells(caster.location).includes(status.location));
    }).map((target) => ({ ...cast, target }));
  });
}

function requireCardId(value: string, path: string): void {
  if (!value.trim() || value.length > 256) throw new RangeError(`${path} must be 1-256 characters`);
}

function validateCardDefinition(card: GameCardDefinition, path: string): void {
  const elements: readonly GameElement[] = ['earth', 'fire', 'water', 'air'];
  if (card.cardType === 'avatar') {
    if (typeof card.drawSpell !== 'boolean') throw new RangeError(`${path}.drawSpell must be boolean`);
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
    if (card.connectsBurrowedAllies !== undefined && typeof card.connectsBurrowedAllies !== 'boolean') {
      throw new RangeError(`${path}.connectsBurrowedAllies must be boolean`);
    }
    if (card.sacrificeToDestroyNearbySite !== undefined
      && card.sacrificeToDestroyNearbySite !== true) {
      throw new RangeError(`${path}.sacrificeToDestroyNearbySite must be true when defined`);
    }
    return;
  }
  if (card.cardType === 'magic') {
    if (card.burrowTargetMinion !== undefined && typeof card.burrowTargetMinion !== 'boolean') {
      throw new RangeError(`${path}.burrowTargetMinion must be boolean`);
    }
    if (card.submergeTargetMinion !== undefined && card.submergeTargetMinion !== true) {
      throw new RangeError(`${path}.submergeTargetMinion must be true when defined`);
    }
    if (card.teleportAllyToTargetSite !== undefined && card.teleportAllyToTargetSite !== true) {
      throw new RangeError(`${path}.teleportAllyToTargetSite must be true when defined`);
    }
    if (card.returnMinionFromOwnCemetery !== undefined
      && card.returnMinionFromOwnCemetery !== true) {
      throw new RangeError(`${path}.returnMinionFromOwnCemetery must be true when defined`);
    }
    if (card.disableTargetNearbyMinionUntilNextTurn !== undefined
      && card.disableTargetNearbyMinionUntilNextTurn !== true) {
      throw new RangeError(`${path}.disableTargetNearbyMinionUntilNextTurn must be true when defined`);
    }
    if (card.grantChargeToAllyThisTurn !== undefined
      && card.grantChargeToAllyThisTurn !== true) {
      throw new RangeError(`${path}.grantChargeToAllyThisTurn must be true when defined`);
    }
    if (card.lureEnemyMinionOneStepCloser !== undefined
      && card.lureEnemyMinionOneStepCloser !== true) {
      throw new RangeError(`${path}.lureEnemyMinionOneStepCloser must be true when defined`);
    }
    const effectCount = Number(card.burrowTargetMinion === true)
      + Number(card.submergeTargetMinion === true)
      + Number(card.damageEachUnitAtLocationWithinTwoSteps !== undefined)
      + Number(card.damageRandomUnitAtLocation !== undefined)
      + Number(card.damageTargetUnit !== undefined)
      + Number(card.disableTargetNearbyMinionUntilNextTurn === true)
      + Number(card.grantChargeToAllyThisTurn === true)
      + Number(card.healController !== undefined)
      + Number(card.lureEnemyMinionOneStepCloser === true)
      + Number(card.returnMinionFromOwnCemetery === true)
      + Number(card.teleportAllyToTargetSite === true);
    if (effectCount !== 1) {
      throw new RangeError(`${path} must define exactly one supported Magic effect`);
    }
    if (card.targetNearby !== undefined && typeof card.targetNearby !== 'boolean') {
      throw new RangeError(`${path}.targetNearby must be boolean`);
    }
    if (card.targetNearby !== undefined && card.damageTargetUnit === undefined) {
      throw new RangeError(`${path}.targetNearby requires damageTargetUnit`);
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
  if (card.gainsStealthAtEndOfTurn !== undefined && typeof card.gainsStealthAtEndOfTurn !== 'boolean') {
    throw new RangeError(`${path}.gainsStealthAtEndOfTurn must be boolean`);
  }
  if (card.immobile !== undefined && typeof card.immobile !== 'boolean') {
    throw new RangeError(`${path}.immobile must be boolean`);
  }
  if (card.movementBonus !== undefined
    && (!Number.isSafeInteger(card.movementBonus)
      || card.movementBonus < 1
      || card.movementBonus > 2)) {
    throw new RangeError(`${path}.movementBonus must be a safe integer between 1 and 2`);
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
  if (card.stealth !== undefined && typeof card.stealth !== 'boolean') {
    throw new RangeError(`${path}.stealth must be boolean`);
  }
  if (card.strikesFirstWhileAttacking !== undefined && typeof card.strikesFirstWhileAttacking !== 'boolean') {
    throw new RangeError(`${path}.strikesFirstWhileAttacking must be boolean`);
  }
  if (card.submerge !== undefined && typeof card.submerge !== 'boolean') {
    throw new RangeError(`${path}.submerge must be boolean`);
  }
  if (card.ward !== undefined && typeof card.ward !== 'boolean') {
    throw new RangeError(`${path}.ward must be boolean`);
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
    if (cards[cardId]?.cardType !== 'minion' && cards[cardId]?.cardType !== 'magic') {
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
          life: card.life,
        }
        : card.cardType === 'site'
          ? {
            cardType: 'site' as const,
            ...(card.connectsBurrowedAllies === true ? { connectsBurrowedAllies: true } : {}),
            elements: [...card.elements],
            ...(card.genesisDiscardTopSpells === 2 ? { genesisDiscardTopSpells: 2 as const } : {}),
            ...(card.genesisDrawSpellPerAdjacentSameCard === true
              ? { genesisDrawSpellPerAdjacentSameCard: true }
              : {}),
            ...(card.genesisGainMana ? { genesisGainMana: card.genesisGainMana } : {}),
            ...(card.sacrificeToDestroyNearbySite === true
              ? { sacrificeToDestroyNearbySite: true as const }
              : {}),
          }
          : card.cardType === 'magic'
            ? {
              cardType: 'magic' as const,
              ...(card.burrowTargetMinion === true
                ? { burrowTargetMinion: true }
                : card.submergeTargetMinion === true
                  ? { submergeTargetMinion: true as const }
                : card.damageEachUnitAtLocationWithinTwoSteps !== undefined
                  ? { damageEachUnitAtLocationWithinTwoSteps: card.damageEachUnitAtLocationWithinTwoSteps }
                : card.damageRandomUnitAtLocation !== undefined
                  ? { damageRandomUnitAtLocation: card.damageRandomUnitAtLocation }
                : card.damageTargetUnit !== undefined
                  ? { damageTargetUnit: card.damageTargetUnit }
                  : card.disableTargetNearbyMinionUntilNextTurn === true
                    ? { disableTargetNearbyMinionUntilNextTurn: true as const }
                  : card.grantChargeToAllyThisTurn === true
                    ? { grantChargeToAllyThisTurn: true as const }
                  : card.lureEnemyMinionOneStepCloser === true
                    ? { lureEnemyMinionOneStepCloser: true as const }
                  : card.healController !== undefined
                    ? { healController: card.healController }
                    : card.returnMinionFromOwnCemetery === true
                      ? { returnMinionFromOwnCemetery: true as const }
                      : { teleportAllyToTargetSite: true as const }),
              manaCost: card.manaCost,
              ...(card.targetNearby === true ? { targetNearby: true } : {}),
              thresholds: { ...card.thresholds },
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
            defense: card.defense,
            ...(card.diesAtEndOfControllerTurn === true
              ? { diesAtEndOfControllerTurn: true as const }
              : {}),
            ...(card.genesisDrawSpell === true ? { genesisDrawSpell: true } : {}),
            ...(card.genesisDrawSite === true ? { genesisDrawSite: true } : {}),
            ...(card.genesisLoseControllerLife === 2 ? { genesisLoseControllerLife: 2 as const } : {}),
            ...(card.gainsStealthAtEndOfTurn === true ? { gainsStealthAtEndOfTurn: true } : {}),
            ...(card.immobile === true ? { immobile: true } : {}),
            ...(card.lethal === true ? { lethal: true } : {}),
            manaCost: card.manaCost,
            ...(card.movementBonus ? { movementBonus: card.movementBonus } : {}),
            ...(card.movesOnlyForward === true ? { movesOnlyForward: true } : {}),
            ...(card.movesOnlySideways === true ? { movesOnlySideways: true } : {}),
            ...(card.mustBeCastBurrowed === true ? { mustBeCastBurrowed: true } : {}),
            ...(card.mustBeCastSubmerged === true ? { mustBeCastSubmerged: true } : {}),
            ...(card.mustBeCastToWaterSite === true ? { mustBeCastToWaterSite: true } : {}),
            ...(card.provides ? { provides: card.provides } : {}),
            ...(card.ranged === true ? { ranged: true } : {}),
            ...(card.shootsDragProjectile === true ? { shootsDragProjectile: true } : {}),
            ...(card.stealth === true ? { stealth: true } : {}),
            ...(card.strikesFirstWhileAttacking === true ? { strikesFirstWhileAttacking: true } : {}),
            ...(card.submerge === true ? { submerge: true } : {}),
            ...(card.summonToAnySite === true ? { summonToAnySite: true } : {}),
            ...(card.mustBeCastToOuterColumn === true ? { mustBeCastToOuterColumn: true } : {}),
            ...(card.tapForMana ? { tapForMana: card.tapForMana } : {}),
            thresholds: { ...card.thresholds },
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
  Object.values(state.realm.sites)
    .filter((site) => site.controller === seat)
    .forEach((site) => {
      if (isRubble(site)) return;
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

function observePlayer(state: GameState, player: PlayerState, owner: GameSeat, viewer: GameSeat): ObservedPlayer {
  const own = owner === viewer;
  const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
  if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
  return deepFreeze({
    affinity: affinity(state, owner),
    atlasCount: player.atlas.length,
    avatar: {
      attack: avatarDefinition.attack,
      cardId: player.avatar.card.cardId,
      deathDoorTurn: player.avatar.deathDoorTurn,
      defense: avatarDefinition.defense,
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
    return {
      attack: definition.attack,
      cardId: unit.cardId,
      controller: unit.controller,
      damage: unit.damage,
      defense: definition.defense,
      disabled: minionDisabled(state, unit),
      instanceId: unit.instanceId,
      location: unit.location,
      owner: unit.owner,
      region: unit.region,
      stealthed: unit.stealthed,
      summoningSickness: unit.summoningSickness,
      tapped: unit.tapped,
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
    realm: { sites, units },
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
  voidwalk: boolean;
}> {
  if (ref.kind === 'avatar') {
    const avatar = state.players[ref.seat].avatar;
    if (avatar.card.instanceId !== ref.instanceId) throw new Error('unreachable Avatar reference');
    const definition = cardDefinition(state, avatar.card.cardId);
    if (definition.cardType !== 'avatar') throw new Error('Avatar lacks Avatar definition');
    return {
      airborne: false,
      attack: definition.attack,
      burrowing: false,
      canAttackSites: true,
      canMoveToDefend: true,
      canRespondToAttack: true,
      charge: false,
      connectsTopBottom: false,
      disabled: false,
      immobile: false,
      lethal: false,
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
      voidwalk: false,
    };
  }
  const unit = state.realm.units.find(({ instanceId }) => instanceId === ref.instanceId);
  if (!unit || unit.controller !== ref.seat) throw new Error('unreachable minion reference');
  const definition = cardDefinition(state, unit.cardId);
  if (definition.cardType !== 'minion') throw new Error('minion lacks minion definition');
  const disabled = minionDisabled(state, unit);
  return {
    airborne: !disabled && definition.airborne === true && unit.region === 'surface',
    attack: definition.attack,
    burrowing: !disabled && definition.burrowing === true,
    canAttackSites: !disabled && definition.cannotAttackSites !== true,
    canMoveToDefend: !disabled && definition.cannotDefend !== true,
    canRespondToAttack: !disabled && definition.cannotDefendOrIntercept !== true,
    charge: !disabled
      && (definition.charge === true || Boolean(unit.temporaryChargeSources?.length)),
    connectsTopBottom: !disabled && definition.connectsTopBottom === true,
    disabled,
    immobile: !disabled && definition.immobile === true,
    lethal: !disabled && definition.lethal === true,
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
    voidwalk: !disabled && definition.voidwalk === true,
  };
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
): readonly (readonly GameLocation[])[] {
  if (!locationExists(state, start)) return [];
  if (immobile) return [[start]];
  const paths: GameLocation[][] = [[start]];
  let frontier: GameLocation[][] = [[start]];
  for (let step = 0; step < maximumSteps; step += 1) {
    frontier = frontier.flatMap((path) => {
      const current = path.at(-1)!;
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
        .map((candidate) => [...path, candidate]);
    });
    paths.push(...frontier);
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

function rangedDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  const directions = ['east', 'north', 'south', 'west'] as const;
  const allUnits = [...unitRefs(state, 'north'), ...unitRefs(state, 'south')];
  return unitRefs(state, seat).flatMap((shooter) => {
    const status = unitStatus(state, shooter);
    if (!status.ranged || status.tapped || status.summoningSickness) return [];
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
      const next = projectileStep(status.location, direction);
      const nextLocation = next ? { cell: next, region: status.region } : undefined;
      const path = pathLocations([
        status.location,
        ...(nextLocation && locationExists(state, nextLocation) ? [nextLocation.cell] : []),
      ], status.region);
      const hits = nextLocation && locationExists(state, nextLocation)
        ? allUnits
          .filter((ref) => {
            const target = unitStatus(state, ref);
            return !target.stealthed
              && target.location === nextLocation.cell
              && target.region === nextLocation.region;
          })
          .sort((left, right) => left.instanceId.localeCompare(right.instanceId))
        : [];
      return hits.length > 0
        ? hits.map((hit) => ({ direction, hit, kind: 'shoot-projectile' as const, path, shooterInstanceId: shooter.instanceId }))
        : [{ direction, hit: null, kind: 'shoot-projectile' as const, path, shooterInstanceId: shooter.instanceId }];
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
    const remaining = unitStatus(state, pending.attacker).attack - assigned;
    const last = pending.allocations.length === pending.combatants.length - 1;
    const amounts = last ? [remaining] : Array.from({ length: remaining + 1 }, (_, amount) => amount);
    return amounts.map((amount) => ({
      amount,
      kind: 'allocate-strike' as const,
      targetInstanceId: target.instanceId,
    }));
  }
  if (!player.domainEstablished) {
    return player.hand.atlas.map(({ cardId, instanceId }) => ({
      cardId,
      cardInstanceId: instanceId,
      cell: player.avatar.location,
      kind: 'play-site',
    }));
  }
  const cells = player.avatar.tapped ? [] : legalSiteCells(state, seat);
  const avatarDefinition = cardDefinition(state, player.avatar.card.cardId);
  if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
  return [
    ...player.hand.atlas.flatMap(({ cardId, instanceId }) => cells.map((cell) => ({
      cardId,
      cardInstanceId: instanceId,
      cell,
      kind: 'play-site' as const,
    }))),
    ...(player.avatar.tapped ? [] : [{ kind: 'draw-site' as const }]),
    ...(!player.avatar.tapped && avatarDefinition.drawSpell ? [{ kind: 'draw-spell' as const }] : []),
    ...summonDescriptors(state, seat),
    ...magicDescriptors(state, seat),
    ...siteDestructionDescriptors(state, seat),
    ...manaAbilityDescriptors(state, seat),
    ...movementDescriptors(state, seat),
    ...dragProjectileDescriptors(state, seat),
    ...rangedDescriptors(state, seat),
    { kind: 'end-turn' },
  ];
}

function actionLabel(descriptor: GameActionDescriptor): string {
  if (descriptor.kind === 'mulligan') {
    const count = descriptor.atlasOrder.length + descriptor.spellbookOrder.length;
    return count === 0
      ? 'Keep opening hand'
      : `Mulligan ${count} (${descriptor.atlasOrder.length} atlas, ${descriptor.spellbookOrder.length} spellbook)`;
  }
  if (descriptor.kind === 'draw') return `Draw from ${descriptor.zone}`;
  if (descriptor.kind === 'draw-site') return 'Draw a site with Avatar';
  if (descriptor.kind === 'draw-spell') return 'Draw a spell with Avatar';
  if (descriptor.kind === 'play-site') return `Play ${descriptor.cardId} at ${descriptor.cell}`;
  if (descriptor.kind === 'activate-site-destruction') {
    return `Sacrifice site to destroy ${descriptor.targetCell}`;
  }
  if (descriptor.kind === 'summon-minion') {
    return `Summon ${descriptor.cardId} at ${descriptor.cell}${descriptor.region ? ` ${descriptor.region}` : ''} (${descriptor.manaCost} mana)`;
  }
  if (descriptor.kind === 'cast-magic') {
    return descriptor.cemeteryMinionInstanceId
      ? `Cast ${descriptor.cardId} to return minion ${descriptor.cemeteryMinionInstanceId.slice(0, 15)}…`
      : descriptor.ally && descriptor.temptedEnemy && descriptor.temptedDestination
        ? `Cast ${descriptor.cardId}: ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}… tempts minion ${descriptor.temptedEnemy.instanceId.slice(0, 15)}… to ${descriptor.temptedDestination.cell}`
      : descriptor.target
      ? `Cast ${descriptor.cardId} on ${descriptor.target.kind} ${descriptor.target.instanceId.slice(0, 15)}…`
      : descriptor.ally && descriptor.targetLocation
        ? `Cast ${descriptor.cardId} to teleport ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}… to ${descriptor.targetLocation.cell}`
      : descriptor.ally
        ? `Cast ${descriptor.cardId} to grant Charge to ${descriptor.ally.kind} ${descriptor.ally.instanceId.slice(0, 15)}…`
      : descriptor.targetLocation
        ? `Cast ${descriptor.cardId} at ${descriptor.targetLocation.cell} ${descriptor.targetLocation.region}`
      : `Cast ${descriptor.cardId}`;
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
  if (descriptor.kind === 'activate-mana') {
    return 'Tap ' + descriptor.unitInstanceId.slice(0, 15) + '… for ' + descriptor.amount + ' mana';
  }
  return 'End turn';
}

export function legalGameActions(state: GameState, seat: GameSeat): readonly GameLegalAction[] {
  return orderLegalActions(actionDescriptors(state, seat).map((descriptor) => ({
    actionId: opaqueActionId('sorcery-core-v1', seat, state.stateVersion, descriptor),
    descriptor,
    label: actionLabel(descriptor),
    seat,
    stateVersion: state.stateVersion,
  })));
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
      payload: { instanceId, seat: controller },
      type: 'stealth-lost',
    })),
  ];
}

function resolveMinionDeaths(
  state: GameState,
  startingPlayers: GameState['players'],
  units: readonly UnitInstance[],
  deaths: readonly UnitInstance[],
  defeatedAvatars: ReadonlySet<GameSeat>,
): Readonly<{
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
  const deadIds = new Set(deaths.map(({ instanceId }) => instanceId));
  const survivingUnits = units.filter(({ instanceId }) => !deadIds.has(instanceId));
  const deckLosers = new Set<GameSeat>();
  for (const dead of deaths) {
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
    if (!definition.deathriteDrawSite) continue;
    const owner = players[dead.owner];
    const [drawn, ...atlas] = owner.atlas;
    if (!drawn) {
      deckLosers.add(dead.owner);
      continue;
    }
    players[dead.owner] = deepFreeze({
      ...owner,
      atlas,
      hand: { ...owner.hand, atlas: [...owner.hand.atlas, drawn] },
    });
    deathOutcomes.push({
      payload: { seat: dead.owner, sourceInstanceId: dead.instanceId },
      type: 'site-drawn',
    });
  }
  for (const dead of deaths) {
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
    deathOutcomes.push({
      payload: { cardId: dead.cardId, instanceId: dead.instanceId, owner: dead.owner },
      type: 'minion-died',
    });
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
    outcomes: deathOutcomes,
    players: deepFreeze(players),
    terminal,
    units: survivingUnits,
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
      realm: { ...current.realm, units: resolution.units },
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
  const banishedState = deepFreeze({ ...state, realm: { ...state.realm, units } });
  const deathResolution = resolveMinionDeaths(
    banishedState,
    state.players,
    units,
    deaths,
    new Set<GameSeat>(),
  );
  const banishedOutcomes: readonly GameOutcome[] = banished.map((unit) => ({
    payload: { cardId: unit.cardId, instanceId: unit.instanceId, owner: unit.owner },
    type: 'minion-banished',
  }));
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
    realm: { ...state.realm, units: deathResolution.units },
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
    const moved = moveUnit(current, ref, next, false);
    current = deepFreeze({ ...current, players: moved.players, realm: moved.realm });
    actualPath.push(next);
    const settlement = settleRegionOccupancy(current);
    current = settlement.state;
    outcomes.push(...settlement.outcomes);
    removals.push(...settlement.removals);
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
  const terrainState = deepFreeze({
    ...state,
    realm: { sites, units },
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
    realm: deepFreeze({ sites, units: settlement.state.realm.units }),
    terminal: settlement.state.terminal,
  };
}

function resolveFightWindow(
  state: GameState,
  pending: PendingCombat,
  outcomes: readonly GameOutcome[],
  attackerStrikes: boolean,
  combatantsStrike: boolean,
  interactingRefs?: readonly GameUnitRef[],
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const allocations = new Map(pending.allocations.map(({ amount, targetInstanceId }) =>
    [targetInstanceId, amount]));
  const damage = new Map<StateHash, number>();
  const lethalDamage = new Set<StateHash>();
  const attackerStatus = unitStatus(state, pending.attacker);
  const attackerCanStrike = attackerStrikes && !attackerStatus.disabled;
  const strikingCombatants = combatantsStrike
    ? pending.combatants.filter((ref) => !unitStatus(state, ref).disabled)
    : [];
  if (strikingCombatants.length > 0) {
    damage.set(
      pending.attacker.instanceId,
      strikingCombatants.reduce((total, ref) => total + unitStatus(state, ref).attack, 0),
    );
  }
  if (attackerCanStrike) {
    pending.combatants.forEach((ref) => damage.set(ref.instanceId, allocations.get(ref.instanceId) ?? 0));
  }
  pending.combatants.forEach((ref) => {
    const striker = unitStatus(state, ref);
    if (strikingCombatants.some(({ instanceId }) => instanceId === ref.instanceId)
      && striker.lethal
      && striker.attack > 0) lethalDamage.add(pending.attacker.instanceId);
    if (attackerCanStrike && attackerStatus.lethal && (allocations.get(ref.instanceId) ?? 0) > 0) {
      lethalDamage.add(ref.instanceId);
    }
  });

  const players: Record<GameSeat, PlayerState> = {
    north: state.players.north,
    south: state.players.south,
  };
  let units = [...state.realm.units];
  const [interactedUnits, stealthOutcomes] = loseStealth(units, interactingRefs ?? [
    ...(attackerCanStrike ? [pending.attacker] : []),
    ...strikingCombatants,
  ]);
  units = [...interactedUnits];
  const defeatedAvatars = new Set<GameSeat>();
  const deaths: UnitInstance[] = [];
  const damageOutcomes: GameOutcome[] = [];

  const damagedRefs = [
    ...(strikingCombatants.length > 0 ? [pending.attacker] : []),
    ...(attackerCanStrike ? pending.combatants : []),
  ];
  for (const ref of damagedRefs) {
    const amount = damage.get(ref.instanceId) ?? 0;
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
    const accumulated = unit.damage + amount;
    units[index] = deepFreeze({ ...unit, damage: accumulated });
    damageOutcomes.push({
      payload: { accumulated, amount, direct: true, instanceId: ref.instanceId, seat: ref.seat },
      type: 'damage-dealt',
    });
    const definition = cardDefinition(state, unit.cardId);
    if (definition.cardType !== 'minion') throw new Error('fight minion lacks minion definition');
    if (accumulated > 0
      && (accumulated >= definition.defense || lethalDamage.has(ref.instanceId))) {
      deaths.push(units[index]!);
    }
  }
  damageOutcomes.push(...stealthOutcomes);

  const deathResolution = resolveMinionDeaths(state, players, units, deaths, defeatedAvatars);

  return [
    deepFreeze({
      ...state,
      decisionSeat: state.activeSeat,
      pendingCombat: null,
      phase: deathResolution.terminal.status === 'finished' ? 'terminal' : 'main',
      players: deathResolution.players,
      realm: { ...state.realm, units: deathResolution.units },
      terminal: deathResolution.terminal,
    }),
    [...outcomes, ...damageOutcomes, ...deathResolution.outcomes],
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
  if (returnStrikes && attacker.strikesFirstWhileAttacking) {
    const [earlyState, earlyOutcomes, earlyDraws] = resolveFightWindow(
      state,
      pending,
      outcomes,
      true,
      false,
    );
    const survivors = pending.combatants.filter((ref) =>
      ref.kind === 'avatar'
        || earlyState.realm.units.some(({ instanceId }) => instanceId === ref.instanceId));
    if (earlyState.terminal.status === 'finished' || survivors.length === 0) {
      return [withStateVersion(earlyState, {}), earlyOutcomes, earlyDraws];
    }
    const [resolved, resolvedOutcomes, normalDraws] = resolveFightWindow(
      earlyState,
      deepFreeze({ ...pending, combatants: survivors }),
      earlyOutcomes,
      false,
      true,
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
    const amount = unitStatus(state, pending.attacker).attack;
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
  const [units, stealthOutcomes] = loseStealth(state.realm.units, [pending.attacker]);
  const player = state.players[target.seat];
  const [updatedPlayer, lost, reachedDeathsDoor] = loseAvatarLife(player, amount, state.turnNumber);
  const life = updatedPlayer.avatar.life;
  return [
    withStateVersion(state, {
      decisionSeat: state.activeSeat,
      pendingCombat: null,
      phase: 'main',
      players: replacePlayer(state, target.seat, updatedPlayer),
      realm: { ...state.realm, units },
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
      ...stealthOutcomes,
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

  if (descriptor.kind === 'play-site') {
    const card = player.hand.atlas.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    if (!card) throw new Error('unreachable site card');
    const definition = cardDefinition(state, card.cardId);
    if (definition.cardType !== 'site') throw new Error('unreachable non-site card');
    const previousSite = state.realm.sites[descriptor.cell];
    const replacingRubble = previousSite !== undefined && isRubble(previousSite);
    const legalCell = !player.domainEstablished
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
    const genesisDrawFailed = genesisSpellDraws.length < genesisSpellDrawCount;
    const updatedPlayer = deepFreeze({
      ...player,
      avatar: { ...player.avatar, tapped: true },
      cemetery: [...player.cemetery, ...genesisSpellDiscards],
      domainEstablished: true,
      hand: {
        ...player.hand,
        atlas: player.hand.atlas.filter(({ instanceId }) => instanceId !== card.instanceId),
        spellbook: [...player.hand.spellbook, ...genesisSpellDraws],
      },
      mana: player.mana + 1 + (definition.genesisGainMana ?? 0),
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
    const placedState = deepFreeze({
      ...state,
      players: replacePlayer(state, seat, updatedPlayer),
      realm: {
        ...state.realm,
        sites: { ...state.realm.sites, [descriptor.cell]: site },
        units: placedUnits,
      },
    });
    const settlement = settleRegionOccupancy(placedState);
    const terminal = genesisDrawFailed
      ? { loser: seat, reason: 'deck_empty' as const, status: 'finished' as const, winner }
      : settlement.state.terminal;
    const settlementOutcomes = genesisDrawFailed
      ? settlement.outcomes.filter(({ type }) => type !== 'game-ended')
      : settlement.outcomes;
    return [
      withStateVersion(state, {
        ...(terminal.status === 'finished' ? { phase: 'terminal' as const } : {}),
        players: settlement.state.players,
        realm: settlement.state.realm,
        terminal,
      }),
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
        ...(definition.genesisGainMana
          ? [{
            payload: { amount: definition.genesisGainMana, seat, sourceInstanceId: card.instanceId },
            type: 'mana-gained',
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
        ...settlementOutcomes,
        ...(genesisDrawFailed
          ? [{ payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' }]
          : []),
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
        && (candidate.target === undefined && descriptor.target === undefined
          || candidate.target !== undefined
            && descriptor.target !== undefined
            && candidate.target.instanceId === descriptor.target.instanceId
            && candidate.target.kind === descriptor.target.kind
            && candidate.target.seat === descriptor.target.seat)
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
    if (!card || !definition || definition.cardType !== 'magic' || !legal) {
      throw new Error('unreachable illegal Magic cast');
    }
    const caster: GameUnitRef = {
      instanceId: descriptor.casterInstanceId,
      kind: 'avatar',
      seat,
    };
    const paidPlayer = deepFreeze({
      ...player,
      hand: {
        ...player.hand,
        spellbook: player.hand.spellbook.filter(({ instanceId }) => instanceId !== card.instanceId),
      },
      mana: player.mana - definition.manaCost,
    });
    const paidState = deepFreeze({ ...state, players: replacePlayer(state, seat, paidPlayer) });
    const owner = paidState.players[card.owner];
    const castState = deepFreeze({
      ...paidState,
      players: replacePlayer(paidState, card.owner, deepFreeze({
        ...owner,
        cemetery: [...owner.cemetery, card],
      })),
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
        ...(descriptor.targetLocation ? { targetLocation: descriptor.targetLocation } : {}),
        ...(descriptor.ally
          ? { allyInstanceId: descriptor.ally.instanceId, allySeat: descriptor.ally.seat }
          : {}),
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
          castOutcome,
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
        return [withStateVersion(castState, {}), [castOutcome, resolved], []];
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
          castOutcome,
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
    if (definition.grantChargeToAllyThisTurn === true) {
      if (!descriptor.ally) throw new Error('unreachable Charge cast');
      if (descriptor.ally.kind === 'avatar') {
        return [
          withStateVersion(castState, {}),
          [
            castOutcome,
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
          castOutcome,
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
    if (definition.lureEnemyMinionOneStepCloser === true) {
      if (!descriptor.ally || !descriptor.temptedEnemy || !descriptor.temptedDestination) {
        return [withStateVersion(castState, {}), [castOutcome, resolved], []];
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
      const outcomes = [castOutcome, lured, ...path.outcomes];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(path.state, {}),
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
            castOutcome,
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
      return [
        withStateVersion(disabledState, {}),
        [
          castOutcome,
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
          resolved,
        ],
        [],
      ];
    }
    if (definition.burrowTargetMinion === true || definition.submergeTargetMinion === true) {
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
            castOutcome,
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
        return [withStateVersion(castState, {}), [castOutcome, resolved], []];
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
      const outcomes = [castOutcome, movedOutcome, ...settlement.outcomes];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(settlement.state, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
      ];
    }
    if (definition.teleportAllyToTargetSite === true) {
      if (!descriptor.ally || !descriptor.targetLocation || !descriptor.targetSiteInstanceId) {
        throw new Error('unreachable Teleport cast');
      }
      const status = unitStatus(castState, descriptor.ally);
      const from: GameLocation = { cell: status.location, region: status.region };
      if (sameLocation(from, descriptor.targetLocation)) {
        return [withStateVersion(castState, {}), [castOutcome, resolved], []];
      }
      const moved = moveUnit(castState, descriptor.ally, descriptor.targetLocation, false);
      const teleportedState = deepFreeze({
        ...castState,
        players: moved.players,
        realm: moved.realm,
      });
      const teleported: GameOutcome = {
        payload: {
          from,
          seat: descriptor.ally.seat,
          sourceInstanceId: card.instanceId,
          targetInstanceId: descriptor.ally.instanceId,
          targetSiteInstanceId: descriptor.targetSiteInstanceId,
          to: descriptor.targetLocation,
        },
        type: 'unit-teleported',
      };
      const settlement = settleRegionOccupancy(teleportedState);
      const outcomes = [castOutcome, teleported, ...settlement.outcomes];
      const terminalIndex = outcomes.findIndex(({ type }) => type === 'game-ended');
      return [
        withStateVersion(settlement.state, {}),
        terminalIndex < 0
          ? [...outcomes, resolved]
          : [...outcomes.slice(0, terminalIndex), resolved, ...outcomes.slice(terminalIndex)],
        [],
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
        return [withStateVersion(castState, {}), [castOutcome, resolved], []];
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
        [castOutcome, ...allocationOutcomes],
        true,
        false,
        [caster],
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
        return [withStateVersion(castState, {}), [castOutcome, resolved], []];
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
        [castOutcome, {
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
    const target = unitStatus(castState, descriptor.target);
    const pending: PendingCombat = deepFreeze({
      allocations: [{ amount: definition.damageTargetUnit, targetInstanceId: descriptor.target.instanceId }],
      attacker: caster,
      attackingSeat: seat,
      cell: target.location,
      combatants: [descriptor.target],
      defenders: [],
      originalTarget: descriptor.target,
      ...(target.region === 'surface' ? {} : { region: target.region as 'underground' | 'underwater' | 'void' }),
      targetRemoved: false,
    });
    const [damaged, outcomes, randomDraws] = resolveFightWindow(
      castState,
      pending,
      [castOutcome, {
        payload: {
          amount: definition.damageTargetUnit,
          sourceInstanceId: card.instanceId,
          targetInstanceId: descriptor.target.instanceId,
        },
        type: 'magic-damage-allocated',
      }],
      true,
      false,
      [caster],
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

  if (descriptor.kind === 'summon-minion') {
    const card = player.hand.spellbook.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    const definition = card && cardDefinition(state, card.cardId);
    const legal = summonDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'summon-minion'
        && candidate.cardInstanceId === descriptor.cardInstanceId
        && candidate.casterInstanceId === descriptor.casterInstanceId
        && candidate.cell === descriptor.cell
        && candidate.manaCost === descriptor.manaCost
        && (candidate.region ?? 'surface') === (descriptor.region ?? 'surface'));
    if (!card || !definition || definition.cardType !== 'minion' || !legal) {
      throw new Error('unreachable illegal minion summon');
    }
    const unit: UnitInstance = deepFreeze({
      ...card,
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
      hand: {
        ...player.hand,
        spellbook: player.hand.spellbook.filter(({ instanceId }) => instanceId !== card.instanceId),
      },
      mana: player.mana - definition.manaCost,
    });
    const realm = { ...state.realm, units: [...state.realm.units, unit] };
    const summoned: GameOutcome = {
      payload: {
        cardId: card.cardId,
        casterInstanceId: descriptor.casterInstanceId,
        cell: descriptor.cell,
        instanceId: card.instanceId,
        manaPaid: definition.manaCost,
        ...(descriptor.region ? { region: descriptor.region } : {}),
        seat,
      },
      type: 'minion-summoned',
    };
    const summonedState = deepFreeze({
      ...state,
      players: replacePlayer(state, seat, updatedPlayer),
      realm,
    });
    const settlement = settleRegionOccupancy(summonedState);
    const summonedUnitSurvived = settlement.state.realm.units
      .some(({ instanceId }) => instanceId === unit.instanceId);
    if (!summonedUnitSurvived || settlement.state.terminal.status === 'finished') {
      return [
        withStateVersion(settlement.state, {}),
        [summoned, ...settlement.outcomes],
        [],
      ];
    }
    const settledPlayer = settlement.state.players[seat];
    const genesisDrawZone = definition.genesisDrawSite
      ? 'atlas'
      : definition.genesisDrawSpell ? 'spellbook' : undefined;
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
          summoned,
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
        [],
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
            summoned,
            ...settlement.outcomes,
            { payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' },
          ],
          [],
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
          summoned,
          ...settlement.outcomes,
          {
            payload: { seat, sourceInstanceId: card.instanceId },
            type: genesisDrawZone === 'atlas' ? 'site-drawn' : 'spell-drawn',
          },
        ],
        [],
      ];
    }
    return [
      withStateVersion(settlement.state, {}),
      [summoned, ...settlement.outcomes],
      [],
    ];
  }

  if (descriptor.kind === 'activate-mana') {
    const legal = manaAbilityDescriptors(state, seat).some((candidate) =>
      candidate.kind === 'activate-mana'
        && candidate.amount === descriptor.amount
        && candidate.unitInstanceId === descriptor.unitInstanceId);
    const unit = state.realm.units.find(({ instanceId }) => instanceId === descriptor.unitInstanceId);
    if (!legal || !unit) throw new Error('unreachable illegal mana activation');
    const [units, stealthOutcomes] = loseStealth(
      state.realm.units.map((candidate) => candidate.instanceId === unit.instanceId
        ? deepFreeze({ ...candidate, tapped: true })
        : candidate),
      [{ instanceId: unit.instanceId, kind: 'minion', seat }],
    );
    return [
      withStateVersion(state, {
        players: replacePlayer(state, seat, deepFreeze({ ...player, mana: player.mana + descriptor.amount })),
        realm: { ...state.realm, units },
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
        ...stealthOutcomes,
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
    const [units, stealthOutcomes] = loseStealth(tapped.realm.units, [shooter]);
    const shotState = deepFreeze({
      ...state,
      players: tapped.players,
      realm: { ...tapped.realm, units },
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
      return [withStateVersion(shotState, {}), [shot, ...stealthOutcomes], []];
    }
    const amount = shooterStatus.attack;
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
    }), [shot, ...stealthOutcomes, strike], false);
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
    const [units, stealthOutcomes] = loseStealth(tapped.realm.units, [shooter]);
    const shotState = deepFreeze({
      ...state,
      players: tapped.players,
      realm: { ...tapped.realm, units },
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
      return [withStateVersion(shotState, {}), [shot, ...stealthOutcomes], []];
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
    const outcomes = [shot, ...stealthOutcomes, dragged, ...path.outcomes];
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
    if (!defenderArrived || path.state.terminal.status === 'finished') {
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
    const removesSite = pending.originalTarget?.kind === 'site' && !pending.targetRemoved;
    return [
      withStateVersion(path.state, {
        pendingCombat: deepFreeze({
          ...pending,
          defenders: [...pending.defenders, ref],
          targetRemoved: pending.targetRemoved || removesSite,
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
            payload: { instanceId: pending.originalTarget!.instanceId, kind: 'site' },
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
    if (deck.length === 0) {
      const winner = otherSeat(seat);
      const players = avatarDraw
        ? replacePlayer(state, seat, deepFreeze({
          ...player,
          avatar: { ...player.avatar, tapped: true },
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
      ...(avatarDraw ? { avatar: { ...player.avatar, tapped: true } } : {}),
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
  const endingPlayer = deepFreeze({ ...endState.players[seat], mana: 0 });
  const nextPlayer = endState.players[nextSeat];
  const startingPlayer = deepFreeze({
    ...nextPlayer,
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
  const expiredDisableEffects = endState.realm.units.flatMap((unit) =>
    (unit.disableEffects ?? [])
      .filter(({ expiresAtSeat }) => expiresAtSeat === nextSeat)
      .map((effect) => ({ effect, unit })));
  const chargeExpired: GameOutcome[] = [];
  const units = endState.realm.units.map((unit) => {
    const {
      disableEffects: previousDisableEffects,
      temporaryChargeSources,
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
    return deepFreeze({
      ...baseUnit,
      ...(disableEffects.length > 0 ? { disableEffects } : {}),
      damage: 0,
      ...(stealthGainedIds.has(unit.instanceId) ? { stealthed: true } : {}),
      ...(unit.controller === seat ? { summoningSickness: false } : {}),
      ...(unit.controller === nextSeat ? { tapped: false } : {}),
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
      realm: { ...endState.realm, units },
      turnNumber,
    }),
    [
      ...endOfTurnDeaths.outcomes,
      ...stealthGained.map(({ controller, instanceId }) => ({
        payload: { instanceId, seat: controller },
        type: 'stealth-gained',
      })),
      ...chargeExpired,
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
  const [nextState, outcomes, randomDraws] = applyDescriptor(state, action.descriptor, session.manifest);
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
