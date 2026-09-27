import assert from 'node:assert/strict';
import test from 'node:test';

import { createGameManifest, type GameCardDefinition, type GameManifestInput } from '../../src/engine/game.ts';

const authority = {
  contentHash: 'sha256:2222222222222222222222222222222222222222222222222222222222222222' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-spatial-prevention-test',
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

test('nearby damage prevention preserves each supported selector and alliedOnly', () => {
  for (const preventsDamageFrom of ['ranged-strikes', 'magic', 'earth-magic', 'fire-magic', 'water-magic', 'air-magic'] as const) {
    const manifest = createGameManifest(input(minion({ nearbyDamagePrevention: { alliedOnly: true, preventsDamageFrom } })));
    const card = manifest.cards.minion;
    assert(card?.cardType === 'minion');
    assert.deepEqual(card.nearbyDamagePrevention, { alliedOnly: true, preventsDamageFrom });
  }
  for (const nearbyDamagePrevention of [
    { takesLessDamage: 1 },
    { takesLessDamage: 100, alliedOnly: true },
    { preventsDamageFromUnitsWithPowerAtLeast: 1 },
    { preventsDamageFromUnitsWithPowerAtLeast: 100, alliedOnly: true },
  ] as const) {
    const manifest = createGameManifest(input(minion({ nearbyDamagePrevention })));
    assert.deepEqual((manifest.cards.minion as Extract<GameCardDefinition, { cardType: 'minion' }>).nearbyDamagePrevention, nearbyDamagePrevention);
  }
});

test('nearby provider prevention is independent from target-local prevention', () => {
  const manifest = createGameManifest(input(minion({
    takesLessDamage: 2,
    nearbyDamagePrevention: { preventsDamageFrom: 'magic' },
  })));
  const card = manifest.cards.minion;
  assert(card?.cardType === 'minion');
  assert.equal(card.takesLessDamage, 2);
  assert.deepEqual(card.nearbyDamagePrevention, { preventsDamageFrom: 'magic' });
});

test('nearby provider prevention rejects malformed selectors and nested keys', () => {
  for (const candidate of [
    {},
    { takesLessDamage: 1, preventsDamageFrom: 'magic' },
    { takesLessDamage: 1, preventsDamageFromUnitsWithPowerAtLeast: 2 },
    { preventsDamageFrom: 'magic', preventsDamageFromUnitsWithPowerAtLeast: 2 },
  ]) {
    assert.throws(
      () => createGameManifest(input(minion({ nearbyDamagePrevention: candidate as never }))),
      /exactly one prevention selector/,
    );
  }
  assert.throws(
    () => createGameManifest(input(minion({ nearbyDamagePrevention: { takesLessDamage: 1, ward: true } as never }))),
    /nearbyDamagePrevention\.ward is unsupported/,
  );
  assert.throws(
    () => createGameManifest(input(minion({ nearbyDamagePrevention: { alliedOnly: false, takesLessDamage: 1 } as never }))),
    /alliedOnly must be true/,
  );
});

test('nearby provider prevention bounds numeric selectors', () => {
  for (const value of [0, -1, 101, 1.5, null, '2']) {
    assert.throws(
      () => createGameManifest(input(minion({ nearbyDamagePrevention: { takesLessDamage: value } as never }))),
      /takesLessDamage/,
    );
    assert.throws(
      () => createGameManifest(input(minion({ nearbyDamagePrevention: { preventsDamageFromUnitsWithPowerAtLeast: value } as never }))),
      /preventsDamageFromUnitsWithPowerAtLeast/,
    );
  }
});

test('nearby provider prevention is cloned into the canonical manifest', () => {
  const source: { alliedOnly?: true; takesLessDamage?: number } = { alliedOnly: true, takesLessDamage: 1 };
  const manifest = createGameManifest(input(minion({ nearbyDamagePrevention: source })));
  source.takesLessDamage = 100;
  const cloned = (manifest.cards.minion as Extract<GameCardDefinition, { cardType: 'minion' }>).nearbyDamagePrevention;
  assert.notEqual(cloned, source);
  assert.deepEqual(cloned, { alliedOnly: true, takesLessDamage: 1 });
  assert.equal(Object.isFrozen(cloned), true);
});
