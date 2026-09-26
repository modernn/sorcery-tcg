import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import type { PrivateCardSnapshot } from '../../src/authority/private-cards.ts';
import { identityHash } from '../../src/authority/hash.ts';
import type { PrivateStarterPreset } from '../../src/commands/run-private-game-check.ts';
import { createGameManifest, tokenDependencies, type GameCardDefinition } from '../../src/engine/game.ts';
import { RustSessionClient } from '../../src/engine/rust-engine.ts';
import { buildPresetCardPool, prepareBoundExperiment, presetCardCatalog } from '../../src/ingestion/preset-card-pool.ts';
import { loadReviewedCardBindings, mergeReviewedCardBindings } from '../../src/ingestion/reviewed-card-bindings.ts';

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

test('preset ingestion rejects changed printed scalars and null-to-zero token costs', () => {
  const base = presets[0]!;
  for (const [cardId, changed, field] of [
    ['a', { ...facts.a!, attack: 9 }, 'attack'],
    ['a', { ...facts.a!, manaCost: 7 }, 'manaCost'],
    ['a', { ...facts.a!, thresholds: { ...thresholds, earth: 1 } }, 'thresholds'],
    ['avatar', { ...facts.avatar!, life: 99 }, 'life'],
    ['site', { ...facts.site!, elements: ['water'] }, 'elements'],
  ] as const) {
    const changedPreset = { ...base, manifest: { ...base.manifest,
      cards: { ...base.manifest.cards, [cardId]: changed as GameCardDefinition } } };
    assert.throws(() => buildPresetCardPool(authority, [changedPreset]),
      new RegExp(`preset binding differs from source ${field}`));
  }
  const absentTokenAuthority = { ...authority, cards: authority.cards.map((card) =>
    card.stableId === 'token' ? { ...card, manaCost: null } : card) };
  assert.throws(() => buildPresetCardPool(absentTokenAuthority, [base]),
    /preset binding differs from source manaCost: token/);
});

test('reviewed local bindings require exact source identity, complete review and matching printed stats', () => {
  const pool = buildPresetCardPool(authority, presets);
  const source = authority.cards.find((card) => card.stableId === 'unbound')!;
  const row = { cardId: source.stableId, sourceCardHash: identityHash(source as unknown as JsonValue),
    facts: { ...facts.a!, ordinary: true }, review: { entireRulesText: true,
      proofs: ['synthetic vanilla minion movement and combat proof'] } };
  const file = { schemaVersion: 1, authorityHash: authority.authorityHash,
    revisionId: authority.revisionId, cards: [row] };
  const merged = mergeReviewedCardBindings(file, authority, pool);
  assert.deepEqual(merged.get('unbound')?.definition, row.facts);
  assert.equal(pool.has('unbound'), false);
  assert.deepEqual(mergeReviewedCardBindings(file, authority, merged), merged);
  assert.throws(() => mergeReviewedCardBindings({ ...file, revisionId: 'stale' }, authority, pool), /authority/);
  for (const changed of [
    { ...row, sourceCardHash: `sha256:${'f'.repeat(64)}` },
    { ...row, facts: { ...row.facts, attack: 99 } },
    { ...row, facts: { ...row.facts, ordinary: false } },
    { ...row, facts: { ...row.facts, unknownAbility: true } },
    { ...row, review: { ...row.review, entireRulesText: false } },
    { ...row, review: { ...row.review, proofs: [] } },
  ]) assert.throws(() => mergeReviewedCardBindings({ ...file, cards: [changed] }, authority, pool));
  assert.throws(() => mergeReviewedCardBindings({ ...file, cards: [row, row] }, authority, pool), /duplicate/);
  assert.throws(() => mergeReviewedCardBindings(file, authority,
    new Map([...pool, ['unbound', { definition: facts.b!, presetIds: [] }]])), /conflicting/);
});

test('reviewed token bindings retain an explicit absent printed mana cost through pool closure', () => {
  const sourceToken = facts.token!;
  if (sourceToken.cardType !== 'minion') throw new Error('synthetic token fixture must be a minion');
  const token: Extract<GameCardDefinition, { cardType: 'minion' }> = { ...sourceToken, manaCost: null, ordinary: true };
  const cards: Record<string, GameCardDefinition> = { ...facts, token };
  const decks = { avatar: 'avatar', atlas: ['site', 'site', 'site'], spellbook: ['spell', 'a', 'a'] };
  const manifest = createGameManifest({
    authority: { mode: 'private-local', contentHash: authority.authorityHash, revisionId: authority.revisionId },
    cards: { avatar: cards.avatar!, site: cards.site!, a: cards.a!, spell: cards.spell!, token },
    decks: { north: decks, south: decks }, firstSeat: 'north', seed: 7,
  });
  const source = authority.cards.find((card) => card.stableId === 'token')!;
  const nullTokenAuthority: PrivateCardSnapshot = {
    ...authority,
    cards: authority.cards.map((card) => card.stableId === 'token'
      ? { ...card, manaCost: null }
      : card),
  };
  const pool = buildPresetCardPool(nullTokenAuthority, [{
    id: 'air-starter',
    manifest,
  }]);
  const pooledToken = pool.get('token')?.definition;
  assert.equal(pooledToken?.cardType, 'minion');
  assert.equal(pooledToken?.cardType === 'minion' ? pooledToken.manaCost : undefined, null);
  const row = {
    cardId: 'token',
    sourceCardHash: identityHash({ ...source, manaCost: null } as unknown as JsonValue),
    facts: token,
    review: { entireRulesText: true, proofs: ['synthetic token absent-cost review'] },
  };
  const merged = mergeReviewedCardBindings({
    schemaVersion: 1, authorityHash: nullTokenAuthority.authorityHash,
    revisionId: nullTokenAuthority.revisionId, cards: [row],
  }, nullTokenAuthority, pool);
  const mergedToken = merged.get('token')?.definition;
  assert.equal(mergedToken?.cardType, 'minion');
  assert.equal(mergedToken?.cardType === 'minion' ? mergedToken.manaCost : undefined, null);
});

