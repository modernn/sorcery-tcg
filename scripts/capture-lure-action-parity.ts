import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameCheckpoint,
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
  '../tests/engine/fixtures/lure-action-v1.json',
  import.meta.url,
));

const LURE_ID = 'synthetic-lure';
const MOBILE_ENEMY_ID = 'synthetic-lure-mobile';
const IMMOBILE_ENEMY_ID = 'synthetic-lure-immobile';
const MAIN_SEED = 100;

function manifest(seed: number) {
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
      contentHash: identityHash({ fixture: 'synthetic-lure-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-lure-action-v1',
    },
    cards: {
      [IMMOBILE_ENEMY_ID]: {
        attack: 1,
        cardType: 'minion',
        defense: 3,
        immobile: true,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [LURE_ID]: {
        cardType: 'magic',
        lureEnemyMinionOneStepCloser: true,
        manaCost: 1,
        thresholds: { ...zero, water: 1 },
      },
      [MOBILE_ENEMY_ID]: {
        attack: 1,
        cardType: 'minion',
        charge: true,
        defense: 3,
        manaCost: 0,
        stealth: true,
        summonToAnySite: true,
        thresholds: zero,
        ward: true,
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['water'] },
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
        spellbook: Array(10).fill(LURE_ID),
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [MOBILE_ENEMY_ID, IMMOBILE_ENEMY_ID, ...Array(8).fill('south-filler')],
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
  if (!action) throw new Error('expected deterministic Lure setup action');
  const result = stepGame(session, action);
  if (!result.accepted) {
    throw new Error(`issued Lure setup action was rejected: ${result.reason.code}`);
  }
  return result.session;
}

function transition(
  session: GameSession,
  action: GameLegalAction,
  summary: JsonValue,
): JsonValue {
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued Lure action was rejected: ${result.reason.code}`);
  if (!verifyGameReplay(result.session)) throw new Error('Lure replay failed verification');
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

export function captureLureActionParityFixture(): JsonValue {
  const gameManifest = manifest(MAIN_SEED);
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
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
  const immobileCard = session.state.players[session.state.decisionSeat].hand.spellbook
    .find(({ cardId }) => cardId === IMMOBILE_ENEMY_ID);
  if (!immobileCard) throw new Error(`seed ${gameManifest.seed} missing immobile enemy card`);
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardInstanceId === immobileCard.instanceId
    && action.descriptor.cell === 'C3');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'D4');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'D2');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'D3');
  const mobileCard = session.state.players[session.state.decisionSeat].hand.spellbook
    .find(({ cardId }) => cardId === MOBILE_ENEMY_ID);
  if (!mobileCard) throw new Error(`seed ${gameManifest.seed} missing mobile enemy card`);
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardInstanceId === mobileCard.instanceId
    && action.descriptor.cell === 'D3');
  const mobileBeforeMove = session.state.realm.units.find(({ cardId }) =>
    cardId === MOBILE_ENEMY_ID);
  if (!mobileBeforeMove) throw new Error('missing mobile enemy before charge');
  step((action) => action.descriptor.kind === 'move-and-attack'
    && action.descriptor.unitInstanceId === mobileBeforeMove.instanceId
    && action.descriptor.path.length === 1);
  step((action) => action.descriptor.kind === 'decline-attack');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');

  const allyInstanceId = session.state.players.north.avatar.card.instanceId;
  const mobile = session.state.realm.units.find(({ cardId }) => cardId === MOBILE_ENEMY_ID);
  const lureCards = session.state.players.north.hand.spellbook.filter(({ cardId }) =>
    cardId === LURE_ID).slice(0, 3);
  if (!mobile || lureCards.length !== 3) {
    throw new Error('expected complete synthetic Lure position');
  }

  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === lureCards[0]?.instanceId);
  if (actions.length !== 2) {
    throw new Error(`expected two Lure actions, received ${actions.length}`);
  }
  const destinationCells = actions.flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.temptedDestination
      ? [descriptor.temptedDestination.cell]
      : []).sort();
  if (destinationCells.join(',') !== 'C3,D4') {
    throw new Error(`unexpected Lure destinations: ${destinationCells.join(',')}`);
  }

  const firstLure = actions.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.temptedDestination?.cell === 'C3');
  if (!firstLure) throw new Error('expected C3 Lure action');

  const firstResult = stepGame(session, firstLure);
  if (!firstResult.accepted) throw new Error('first Lure rejected');
  const secondLure = legalGameActions(firstResult.session.state, 'north').find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === lureCards[1]?.instanceId
      && descriptor.temptedEnemy?.instanceId === mobile.instanceId
      && descriptor.temptedDestination?.cell === 'C4');
  if (!secondLure) throw new Error('expected C4 Lure action after first cast');
  const secondResult = stepGame(firstResult.session, secondLure);
  if (!secondResult.accepted) throw new Error('second Lure rejected');
  const noOpActions = legalGameActions(secondResult.session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === lureCards[2]?.instanceId);
  if (noOpActions.length !== 1) {
    throw new Error(`expected one no-op Lure action, received ${noOpActions.length}`);
  }
  const noOpAction = noOpActions[0]!;
  const noOpPreview = stepGame(secondResult.session, noOpAction);
  if (!noOpPreview.accepted) throw new Error('no-op Lure rejected');

  const startCheckpoint = createGameCheckpoint(session);

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    allyInstanceId,
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    firstLureTransition: transition(session, firstLure, {
      eventTypes: firstResult.receipt.events.map(({ type }) => type),
      mobileLocation: firstResult.session.state.realm.units.find(({ instanceId }) =>
        instanceId === mobile.instanceId)?.location ?? null,
    }),
    manifestId: gameManifest.manifestId,
    mobileEnemyInstanceId: mobile.instanceId,
    noOpTransition: transition(secondResult.session, noOpAction, {
      eventTypes: noOpPreview.receipt.events.map(({ type }) => type),
      mobileLocation: noOpPreview.session.state.realm.units.find(({ instanceId }) =>
        instanceId === mobile.instanceId)?.location ?? null,
    }),
    schemaVersion: 1,
    seat: 'north',
    secondLureAction: {
      actionId: secondLure.actionId,
      descriptor: secondLure.descriptor,
      label: secondLure.label,
    },
    source: 'typescript-legality-engine',
    startCheckpoint: {
      checkpointId: startCheckpoint.checkpointId,
      expectedSessionHash: startCheckpoint.expectedSessionHash,
      serializedCheckpointHash: identityHash(serializeGameCheckpoint(startCheckpoint)),
      stateHash: hashGameState(session.state),
    },
    stateVersion: session.state.stateVersion,
  };
}

export function serializeLureActionParityFixture(): string {
  return `${canonicalJson(captureLureActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeLureActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Lure parity fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}
