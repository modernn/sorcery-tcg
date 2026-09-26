import assert from 'node:assert/strict';
import test from 'node:test';

import { createGameManifest, type GameCardDefinition, type GameManifestInput } from '../../src/engine/game.ts';

const authority = {
  contentHash: 'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-entry-equipment-test',
};

function input(token: GameCardDefinition, entersCarrying?: readonly string[]): GameManifestInput {
  const cards: Record<string, GameCardDefinition> = {
    avatar: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    site: { cardType: 'site', elements: ['earth'] },
    minion: { attack: 1, cardType: 'minion', defense: 1, ...(entersCarrying ? { entersCarrying } : {}), manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
    token,
  };
  return { authority, cards,
    decks: {
      north: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['minion', 'minion', 'minion'] },
      south: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['minion', 'minion', 'minion'] },
    }, firstSeat: 'north', seed: 1 };
}

const carriableToken = {
  bearerControllerChoosesExtraRandomOutcome: true as const,
  cardType: 'artifact' as const,
  manaCost: null,
  thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  token: true as const,
};

test('minion entersCarrying preserves a valid immutable reference array', () => {
  const entersCarrying = ['token', 'token'];
  const manifest = createGameManifest(input(carriableToken, entersCarrying));
  const minion = manifest.cards.minion;
  assert(minion?.cardType === 'minion');
  assert.deepEqual(minion.entersCarrying, entersCarrying);
  assert.notEqual(minion.entersCarrying, entersCarrying);
});

test('minion entersCarrying requires 1-32 nonempty card references', () => {
  for (const entersCarrying of [[], Array.from({ length: 33 }, (_, index) => `token-${index}`), ['']]) {
    assert.throws(() => createGameManifest(input(carriableToken, entersCarrying)), /entersCarrying/);
  }
});

test('minion entersCarrying requires carriable artifact tokens', () => {
  assert.throws(() => createGameManifest(input({
    attack: 1, cardType: 'minion', defense: 1, manaCost: null, token: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, ['token'])), /token artifact/);
  assert.throws(() => createGameManifest(input({ ...carriableToken, cannotBeCarried: true }, ['token'])), /token artifact/);
});
