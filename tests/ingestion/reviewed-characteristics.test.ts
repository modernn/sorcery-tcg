import assert from 'node:assert/strict';
import test from 'node:test';
import { identityHash } from '../../src/authority/hash.ts';
import type { JsonValue } from '../../src/authority/canonical-json.ts';
import type { PrivateCardSnapshot } from '../../src/authority/private-cards.ts';
import { assertPrintedCardFacts, presetCardCatalog } from '../../src/ingestion/preset-card-pool.ts';
import type { GameCardDefinition } from '../../src/engine/game.ts';
import { mergeReviewedCardBindings } from '../../src/ingestion/reviewed-card-bindings.ts';

const thresholds = { air: 0, earth: 0, fire: 0, water: 0 };
const source = {
  stableId: 'synthetic-token', name: 'Synthetic token', cardType: 'minion' as const,
  attack: 1, defense: 1, manaCost: null, life: null, elements: [], thresholds,
  rarity: 'ordinary' as const, subtypes: ['Beast'], rulesText: '', printingSlugs: [], officialSourceId: null,
};
const authority: PrivateCardSnapshot = {
  authorityHash: identityHash({ fixture: 'reviewed-characteristics' }), revisionId: 'synthetic', cards: [source],
  format: { atlasMinimum: 30, spellbookMinimum: 60, avatarCount: 1,
    copyLimits: { ordinary: 4, exceptional: 3, elite: 2, unique: 1 }, name: 'Constructed',
    effectiveDate: '2026-01-01', parentFormatStableId: null, scope: null },
};
const facts = { cardType: 'minion', attack: 1, defense: 1, manaCost: null, thresholds,
  token: true, ordinary: true, elements: ['air'], subtypes: ['Beast', 'Undead'] };
const row = {
  cardId: source.stableId, sourceCardHash: identityHash(source as unknown as JsonValue), facts,
  characteristicSupplement: { elements: ['air'], subtypes: ['Undead'],
    sources: [{ contentHash: identityHash({ fixture: 'synthetic retained rule source' }), locator: 'Token characteristics' }] },
  review: { entireRulesText: true, proofs: ['synthetic source-characteristic consistency proof'] },
};
function binding(changed: unknown = row) {
  return { schemaVersion: 1, authorityHash: authority.authorityHash, revisionId: authority.revisionId, cards: [changed] };
}

test('reviewed supplements preserve printed absence and combine retained with sourced characteristics', () => {
  const merged = mergeReviewedCardBindings(binding(), authority, new Map());
  assert.deepEqual(merged.get(source.stableId)?.definition, facts);
  assert.equal(source.manaCost, null);
  assert.deepEqual(source.elements, []);
  assert.deepEqual(source.subtypes, ['Beast']);
});

test('supplements cannot replace printed scalars, remove retained traits or evade source identity', () => {
  for (const changed of [
    { ...row, facts: { ...facts, manaCost: 0 } },
    { ...row, facts: { ...facts, attack: 9 } },
    { ...row, facts: { ...facts, thresholds: { ...thresholds, air: 1 } } },
    { ...row, facts: { ...facts, ordinary: false } },
    { ...row, facts: { ...facts, subtypes: ['Undead'] } },
    { ...row, facts: { ...facts, elements: ['water'] } },
    { ...row, facts: { ...facts, subtypes: undefined } },
    { ...row, sourceCardHash: identityHash({ stale: true }) },
    { ...row, characteristicSupplement: undefined },
    { ...row, characteristicSupplement: { ...row.characteristicSupplement, sources: [] } },
    { ...row, characteristicSupplement: { ...row.characteristicSupplement, manaCost: 0 } },
  ]) assert.throws(() => mergeReviewedCardBindings(binding(changed), authority, new Map()));
});

const artifactSource = { ...source, cardType: 'artifact' as const, attack: null, defense: null,
  rarity: null, elements: ['air'] as const, subtypes: ['Device'] };
