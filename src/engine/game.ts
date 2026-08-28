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
const NORTH_START = 'C4';
const SOUTH_START = 'C1';

export type GameSeat = EngineSeat;
export type DeckZone = 'atlas' | 'spellbook';
export type RealmCell = `${'A' | 'B' | 'C' | 'D' | 'E'}${1 | 2 | 3 | 4}`;

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

type PlayerState = Readonly<{
  atlas: readonly CardInstance[];
  avatar: Readonly<{
    card: CardInstance;
    location: RealmCell;
    tapped: boolean;
  }>;
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
    reason: 'deck_empty';
    status: 'finished';
    winner: GameSeat;
  }>;

export type GameState = Readonly<{
  activeSeat: GameSeat;
  engine: EngineState;
  phase: 'draw' | 'main' | 'mulligan' | 'terminal';
  players: Readonly<Record<GameSeat, PlayerState>>;
  realm: Readonly<{
    sites: Readonly<Partial<Record<RealmCell, CardInstance>>>;
  }>;
  schemaVersion: 1;
  stateVersion: number;
  terminal: GameTerminal;
  turnNumber: number;
}>;

type ObservedPlayer = Readonly<{
  atlasCount: number;
  avatar: Readonly<{
    cardId: string;
    instanceId: StateHash;
    location: RealmCell;
    tapped: boolean;
  }>;
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
  phase: GameState['phase'];
  players: Readonly<Record<GameSeat, ObservedPlayer>>;
  realm: Readonly<{
    sites: Readonly<Partial<Record<RealmCell, Readonly<{
      cardId: string;
      instanceId: StateHash;
      owner: GameSeat;
    }>>>>;
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
  | Readonly<{ cardInstanceId: string; cell: RealmCell; kind: 'play-site' }>
  | Readonly<{ kind: 'draw'; zone: DeckZone }>
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

function requireCardId(value: string, path: string): void {
  if (!value.trim() || value.length > 256) throw new RangeError(`${path} must be 1-256 characters`);
}

function validateDeck(deck: GameDeckSpec, path: string): void {
  requireCardId(deck.avatar, `${path}.avatar`);
  for (const zone of ['atlas', 'spellbook'] as const) {
    if (deck[zone].length < 3 || deck[zone].length > MAX_DECK_CARDS) {
      throw new RangeError(`${path}.${zone} must contain 3-${MAX_DECK_CARDS} cards`);
    }
    deck[zone].forEach((cardId, index) => requireCardId(cardId, `${path}.${zone}[${index}]`));
  }
}

export function createGameManifest(input: GameManifestInput): GameManifest {
  createEngineState(input.seed);
  validateDeck(input.decks.north, 'decks.north');
  validateDeck(input.decks.south, 'decks.south');
  requireCardId(input.authority.revisionId, 'authority.revisionId');
  if (!/^sha256:[0-9a-f]{64}$/.test(input.authority.contentHash)) {
    throw new RangeError('authority.contentHash must be a SHA-256 identity');
  }

  const body = deepFreeze({
    authority: { ...input.authority },
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
  return deepFreeze({
    cardId,
    instanceId: identityHash({
      authorityHash: manifest.authority.contentHash,
      cardId,
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
  return deepFreeze({
    engine: shuffledSpellbook.engine,
    player: {
      atlas: shuffledAtlas.cards.slice(3),
      avatar: {
        card: cardInstance(manifest, seat, 'avatar', 0, deck.avatar),
        location: seat === 'north' ? NORTH_START : SOUTH_START,
        tapped: false,
      },
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
    engine: south.engine,
    phase: 'mulligan',
    players: { north: north.player, south: south.player },
    realm: { sites: {} },
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

function observedCard(card: CardInstance): Readonly<{ cardId: string; instanceId: StateHash }> {
  return { cardId: card.cardId, instanceId: card.instanceId };
}

function observePlayer(player: PlayerState, owner: GameSeat, viewer: GameSeat): ObservedPlayer {
  const own = owner === viewer;
  return deepFreeze({
    atlasCount: player.atlas.length,
    avatar: {
      cardId: player.avatar.card.cardId,
      instanceId: player.avatar.card.instanceId,
      location: player.avatar.location,
      tapped: player.avatar.tapped,
    },
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
    Object.entries(state.realm.sites).map(([cell, card]) => [cell, {
      cardId: card.cardId,
      instanceId: card.instanceId,
      owner: card.owner,
    }]),
  ) as GameObservation['realm']['sites'];
  return deepFreeze({
    activeSeat: state.activeSeat,
    phase: state.phase,
    players: {
      north: observePlayer(state.players.north, 'north', viewer),
      south: observePlayer(state.players.south, 'south', viewer),
    },
    realm: { sites },
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

function actionDescriptors(state: GameState, seat: GameSeat): readonly GameActionDescriptor[] {
  if (state.terminal.status === 'finished' || state.phase === 'terminal' || seat !== state.activeSeat) return [];
  const player = state.players[seat];
  if (state.phase === 'mulligan') return mulliganDescriptors(player);
  if (state.phase === 'draw') return [{ kind: 'draw', zone: 'atlas' }, { kind: 'draw', zone: 'spellbook' }];
  if (!player.domainEstablished) {
    return player.hand.atlas.map(({ instanceId }) => ({
      cardInstanceId: instanceId,
      cell: player.avatar.location,
      kind: 'play-site',
    }));
  }
  return [{ kind: 'end-turn' }];
}

function actionLabel(descriptor: GameActionDescriptor): string {
  if (descriptor.kind === 'mulligan') {
    const count = descriptor.atlasOrder.length + descriptor.spellbookOrder.length;
    return count === 0
      ? 'Keep opening hand'
      : `Mulligan ${count} (${descriptor.atlasOrder.length} atlas, ${descriptor.spellbookOrder.length} spellbook)`;
  }
  if (descriptor.kind === 'draw') return `Draw from ${descriptor.zone}`;
  if (descriptor.kind === 'play-site') return `Establish domain at ${descriptor.cell}`;
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
  return Object.values(state.realm.sites).filter((site) => site.owner === seat).length;
}

function applyDescriptor(
  state: GameState,
  descriptor: GameActionDescriptor,
  manifest: GameManifest,
): readonly [GameState, readonly GameOutcome[], readonly EngineRandomDraw[]] {
  const seat = state.activeSeat;
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
      return [withStateVersion(state, { activeSeat: 'south', players }), [outcome], []];
    }
    const firstPlayer = players[manifest.firstSeat];
    const startedPlayer = deepFreeze({ ...firstPlayer, mana: siteCount(state, manifest.firstSeat) });
    return [
      withStateVersion(state, {
        activeSeat: manifest.firstSeat,
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
    const card = player.hand.atlas.find(({ instanceId }) => instanceId === descriptor.cardInstanceId);
    if (!card) throw new Error('unreachable site card');
    const updatedPlayer = deepFreeze({
      ...player,
      avatar: { ...player.avatar, tapped: true },
      domainEstablished: true,
      hand: {
        ...player.hand,
        atlas: player.hand.atlas.filter(({ instanceId }) => instanceId !== card.instanceId),
      },
    });
    return [
      withStateVersion(state, {
        players: replacePlayer(state, seat, updatedPlayer),
        realm: { sites: { ...state.realm.sites, [descriptor.cell]: card } },
      }),
      [{ payload: { cardId: card.cardId, cell: descriptor.cell, instanceId: card.instanceId, seat }, type: 'site-played' }],
      [],
    ];
  }

  if (descriptor.kind === 'draw') {
    const deck = player[descriptor.zone];
    if (deck.length === 0) {
      const winner = otherSeat(seat);
      return [
        withStateVersion(state, {
          phase: 'terminal',
          terminal: { loser: seat, reason: 'deck_empty', status: 'finished', winner },
        }),
        [{ payload: { loser: seat, reason: 'deck_empty', winner }, type: 'game-ended' }],
        [],
      ];
    }
    const [drawn, ...remaining] = deck;
    const updatedPlayer = deepFreeze({
      ...player,
      [descriptor.zone]: remaining,
      hand: { ...player.hand, [descriptor.zone]: [...player.hand[descriptor.zone], drawn!] },
    });
    return [
      withStateVersion(state, { phase: 'main', players: replacePlayer(state, seat, updatedPlayer) }),
      [{ payload: { seat, zone: descriptor.zone }, type: 'card-drawn' }],
      [],
    ];
  }

  const nextSeat = otherSeat(seat);
  const endingPlayer = deepFreeze({ ...player, mana: 0 });
  const nextPlayer = state.players[nextSeat];
  const startingPlayer = deepFreeze({
    ...nextPlayer,
    avatar: { ...nextPlayer.avatar, tapped: false },
    mana: siteCount(state, nextSeat),
  });
  const players = deepFreeze({ ...state.players, [seat]: endingPlayer, [nextSeat]: startingPlayer });
  const turnNumber = state.turnNumber + 1;
  return [
    withStateVersion(state, { activeSeat: nextSeat, phase: 'draw', players, turnNumber }),
    [
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
  if (command.seat !== state.activeSeat) return reject('wrong_seat');
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
      seat: session.state.activeSeat,
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
