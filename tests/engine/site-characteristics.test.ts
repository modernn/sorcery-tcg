import assert from 'node:assert/strict';
import test from 'node:test';
import { createGameManifest, type GameCardDefinition } from '../../src/engine/game.ts';

function manifest(site: GameCardDefinition) {
  return createGameManifest({ authority: { mode: 'synthetic', revisionId: 'site-characteristics',
    contentHash: 'sha256:1111111111111111111111111111111111111111111111111111111111111111' },
  cards: { site, avatar: { cardType: 'avatar', attack: 1, defense: 1, life: 20, drawSpell: false },
    spell: { cardType: 'magic', damageTargetUnit: 1, manaCost: 0, thresholds: { air: 0, earth: 0, fire: 0, water: 0 } } },
  decks: { north: { avatar: 'avatar', atlas: ['site', 'site', 'site'], spellbook: ['spell', 'spell', 'spell'] },
    south: { avatar: 'avatar', atlas: ['site', 'site', 'site'], spellbook: ['spell', 'spell', 'spell'] } }, firstSeat: 'north', seed: 1 });
}

test('Site subtype lists are sparse canonical clones and independent of legacy Tower ability', () => {
  for (const subtypes of [undefined, [], ['River'], ['River', 'Tower']]) {
    for (const isTower of [undefined, true] as const) {
      const site = { cardType: 'site' as const, elements: ['air'] as const,
        ...(subtypes !== undefined ? { subtypes } : {}), ...(isTower ? { isTower } : {}) };
      const result = manifest(site).cards.site as Extract<GameCardDefinition, { cardType: 'site' }>;
      assert.deepEqual(result, site);
      assert.equal('subtypes' in result, subtypes !== undefined);
      if (subtypes !== undefined) assert.notEqual(result.subtypes, subtypes);
    }
  }
});

test('Site subtype admission rejects noncanonical input and independently rejects invalid Tower flags', () => {
  for (const subtypes of [null, 'Tower', [1], ['Tower', 'River'], ['River', 'River'], [''], [' Tower'], ['Tower\n'], ['A\x7fB'], ['é'.repeat(33)], Array.from({ length: 17 }, (_, i) => `T${i.toString().padStart(2, '0')}`)]) {
    assert.throws(() => manifest({ cardType: 'site', elements: [], subtypes } as never), /subtypes/);
  }
  for (const isTower of [null, false, 1, 'true']) {
    assert.throws(() => manifest({ cardType: 'site', elements: [], subtypes: ['Tower'], isTower } as never), /isTower/);
  }
});
