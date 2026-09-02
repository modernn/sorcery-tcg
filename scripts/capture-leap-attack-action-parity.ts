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
  '../tests/engine/fixtures/leap-attack-action-v1.json',
  import.meta.url,
));

const LEAP_ID = 'synthetic-leap-attack';
const ALLY_ID = 'synthetic-leap-ally';
const ORIGIN_ID = 'synthetic-leap-origin';
const STEP_TARGET_ID = 'synthetic-leap-step-target';
const WARDED_ID = 'synthetic-leap-warded';

const DEATHRITE_SOURCE_ID = 'synthetic-leap-source';
const DEATHRITE_FRAGILE_ID = 'synthetic-leap-fragile';
const DEATHRITE_RAIN_ID = 'synthetic-leap-rain';
const DEATHRITE_ENEMY_ID = 'synthetic-leap-enemy';

// Pinned after a successful hunt so test imports do not rescan 1..4096.
const MAIN_SEED = 1;
const DEATHRITE_SEED = 1;

function mainManifest(seed: number) {
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
      contentHash: identityHash({ fixture: 'synthetic-leap-attack-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-leap-attack-action-v1',
    },
    cards: {
      [ALLY_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 4,
        manaCost: 0,
        movementBonus: 2,
        thresholds: zero,
      },
      [LEAP_ID]: {
        cardType: 'magic',
        leapAttackAlly: true,
        manaCost: 1,
        thresholds: { ...zero, fire: 1 },
      },
      [ORIGIN_ID]: {
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [STEP_TARGET_ID]: {
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        stealth: true,
        summonToAnySite: true,
        thresholds: zero,
      },
      [WARDED_ID]: {
        airborne: true,
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
      'north-site': { cardType: 'site', elements: ['fire'] },
      'south-avatar': avatar,
      'south-site': { cardType: 'site', elements: ['water'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [LEAP_ID, ALLY_ID, 'north-filler'],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [ORIGIN_ID, STEP_TARGET_ID, WARDED_ID],
      },
    },
    firstSeat: 'north',
    seed,
  });
}

function deathriteManifest(seed: number) {
  const zero = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const avatar = {
    attack: 1,
    cardType: 'avatar' as const,
    defense: 1,
    drawSpell: false,
    life: 20,
  };
  const fragileA = `${DEATHRITE_FRAGILE_ID}-a`;
  const fragileB = `${DEATHRITE_FRAGILE_ID}-b`;
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'synthetic-leap-attack-deathrite-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-leap-attack-deathrite-v1',
    },
    cards: {
      [LEAP_ID]: {
        cardType: 'magic',
        leapAttackAlly: true,
        manaCost: 0,
        thresholds: zero,
      },
      [DEATHRITE_ENEMY_ID]: {
        attack: 1,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [fragileA]: {
        attack: 0,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      [fragileB]: {
        attack: 0,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      [DEATHRITE_RAIN_ID]: {
        cardType: 'magic',
        damageEachAbovegroundMinion: 1,
        manaCost: 0,
        thresholds: zero,
      },
      [DEATHRITE_SOURCE_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        otherNearbyAlliesPowerBonus: 1,
        thresholds: zero,
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['fire'] },
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
        spellbook: [
          LEAP_ID,
          DEATHRITE_SOURCE_ID,
          fragileA,
          fragileB,
          DEATHRITE_RAIN_ID,
        ],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [
          DEATHRITE_ENEMY_ID,
          'south-filler',
          'south-filler',
          'south-filler',
          'south-filler',
          'south-filler',
        ],
      },
    },
    firstSeat: 'north',
    seed,
  });
}

function take(
  session: GameSession,
  predicate: (action: GameLegalAction) => boolean,
): GameSession {
  const action = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!action) throw new Error('expected deterministic Leap Attack setup action');
  const result = stepGame(session, action);
  if (!result.accepted) {
    throw new Error(`issued Leap Attack setup action was rejected: ${result.reason.code}`);
  }
  return result.session;
}

