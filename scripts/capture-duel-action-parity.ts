import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpoint,
  serializeGameCheckpoint,
} from '../src/engine/checkpoint.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  stepGame,
  verifyGameReplay,
  type GameLegalAction,
  type GameSession,
} from '../src/engine/game.ts';

const FIXTURE_PATH = fileURLToPath(new URL(
  '../tests/engine/fixtures/duel-action-v1.json',
  import.meta.url,
));

const DUEL_ID = 'synthetic-forced-duel';
const ALLY_ID = 'synthetic-duel-ally';
const NORMAL_TARGET_ID = 'synthetic-duel-target';
const WARDED_TARGET_ID = 'synthetic-warded-target';
const UNDERGROUND_ALLY_ID = 'synthetic-underground-duelist';
const UNDERGROUND_TARGET_ID = 'synthetic-underground-deathrite';
const UNDERGROUND_FRAGILE_ID = 'synthetic-underground-fragile';
const UNDERGROUND_BURY_ID = 'synthetic-bury-duelists';

function manifest() {
  const zero = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const avatar = {
    attack: 1,
    cardType: 'avatar' as const,
    defense: 1,
    drawSpell: false,
    life: 20,
  };
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'synthetic-duel-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-duel-action-v1',
    },
    cards: {
      [ALLY_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 4,
        manaCost: 0,
        thresholds: zero,
      },
      [DUEL_ID]: {
        cardType: 'magic',
        fightAllyWithAdjacentEnemy: true,
        manaCost: 1,
        thresholds: { ...zero, earth: 1 },
      },
      [NORMAL_TARGET_ID]: {
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [WARDED_TARGET_ID]: {
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
        ward: true,
      },
      'north-avatar': avatar,
      'north-filler': {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      'north-site': { cardType: 'site', elements: ['earth'] },
      'south-avatar': avatar,
      'south-filler': {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      'south-site': { cardType: 'site', elements: ['water'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [DUEL_ID, ALLY_ID, 'north-filler'],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [NORMAL_TARGET_ID, WARDED_TARGET_ID, 'south-filler'],
      },
    },
    firstSeat: 'north',
    seed: 719,
  });
}

function undergroundManifest() {
  const zero = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const avatar = {
    attack: 1,
    cardType: 'avatar' as const,
    defense: 1,
    drawSpell: false,
    life: 20,
  };
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'synthetic-underground-duel-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-underground-duel-action-v1',
    },
    cards: {
      [DUEL_ID]: {
        cardType: 'magic',
        fightAllyWithAdjacentEnemy: true,
        manaCost: 1,
        thresholds: { ...zero, earth: 1 },
      },
      [UNDERGROUND_BURY_ID]: {
        burrowTargetMinionOrArtifact: true,
        cardType: 'magic',
        manaCost: 0,
        thresholds: zero,
      },
      [UNDERGROUND_ALLY_ID]: {
        attack: 3,
        burrowing: true,
        cardType: 'minion',
        defense: 6,
        manaCost: 0,
        strikesFirstWhileAttacking: true,
        thresholds: zero,
      },
      [UNDERGROUND_TARGET_ID]: {
        attack: 2,
        burrowing: true,
        cardType: 'minion',
        deathriteDamageEachUnitHere: 1,
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [UNDERGROUND_FRAGILE_ID]: {
        attack: 0,
        burrowing: true,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['earth'] },
      'south-avatar': avatar,
      'south-site': { cardType: 'site', elements: ['water'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [
          DUEL_ID,
          UNDERGROUND_ALLY_ID,
          UNDERGROUND_BURY_ID,
          UNDERGROUND_BURY_ID,
          UNDERGROUND_BURY_ID,
          UNDERGROUND_BURY_ID,
        ],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [UNDERGROUND_TARGET_ID, UNDERGROUND_FRAGILE_ID, UNDERGROUND_FRAGILE_ID],
      },
    },
    firstSeat: 'north',
    seed: 1,
  });
}

