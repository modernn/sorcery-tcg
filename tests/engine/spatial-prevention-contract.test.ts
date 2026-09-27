import assert from 'node:assert/strict';
import test from 'node:test';

import { createGameManifest, type GameCardDefinition, type GameManifestInput } from '../../src/engine/game.ts';

const authority = {
  contentHash: 'sha256:2222222222222222222222222222222222222222222222222222222222222222' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-spatial-prevention-test',
};

function input(
  minion: GameCardDefinition,
  site: Extract<GameCardDefinition, { cardType: 'site' }> = { cardType: 'site', elements: ['earth'] },
): GameManifestInput {
  return {
    authority,
    cards: {
      avatar: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
      site,
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

test('site affinity accepts repeated printed production and preserves membership', () => {
  const siteAffinity = { air: 0, earth: 0, fire: 2, water: 0 } as const;
  const manifest = createGameManifest(input(minion(), {
    cardType: 'site', elements: ['fire'], siteAffinity,
  }));
  const site = manifest.cards.site;
  assert(site?.cardType === 'site');
  assert.deepEqual(site.siteAffinity, siteAffinity);
  const legacy = createGameManifest(input(minion())).cards.site;
  assert(legacy?.cardType === 'site');
  assert.equal(legacy.siteAffinity, undefined);
});

test('site affinity rejects incomplete, extra, malformed, and membership-mismatched values', () => {
  const candidates = [
    { air: 0, earth: 0, fire: 2 },
    { air: 0, earth: 0, fire: 2, water: 0, void: 0 },
    { air: 0, earth: 0, fire: 0, water: 0 },
    { air: 0, earth: 1, fire: 2, water: 0 },
    { air: 0, earth: 0, fire: 101, water: 0 },
    { air: 0, earth: 0, fire: 1.5, water: 0 },
    { air: 0, earth: 0, fire: null, water: 0 },
  ];
  for (const siteAffinity of candidates) {
    assert.throws(
      () => createGameManifest(input(minion(), {
        cardType: 'site', elements: ['fire'], siteAffinity: siteAffinity as never,
      })),
      /siteAffinity/,
    );
  }
});

test('site affinity is cloned into the canonical manifest', () => {
  const siteAffinity = { air: 0, earth: 0, fire: 2, water: 0 };
  const manifest = createGameManifest(input(minion(), {
    cardType: 'site', elements: ['fire'], siteAffinity,
  }));
  siteAffinity.fire = 100;
  const cloned = manifest.cards.site;
  assert(cloned?.cardType === 'site');
  assert.notEqual(cloned.siteAffinity, siteAffinity);
  assert.deepEqual(cloned.siteAffinity, { air: 0, earth: 0, fire: 2, water: 0 });
  assert.equal(Object.isFrozen(cloned.siteAffinity), true);
});

test('site entry effect accepts and clones only the closed Stealth effect', () => {
  const manifest = createGameManifest(input(minion(), {
    cardType: 'site', elements: ['earth'], siteEntryEffect: 'grantStealthToEnteringMinion',
  }));
  const site = manifest.cards.site;
  assert(site?.cardType === 'site');
  assert.equal(site.siteEntryEffect, 'grantStealthToEnteringMinion');
  for (const siteEntryEffect of [true, 'killEnteringMinion', {}]) {
    assert.throws(
      () => createGameManifest(input(minion(), {
        cardType: 'site', elements: ['earth'], siteEntryEffect: siteEntryEffect as never,
      })),
      /siteEntryEffect/u,
    );
  }
  const first = createGameManifest(input(minion(), {
    cardType: 'site', elements: ['air'],
    siteEntryEffect: 'grantStealthToEnteringMinion', siteEntryUsage: 'firstEntry',
  }));
  assert.equal(first.cards.site?.siteEntryUsage, 'firstEntry');
  assert.throws(
    () => createGameManifest(input(minion(), {
      cardType: 'site', elements: ['air'], siteEntryUsage: 'firstEntry',
    })),
    /siteEntryUsage/u,
  );
  assert.throws(
    () => createGameManifest(input(minion(), {
      cardType: 'site', elements: ['air'],
      siteEntryEffect: 'grantStealthToEnteringMinion', siteEntryUsage: 'perTurn' as never,
    })),
    /siteEntryUsage/u,
  );
});

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
  const withSubtype = createGameManifest(input(minion({
    nearbyDamagePrevention: { recipientSubtype: 'Faerie', takesLessDamage: 1 },
  })));
  assert.deepEqual(
    (withSubtype.cards.minion as Extract<GameCardDefinition, { cardType: 'minion' }>).nearbyDamagePrevention,
    { recipientSubtype: 'Faerie', takesLessDamage: 1 },
  );
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

test('nearby provider prevention rejects malformed recipient subtypes', () => {
  for (const recipientSubtype of [
    null,
    1,
    {},
    '',
    ' ',
    '\uFEFFFairie',
    'Faerie\uFEFF',
    '\u0000Faerie',
    'é'.repeat(33),
  ]) {
    assert.throws(
      () => createGameManifest(input(minion({
        nearbyDamagePrevention: { recipientSubtype, takesLessDamage: 1 } as never,
      }))),
      /nearbyDamagePrevention\.recipientSubtype/,
    );
  }
});

test('nearby provider prevention is cloned into the canonical manifest', () => {
  const source: { alliedOnly?: true; recipientSubtype?: string; takesLessDamage?: number } = {
    alliedOnly: true,
    recipientSubtype: 'Faerie',
    takesLessDamage: 1,
  };
  const manifest = createGameManifest(input(minion({ nearbyDamagePrevention: source })));
  source.takesLessDamage = 100;
  source.recipientSubtype = 'Mortal';
  const cloned = (manifest.cards.minion as Extract<GameCardDefinition, { cardType: 'minion' }>).nearbyDamagePrevention;
  assert.notEqual(cloned, source);
  assert.deepEqual(cloned, { alliedOnly: true, recipientSubtype: 'Faerie', takesLessDamage: 1 });
  assert.equal(Object.isFrozen(cloned), true);
});
