import assert from 'node:assert/strict';
import test from 'node:test';

import type { PrivateCardSnapshot } from '../../src/authority/private-cards.ts';
import type { NormalizedCard } from '../../src/authority/schemas.ts';
import { runMetaDeckGauntlet } from '../../src/commands/run-meta-deck-gauntlet.ts';
import {
  buildCandidateManifest,
  buildSingleCandidateManifest,
  resolvedDeckToSpec,
} from '../../src/ingestion/candidate-manifest.ts';
import { resolveTopDeckDeck } from '../../src/ingestion/resolve-topdeck-deck.ts';

function card(cardType: NormalizedCard['cardType'], rulesText = ''): NormalizedCard {
  return {
    attack: cardType === 'avatar' || cardType === 'minion' ? 1 : null,
    cardType,
    defense: cardType === 'avatar' || cardType === 'minion' ? 1 : null,
    elements: cardType === 'site' ? ['earth'] : [],
    life: cardType === 'avatar' ? 20 : null,
    manaCost: cardType === 'minion' ? 1 : null,
    name: `Synthetic ${cardType}`,
    officialSourceId: null,
    printingSlugs: [],
    rarity: 'ordinary',
    rulesText,
    stableId: `synthetic:${cardType}`,
    subtypes: [],
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
}

const CARDS = [card('avatar'), card('site'), card('minion')];
const AUTHORITY: PrivateCardSnapshot = {
  authorityHash: `sha256:${'0'.repeat(64)}`,
  cards: CARDS,
  format: {
    atlasMinimum: 3,
    avatarCount: 1,
    copyLimits: { elite: 2, exceptional: 3, ordinary: 4, unique: 1 },
    effectiveDate: '2026-01-01',
    name: 'Synthetic',
    parentFormatStableId: null,
    scope: null,
    spellbookMinimum: 3,
  },
  revisionId: 'synthetic-revision',
};
const DECK = resolveTopDeckDeck({
  opaqueStructuredDeck: {
    Atlas: { 'Synthetic site': 3 },
    Avatar: { 'Synthetic avatar': 1 },
    Spellbook: { 'Synthetic minion': 3 },
  },
  rows: [],
  sourceText: null,
  sourceUrl: null,
}, CARDS);
const SPEC = resolvedDeckToSpec(DECK);

test('candidate manifests preserve supported blank card facts and deterministic identity', () => {
  const result = buildCandidateManifest(AUTHORITY, DECK, 31, { north: SPEC, south: SPEC });
  assert.equal(result.supported, true);
  assert.deepEqual(result.unsupportedCardIds, []);
  assert.deepEqual(result.diagnostics, []);
  assert.deepEqual(result, buildSingleCandidateManifest(AUTHORITY, DECK, 31, SPEC));
  assert.ok(result.manifest);
  assert.equal(result.manifest.cards['synthetic:minion']?.cardType, 'minion');
  assert.deepEqual(result.manifest.decks, { north: SPEC, south: SPEC });
});

test('candidate ingestion fails closed for unbound avatar, site, and minion rules', () => {
  const cards = [
    card('avatar', 'Tap: draw a spell.'),
    card('site', 'Genesis: gain one mana.'),
    card('minion', 'Airborne'),
  ];
  const result = buildCandidateManifest({ ...AUTHORITY, cards }, DECK, 31, {
    north: SPEC,
    south: SPEC,
  });
  assert.equal(result.supported, false);
  assert.equal(result.manifest, null);
  assert.deepEqual(result.unsupportedCardIds, cards.map(({ stableId }) => stableId));
  assert.deepEqual(result.diagnostics, cards.map(({ stableId }) => ({
    cardId: stableId,
    reason: 'unbound-rules-text',
  })));
});

test('missing cards and unsupported facts return diagnostics without constructing an invalid manifest', () => {
  const cards = [card('avatar'), { ...card('minion'), attack: null }];
  const result = buildCandidateManifest({ ...AUTHORITY, cards }, DECK, 31, {
    north: SPEC,
    south: SPEC,
  });
  assert.equal(result.supported, false);
  assert.equal(result.manifest, null);
  assert.deepEqual(result.diagnostics, [
    { cardId: 'synthetic:site', reason: 'missing-card' },
    { cardId: 'synthetic:minion', reason: 'unsupported-card-facts' },
  ]);
});

test('meta gauntlet rejects unsafe output IDs before loading any authority data', async () => {
  for (const outputId of ['', '.', '..', '../escape', '/tmp/escape', 'nested/report', 'nested\\report', 'a'.repeat(129)]) {
    await assert.rejects(runMetaDeckGauntlet([
      '--output-id', outputId,
      '--scenario', '/nonexistent-synthetic-scenario.json',
    ]), /--output-id must be a confined path segment/u);
  }
});