test('reviewed bindings replace matching preset facts only with an explicit prior hash', () => {
  const source = authority.cards.find((card) => card.stableId === 'unbound')!;
  const previous = { ...facts.a!, ordinary: true as const };
  const replacement = { ...previous, charge: true as const };
  const pool = new Map<string, { definition: GameCardDefinition; presetIds: readonly string[] }>([
    ['unbound', { definition: previous, presetIds: ['synthetic-preset'] }],
  ]);
  const row = {
    cardId: source.stableId,
    sourceCardHash: identityHash(source as unknown as JsonValue),
    replacesFactsHash: identityHash(previous as JsonValue),
    facts: replacement,
    review: { entireRulesText: true, proofs: ['synthetic replacement proof'] },
  };
  const file = { schemaVersion: 1, authorityHash: authority.authorityHash,
    revisionId: authority.revisionId, cards: [row] };
  const merged = mergeReviewedCardBindings(file, authority, pool);
  assert.deepEqual(merged.get('unbound')?.definition, replacement);
  assert.deepEqual(merged.get('unbound')?.presetIds, ['reviewed-local', 'synthetic-preset']);
  assert.throws(() => mergeReviewedCardBindings({ ...file,
    cards: [{ ...row, replacesFactsHash: `sha256:${'f'.repeat(64)}` }] }, authority, pool), /replacement/);
  assert.throws(() => mergeReviewedCardBindings({ ...file,
    cards: [{ ...row, replacesFactsHash: identityHash(replacement as JsonValue) }] }, authority,
  new Map()), /replacement/);
});

