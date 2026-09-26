import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import type { PrivateCardSnapshot } from '../../src/authority/private-cards.ts';
import type { PrivateStarterPreset } from '../../src/commands/run-private-game-check.ts';
import { createGameManifest, type GameCardDefinition } from '../../src/engine/game.ts';
import { RustSessionClient } from '../../src/engine/rust-engine.ts';
import { buildPresetCardPool, prepareBoundExperiment, presetCardCatalog } from '../../src/ingestion/preset-card-pool.ts';

const thresholds = { air: 0, earth: 0, fire: 0, water: 0 };
const facts: Record<string, GameCardDefinition> = {
  avatar: { cardType: 'avatar', attack: 1, defense: 1, life: 20, drawSpell: false },
  site: { cardType: 'site', elements: ['earth'] },
  a: { cardType: 'minion', attack: 1, defense: 1, manaCost: 0, thresholds },
  b: { cardType: 'minion', attack: 2, defense: 2, manaCost: 0, thresholds },
  token: { cardType: 'minion', attack: 1, defense: 1, manaCost: 0, thresholds, token: true },
  spell: { cardType: 'magic', manaCost: 0, thresholds, summonTokenToAlliedMinionThenDrawSpell: 'token' },
};
const authority: PrivateCardSnapshot = {
  authorityHash: `sha256:${'0'.repeat(64)}`,
  revisionId: 'synthetic-bindings',
  format: { atlasMinimum: 30, spellbookMinimum: 60, avatarCount: 1,
    copyLimits: { ordinary: 4, exceptional: 3, elite: 2, unique: 1 },
    name: 'Constructed', effectiveDate: '2026-01-01', parentFormatStableId: null, scope: null },
  cards: [...Object.entries(facts), ['unbound', facts.a!] as const].map(([stableId, definition]) => ({
    stableId, name: `Synthetic ${stableId}`, cardType: definition.cardType,
    rulesText: stableId === 'spell' ? 'Synthetic summon and draw.' : '',
    attack: 'attack' in definition ? definition.attack : null,
    defense: 'defense' in definition ? definition.defense : null,
    manaCost: 'manaCost' in definition ? definition.manaCost : null,
    life: definition.cardType === 'avatar' ? definition.life : null,
    elements: definition.cardType === 'site' ? definition.elements : [],
    rarity: 'ordinary', thresholds, subtypes: [], printingSlugs: [], officialSourceId: null,
  })),
};
function preset(id: PrivateStarterPreset['id'], spells: string[]): PrivateStarterPreset {
  const deck = { avatar: 'avatar', atlas: ['site', 'site', 'site'], spellbook: spells };
  const included = ['avatar', 'site', ...spells, ...(spells.includes('spell') ? ['token'] : [])];
  return {
    id, label: id, cardNames: {}, usesOnlyOrdinaryOrExceptionalCards: true,
    manifest: createGameManifest({
      authority: { mode: 'private-local', contentHash: authority.authorityHash, revisionId: authority.revisionId },
      cards: Object.fromEntries(included.map((cardId) => [cardId, facts[cardId]!])),
      decks: { north: deck, south: deck }, firstSeat: 'north', seed: 1,
    }),
  };
}
const presets = [preset('air-starter', ['a', 'spell', 'a']), preset('earth-starter', ['b', 'b', 'b'])];
const input = {
  schemaVersion: 1, seeds: [31], workers: 1,
  candidate: { avatar: 'avatar', atlas: ['site', 'site', 'site'], spellbook: ['spell', 'b', 'a'] },
  opponent: { avatar: 'avatar', atlas: ['site', 'site', 'site'], spellbook: ['b', 'b', 'b'] },
};

