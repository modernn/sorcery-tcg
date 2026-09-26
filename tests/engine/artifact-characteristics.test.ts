import assert from 'node:assert/strict';
import test from 'node:test';

import { createGameManifest, type GameCardDefinition, type GameManifestInput } from '../../src/engine/game.ts';

const authority = {
  contentHash: 'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-artifact-characteristics-test',
};

function input(token: GameCardDefinition): GameManifestInput {
  return { authority, cards: {
    avatar: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    site: { cardType: 'site', elements: ['earth'] },
    spell: { cardType: 'magic', effectProgram: { effects: [{ count: 1, destination: 'source', op: 'conjure-token', token: 'token' }] }, manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
    token,
  }, decks: {
    north: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['spell', 'spell', 'spell'] },
    south: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['spell', 'spell', 'spell'] },
  }, firstSeat: 'north', seed: 1 };
}

test('artifact characteristics preserve absent and explicit empty arrays, with canonical clones', () => {
  const empty = createGameManifest(input({ bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', elements: [],
    manaCost: null, rarity: 'ordinary', subtypes: [], thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true }));
  const card = empty.cards.token as Extract<GameCardDefinition, { cardType: 'artifact' }>;
  assert.deepEqual(card.elements, []);
  assert.deepEqual(card.subtypes, []);
  const absent = createGameManifest(input({ bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true }));
  assert.equal('elements' in absent.cards.token!, false);
  assert.equal('subtypes' in absent.cards.token!, false);
});

test('artifact characteristics compose with legacy effects and preserve canonical arrays', () => {
  const elements = ['earth', 'water'] as const;
  const subtypes = ['Relic', 'Token'] as const;
  const source = { bearerControllerChoosesExtraRandomOutcome: true as const, cardType: 'artifact' as const,
    elements, manaCost: null, rarity: 'elite' as const, subtypes,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true as const };
  const manifest = createGameManifest(input(source));
  const card = manifest.cards.token as Extract<GameCardDefinition, { cardType: 'artifact' }>;
  assert.notEqual(card.elements, source.elements);
  assert.notEqual(card.subtypes, source.subtypes);
  assert.deepEqual(card, {
    bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', elements: ['earth', 'water'], manaCost: null,
    rarity: 'elite', subtypes: ['Relic', 'Token'], thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true,
  });
});

test('artifact characteristics reject noncanonical arrays and unsupported rarity', () => {
  const base = { bearerControllerChoosesExtraRandomOutcome: true as const, cardType: 'artifact' as const, manaCost: null,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true as const };
  assert.throws(() => createGameManifest(input({ ...base, elements: ['water', 'earth'] })), /elements/);
  assert.throws(() => createGameManifest(input({ ...base, subtypes: ['Token', 'Token'] })), /subtypes/);
  assert.throws(() => createGameManifest(input({ ...base, rarity: 'mythic' } as never)), /rarity/);
  assert.throws(() => createGameManifest(input({ ...base, elements: null } as never)), /elements/);
  assert.throws(() => createGameManifest(input({ ...base, subtypes: null } as never)), /subtypes/);
  assert.throws(() => createGameManifest(input({ ...base, rarity: null } as never)), /rarity/);
});