test('optional reviewed binding file fails closed on malformed data and file aliases', async () => {
  const root = await mkdtemp(join(tmpdir(), 'reviewed-bindings-'));
  const pool = buildPresetCardPool(authority, presets);
  try {
    assert.equal(await loadReviewedCardBindings(root, authority, pool), pool);
    await mkdir(join(root, 'bindings'));
    const path = join(root, 'bindings/reviewed.json');
    await writeFile(path, '{"schemaVersion":1,"schemaVersion":2}');
    await assert.rejects(loadReviewedCardBindings(root, authority, pool));
    await rm(path);
    await writeFile(join(root, 'other.json'), '{}');
    await symlink(join(root, 'other.json'), path);
    await assert.rejects(loadReviewedCardBindings(root, authority, pool));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('reviewed sites retain printed rarity for shared ordinary-site effects', () => {
  const source = authority.cards.find((card) => card.stableId === 'site')!;
  const row = { cardId: source.stableId, sourceCardHash: identityHash(source as unknown as JsonValue),
    facts: { ...facts.site!, ordinary: true }, review: { entireRulesText: true,
      proofs: ['synthetic ordinary-site ability suppression scenario'] } };
  const file = { schemaVersion: 1, authorityHash: authority.authorityHash,
    revisionId: authority.revisionId, cards: [row] };
  assert.equal(mergeReviewedCardBindings(file, authority, new Map()).size, 1);
  assert.throws(() => mergeReviewedCardBindings({ ...file,
    cards: [{ ...row, facts: facts.site }] }, authority, new Map()), /source ordinary/);
  const exceptional = { ...source, rarity: 'exceptional' as const };
  assert.throws(() => mergeReviewedCardBindings({ ...file,
    cards: [{ ...row, sourceCardHash: identityHash(exceptional as unknown as JsonValue) }] },
  { ...authority, cards: [exceptional] }, new Map()), /source ordinary/);
});

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

test('programmed token dependencies retain the full transitive pool closure', async () => {
  const programmedSpell: GameCardDefinition = {
    cardType: 'magic', manaCost: 0, thresholds,
    effectProgram: { effects: [
      { op: 'summon-token', token: 'token', count: 1, destination: 'source' },
      { op: 'summon-token', token: 'token2', count: 1, destination: 'source' },
    ] },
  };
  const tokenFact = facts.token as Extract<GameCardDefinition, { cardType: 'minion' }>;
  const tokenWithGenesis: GameCardDefinition = {
    ...tokenFact,
    genesisProgram: { effects: [{ op: 'summon-token', token: 'token3', count: 1, destination: 'source' }] },
  };
  const nestedFacts: Record<string, GameCardDefinition> = {
    avatar: facts.avatar!, site: facts.site!, a: facts.a!, b: facts.b!, programmedSpell, token: tokenWithGenesis,
    token2: { ...tokenFact, attack: 2 }, token3: { ...tokenFact, attack: 3 },
  };
  const extraCards = ['programmedSpell', 'token2', 'token3'].map((stableId) => {
    const definition = nestedFacts[stableId]!;
    return { stableId, name: `Synthetic ${stableId}`, cardType: definition.cardType,
      rulesText: '', attack: 'attack' in definition ? definition.attack : null,
      defense: 'defense' in definition ? definition.defense : null,
      manaCost: 'manaCost' in definition ? definition.manaCost : null,
      life: null, elements: [], rarity: 'ordinary' as const, thresholds, subtypes: [],
      printingSlugs: [], officialSourceId: null };
  });
  const nestedAuthority = { ...authority, cards: [...authority.cards, ...extraCards] };
  const nestedDeck = { avatar: 'avatar', atlas: ['site', 'site', 'site'], spellbook: ['programmedSpell', 'a', 'b'] };
  const nestedPreset: PrivateStarterPreset = {
    id: 'air-starter', label: 'nested-program', cardNames: {}, usesOnlyOrdinaryOrExceptionalCards: true,
    manifest: createGameManifest({
      authority: { mode: 'private-local', contentHash: authority.authorityHash, revisionId: authority.revisionId },
      cards: Object.fromEntries(Object.keys(nestedFacts).map((id) => [id, nestedFacts[id]!])),
      decks: { north: nestedDeck, south: nestedDeck }, firstSeat: 'north', seed: 1,
    }),
  };
  const nestedPool = buildPresetCardPool(nestedAuthority, [nestedPreset]);
  assert.deepEqual(tokenDependencies(programmedSpell), ['token', 'token2']);
  const nestedInput = { ...input, candidate: nestedDeck, opponent: { ...nestedDeck, spellbook: ['a', 'b', 'b'] } };
  const prepared = prepareBoundExperiment(nestedInput, nestedAuthority, nestedPool);
  assert.deepEqual(Object.keys(prepared.baseManifest.cards),
    ['a', 'avatar', 'b', 'programmedSpell', 'site', 'token', 'token2', 'token3']);
  const client = await RustSessionClient.start();
  try {
    const result = await client.newSession(canonicalJson(prepared.baseManifest as unknown as JsonValue));
    assert.equal(result.manifestId, prepared.baseManifest.manifestId);
    assert.equal(await client.verifyReplay(), true);
  } finally {
    await client.close();
  }
  const pruned = prepareBoundExperiment({ ...nestedInput, candidate: nestedInput.opponent }, nestedAuthority, nestedPool);
  assert.deepEqual(Object.keys(pruned.baseManifest.cards), ['a', 'avatar', 'b', 'site']);
  const missing = new Map(nestedPool); missing.delete('token3');
  assert.throws(() => prepareBoundExperiment(nestedInput, nestedAuthority, missing), /token dependency/);
  const nonToken = new Map(nestedPool);
  nonToken.set('token3', { definition: facts.a as Extract<GameCardDefinition, { cardType: 'minion' }>, presetIds: [] });
  assert.throws(() => prepareBoundExperiment(nestedInput, nestedAuthority, nonToken), /token minion/);
});

test('deck-only input rejects unbound cards, rule overrides, invalid zones and excessive requests', () => {
  const pool = buildPresetCardPool(authority, presets);
  assert.throws(() => prepareBoundExperiment({ ...input, candidate: { ...input.candidate, spellbook: ['unbound'] } }, authority, pool), /no reviewed binding/);
  for (const changed of [
    { ...input, baseManifest: {} }, { ...input, cards: facts }, { ...input, seeds: [] },
    { ...input, workers: 65 }, { ...input, seeds: [-1] },
    { ...input, candidate: { ...input.candidate, atlas: ['b', 'b', 'b'] } },
    { ...input, candidate: { ...input.candidate, spellbook: ['token', 'token', 'token'] } },
    { ...input, candidate: { ...input.candidate, spellbook: [{ cardId: 'a', copies: 201 }] } },
    { ...input, candidate: { ...input.candidate, spellbook: [{ cardId: 'a', copies: 200 }, { cardId: 'b', copies: 1 }] } },
  ]) assert.throws(() => prepareBoundExperiment(changed, authority, pool));
  const missingToken = new Map(pool);
  missingToken.delete('token');
  assert.throws(() => prepareBoundExperiment(input, authority, missingToken), /token dependency/);
});
