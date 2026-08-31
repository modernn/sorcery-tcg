import assert from 'node:assert/strict';
import test from 'node:test';

import { sha256 } from '../../src/authority/hash.ts';
import {
  ingestSorceryCards,
  SORCERY_CARD_API_POLICY,
  SorceryCardIngestionError,
} from '../../src/ingestion/sorcery-cards.ts';

const RETRIEVED_AT = '2026-08-31T18:00:00Z';

function engine(changes: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    air: 0,
    attack: null,
    back: null,
    category: 'Spell',
    cost: 4,
    defense: null,
    earth: 0,
    elements: ['None'],
    fire: 0,
    keywords: [],
    life: null,
    rarity: 'Unique',
    rules: 'Synthetic rules are not retained by the identity adapter.',
    slot: 'Unique',
    subtypes: ['Relic'],
    type: 'Artifact',
    umbrellas: [],
    water: 0,
    ...changes,
  };
}

function printing(changes: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: 'official-printing-standard',
    meta: {
      artist: { name: 'Synthetic Artist', slug: 'synthetic_artist' },
      back: null,
      finish: 'Standard',
      flavor: null,
      product: 'Booster',
      typeline: 'A synthetic Unique Artifact',
    },
    printedAt: '2024-10-04T07:00:00.000Z',
    set: {
      code: '004',
      name: 'Synthetic Legends',
      releasedAt: '2024-10-04T07:00:00.000Z',
    },
    slug: '004-synthetic_treasures-b-s',
    ...changes,
  };
}

function card(changes: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    engine: engine(),
    id: 'official-card-treasures',
    name: 'Synthetic Treasures',
    printings: [
      printing({
        id: 'official-printing-foil',
        meta: {
          artist: { name: 'Synthetic Artist', slug: 'synthetic_artist' },
          back: null,
          finish: 'Foil',
          flavor: null,
          product: 'Booster',
          typeline: 'A synthetic Unique Artifact',
        },
        printedAt: '2024-10-05T07:00:00.000Z',
        slug: '004-synthetic_treasures-b-f',
      }),
      printing(),
    ],
    slug: 'synthetic_treasures',
    ...changes,
  };
}

test('ingests one official response into stable card and printing identities', async () => {
  const responseText = JSON.stringify([card()]);
  const calls: Array<{ input: string; init: RequestInit | undefined }> = [];
  const fetchImpl: typeof fetch = async (input, init) => {
    calls.push({ input: String(input), init });
    return new Response(responseText, { headers: { 'Content-Type': 'application/json' } });
  };

  const result = await ingestSorceryCards({ fetchImpl, retrievedAt: RETRIEVED_AT });

  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.input, 'https://api.sorcerytcg.com/api/cards');
  assert.equal(calls[0]?.init?.method, 'GET');
  assert.deepEqual(calls[0]?.init?.headers, { Accept: 'application/json' });
  assert.equal(result.source.responseByteHash, sha256(Buffer.from(responseText)));
  assert.equal(result.source.retrievedAt, '2026-08-31T18:00:00.000Z');
  assert.deepEqual(result.source.policy, SORCERY_CARD_API_POLICY);
  assert.deepEqual(result.cards, [
    {
      category: 'Spell',
      name: 'Synthetic Treasures',
      officialCardId: 'official-card-treasures',
      officialCardSlug: 'synthetic_treasures',
      printings: [
        {
          finish: 'Standard',
          officialPrintingId: 'official-printing-standard',
          officialPrintingSlug: '004-synthetic_treasures-b-s',
          printedAt: '2024-10-04T07:00:00.000Z',
          product: 'Booster',
          set: {
            code: '004',
            name: 'Synthetic Legends',
            releasedAt: '2024-10-04T07:00:00.000Z',
          },
        },
        {
          finish: 'Foil',
          officialPrintingId: 'official-printing-foil',
          officialPrintingSlug: '004-synthetic_treasures-b-f',
          printedAt: '2024-10-05T07:00:00.000Z',
          product: 'Booster',
          set: {
            code: '004',
            name: 'Synthetic Legends',
            releasedAt: '2024-10-04T07:00:00.000Z',
          },
        },
      ],
      type: 'Artifact',
    },
  ]);
});

test('rejects undocumented response fields and invalid documented enum values', async () => {
  const extraFieldFetch: typeof fetch = async () => new Response(
    JSON.stringify([card({ image: 'https://private-cdn.invalid/card.jpg' })]),
    { headers: { 'Content-Type': 'application/json' } },
  );
  await assert.rejects(
    () => ingestSorceryCards({ fetchImpl: extraFieldFetch, retrievedAt: RETRIEVED_AT }),
    (error: unknown) => {
      assert.ok(error instanceof SorceryCardIngestionError);
      assert.equal(error.code, 'invalid_response');
      assert.equal(error.path, '/0');
      return true;
    },
  );

  const invalidProduct = printing();
  invalidProduct.meta = { ...(invalidProduct.meta as Record<string, unknown>), product: 'UnknownProduct' };
  const invalidProductFetch: typeof fetch = async () => new Response(
    JSON.stringify([card({ printings: [invalidProduct] })]),
    { headers: { 'Content-Type': 'application/json' } },
  );
  await assert.rejects(
    () => ingestSorceryCards({ fetchImpl: invalidProductFetch, retrievedAt: RETRIEVED_AT }),
    (error: unknown) => {
      assert.ok(error instanceof SorceryCardIngestionError);
      assert.equal(error.code, 'invalid_response');
      assert.equal(error.path, '/0/printings/0/meta/product');
      return true;
    },
  );
});

test('rejects duplicate official identities, duplicate JSON keys, and oversized responses', async () => {
  const duplicateIdentityFetch: typeof fetch = async () => new Response(
    JSON.stringify([card(), card({ name: 'Second Name', slug: 'second_slug' })]),
    { headers: { 'Content-Type': 'application/json' } },
  );
  await assert.rejects(
    () => ingestSorceryCards({ fetchImpl: duplicateIdentityFetch, retrievedAt: RETRIEVED_AT }),
    (error: unknown) => {
      assert.ok(error instanceof SorceryCardIngestionError);
      assert.equal(error.code, 'invalid_response');
      assert.equal(error.path, '/1/id');
      return true;
    },
  );

  const duplicateJsonFetch: typeof fetch = async () => new Response(
    '[{"id":"first","id":"second"}]',
    { headers: { 'Content-Type': 'application/json' } },
  );
  await assert.rejects(
    () => ingestSorceryCards({ fetchImpl: duplicateJsonFetch, retrievedAt: RETRIEVED_AT }),
    (error: unknown) => {
      assert.ok(error instanceof SorceryCardIngestionError);
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
    () => ingestSorceryCards({ fetchImpl: oversizedFetch, retrievedAt: RETRIEVED_AT }),
    (error: unknown) => {
      assert.ok(error instanceof SorceryCardIngestionError);
      assert.equal(error.code, 'response_too_large');
      return true;
    },
  );
});

test('rejects invalid options before making a request', async () => {
  let calls = 0;
  const fetchImpl: typeof fetch = async () => {
    calls += 1;
    return new Response('[]', { headers: { 'Content-Type': 'application/json' } });
  };
  await assert.rejects(
    () => ingestSorceryCards({ fetchImpl, retrievedAt: 'not-a-timestamp' }),
    (error: unknown) => {
      assert.ok(error instanceof SorceryCardIngestionError);
      assert.equal(error.code, 'invalid_input');
      assert.equal(error.path, '/retrievedAt');
      return true;
    },
  );
  assert.equal(calls, 0);
});
