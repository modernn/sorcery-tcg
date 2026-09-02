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
  '../tests/engine/fixtures/drag-projectile-action-v1.json',
  import.meta.url,
));

const PUDGE_ID = 'synthetic-pudge';
const BLOCKER_ID = 'synthetic-pudge-blocker';
const TARGET_ID = 'synthetic-pudge-target';
const RAIN_ID = 'synthetic-pudge-rain';
const FRAGILE_A_ID = 'synthetic-pudge-fragile-a';
const FRAGILE_B_ID = 'synthetic-pudge-fragile-b';
const POWER_TARGET_ID = 'synthetic-pudge-power-target';

const MAIN_SEED = 5;
const DEATHRITE_SEED = 218;

function handCard(session: GameSession, seat: 'north' | 'south', cardId: string) {
  return session.state.players[seat].hand.spellbook.find(({ cardId: id }) => id === cardId);
}

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
      contentHash: identityHash({ fixture: 'synthetic-drag-projectile-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-drag-projectile-action-v1',
    },
    cards: {
      [BLOCKER_ID]: {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        immobile: false,
        manaCost: 0,
        shootsDragProjectile: false,
        stealth: true,
        thresholds: zero,
      },
      [PUDGE_ID]: {
        attack: 5,
        cardType: 'minion',
        defense: 5,
        immobile: true,
        manaCost: 1,
        shootsDragProjectile: true,
        thresholds: { ...zero, earth: 1 },
      },
      [TARGET_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 6,
        manaCost: 1,
        thresholds: { ...zero, earth: 1 },
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
      'south-site': { cardType: 'site', elements: ['earth'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [PUDGE_ID, BLOCKER_ID, ...Array(8).fill('north-filler')],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [TARGET_ID, ...Array(9).fill('south-filler')],
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
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'synthetic-drag-projectile-deathrite-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-drag-projectile-deathrite-v1',
    },
    cards: {
      [FRAGILE_A_ID]: {
        attack: 1,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        stealth: true,
        thresholds: zero,
      },
      [FRAGILE_B_ID]: {
        attack: 1,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        stealth: true,
        thresholds: zero,
      },
      [POWER_TARGET_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 10,
        manaCost: 1,
        otherNearbyAlliesPowerBonus: 1,
        thresholds: { ...zero, earth: 1 },
      },
      [PUDGE_ID]: {
        attack: 5,
        cardType: 'minion',
        defense: 5,
        immobile: true,
        manaCost: 1,
        shootsDragProjectile: true,
        thresholds: { ...zero, earth: 1 },
      },
      [RAIN_ID]: {
        cardType: 'magic',
        damageEachAbovegroundMinion: 1,
        manaCost: 0,
        thresholds: zero,
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
      'south-site': { cardType: 'site', elements: ['earth'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [PUDGE_ID, RAIN_ID, ...Array(8).fill('north-filler')],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [FRAGILE_A_ID, FRAGILE_B_ID, POWER_TARGET_ID, ...Array(7).fill('south-filler')],
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
  if (!action) throw new Error('expected deterministic drag projectile setup action');
  const result = stepGame(session, action);
  if (!result.accepted) {
    throw new Error(`issued drag projectile setup action was rejected: ${result.reason.code}`);
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

  step(keep);
  step(keep);
  const openingPudge = handCard(session, 'north', PUDGE_ID);
  if (!openingPudge) throw new Error(`seed ${gameManifest.seed} missing opening Pudge`);
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardInstanceId === openingPudge.instanceId
    && action.descriptor.cell === 'C4');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
  const blockerCard = handCard(session, 'north', BLOCKER_ID);
  if (!blockerCard) throw new Error(`seed ${gameManifest.seed} missing Blocker before C3 summon`);
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardInstanceId === blockerCard.instanceId
    && action.descriptor.cell === 'C3');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
  const targetCard = handCard(session, 'south', TARGET_ID);
  if (!targetCard) throw new Error(`seed ${gameManifest.seed} missing target before C2 summon`);
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardInstanceId === targetCard.instanceId
    && action.descriptor.cell === 'C2');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  return session;
}

function transition(
  session: GameSession,
  action: GameLegalAction,
  summary: JsonValue,
): JsonValue {
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued drag projectile action was rejected: ${result.reason.code}`);
  if (!verifyGameReplay(result.session)) throw new Error('drag projectile replay failed verification');
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

function captureMovementDeathriteParity(): JsonValue {
  const gameManifest = deathriteManifest(DEATHRITE_SEED);
  let session = createGameSession(gameManifest);
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    session = take(session, predicate);
  };
  const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0;

  step(keep);
  step(keep);
  const openingPudge = handCard(session, 'north', PUDGE_ID);
  if (!openingPudge) throw new Error(`seed ${gameManifest.seed} missing opening Pudge for Deathrite`);
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardInstanceId === openingPudge.instanceId
    && action.descriptor.cell === 'C4');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
  for (const fragileId of [FRAGILE_A_ID, FRAGILE_B_ID]) {
    const fragileCard = handCard(session, 'south', fragileId);
    if (!fragileCard) {
      throw new Error(`seed ${gameManifest.seed} missing fragile ${fragileId} before C1 summon`);
    }
    step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardInstanceId === fragileCard.instanceId
      && action.descriptor.cell === 'C1');
  }
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
  const targetCard = handCard(session, 'south', POWER_TARGET_ID);
  if (!targetCard) throw new Error(`seed ${gameManifest.seed} missing power target before C2 summon`);
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardInstanceId === targetCard.instanceId
    && action.descriptor.cell === 'C2');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  const rainCard = handCard(session, 'north', RAIN_ID);
  if (!rainCard) throw new Error(`seed ${gameManifest.seed} missing rain card before cast`);
  step((action) => action.descriptor.kind === 'cast-magic'
    && action.descriptor.cardInstanceId === rainCard.instanceId);

  const pudge = session.state.realm.units.find(({ cardId }) => cardId === PUDGE_ID);
  const target = session.state.realm.units.find(({ cardId }) => cardId === POWER_TARGET_ID);
  const fragiles = session.state.realm.units.filter(({ cardId }) =>
    cardId === FRAGILE_A_ID || cardId === FRAGILE_B_ID);
  if (!pudge || !target || fragiles.length !== 2) {
    throw new Error('expected complete drag projectile Deathrite position');
  }

  const dragAction = legalGameActions(session.state, 'north').find(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile'
      && descriptor.shooterInstanceId === pudge.instanceId
      && descriptor.hit?.instanceId === target.instanceId
      && descriptor.direction === 'south'
      && !descriptor.fightOnArrival);
  if (!dragAction) throw new Error('expected drag projectile Deathrite action without fight');

  const interrupted = stepGame(session, dragAction);
  if (!interrupted.accepted) {
    throw new Error(`drag projectile Deathrite was rejected: ${interrupted.reason.code}`);
  }
  const continuation = interrupted.session.state.pendingDeathrites?.continuation;
  if (continuation?.kind !== 'drag-projectile') {
    throw new Error('expected drag-projectile continuation');
  }
  const pendingCheckpoint = createGameCheckpoint(interrupted.session);
  const serializedPending = serializeGameCheckpoint(pendingCheckpoint);
  const restored = resumeGameCheckpoint(parseGameCheckpoint(serializedPending));
  if (canonicalJson(restored as unknown as JsonValue)
    !== canonicalJson(interrupted.session as unknown as JsonValue)) {
    throw new Error('drag projectile Deathrite checkpoint did not restore byte-identically');
  }
  const orderActions = legalGameActions(restored.state, restored.state.decisionSeat)
    .filter(({ descriptor }) => descriptor.kind === 'order-deathrites');
  if (orderActions.length !== 2) {
    throw new Error(`expected two Deathrite order actions, received ${orderActions.length}`);
  }
  const selectedOrderAction = orderActions[0]!;
  const resolved = stepGame(restored, selectedOrderAction);
  if (!resolved.accepted) {
    throw new Error(`drag projectile Deathrite order was rejected: ${resolved.reason.code}`);
  }
  if (!verifyGameReplay(resolved.session)) {
    throw new Error('drag projectile Deathrite replay failed verification');
  }
  const resolvedCheckpoint = createGameCheckpoint(resolved.session);

  return {
    dragAction,
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
      targetLocationAfterFirstDrag: interrupted.session.state.realm.units.find(({ instanceId }) =>
        instanceId === target.instanceId)?.location ?? null,
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
      targetLocationAfterResume: resolved.session.state.realm.units.find(({ instanceId }) =>
        instanceId === target.instanceId)?.location ?? null,
    },
  };
}

function unitSummary(resultSession: GameSession, instanceId: string): JsonValue {
  const unit = resultSession.state.realm.units.find((candidate) =>
    candidate.instanceId === instanceId);
  return {
    damage: unit?.damage ?? null,
    location: unit?.location ?? null,
    tapped: unit?.tapped ?? null,
    warded: unit?.warded ?? null,
  };
}

export function captureDragProjectileActionParityFixture(): JsonValue {
  const gameManifest = mainManifest(MAIN_SEED);
  const session = setupMainSession(gameManifest);

  const pudge = session.state.realm.units.find(({ cardId }) => cardId === PUDGE_ID);
  const target = session.state.realm.units.find(({ cardId }) => cardId === TARGET_ID);
  const blocker = session.state.realm.units.find(({ cardId }) => cardId === BLOCKER_ID);
  if (!pudge || !target || !blocker) {
    throw new Error('expected complete synthetic drag projectile position');
  }

  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile'
      && descriptor.shooterInstanceId === pudge.instanceId
      && descriptor.direction === 'south'
      && descriptor.hit?.instanceId === target.instanceId);
  if (actions.length !== 2) {
    throw new Error(`expected two drag projectile actions, received ${actions.length}`);
  }
  const noFightAction = actions.find(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile' && !descriptor.fightOnArrival);
  const fightAction = actions.find(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival);
  if (!noFightAction || !fightAction) {
    throw new Error('expected fight and no-fight drag projectile actions');
  }

  const startCheckpoint = createGameCheckpoint(session);
  const noFightPreview = stepGame(session, noFightAction);
  const fightPreview = stepGame(session, fightAction);
  if (!noFightPreview.accepted || !fightPreview.accepted) {
    throw new Error('expected drag projectile preview steps to succeed');
  }

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    blockerInstanceId: blocker.instanceId,
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    fightTransition: transition(session, fightAction, {
      eventTypes: fightPreview.receipt.events.map(({ type }) => type),
      pudge: unitSummary(fightPreview.session, pudge.instanceId),
      target: unitSummary(fightPreview.session, target.instanceId),
    }),
    manifestId: gameManifest.manifestId,
    movementDeathrite: captureMovementDeathriteParity(),
    noFightTransition: transition(session, noFightAction, {
      eventTypes: noFightPreview.receipt.events.map(({ type }) => type),
      pudge: unitSummary(noFightPreview.session, pudge.instanceId),
      target: unitSummary(noFightPreview.session, target.instanceId),
    }),
    schemaVersion: 1,
    seat: 'north',
    shooterInstanceId: pudge.instanceId,
    source: 'typescript-legality-engine',
    startCheckpoint: {
      checkpointId: startCheckpoint.checkpointId,
      expectedSessionHash: startCheckpoint.expectedSessionHash,
      serializedCheckpointHash: identityHash(serializeGameCheckpoint(startCheckpoint)),
      stateHash: hashGameState(session.state),
    },
    stateVersion: session.state.stateVersion,
    targetInstanceId: target.instanceId,
  };
}

export function serializeDragProjectileActionParityFixture(): string {
  return `${canonicalJson(captureDragProjectileActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeDragProjectileActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`drag projectile parity fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}
