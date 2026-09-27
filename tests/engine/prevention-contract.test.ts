import assert from 'node:assert/strict';
import test from 'node:test';

import { createGameManifest, type GameCardDefinition, type GameManifestInput } from '../../src/engine/game.ts';

const authority = {
  contentHash: 'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-prevention-test',
};

function input(
  minion: GameCardDefinition,
  avatar: Extract<GameCardDefinition, { cardType: 'avatar' }> = {
    attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20,
  },
): GameManifestInput {
  return {
    authority,
    cards: {
      avatar,
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

test('preventsDamageFrom preserves each supported enum value', () => {
  for (const preventsDamageFrom of ['ranged-strikes', 'magic', 'earth-magic', 'fire-magic', 'water-magic', 'air-magic'] as const) {
    const manifest = createGameManifest(input(minion({ preventsDamageFrom })));
    const card = manifest.cards.minion;
    assert(card?.cardType === 'minion');
    assert.equal(card.preventsDamageFrom, preventsDamageFrom);
  }
});

test('preventsDamageFrom rejects malformed values and competing prevention facts', () => {
  for (const preventsDamageFrom of ['physical', '', null, 1]) {
    assert.throws(() => createGameManifest(input(minion({ preventsDamageFrom } as never))), /preventsDamageFrom/);
  }
  for (const competing of [
    { takesLessDamage: 1 },
    { ward: true },
    { preventsDamageFromUnitsWithPowerAtLeast: 2 },
  ] as const) {
    assert.throws(
      () => createGameManifest(input(minion({ preventsDamageFrom: 'magic', ...competing }))),
      /competing damage prevention effects/,
    );
  }
});

test('takesLessDamage preserves bounded values for minions and avatars', () => {
  const minionManifest = createGameManifest(input(minion({ takesLessDamage: 42 })));
  assert.equal((minionManifest.cards.minion as Extract<GameCardDefinition, { cardType: 'minion' }>).takesLessDamage, 42);
  const avatarManifest = createGameManifest(input(minion(), {
    attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20, takesLessDamage: 100,
  }));
  assert.equal((avatarManifest.cards.avatar as Extract<GameCardDefinition, { cardType: 'avatar' }>).takesLessDamage, 100);
  for (const value of [0, -1, 101, 1.5, null, '2']) {
    assert.throws(() => createGameManifest(input(minion({ takesLessDamage: value } as never))), /takesLessDamage/);
    assert.throws(() => createGameManifest(input(minion(), {
      attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20, takesLessDamage: value,
    } as never)), /takesLessDamage/);
  }
});