function take(
  session: GameSession,
  predicate: (action: GameLegalAction) => boolean,
): GameSession {
  const action = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!action) throw new Error('expected deterministic Duel setup action');
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued Duel setup action was rejected: ${result.reason.code}`);
  return result.session;
}

function transition(
  session: GameSession,
  action: GameLegalAction,
  allyInstanceId: string,
  targetInstanceId: string,
): JsonValue {
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued Duel action was rejected: ${result.reason.code}`);
  if (!verifyGameReplay(result.session)) throw new Error('Duel replay failed verification');
  const checkpoint = createGameCheckpoint(result.session);
  const target = result.session.state.realm.units.find(({ instanceId }) => instanceId === targetInstanceId);
  return {
    checkpointId: checkpoint.checkpointId,
    expectedSessionHash: checkpoint.expectedSessionHash,
    receipt: result.receipt,
    replayVerified: true,
    selectedActionId: action.actionId,
    serializedCheckpointHash: identityHash(serializeGameCheckpoint(checkpoint)),
    stateHash: hashGameState(result.session.state),
    summary: {
      allyDamage: result.session.state.realm.units.find(({ instanceId }) =>
        instanceId === allyInstanceId)?.damage ?? null,
      mana: result.session.state.players.north.mana,
      targetPresent: target !== undefined,
      targetWarded: target?.warded ?? null,
    },
  };
}

function captureUndergroundDeathriteParity(): JsonValue {
  const gameManifest = undergroundManifest();
  let session = createGameSession(gameManifest);
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    session = take(session, predicate);
  };
  const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0;
  const summon = (cardId: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === cardId
      && action.descriptor.cell === 'C4'
      && action.descriptor.region === undefined;

  step(keep);
  step(keep);
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
  step(summon(UNDERGROUND_ALLY_ID));
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
  step(summon(UNDERGROUND_TARGET_ID));
  step(summon(UNDERGROUND_FRAGILE_ID));
  step(summon(UNDERGROUND_FRAGILE_ID));
  step((action) => action.descriptor.kind === 'end-turn');
  for (let turn = 0; turn < 3; turn += 1) {
    step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    while (session.state.players.north.hand.spellbook.some(({ cardId }) =>
      cardId === UNDERGROUND_BURY_ID)) {
      const target = session.state.realm.units.find(({ region }) => region === 'surface');
      if (!target) break;
      step((action) => action.descriptor.kind === 'cast-magic'
        && action.descriptor.cardId === UNDERGROUND_BURY_ID
        && action.descriptor.target?.instanceId === target.instanceId);
    }
    const ready = session.state.realm.units.every(({ region }) => region === 'underground')
      && session.state.players.north.hand.spellbook.some(({ cardId }) => cardId === DUEL_ID);
    if (ready) break;
    step((action) => action.descriptor.kind === 'end-turn');
    step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    step((action) => action.descriptor.kind === 'end-turn');
  }

  const ally = session.state.realm.units.find(({ cardId }) => cardId === UNDERGROUND_ALLY_ID);
  const target = session.state.realm.units.find(({ cardId }) => cardId === UNDERGROUND_TARGET_ID);
  const duel = session.state.players.north.hand.spellbook.find(({ cardId }) => cardId === DUEL_ID);
  if (!ally || !target || !duel) throw new Error('expected complete underground Duel position');
  const duelAction = legalGameActions(session.state, 'north').find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === duel.instanceId
      && descriptor.ally?.instanceId === ally.instanceId
      && descriptor.target?.instanceId === target.instanceId);
  if (!duelAction) throw new Error('expected underground Duel action');
  const interrupted = stepGame(session, duelAction);
  if (!interrupted.accepted) throw new Error(`underground Duel was rejected: ${interrupted.reason.code}`);
  if (interrupted.receipt.randomDraws.length !== 0) throw new Error('underground Duel unexpectedly used randomness');
  const continuation = interrupted.session.state.pendingDeathrites?.continuation;
  if (continuation?.kind !== 'first-strike' || continuation.pending.region !== 'underground') {
    throw new Error('expected underground FirstStrike continuation');
  }
  const pendingCheckpoint = createGameCheckpoint(interrupted.session);
  const serializedPending = serializeGameCheckpoint(pendingCheckpoint);
  const restored = resumeGameCheckpoint(parseGameCheckpoint(serializedPending));
  if (canonicalJson(restored as unknown as JsonValue)
    !== canonicalJson(interrupted.session as unknown as JsonValue)) {
    throw new Error('underground Duel checkpoint did not restore byte-identically');
  }
  const orderActions = legalGameActions(restored.state, restored.state.decisionSeat)
    .filter(({ descriptor }) => descriptor.kind === 'order-deathrites');
  if (orderActions.length !== 2) {
    throw new Error(`expected two underground Deathrite order actions, received ${orderActions.length}`);
  }
  const selectedOrderAction = orderActions[0]!;
  const resolved = stepGame(restored, selectedOrderAction);
  if (!resolved.accepted) throw new Error(`underground Deathrite order was rejected: ${resolved.reason.code}`);
  if (!verifyGameReplay(resolved.session)) throw new Error('underground Duel replay failed verification');
  const resolvedCheckpoint = createGameCheckpoint(resolved.session);

  return {
    duelAction,
    manifestId: gameManifest.manifestId,
    pending: {
      checkpointId: pendingCheckpoint.checkpointId,
      checkpointRoundTrip: true,
      continuation,
      decisionSeat: interrupted.session.state.decisionSeat,
      expectedSessionHash: pendingCheckpoint.expectedSessionHash,
      orderActions,
      phase: interrupted.session.state.phase,
      receipt: interrupted.receipt,
      serializedCheckpointHash: identityHash(serializedPending),
      stateHash: hashGameState(interrupted.session.state),
      stateVersion: interrupted.session.state.stateVersion,
    },
    resolved: {
      checkpointId: resolvedCheckpoint.checkpointId,
      decisionSeat: resolved.session.state.decisionSeat,
      expectedSessionHash: resolvedCheckpoint.expectedSessionHash,
      phase: resolved.session.state.phase,
      receipt: resolved.receipt,
      replayVerified: true,
      selectedOrderActionId: selectedOrderAction.actionId,
      serializedCheckpointHash: identityHash(serializeGameCheckpoint(resolvedCheckpoint)),
      stateHash: hashGameState(resolved.session.state),
      stateVersion: resolved.session.state.stateVersion,
    },
  };
}

