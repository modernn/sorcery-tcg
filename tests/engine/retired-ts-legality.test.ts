import assert from 'node:assert/strict';
import test from 'node:test';

import {
  createGameManifest,
  createGameSession,
  legalGameActions,
  observeGame,
  replayGame,
  stepGame,
  verifyGameReplay,
  type GameSession,
} from '../../src/engine/game.ts';

const SYNTHETIC_AUTHORITY_HASH =
  'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const;

function tinyManifest() {
  const decks = {
    north: {
      atlas: ['n-site-1', 'n-site-2', 'n-site-3'],
      avatar: 'n-avatar',
      spellbook: ['n-spell-1', 'n-spell-2', 'n-spell-3', 'n-spell-4'],
    },
    south: {
      atlas: ['s-site-1', 's-site-2', 's-site-3'],
      avatar: 's-avatar',
      spellbook: ['s-spell-1', 's-spell-2', 's-spell-3', 's-spell-4'],
    },
  };
  const cards = {
    'n-avatar': { attack: 0, cardType: 'avatar' as const, defense: 0, drawSpell: true, life: 20 },
    's-avatar': { attack: 0, cardType: 'avatar' as const, defense: 0, drawSpell: true, life: 20 },
    'n-site-1': { cardType: 'site' as const, elements: ['earth'] as const },
    'n-site-2': { cardType: 'site' as const, elements: ['earth'] as const },
    'n-site-3': { cardType: 'site' as const, elements: ['earth'] as const },
    's-site-1': { cardType: 'site' as const, elements: ['water'] as const },
    's-site-2': { cardType: 'site' as const, elements: ['water'] as const },
    's-site-3': { cardType: 'site' as const, elements: ['water'] as const },
    'n-spell-1': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    'n-spell-2': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    'n-spell-3': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    'n-spell-4': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    's-spell-1': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    's-spell-2': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    's-spell-3': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    's-spell-4': {
      attack: 1,
      cardType: 'minion' as const,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  };
  return createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-retired-ts-legality-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 1,
  });
}

test('retired TypeScript legality exports fail closed', () => {
  const manifest = tinyManifest();
  assert.throws(() => createGameSession(manifest), /createGameSession is retired/);
  assert.throws(
    () => legalGameActions({} as GameSession['state'], 'north'),
    /legalGameActions is retired/,
  );
  assert.throws(
    () => observeGame({} as GameSession['state'], 'north'),
    /observeGame is retired/,
  );
  assert.throws(
    () => stepGame({} as GameSession, {
      actionId: 'x',
      seat: 'north',
      stateVersion: 0,
    }),
    /stepGame is retired/,
  );
  assert.throws(() => replayGame(manifest, []), /replayGame is retired/);
  assert.throws(() => verifyGameReplay({} as GameSession), /verifyGameReplay is retired/);
});