function setupMainSession(gameManifest = mainManifest(MAIN_SEED)): GameSession {
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
  const drawAtlas = (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas';
  const summon = (cardId: string, cell: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === cardId
      && action.descriptor.cell === cell;

  step(keep);
  step(keep);
  step(playSite('C4'));
  step(summon(ALLY_ID, 'C4'));
  step(endTurn);
  step(drawAtlas);
  step(playSite('C1'));
  step(summon(ORIGIN_ID, 'C4'));
  step(endTurn);
  step(drawAtlas);
  step(playSite('C3'));
  step(endTurn);
  step(drawAtlas);
  step(summon(STEP_TARGET_ID, 'C3'));
  step(summon(WARDED_ID, 'C3'));
  step(endTurn);
  step(drawAtlas);
  return session;
}

function transition(
  session: GameSession,
  action: GameLegalAction,
  summary: JsonValue,
): JsonValue {
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued Leap Attack action was rejected: ${result.reason.code}`);
  if (!verifyGameReplay(result.session)) throw new Error('Leap Attack replay failed verification');
  const checkpoint = createGameCheckpoint(result.session);
  return {
    checkpointId: checkpoint.checkpointId,
    expectedSessionHash: checkpoint.expectedSessionHash,
    receipt: result.receipt,
    replayVerified: true,
    selectedActionId: action.actionId,
    serializedCheckpointHash: identityHash(serializeGameCheckpoint(checkpoint)),
    stateHash: hashGameState(result.session.state),
    summary,
  };
}

function captureMovementDeathriteParityForManifest(gameManifest: ReturnType<typeof deathriteManifest>): JsonValue {
  const fragileIds = [`${DEATHRITE_FRAGILE_ID}-a`, `${DEATHRITE_FRAGILE_ID}-b`];
  let session = createGameSession(gameManifest);
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    session = take(session, predicate);
  };
  const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0;

  step(keep);
  step(keep);
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
  for (const fragileId of fragileIds) {
    step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === fragileId
      && action.descriptor.cell === 'C4');
  }
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardId === DEATHRITE_SOURCE_ID
    && action.descriptor.cell === 'C3');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardId === DEATHRITE_ENEMY_ID
    && action.descriptor.cell === 'C2');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'cast-magic' && action.descriptor.cardId === DEATHRITE_RAIN_ID);

  const source = session.state.realm.units.find(({ cardId }) => cardId === DEATHRITE_SOURCE_ID);
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === DEATHRITE_ENEMY_ID);
  const fragiles = session.state.realm.units.filter(({ cardId }) => fragileIds.includes(cardId));
  if (!source || !enemy || fragiles.length !== 2) {
    throw new Error('expected complete Leap Attack Deathrite position');
  }

  const leapAction = legalGameActions(session.state, 'north').find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardId === LEAP_ID
      && descriptor.ally?.instanceId === source.instanceId
      && descriptor.allyDestination?.cell === 'C2');
  if (!leapAction) throw new Error('expected Leap Attack Deathrite cast action');

  const interrupted = stepGame(session, leapAction);
  if (!interrupted.accepted) {
    throw new Error(`Leap Attack Deathrite was rejected: ${interrupted.reason.code}`);
  }
  const continuation = interrupted.session.state.pendingDeathrites?.continuation;
  if (continuation?.kind !== 'leap-attack') {
    throw new Error('expected leap-attack continuation');
  }
  const pendingCheckpoint = createGameCheckpoint(interrupted.session);
  const serializedPending = serializeGameCheckpoint(pendingCheckpoint);
  const restored = resumeGameCheckpoint(parseGameCheckpoint(serializedPending));
  if (canonicalJson(restored as unknown as JsonValue)
    !== canonicalJson(interrupted.session as unknown as JsonValue)) {
    throw new Error('Leap Attack Deathrite checkpoint did not restore byte-identically');
  }
  const orderActions = legalGameActions(restored.state, restored.state.decisionSeat)
    .filter(({ descriptor }) => descriptor.kind === 'order-deathrites');
  if (orderActions.length !== 2) {
    throw new Error(`expected two Deathrite order actions, received ${orderActions.length}`);
  }
  const selectedOrderAction = orderActions[0]!;
  const resolved = stepGame(restored, selectedOrderAction);
  if (!resolved.accepted) {
    throw new Error(`Leap Attack Deathrite order was rejected: ${resolved.reason.code}`);
  }
  if (!verifyGameReplay(resolved.session)) {
    throw new Error('Leap Attack Deathrite replay failed verification');
  }
  const resolvedCheckpoint = createGameCheckpoint(resolved.session);

  return {
    leapAction,
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

function captureMovementDeathriteParity(): JsonValue {
  return captureMovementDeathriteParityForManifest(deathriteManifest(DEATHRITE_SEED));
}

export function captureLeapAttackActionParityFixture(): JsonValue {
  const gameManifest = mainManifest(MAIN_SEED);
  const session = setupMainSession(gameManifest);

  const allyInstanceId = session.state.realm.units.find(({ cardId }) =>
    cardId === ALLY_ID)?.instanceId;
  const originInstanceId = session.state.realm.units.find(({ cardId }) =>
    cardId === ORIGIN_ID)?.instanceId;
  const stepTargetInstanceId = session.state.realm.units.find(({ cardId }) =>
    cardId === STEP_TARGET_ID)?.instanceId;
  const wardedInstanceId = session.state.realm.units.find(({ cardId }) =>
    cardId === WARDED_ID)?.instanceId;
  const leapInstanceId = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === LEAP_ID)?.instanceId;
  if (!allyInstanceId || !originInstanceId || !stepTargetInstanceId
    || !wardedInstanceId || !leapInstanceId) {
    throw new Error('expected complete synthetic Leap Attack position');
  }

  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === leapInstanceId
      && descriptor.ally?.instanceId === allyInstanceId);
  if (actions.length !== 2) {
    throw new Error(`expected two Leap Attack actions, received ${actions.length}`);
  }
  const noStepAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.allyDestination?.cell === 'C4');
  const stepAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.allyDestination?.cell === 'C3');
  if (!noStepAction || !stepAction) {
    throw new Error('expected stay and step Leap Attack actions');
  }
  if (stepAction.descriptor.kind !== 'cast-magic') {
    throw new Error('expected Leap Attack cast descriptor');
  }
  const startCheckpoint = createGameCheckpoint(session);

  const noStepResult = stepGame(session, noStepAction);
  if (!noStepResult.accepted) throw new Error('stay Leap Attack rejected');
  const stepResult = stepGame(session, stepAction);
  if (!stepResult.accepted) throw new Error('step Leap Attack rejected');

  const noStepAlly = noStepResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === allyInstanceId);
  const stepAlly = stepResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === allyInstanceId);
  const stepWarded = stepResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === wardedInstanceId);
  const stepped = stepResult.receipt.events.find(({ type }) => type === 'unit-stepped');

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    allyInstanceId,
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    manifestId: gameManifest.manifestId,
    movementDeathrite: captureMovementDeathriteParity(),
    noStepTransition: transition(session, noStepAction, {
      allyLocation: noStepAlly?.location ?? null,
      allyTapped: noStepAlly?.tapped ?? null,
      c3EnemiesRemain: [stepTargetInstanceId, wardedInstanceId].every((instanceId) =>
        noStepResult.session.state.realm.units.some((unit) => unit.instanceId === instanceId)),
      originDead: !noStepResult.session.state.realm.units.some(({ instanceId }) =>
        instanceId === originInstanceId),
      unitStepped: noStepResult.receipt.events.some(({ type }) => type === 'unit-stepped'),
    }),
    originInstanceId,
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
    stepTargetInstanceId,
    stepTransition: transition(session, stepAction, {
      allyDamage: stepAlly?.damage ?? null,
      allyLocation: stepAlly?.location ?? null,
      allyTapped: stepAlly?.tapped ?? null,
      leapInCemetery: stepResult.session.state.players.north.cemetery.some(({ cardId }) =>
        cardId === LEAP_ID),
      originAlive: stepResult.session.state.realm.units.some(({ instanceId }) =>
        instanceId === originInstanceId),
      stepTargetDead: !stepResult.session.state.realm.units.some(({ instanceId }) =>
        instanceId === stepTargetInstanceId),
      strikeCount: stepResult.receipt.events.filter(({ type }) =>
        type === 'strike-damage-allocated').length,
      unitStepped: stepped?.payload ?? null,
      wardBroken: stepWarded?.warded === false,
    }),
    wardedInstanceId,
  };
}

export function serializeLeapAttackActionParityFixture(): string {
  return `${canonicalJson(captureLeapAttackActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeLeapAttackActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Leap Attack fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}
