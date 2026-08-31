import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { identityHash } from '../../src/authority/hash.ts';
import { runImportTopDeckCandidatesCommand } from '../../src/commands/import-topdeck-candidates.ts';
import {
  createTopDeckCandidateSnapshot,
  publishTopDeckCandidateSnapshot,
} from '../../src/ingestion/topdeck-candidates.ts';
import { ingestTopDeckTournaments } from '../../src/ingestion/topdeck.ts';

const RETRIEVED_AT = '2026-08-31T18:00:00.000Z';

function response(status = 200): Response {
  return new Response(JSON.stringify([{
    TID: 'event-1',
    eventData: { city: 'Private City', country: 'XX' },
    format: '',
    game: 'Sorcery: Contested Realm',
    standings: [
      {
        deckObj: { Avatar: { Pathfinder: 1 } },
        decklist: '~~Avatar~~\n1 Pathfinder\nunknown row',
        id: 'secret-player-id',
        name: 'Secret Player Name',
        standing: 1,
      },
      {
        decklist: null,
        id: 'second-secret-id',
        name: 'Second Secret Name',
        standing: 2,
      },
    ],
    startDate: 1_788_115_200,
    swissNum: 4,
    topCut: 0,
    tournamentName: 'Location-bearing tournament name',
  }]), { headers: { 'Content-Type': 'application/json' }, status });
}

test('creates a canonical candidate snapshot without participant identity or location data', async () => {
  const ingestion = await ingestTopDeckTournaments({
    apiKey: 'synthetic-key',
    fetchImpl: async () => response(),
    lastDays: 30,
    retrievedAt: RETRIEVED_AT,
  });
  const snapshot = createTopDeckCandidateSnapshot(ingestion);
  const { contentHash, ...body } = snapshot;

  assert.equal(contentHash, identityHash(body as unknown as JsonValue));
  assert.deepEqual(snapshot.tournaments, [{
    candidates: [{
      deck: {
        opaqueStructuredDeck: { Avatar: { Pathfinder: 1 } },
        rows: [
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
            lineNumber: 3,
            mappingStatus: 'unresolved',
            parseStatus: 'unrecognized',
            quantity: null,
            raw: 'unknown row',
            section: 'Avatar',
            sourceCardName: null,
          },
        ],
        sourceUrl: null,
      },
      resultEvidence: { placement: 1 },
    }],
    participantCount: 2,
    startedAt: '2026-08-30T18:40:00.000Z',
    tournamentId: 'event-1',
  }]);
  assert.equal(snapshot.source.apiVersion, 'v2');
  assert.equal(snapshot.source.attribution.text, 'Data provided by TopDeck.gg');
  assert.equal(snapshot.source.attribution.url, 'https://topdeck.gg');
  assert.equal(snapshot.source.retrievedAt, RETRIEVED_AT);
  const serialized = canonicalJson(snapshot as unknown as JsonValue);
  for (const privateValue of [
    'Secret Player Name',
    'secret-player-id',
    'Private City',
    'Location-bearing tournament name',
  ]) assert.equal(serialized.includes(privateValue), false);
});

test('publishes atomically under the ignored authority boundary and never overwrites', async () => {
  const repositoryRoot = await mkdtemp(join(tmpdir(), 'sorcery-topdeck-'));
  try {
    const ingestion = await ingestTopDeckTournaments({
      apiKey: 'synthetic-key',
      fetchImpl: async () => response(),
      lastDays: 30,
      retrievedAt: RETRIEVED_AT,
    });
    const snapshot = createTopDeckCandidateSnapshot(ingestion);
    const path = await publishTopDeckCandidateSnapshot(snapshot, repositoryRoot);
    assert.ok(path.startsWith(join(repositoryRoot, '.local', 'authority', 'topdeck-candidates')));
    assert.equal(await readFile(path, 'utf8'), canonicalJson(snapshot as unknown as JsonValue));
    await assert.rejects(() => publishTopDeckCandidateSnapshot(snapshot, repositoryRoot), { code: 'EEXIST' });
    assert.equal(await readFile(path, 'utf8'), canonicalJson(snapshot as unknown as JsonValue));
  } finally {
    await rm(repositoryRoot, { force: true, recursive: true });
  }
});

test('manual command requires the API key and makes one request without retrying HTTP failures', async () => {
  let calls = 0;
  const missingErrors: string[] = [];
  assert.equal(await runImportTopDeckCandidatesCommand([], {
    apiKey: '',
    fetchImpl: async () => {
      calls += 1;
      return response();
    },
  }, { stderr: (line) => missingErrors.push(line), stdout: () => undefined }), 1);
  assert.equal(calls, 0);
  assert.deepEqual(missingErrors, ['TOPDECK_API_KEY is required for the manual TopDeck import.']);

  for (const [status, expected] of [
    [401, 'TopDeck authentication failed (HTTP 401); request was not retried.'],
    [403, 'TopDeck access was forbidden (HTTP 403); request was not retried.'],
    [429, 'TopDeck rate-limited the request (HTTP 429); request was not retried.'],
  ] as const) {
    const errors: string[] = [];
    calls = 0;
    assert.equal(await runImportTopDeckCandidatesCommand([], {
      apiKey: 'synthetic-key',
      fetchImpl: async () => {
        calls += 1;
        return response(status);
      },
      now: () => new Date(RETRIEVED_AT),
    }, { stderr: (line) => errors.push(line), stdout: () => undefined }), 1);
    assert.equal(calls, 1);
    assert.deepEqual(errors, [expected]);
  }
});

test('manual command writes one successful private snapshot', async () => {
  const repositoryRoot = await mkdtemp(join(tmpdir(), 'sorcery-topdeck-command-'));
  const output: string[] = [];
  let calls = 0;
  try {
    assert.equal(await runImportTopDeckCandidatesCommand(['--last-days', '7'], {
      apiKey: 'synthetic-key',
      fetchImpl: async () => {
        calls += 1;
        return response();
      },
      now: () => new Date(RETRIEVED_AT),
      repositoryRoot,
    }, { stderr: () => undefined, stdout: (line) => output.push(line) }), 0);
    assert.equal(calls, 1);
    const receipt = JSON.parse(output[0]!) as { outputPath: string; status: string };
    assert.equal(receipt.status, 'created');
    assert.match(receipt.outputPath, /^\.local\/authority\/topdeck-candidates\/topdeck-candidates-[0-9a-f]{64}\.json$/u);
    await readFile(join(repositoryRoot, ...receipt.outputPath.split('/')), 'utf8');
  } finally {
    await rm(repositoryRoot, { force: true, recursive: true });
  }
});