test('preset pool merges provenance deterministically and rejects authority or fact conflicts', () => {
  const pool = buildPresetCardPool(authority, presets);
  assert.deepEqual(pool, buildPresetCardPool(authority, [...presets].reverse()));
  assert.deepEqual(pool.get('avatar')?.presetIds, ['air-starter', 'earth-starter']);
  assert.equal(pool.size, 6);
  assert.throws(() => buildPresetCardPool({ ...authority, revisionId: 'changed' }, presets), /binding/);
  assert.throws(() => buildPresetCardPool({ ...authority, authorityHash: `sha256:${'1'.repeat(64)}` }, presets), /binding/);
  const changed = { ...presets[0]!, manifest: { ...presets[0]!.manifest,
    cards: { ...presets[0]!.manifest.cards, avatar: { ...facts.avatar!, attack: 5 } as GameCardDefinition } } };
  assert.throws(() => buildPresetCardPool(authority, [...presets, changed]), /conflicting preset facts/);
  assert.throws(() => buildPresetCardPool({ ...authority, cards: [] }, presets), /absent from authority/);
});

test('private catalog distinguishes unbound cards and retains exact source and fact identities', () => {
  const pool = buildPresetCardPool(authority, presets);
  const catalog = presetCardCatalog(authority, pool);
  assert.equal(catalog.boundCardCount, 6);
  assert.equal(catalog.rankedEligible, false);
  assert.deepEqual(catalog, presetCardCatalog({ ...authority, cards: [...authority.cards].reverse() }, pool));
  const missing = catalog.cards.find((card) => card.cardId === 'unbound')!;
  assert.equal(missing.engineSupported, false);
  assert.equal(missing.facts, null);
  const spell = catalog.cards.find((card) => card.cardId === 'spell')!;
  assert.deepEqual(spell.facts, facts.spell);
  assert.match(spell.sourceCardHash, /^sha256:/);
  assert.match(spell.factsHash!, /^sha256:/);
});

test('cross-preset compositions preserve facts and token dependencies and are admitted by Rust', async () => {
  const pool = buildPresetCardPool(authority, presets);
  const request = prepareBoundExperiment(input, authority, pool);
  assert.deepEqual(Object.keys(request.baseManifest.cards), ['a', 'avatar', 'b', 'site', 'spell', 'token']);
  for (const [cardId, definition] of Object.entries(request.baseManifest.cards)) {
    assert.deepEqual(definition, facts[cardId]);
  }
  const counted = { ...input, candidate: { ...input.candidate,
    spellbook: [{ cardId: 'a', copies: 1 }, { cardId: 'b', copies: 1 }, { cardId: 'spell', copies: 1 }] } };
  assert.deepEqual(request, prepareBoundExperiment(counted, authority, pool));
  const client = await RustSessionClient.start();
  try {
    const result = await client.newSession(canonicalJson(request.baseManifest as unknown as JsonValue));
    assert.equal(result.manifestId, request.baseManifest.manifestId);
    assert.equal(await client.verifyReplay(), true);
  } finally {
    await client.close();
  }
});

test('deck-only input rejects unbound cards, rule overrides, invalid zones and excessive requests', () => {
  const pool = buildPresetCardPool(authority, presets);
  assert.throws(() => prepareBoundExperiment({ ...input, candidate: { ...input.candidate, spellbook: ['unbound'] } }, authority, pool), /no preset binding/);
  for (const changed of [
    { ...input, baseManifest: {} }, { ...input, cards: facts }, { ...input, seeds: [] },
    { ...input, workers: 9 }, { ...input, seeds: [-1] },
    { ...input, candidate: { ...input.candidate, atlas: ['b', 'b', 'b'] } },
    { ...input, candidate: { ...input.candidate, spellbook: ['token', 'token', 'token'] } },
    { ...input, candidate: { ...input.candidate, spellbook: [{ cardId: 'a', copies: 201 }] } },
    { ...input, candidate: { ...input.candidate, spellbook: [{ cardId: 'a', copies: 200 }, { cardId: 'b', copies: 1 }] } },
  ]) assert.throws(() => prepareBoundExperiment(changed, authority, pool));
  const missingToken = new Map(pool);
  missingToken.delete('token');
  assert.throws(() => prepareBoundExperiment(input, authority, missingToken), /token dependency/);
});
