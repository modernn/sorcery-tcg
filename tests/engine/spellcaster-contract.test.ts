import assert from 'node:assert/strict';
import test from 'node:test';

import { createGameManifest, type GameCardDefinition, type GameManifestInput } from '../../src/engine/game.ts';

const authority = {
  contentHash: 'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-spellcaster-test',
};

function input(minion: GameCardDefinition): GameManifestInput {
  return {
    authority,
    cards: {
      avatar: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
      site: { cardType: 'site', elements: ['earth'] },
      minion,
    },
    decks: {
      north: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['minion', 'minion', 'minion'] },
      south: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['minion', 'minion', 'minion'] },
    },
    firstSeat: 'north',
    seed: 1,
  };
}

function minion(overrides: Partial<Extract<GameCardDefinition, { cardType: 'minion' }>> = {}) {
  return {
    attack: 1,
    cardType: 'minion' as const,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    ...overrides,
  };
}

test('spellcasterElements preserves a valid canonical fact in the manifest', () => {
  const elements = ['earth', 'fire'] as const;
  const source = minion({ spellcaster: true, spellcasterElements: elements });
  const manifest = createGameManifest(input(source));
  const stored = manifest.cards.minion;
  assert(stored?.cardType === 'minion');
  assert.equal(stored.spellcaster, true);
  assert.deepEqual(stored.spellcasterElements, elements);
  assert.notEqual(stored.spellcasterElements, elements);
  assert.notEqual(stored, source);
});

test('spellcasterElements is optional when spellcaster is present', () => {
  const manifest = createGameManifest(input(minion({ spellcaster: true })));
  const stored = manifest.cards.minion;
  assert(stored?.cardType === 'minion');
  assert.equal(stored.spellcaster, true);
  assert.equal('spellcasterElements' in stored, false);
});

test('spellcasterElements requires spellcaster and a nonempty unique supported set', () => {
  assert.throws(
    () => createGameManifest(input(minion({ spellcasterElements: ['fire'] }))),
    /spellcasterElements requires spellcaster/,
  );
  assert.throws(
    () => createGameManifest(input(minion({ spellcaster: false, spellcasterElements: ['fire'] }))),
    /spellcasterElements requires spellcaster/,
  );
  for (const spellcasterElements of [[], ['fire', 'fire'], ['arcane'], null, 'fire']) {
    assert.throws(
      () => createGameManifest(input(minion({ spellcaster: true, spellcasterElements } as never))),
      /spellcasterElements/,
    );
  }
});
