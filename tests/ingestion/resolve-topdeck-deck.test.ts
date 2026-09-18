import assert from 'node:assert/strict';
import test from 'node:test';

import type { NormalizedCard } from '../../src/authority/schemas.ts';
import { resolveTopDeckDeck } from '../../src/ingestion/resolve-topdeck-deck.ts';
import { selectTopDecksPerAvatar } from '../../src/ingestion/select-meta-decks.ts';
import type { TopDeckCandidateSnapshot } from '../../src/ingestion/topdeck-candidates.ts';

const THRESHOLDS = { air: 0, earth: 0, fire: 0, water: 0 };

function card(
  name: string,
  cardType: NormalizedCard['cardType'],
  stableId = `card:${name.toLocaleLowerCase('en-US').replaceAll(/\s+/gu, '-')}`,
): NormalizedCard {
  return {
    attack: cardType === 'minion' || cardType === 'avatar' ? 1 : null,
    cardType,
    defense: cardType === 'minion' || cardType === 'avatar' ? 1 : null,
    elements: cardType === 'site' ? ['earth'] : [],
    life: cardType === 'avatar' ? 20 : null,
    manaCost: cardType === 'minion' ? 1 : null,
    name,
    officialSourceId: null,
    printingSlugs: [],
    rarity: 'ordinary',
    rulesText: '',
    stableId,
    subtypes: [],
    thresholds: THRESHOLDS,
  };
}

const CARDS = [
  card('Pathfinder', 'avatar'),
  card('Firebolts', 'magic', 'card:firebolts-magic'),
  card('Humble Village', 'site'),
  card('Wild Boars', 'minion'),
] as const;

test('resolves opaque and text TopDeck rows into a canonical deck identity', () => {
  const resolved = resolveTopDeckDeck({
    opaqueStructuredDeck: {
      Atlas: { 'Humble Village': 30 },
      Avatar: { Pathfinder: 1 },
      Spellbook: { 'Wild Boars': 60 },
    },
    rows: [],
    sourceText: null,
    sourceUrl: null,
  }, CARDS);

  assert.equal(resolved.deckResolvable, true);
  assert.equal(resolved.avatarName, 'Pathfinder');
  assert.equal(resolved.avatarStableId, CARDS[0]!.stableId);
  assert.equal(resolved.atlas.length, 1);
  assert.equal(resolved.spellbook.length, 1);
  assert.ok(resolved.deckId?.startsWith('sha256:'));
});

test('selects up to three distinct top placements per avatar', () => {
  const snapshot: TopDeckCandidateSnapshot = {
    contentHash: 'sha256:0000000000000000000000000000000000000000000000000000000000000000',
    schemaVersion: 1,
    source: {
      apiVersion: 'v2',
      attribution: { text: 'Data provided by TopDeck.gg', url: 'https://topdeck.gg' },
      endpoint: 'https://topdeck.gg/api/v2/tournaments',
      retrievedAt: '2026-08-31T18:00:00.000Z',
      responseByteHash: 'sha256:0000000000000000000000000000000000000000000000000000000000000000',
    },
    tournaments: [{
      candidates: [
        {
          deck: {
            opaqueStructuredDeck: {
              Atlas: { 'Humble Village': 30 },
              Avatar: { Pathfinder: 1 },
              Spellbook: { 'Wild Boars': 60 },
            },
            rows: [],
            sourceText: null,
            sourceUrl: null,
          },
          resultEvidence: { placement: 1 },
        },
        {
          deck: {
            opaqueStructuredDeck: {
              Atlas: { 'Humble Village': 29, Stream: 1 },
              Avatar: { Pathfinder: 1 },
              Spellbook: { 'Wild Boars': 60 },
            },
            rows: [],
            sourceText: null,
            sourceUrl: null,
          },
          resultEvidence: { placement: 2 },
        },
        {
          deck: {
            opaqueStructuredDeck: {
              Atlas: { 'Humble Village': 30 },
              Avatar: { Sparkmage: 1 },
              Spellbook: { 'Wild Boars': 60 },
            },
            rows: [],
            sourceText: null,
            sourceUrl: null,
          },
          resultEvidence: { placement: 3 },
        },
      ],
      participantCount: 3,
      startedAt: '2026-08-30T18:40:00.000Z',
      tournamentId: 'event-1',
    }],
  };
  const cards = [
    ...CARDS,
    card('Sparkmage', 'avatar', 'card:sparkmage'),
    card('Stream', 'site', 'card:stream'),
  ];
  const selection = selectTopDecksPerAvatar(snapshot, cards, 3);

  assert.equal(selection.candidates.length, 3);
  assert.equal(selection.byAvatar[CARDS[0]!.stableId]?.length, 2);
  assert.equal(selection.byAvatar['card:sparkmage']?.length, 1);
  assert.deepEqual(selection.skipped, { missingDeck: 0, unresolvedDeck: 0 });
});
