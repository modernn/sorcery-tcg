import assert from 'node:assert/strict';
import test from 'node:test';

import { sha256 } from '../../src/authority/hash.ts';
import {
  ingestTopDeckTournaments,
  TOPDECK_ATTRIBUTION,
  TopDeckIngestionError,
} from '../../src/ingestion/topdeck.ts';

const RETRIEVED_AT = '2026-08-31T18:00:00Z';

function tournament(changes: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    TID: 'sorcery-test-1',
    eventData: { city: 'Auckland', country: 'NZ' },
    format: '',
    game: 'Sorcery: Contested Realm',
    standings: [
      {
        standing: 2,
        name: 'South Player',
        id: 'player-south',
        decklist: 'https://sorcerytcg.com/decks/example',
      },
      {
        standing: 1,
        name: 'North Player',
        id: 'player-north',
        decklist: '~~Avatar~~\r\n1 Pathfinder\r\n~~Spellbook~~\r\n2 Firebolts\r\nambiguous row',
        deckObj: { Avatar: { Pathfinder: 1 }, Spellbook: { Firebolts: 2 } },
      },
      {
        standing: 3,
        name: 'No Deck Player',
        id: 'player-none',
        decklist: null,
      },
    ],
    startDate: 1_788_115_200,
    swissNum: 6,
    topCut: 8,
    tournamentName: 'Synthetic Sorcery Championship',
    ...changes,
  };
}

test('ingests one bounded TopDeck API response with attribution and unresolved deck rows', async () => {
  const responseText = JSON.stringify([tournament()]);
  const calls: Array<{ input: string; init: RequestInit | undefined }> = [];
  const fetchImpl: typeof fetch = async (input, init) => {
    calls.push({ input: String(input), init });
    return new Response(responseText, { headers: { 'Content-Type': 'application/json' } });
  };

  const result = await ingestTopDeckTournaments({
    apiKey: 'synthetic-api-key',
    fetchImpl,
    lastDays: 30,
    retrievedAt: RETRIEVED_AT,
  });

  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.input, 'https://topdeck.gg/api/v2/tournaments');
  assert.equal(calls[0]?.init?.method, 'POST');
  assert.equal((calls[0]?.init?.headers as Record<string, string>).Authorization, 'synthetic-api-key');
  assert.deepEqual(JSON.parse(String(calls[0]?.init?.body)), {
    columns: ['name', 'id', 'decklist'],
    format: '',
    game: 'Sorcery: Contested Realm',
    last: 30,
  });
  assert.deepEqual(result.source.attribution, TOPDECK_ATTRIBUTION);
  assert.equal(result.source.responseByteHash, sha256(Buffer.from(responseText)));
  assert.equal(result.source.retrievedAt, '2026-08-31T18:00:00.000Z');

  const event = result.tournaments[0];
  assert.ok(event);
  assert.equal(event.status, 'completed');
  assert.equal(event.tournamentId, 'sorcery-test-1');
  assert.equal(event.participantCount, 3);
  assert.deepEqual(event.placements.map(({ placement }) => placement), [1, 2, 3]);

  const deck = event.placements[0]?.deck;
  assert.ok(deck);
  assert.deepEqual(deck.opaqueStructuredDeck, {
    Avatar: { Pathfinder: 1 },
    Spellbook: { Firebolts: 2 },
  });
  assert.deepEqual(deck.rows, [
    {
      lineNumber: 2,
      mappingStatus: 'unresolved',
      parseStatus: 'parsed',
      quantity: 1,
      raw: '1 Pathfinder',
      section: 'Avatar',
      sourceCardName: 'Pathfinder',
    },
    {
      lineNumber: 4,
      mappingStatus: 'unresolved',
      parseStatus: 'parsed',
      quantity: 2,
      raw: '2 Firebolts',
      section: 'Spellbook',
      sourceCardName: 'Firebolts',
    },
    {
      lineNumber: 5,
      mappingStatus: 'unresolved',
      parseStatus: 'unrecognized',
      quantity: null,
      raw: 'ambiguous row',
      section: 'Spellbook',
      sourceCardName: null,
    },
  ]);
  assert.equal(event.placements[1]?.deck?.sourceUrl, 'https://sorcerytcg.com/decks/example');
  assert.deepEqual(event.placements[1]?.deck?.rows, []);
  assert.equal(event.placements[2]?.deck, null);
});

test('rejects malformed input and unsupported response shapes before normalizing', async () => {
  let calls = 0;
  const unusedFetch: typeof fetch = async () => {
    calls += 1;
    return new Response('[]', { headers: { 'Content-Type': 'application/json' } });
  };
  await assert.rejects(
    () => ingestTopDeckTournaments({
      apiKey: ' key-with-whitespace ',
      fetchImpl: unusedFetch,
      lastDays: 30,
      retrievedAt: RETRIEVED_AT,
    }),
    (error: unknown) => {
      assert.ok(error instanceof TopDeckIngestionError);
      assert.equal(error.code, 'invalid_input');
      assert.equal(error.path, '/apiKey');
      return true;
    },
  );
  assert.equal(calls, 0);

  const invalidFetch: typeof fetch = async () => new Response(
    JSON.stringify([tournament({ game: 'Magic: The Gathering' })]),
    { headers: { 'Content-Type': 'application/json' } },
  );
  await assert.rejects(
    () => ingestTopDeckTournaments({
      apiKey: 'key',
      fetchImpl: invalidFetch,
      lastDays: 30,
      retrievedAt: RETRIEVED_AT,
    }),
    (error: unknown) => {
      assert.ok(error instanceof TopDeckIngestionError);
      assert.equal(error.code, 'invalid_response');
      assert.equal(error.path, '/0/game');
      return true;
    },
  );

  const teamFetch: typeof fetch = async () => new Response(
    JSON.stringify([tournament({ isTeamEvent: true, teamSize: 3 })]),
    { headers: { 'Content-Type': 'application/json' } },
  );
  await assert.rejects(
    () => ingestTopDeckTournaments({
      apiKey: 'key',
      fetchImpl: teamFetch,
      lastDays: 30,
      retrievedAt: RETRIEVED_AT,
    }),
    (error: unknown) => {
      assert.ok(error instanceof TopDeckIngestionError);
      assert.equal(error.code, 'unsupported_team_event');
      assert.equal(error.path, '/0');
      return true;
    },
  );
});

test('rejects duplicate JSON keys and oversized responses without a live request', async () => {
  const duplicateFetch: typeof fetch = async () => new Response(
    '[{"TID":"first","TID":"second"}]',
    { headers: { 'Content-Type': 'application/json' } },
  );
  await assert.rejects(
    () => ingestTopDeckTournaments({
      apiKey: 'key',
      fetchImpl: duplicateFetch,
      lastDays: 1,
      retrievedAt: RETRIEVED_AT,
    }),
    (error: unknown) => {
      assert.ok(error instanceof TopDeckIngestionError);
      assert.equal(error.code, 'invalid_json');
      return true;
    },
  );

  const oversizedFetch: typeof fetch = async () => new Response('[]', {
    headers: {
      'Content-Length': '5000001',
      'Content-Type': 'application/json',
    },
  });
  await assert.rejects(
    () => ingestTopDeckTournaments({
      apiKey: 'key',
      fetchImpl: oversizedFetch,
      lastDays: 1,
      retrievedAt: RETRIEVED_AT,
    }),
    (error: unknown) => {
      assert.ok(error instanceof TopDeckIngestionError);
      assert.equal(error.code, 'response_too_large');
      return true;
    },
  );
});
