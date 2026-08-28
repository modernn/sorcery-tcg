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
    elements: readonly GameElement[];
    genesisDrawSpellPerAdjacentSameCard?: boolean;
    genesisGainMana?: number;
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
    genesisDrawSpell?: boolean;
    genesisDrawSite?: boolean;
    gainsStealthAtEndOfTurn?: boolean;
    lethal?: boolean;
    manaCost: number;
    movementBonus?: 1 | 2;
    movesOnlyForward?: boolean;
    movesOnlySideways?: boolean;
    mustBeCastBurrowed?: boolean;
    mustBeCastSubmerged?: boolean;
    provides?: GameElement;
    ranged?: boolean;
    stealth?: boolean;
    strikesFirstWhileAttacking?: boolean;
    submerge?: boolean;
    summonToAnySite?: boolean;
    mustBeCastToOuterColumn?: boolean;
    tapForMana?: number;
    thresholds: GameThresholds;
    voidwalk?: boolean;
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

type UnitInstance = Readonly<CardInstance & {
  controller: GameSeat;
  damage: number;
  location: RealmCell;
  region: GameRegion;
  stealthed: boolean;
  summoningSickness: boolean;
  tapped: boolean;
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
    sites: Readonly<Partial<Record<RealmCell, SiteInstance>>>;
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
    sites: Readonly<Partial<Record<RealmCell, Readonly<{
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
    cardId: string;
    cardInstanceId: string;
    casterInstanceId: string;
    cell: RealmCell;
    kind: 'summon-minion';
    manaCost: number;
    region?: 'underground' | 'underwater' | 'void';
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

function legalSiteCells(state: GameState, seat: GameSeat): readonly RealmCell[] {
  return [...new Set(Object.entries(state.realm.sites)
    .filter(([, site]) => site.controller === seat)
    .flatMap(([cell]) => borderingCells(cell as RealmCell)))]
    .filter((cell) => !state.realm.sites[cell])
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
  if (!site) return false;
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
    return [
      ...(definition.mustBeCastToOuterColumn
        ? (definition.summonToAnySite ? siteCells : controlledCells)
          .filter((cell) => cell[0] === 'A' || cell[0] === 'E')
        : definition.summonToAnySite ? siteCells : controlledCells).flatMap((cell) => [
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
  if (card.genesisDrawSite && card.genesisDrawSpell) {
    throw new RangeError(`${path} simultaneous Genesis site and spell draws are unsupported`);
  }
  if (card.gainsStealthAtEndOfTurn !== undefined && typeof card.gainsStealthAtEndOfTurn !== 'boolean') {
    throw new RangeError(`${path}.gainsStealthAtEndOfTurn must be boolean`);
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
  if (card.ranged !== undefined && typeof card.ranged !== 'boolean') {
    throw new RangeError(`${path}.ranged must be boolean`);
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
    if (cards[cardId]?.cardType !== 'minion') {
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
            elements: [...card.elements],
            ...(card.genesisDrawSpellPerAdjacentSameCard === true
              ? { genesisDrawSpellPerAdjacentSameCard: true }
              : {}),
            ...(card.genesisGainMana ? { genesisGainMana: card.genesisGainMana } : {}),
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
            ...(card.genesisDrawSpell === true ? { genesisDrawSpell: true } : {}),
            ...(card.genesisDrawSite === true ? { genesisDrawSite: true } : {}),
            ...(card.gainsStealthAtEndOfTurn === true ? { gainsStealthAtEndOfTurn: true } : {}),
            ...(card.lethal === true ? { lethal: true } : {}),
            manaCost: card.manaCost,
            ...(card.movementBonus ? { movementBonus: card.movementBonus } : {}),
            ...(card.movesOnlyForward === true ? { movesOnlyForward: true } : {}),
            ...(card.movesOnlySideways === true ? { movesOnlySideways: true } : {}),
            ...(card.mustBeCastBurrowed === true ? { mustBeCastBurrowed: true } : {}),
            ...(card.mustBeCastSubmerged === true ? { mustBeCastSubmerged: true } : {}),
            ...(card.provides ? { provides: card.provides } : {}),
            ...(card.ranged === true ? { ranged: true } : {}),
            ...(card.stealth === true ? { stealth: true } : {}),
            ...(card.strikesFirstWhileAttacking === true ? { strikesFirstWhileAttacking: true } : {}),
            ...(card.submerge === true ? { submerge: true } : {}),
            ...(card.summonToAnySite === true ? { summonToAnySite: true } : {}),
            ...(card.mustBeCastToOuterColumn === true ? { mustBeCastToOuterColumn: true } : {}),
            ...(card.tapForMana ? { tapForMana: card.tapForMana } : {}),
            thresholds: { ...card.thresholds },
            ...(card.voidwalk === true ? { voidwalk: true } : {}),
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
): Readonly<{ engine: EngineState; index: number; randomDraws: readonly EngineRandomDraw[] }> {
  const limit = Math.floor(UINT32_RANGE / exclusiveMaximum) * exclusiveMaximum;
  const randomDraws: EngineRandomDraw[] = [];
  let nextEngine = engine;
  while (true) {
    const prePrngStateHash = identityHash(asJson(nextEngine.prng));
    const draw = drawUint32(nextEngine);
    nextEngine = draw.nextState;
    randomDraws.push(deepFreeze({
      domain: { accepted: draw.value < limit, exclusiveMaximum, kind: 'shuffle_index_candidate' },
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

function affinity(state: GameState, seat: GameSeat): GameThresholds {
  const total: Record<GameElement, number> = { air: 0, earth: 0, fire: 0, water: 0 };
  Object.values(state.realm.sites)
    .filter((site) => site.controller === seat)
    .forEach((site) => {
      const definition = cardDefinition(state, site.cardId);
      if (definition.cardType !== 'site') throw new Error('realm site lacks site definition');
      definition.elements.forEach((element) => {
        total[element] += 1;
      });
    });
  state.realm.units
    .filter((unit) => unit.controller === seat)
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
  return {
    airborne: definition.airborne === true && unit.region === 'surface',
    attack: definition.attack,
    burrowing: definition.burrowing === true,
    canAttackSites: definition.cannotAttackSites !== true,
    canMoveToDefend: definition.cannotDefend !== true,
    canRespondToAttack: definition.cannotDefendOrIntercept !== true,
    charge: definition.charge === true,
    connectsTopBottom: definition.connectsTopBottom === true,
    lethal: definition.lethal === true,
    location: unit.location,
    movementSteps: 1 + (definition.movementBonus ?? 0),
    movesOnlyForward: definition.movesOnlyForward === true,
    movesOnlySideways: definition.movesOnlySideways === true,
    ranged: definition.ranged === true,
    region: unit.region,
    stealthed: unit.stealthed,
    strikesFirstWhileAttacking: definition.strikesFirstWhileAttacking === true,
    submerge: definition.submerge === true,
    summoningSickness: unit.summoningSickness,
    tapped: unit.tapped,
    voidwalk: definition.voidwalk === true,
  };
}

function readyUnit(state: GameState, ref: GameUnitRef): boolean {
  const unit = unitStatus(state, ref);
  return !unit.tapped && (!unit.summoningSickness || unit.charge);
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

function movementPaths(
  state: GameState,
  start: GameLocation,
  maximumSteps: number,
  airborne = false,
  movesOnlySideways = false,
  movesOnlyForwardFor: GameSeat | null = null,
  burrowing = false,
  submerge = false,
  voidwalk = false,
  connectsTopBottom = false,
): readonly (readonly GameLocation[])[] {
  if (!locationExists(state, start)) return [];
  const paths: GameLocation[][] = [[start]];
  let frontier: GameLocation[][] = [[start]];
  for (let step = 0; step < maximumSteps; step += 1) {
    frontier = frontier.flatMap((path) => {
      const current = path.at(-1)!;
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
        .filter((candidate) => locationExists(state, candidate)
          && (!movesOnlySideways
            || candidate.region === current.region && candidate.cell[1] === current.cell[1])
          && (!movesOnlyForwardFor
            || candidate.region === current.region
              && candidate.cell[0] === current.cell[0]
              && (Number(candidate.cell[1]) - Number(current.cell[1])
                === (movesOnlyForwardFor === 'north' ? -1 : 1)
                || connectsTopBottom && (movesOnlyForwardFor === 'north'
                  ? current.cell[1] === '1' && candidate.cell[1] === '4'
                  : current.cell[1] === '4' && candidate.cell[1] === '1')))
          && !path.some((from, index) =>
            sameLocation(from, current) && path[index + 1] !== undefined
              && sameLocation(path[index + 1]!, candidate)))
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
    unit.airborne,
    unit.movesOnlySideways,
    unit.movesOnlyForward ? ref.seat : null,
    unit.burrowing,
    unit.submerge,
    unit.voidwalk,
    unit.connectsTopBottom,
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
      unit.airborne,
      unit.movesOnlySideways,
      unit.movesOnlyForward ? ref.seat : null,
      unit.burrowing,
      unit.submerge,
      unit.voidwalk,
      unit.connectsTopBottom,
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

function manaAbilityDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  return state.realm.units.flatMap((unit) => {
    if (unit.controller !== seat || unit.tapped || unit.summoningSickness) return [];
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion' && definition.tapForMana
      ? [{ amount: definition.tapForMana, kind: 'activate-mana' as const, unitInstanceId: unit.instanceId }]
      : [];
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
    ...manaAbilityDescriptors(state, seat),
    ...movementDescriptors(state, seat),
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
  if (descriptor.kind === 'summon-minion') {
    return `Summon ${descriptor.cardId} at ${descriptor.cell}${descriptor.region ? ` ${descriptor.region}` : ''} (${descriptor.manaCost} mana)`;
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

function moveAndTapUnit(
  state: GameState,
  ref: GameUnitRef,
  location: GameLocation,
): Readonly<{ players: GameState['players']; realm: GameState['realm'] }> {
  if (ref.kind === 'avatar') {
    const player = state.players[ref.seat];
    if (player.avatar.card.instanceId !== ref.instanceId) throw new Error('unreachable Avatar move');
    return {
      players: replacePlayer(state, ref.seat, deepFreeze({
        ...player,
        avatar: { ...player.avatar, location: location.cell, region: location.region, tapped: true },
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
        ? deepFreeze({ ...unit, location: location.cell, region: location.region, tapped: true })
        : unit),
    },
  };
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

function resolveFightWindow(
  state: GameState,
  pending: PendingCombat,
  outcomes: readonly GameOutcome[],
  attackerStrikes: boolean,
  combatantsStrike: boolean,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const allocations = new Map(pending.allocations.map(({ amount, targetInstanceId }) =>
    [targetInstanceId, amount]));
  const damage = new Map<StateHash, number>();
  const lethalDamage = new Set<StateHash>();
  const attackerStatus = unitStatus(state, pending.attacker);
  if (combatantsStrike) {
    damage.set(
      pending.attacker.instanceId,
      pending.combatants.reduce((total, ref) => total + unitStatus(state, ref).attack, 0),
    );
  }
  if (attackerStrikes) {
    pending.combatants.forEach((ref) => damage.set(ref.instanceId, allocations.get(ref.instanceId) ?? 0));
  }
  pending.combatants.forEach((ref) => {
    const striker = unitStatus(state, ref);
    if (combatantsStrike && striker.lethal && striker.attack > 0) lethalDamage.add(pending.attacker.instanceId);
    if (attackerStrikes && attackerStatus.lethal && (allocations.get(ref.instanceId) ?? 0) > 0) {
      lethalDamage.add(ref.instanceId);
    }
  });

  const players: Record<GameSeat, PlayerState> = {
    north: state.players.north,
    south: state.players.south,
  };
  let units = [...state.realm.units];
  const [interactedUnits, stealthOutcomes] = loseStealth(units, [
    ...(attackerStrikes ? [pending.attacker] : []),
    ...(combatantsStrike ? pending.combatants : []),
  ]);
  units = [...interactedUnits];
  const defeatedAvatars = new Set<GameSeat>();
  const deaths: UnitInstance[] = [];
  const damageOutcomes: GameOutcome[] = [];

  const damagedRefs = [
    ...(combatantsStrike ? [pending.attacker] : []),
    ...(attackerStrikes ? pending.combatants : []),
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

  const deadIds = new Set(deaths.map(({ instanceId }) => instanceId));
  units = units.filter(({ instanceId }) => !deadIds.has(instanceId));
  const deckLosers = new Set<GameSeat>();
  for (const dead of deaths) {
    const definition = cardDefinition(state, dead.cardId);
    if (definition.cardType !== 'minion') continue;
    if (definition.deathriteHeal) {
      const controller = players[dead.controller];
      const avatarDefinition = cardDefinition(state, controller.avatar.card.cardId);
      if (avatarDefinition.cardType !== 'avatar') throw new Error('player Avatar lacks Avatar definition');
      const life = controller.avatar.life === 0
        ? 0
        : Math.min(avatarDefinition.life, controller.avatar.life + definition.deathriteHeal);
      const amount = life - controller.avatar.life;
      players[dead.controller] = deepFreeze({
        ...controller,
        avatar: { ...controller.avatar, life },
      });
      damageOutcomes.push({
        payload: {
          amount,
          attemptedAmount: definition.deathriteHeal,
          life,
          seat: dead.controller,
          sourceInstanceId: dead.instanceId,
        },
        type: 'avatar-healed',
      });
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
    damageOutcomes.push({
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
    damageOutcomes.push({
      payload: { cardId: dead.cardId, instanceId: dead.instanceId, owner: dead.owner },
      type: 'minion-died',
    });
  }

  let terminal: GameTerminal = { status: 'active' };
  const endingOutcomes: GameOutcome[] = [];
  const losers = new Set([...defeatedAvatars, ...deckLosers]);
  if (losers.size === 2) {
    const reason = defeatedAvatars.size === 2 && deckLosers.size === 0
      ? 'simultaneous_avatar_defeat'
      : 'simultaneous_defeat';
    terminal = { reason, result: 'draw', status: 'finished' };
    endingOutcomes.push({
      payload: { reason, result: 'draw' },
      type: 'game-ended',
    });
  } else if (losers.size === 1) {
    const loser = [...losers][0]!;
    const winner = otherSeat(loser);
    const reason = defeatedAvatars.has(loser) ? 'avatar_defeated' : 'deck_empty';
    terminal = { loser, reason, status: 'finished', winner };
    endingOutcomes.push({
      payload: { loser, reason, winner },
      type: 'game-ended',
    });
  }

  return [
    deepFreeze({
      ...state,
      decisionSeat: state.activeSeat,
      pendingCombat: null,
      phase: terminal.status === 'finished' ? 'terminal' : 'main',
      players: deepFreeze(players),
      realm: { ...state.realm, units },
      terminal,
    }),
    [...outcomes, ...damageOutcomes, ...endingOutcomes],
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
  const life = Math.max(0, player.avatar.life - amount);
  const lost = player.avatar.life - life;
  const updatedPlayer = deepFreeze({
    ...player,
    avatar: {
      ...player.avatar,
      ...(player.avatar.life > 0 && life === 0 ? { deathDoorTurn: state.turnNumber } : {}),
      life,
    },
  });
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
      ...(player.avatar.life > 0 && life === 0
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

  if (descriptor.kind === 'play-site') {
    const card = player.hand.atlas.find(({ cardId, instanceId }) =>
      instanceId === descriptor.cardInstanceId && cardId === descriptor.cardId);
    if (!card) throw new Error('unreachable site card');
    const definition = cardDefinition(state, card.cardId);
    if (definition.cardType !== 'site') throw new Error('unreachable non-site card');
    const legalCell = !player.domainEstablished
      ? !player.avatar.tapped
        && descriptor.cell === player.avatar.location
        && !state.realm.sites[descriptor.cell]
      : !player.avatar.tapped && legalSiteCells(state, seat).includes(descriptor.cell);
    if (!legalCell) throw new Error('unreachable illegal site cell');
    const site = deepFreeze({ ...card, controller: seat });
    const genesisSpellDrawCount = definition.genesisDrawSpellPerAdjacentSameCard
      ? borderingCells(descriptor.cell)
        .filter((cell) => state.realm.sites[cell]?.cardId === card.cardId).length
      : 0;
    const genesisSpellDraws = player.spellbook.slice(0, genesisSpellDrawCount);
    const genesisDrawFailed = genesisSpellDraws.length < genesisSpellDrawCount;
    const updatedPlayer = deepFreeze({
      ...player,
      avatar: { ...player.avatar, tapped: true },
      domainEstablished: true,
      hand: {
        ...player.hand,
        atlas: player.hand.atlas.filter(({ instanceId }) => instanceId !== card.instanceId),
        spellbook: [...player.hand.spellbook, ...genesisSpellDraws],
      },
      mana: player.mana + 1 + (definition.genesisGainMana ?? 0),
      spellbook: player.spellbook.slice(genesisSpellDraws.length),
    });
    const winner = otherSeat(seat);
    return [
      withStateVersion(state, {
        ...(genesisDrawFailed
          ? {
            phase: 'terminal' as const,
            terminal: { loser: seat, reason: 'deck_empty' as const, status: 'finished' as const, winner },
          }
          : {}),
        players: replacePlayer(state, seat, updatedPlayer),
        realm: {
          ...state.realm,
          sites: { ...state.realm.sites, [descriptor.cell]: site },
          units: state.realm.units.map((unit) => unit.location === descriptor.cell && unit.region === 'void'
            ? { ...unit, region: 'surface' as const }
            : unit),
        },
      }),
      [
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
        ...(genesisDrawFailed
          ? [{ payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' }]
          : []),
      ],
      [],
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
    const genesisDrawZone = definition.genesisDrawSite
      ? 'atlas'
      : definition.genesisDrawSpell ? 'spellbook' : undefined;
    if (genesisDrawZone) {
      const [drawn, ...remaining] = updatedPlayer[genesisDrawZone];
      if (!drawn) {
        const winner = otherSeat(seat);
        return [
          withStateVersion(state, {
            phase: 'terminal',
            players: replacePlayer(state, seat, updatedPlayer),
            realm,
            terminal: { loser: seat, reason: 'deck_empty', status: 'finished', winner },
          }),
          [summoned, { payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' }],
          [],
        ];
      }
      const drawingPlayer = deepFreeze({
        ...updatedPlayer,
        [genesisDrawZone]: remaining,
        hand: {
          ...updatedPlayer.hand,
          [genesisDrawZone]: [...updatedPlayer.hand[genesisDrawZone], drawn],
        },
      });
      return [
        withStateVersion(state, {
          players: replacePlayer(state, seat, drawingPlayer),
          realm,
        }),
        [
          summoned,
          {
            payload: { seat, sourceInstanceId: card.instanceId },
            type: genesisDrawZone === 'atlas' ? 'site-drawn' : 'spell-drawn',
          },
        ],
        [],
      ];
    }
    return [
      withStateVersion(state, {
        players: replacePlayer(state, seat, updatedPlayer),
        realm,
      }),
      [summoned],
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
    const moved = moveAndTapUnit(state, ref, descriptor.to);
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
      withStateVersion(state, {
        pendingCombat: pending,
        phase: 'attack',
        players: moved.players,
        realm: moved.realm,
      }),
      [{
        payload: {
          from: descriptor.from,
          path: descriptor.path,
          seat,
          steps: descriptor.path.length - 1,
          to: descriptor.to,
          unitInstanceId: descriptor.unitInstanceId,
        },
        type: 'move-and-attack-activated',
      }],
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
    const moved = moveAndTapUnit(state, ref, destination);
    const removesSite = pending.originalTarget?.kind === 'site' && !pending.targetRemoved;
    return [
      withStateVersion(state, {
        pendingCombat: deepFreeze({
          ...pending,
          defenders: [...pending.defenders, ref],
          targetRemoved: pending.targetRemoved || removesSite,
        }),
        players: moved.players,
        realm: moved.realm,
      }),
      [
        {
          payload: {
            from: descriptor.from,
            instanceId: ref.instanceId,
            path: descriptor.path,
            seat,
            steps: descriptor.path.length - 1,
            to: descriptor.to,
          },
          type: 'defender-joined',
        },
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
  const nextSeat = otherSeat(seat);
  const endingPlayer = deepFreeze({ ...player, mana: 0 });
  const nextPlayer = state.players[nextSeat];
  const startingPlayer = deepFreeze({
    ...nextPlayer,
    avatar: { ...nextPlayer.avatar, tapped: false },
    mana: siteCount(state, nextSeat),
  });
  const players = deepFreeze({ ...state.players, [seat]: endingPlayer, [nextSeat]: startingPlayer });
  const stealthGained = state.realm.units.filter((unit) => {
    if (unit.controller !== seat || unit.stealthed) return false;
    const definition = cardDefinition(state, unit.cardId);
    return definition.cardType === 'minion' && definition.gainsStealthAtEndOfTurn === true;
  });
  const stealthGainedIds = new Set(stealthGained.map(({ instanceId }) => instanceId));
  const units = state.realm.units.map((unit) => deepFreeze({
    ...unit,
    damage: 0,
    ...(stealthGainedIds.has(unit.instanceId) ? { stealthed: true } : {}),
    ...(unit.controller === seat ? { summoningSickness: false } : {}),
    ...(unit.controller === nextSeat ? { tapped: false } : {}),
  }));
  const turnNumber = state.turnNumber + 1;
  return [
    withStateVersion(state, {
      activeSeat: nextSeat,
      decisionSeat: nextSeat,
      pendingCombat: null,
      phase: 'draw',
      players,
      realm: { ...state.realm, units },
      turnNumber,
    }),
    [
      ...stealthGained.map(({ controller, instanceId }) => ({
        payload: { instanceId, seat: controller },
        type: 'stealth-gained',
      })),
      { payload: { seat, turnNumber: state.turnNumber }, type: 'turn-ended' },
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