const artifactFacts = { cardType: 'artifact', manaCost: null, thresholds, token: true,
  grantsBearerPower: 2, rarity: 'ordinary', elements: ['earth', 'air'], subtypes: ['Device', 'Weapon'] };
const artifactRow = { ...row, sourceCardHash: identityHash(artifactSource as unknown as JsonValue),
  facts: artifactFacts, characteristicSupplement: { rarity: 'ordinary', elements: ['earth'], subtypes: ['Weapon'],
    sources: row.characteristicSupplement.sources } };
const artifactAuthority: PrivateCardSnapshot = { ...authority, cards: [artifactSource] };

test('reviewed artifact supplements fill absent rarity while preserving retained traits and source identity', () => {
  const originalHash = identityHash(artifactSource as unknown as JsonValue);
  const pool = mergeReviewedCardBindings(binding(artifactRow), artifactAuthority, new Map());
  assert.deepEqual(pool.get(source.stableId)?.definition, artifactFacts);
  assert.equal(identityHash(artifactSource as unknown as JsonValue), originalHash);
  assert.equal(artifactSource.rarity, null);
  assert.equal(presetCardCatalog(artifactAuthority, pool).cards[0]?.rarity, 'ordinary');
  assert.throws(() => assertPrintedCardFacts(artifactSource, artifactFacts as GameCardDefinition, 'preset binding'));
  const retainedFacts = { ...artifactFacts, rarity: undefined, elements: ['air'], subtypes: ['Device'] };
  assertPrintedCardFacts(artifactSource, retainedFacts as GameCardDefinition, 'preset binding');
});

test('artifact supplements cannot overwrite rarity, invent unreviewed characteristics, or bypass printed scalars', () => {
  for (const changed of [
    { ...artifactRow, facts: { ...artifactFacts, rarity: undefined } },
    { ...artifactRow, facts: { ...artifactFacts, rarity: 'elite' } },
    { ...artifactRow, facts: { ...artifactFacts, subtypes: ['Weapon'] } },
    { ...artifactRow, facts: { ...artifactFacts, subtypes: undefined } },
    { ...artifactRow, facts: { ...artifactFacts, elements: ['earth'] } },
    { ...artifactRow, facts: { ...artifactFacts, manaCost: 0 } },
    { ...artifactRow, facts: { ...artifactFacts, thresholds: { ...thresholds, air: 1 } } },
    { ...artifactRow, characteristicSupplement: undefined },
    { ...artifactRow, characteristicSupplement: { ...artifactRow.characteristicSupplement, sources: [] } },
    { ...artifactRow, sourceCardHash: identityHash({ stale: true }) },
  ]) assert.throws(() => mergeReviewedCardBindings(binding(changed), artifactAuthority, new Map()));

  const retained = { ...artifactSource, rarity: 'elite' as const };
  const retainedRow = { ...artifactRow, sourceCardHash: identityHash(retained as unknown as JsonValue) };
  const knownFacts = { ...artifactFacts, rarity: 'elite', elements: ['air'], subtypes: ['Device'] };
  const knownPool = mergeReviewedCardBindings(binding({ ...retainedRow, facts: knownFacts,
    characteristicSupplement: undefined }), { ...authority, cards: [retained] }, new Map());
  assert.deepEqual(knownPool.get(source.stableId)?.definition, knownFacts);
  for (const rarity of ['ordinary', 'elite']) {
    assert.throws(() => mergeReviewedCardBindings(binding({ ...retainedRow,
      facts: { ...artifactFacts, rarity }, characteristicSupplement: { ...artifactRow.characteristicSupplement, rarity },
    }), { ...authority, cards: [retained] }, new Map()), /only fill absent artifact rarity/);
  }
  assert.throws(() => mergeReviewedCardBindings(binding({ ...row,
    characteristicSupplement: { ...row.characteristicSupplement, rarity: 'ordinary' },
  }), authority, new Map()), /only fill absent artifact rarity/);
});