export function captureDuelActionParityFixture(): JsonValue {
  const gameManifest = manifest();
  let session = createGameSession(gameManifest);
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    session = take(session, predicate);
  };
  const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0;
  const endTurn = (action: GameLegalAction): boolean => action.descriptor.kind === 'end-turn';
  const playSite = (cell: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'play-site' && action.descriptor.cell === cell;
  const summon = (cardId: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === cardId
      && action.descriptor.cell === 'C4';

  step(keep);
  step(keep);
  step(playSite('C4'));
  step(summon(ALLY_ID));
  step(endTurn);
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  step(playSite('C1'));
  step(summon(NORMAL_TARGET_ID));
  step(summon(WARDED_TARGET_ID));
  step(endTurn);
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');

  const allyInstanceId = session.state.realm.units.find(({ cardId }) =>
    cardId === ALLY_ID)?.instanceId;
  const normalTargetInstanceId = session.state.realm.units.find(({ cardId }) =>
    cardId === NORMAL_TARGET_ID)?.instanceId;
  const wardedTargetInstanceId = session.state.realm.units.find(({ cardId }) =>
    cardId === WARDED_TARGET_ID)?.instanceId;
  const duelInstanceId = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === DUEL_ID)?.instanceId;
  if (!allyInstanceId || !normalTargetInstanceId || !wardedTargetInstanceId || !duelInstanceId) {
    throw new Error('expected complete synthetic Duel position');
  }
  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === duelInstanceId
      && descriptor.ally?.instanceId === allyInstanceId);
  if (actions.length !== 2) throw new Error(`expected two Duel actions, received ${actions.length}`);
  const normalAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.instanceId === normalTargetInstanceId);
  const wardedAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.instanceId === wardedTargetInstanceId);
  if (!normalAction || !wardedAction) throw new Error('expected normal and warded Duel actions');
  const startCheckpoint = createGameCheckpoint(session);

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    allyInstanceId,
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    manifestId: gameManifest.manifestId,
    normalTargetInstanceId,
    normalTransition: transition(session, normalAction, allyInstanceId, normalTargetInstanceId),
    schemaVersion: 1,
    seat: 'north',
    source: 'typescript-legality-engine',
    startCheckpoint: {
      checkpointId: startCheckpoint.checkpointId,
      expectedSessionHash: startCheckpoint.expectedSessionHash,
      serializedCheckpointHash: identityHash(serializeGameCheckpoint(startCheckpoint)),
      stateHash: hashGameState(session.state),
    },
    stateVersion: session.state.stateVersion,
    undergroundDeathrite: captureUndergroundDeathriteParity(),
    wardedTargetInstanceId,
    wardedTransition: transition(session, wardedAction, allyInstanceId, wardedTargetInstanceId),
  };
}

export function serializeDuelActionParityFixture(): string {
  return `${canonicalJson(captureDuelActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeDuelActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Duel fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}
