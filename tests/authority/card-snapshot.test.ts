import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { sha256 } from '../../src/authority/hash.ts';
import { normalizeCards } from '../../src/authority/normalize-cards.ts';
import {
  AuthorityValidationError,
  type Diagnostic,
  type SourceMetadata,
} from '../../src/authority/schemas.ts';

const VALID_BYTES = readFileSync(new URL('./fixtures/cards-valid.json', import.meta.url));
const MALFORMED_BYTES = readFileSync(new URL('./fixtures/cards-malformed.json', import.meta.url));
const DUPLICATE_CASES = JSON.parse(
  readFileSync(new URL('./fixtures/cards-duplicates.json', import.meta.url), 'utf8'),
) as Readonly<Record<'stableIdCollision' | 'printingSlugCollision', unknown>>;

function bytes(value: unknown): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(value));
}

function sourceMetadata(rawBytes: Uint8Array, overrides: Record<string, unknown> = {}): SourceMetadata {
  const byteHash = sha256(rawBytes);
  return {
    sourceId: 'source:synthetic-cards-2026',
    url: 'https://api.sorcerytcg.com/api/cards',
    authorityClass: 'official',
    retrievedAt: '2026-08-20T00:00:00Z',
    effectiveDate: '2026-08-20',
    mediaType: 'application/json',
    byteHash,
    derivation: { method: 'verbatim', parentByteHashes: [], notes: 'Synthetic test records' },
    licenseStatus: 'permission-required',
    storageMode: 'manifest-only',
    storagePolicy: 'prohibited',
    durableLocator: 'urn:' + byteHash,
    acquisitionProcedureHash: null,
    ...overrides,
  } as SourceMetadata;
}

function captureDiagnostics(run: () => unknown): readonly Diagnostic[] {
  try {
    run();
  } catch (error: unknown) {
    if (error instanceof AuthorityValidationError) return error.diagnostics;
    throw error;
  }
  return assert.fail('expected AuthorityValidationError');
}

function pathsAndCodes(diagnostics: readonly Diagnostic[]): readonly Pick<Diagnostic, 'path' | 'code'>[] {
  return diagnostics.map(({ path, code }) => ({ path, code }));
}

test.todo('DATA-02 normalizes the same pinned synthetic card input byte-identically on repeated runs');
test.todo('DATA-02 reordered object properties produce identical canonical card bytes and hashes');

test('DATA-02 valid cards retain official source identifiers printing slugs and deterministic project stable IDs', () => {
  const artifact = normalizeCards(VALID_BYTES, sourceMetadata(VALID_BYTES));

  assert.equal(artifact.identity.artifactKind, 'card-snapshot');
  assert.equal(artifact.identity.schemaVersion, 1);
  assert.deepEqual(artifact.identity.parentRefs, []);
  assert.deepEqual(artifact.identity.sourceRefs, [
    { sourceId: 'source:synthetic-cards-2026', byteHash: sha256(VALID_BYTES) },
  ]);
  assert.equal(artifact.identity.payload.cards.length, 2);
  assert.deepEqual(
    artifact.identity.payload.cards.map(({ officialSourceId, printingSlugs }) => ({
      officialSourceId,
      printingSlugs,
    })),
    [
      { officialSourceId: 'synthetic-card-air-001', printingSlugs: ['synthetic-air-scout-alpha'] },
      { officialSourceId: 'synthetic-card-fire-001', printingSlugs: ['synthetic-fire-keeper-alpha'] },
    ],
  );
  assert.ok(artifact.identity.payload.cards.every(({ stableId }) => /^card:[0-9a-f]{64}$/.test(stableId)));
  assert.notEqual(artifact.identity.payload.cards[0]?.stableId, artifact.identity.payload.cards[1]?.stableId);
  const displayVariant = JSON.parse(new TextDecoder().decode(VALID_BYTES)) as {
    cards: Array<Record<string, unknown>>;
  };
  displayVariant.cards.reverse();
  displayVariant.cards.forEach((card, index) => {
    card.name = 'Display Variant ' + index;
    card.rulesText = 'Display-only change';
  });
  const displayBytes = bytes(displayVariant);
  const displayArtifact = normalizeCards(displayBytes, sourceMetadata(displayBytes));
  assert.deepEqual(
    displayArtifact.identity.payload.cards.map(({ officialSourceId, stableId }) => ({
      officialSourceId,
      stableId,
    })),
    artifact.identity.payload.cards.map(({ officialSourceId, stableId }) => ({
      officialSourceId,
      stableId,
    })),
  );
});

test('DATA-02 rejects malformed cards and unknown fields without repair at exact paths', () => {
  assert.deepEqual(
    pathsAndCodes(captureDiagnostics(() => normalizeCards(MALFORMED_BYTES, sourceMetadata(MALFORMED_BYTES)))),
    [
      { path: '/cards/0/unexpected', code: 'unrecognized_key' },
      { path: '/cards/1/name', code: 'invalid_type' },
    ],
  );
});

test('DATA-02 rejects duplicate derived stable IDs and printing slugs', () => {
  const stableIdBytes = bytes(DUPLICATE_CASES.stableIdCollision);
  assert.deepEqual(
    pathsAndCodes(captureDiagnostics(() => normalizeCards(stableIdBytes, sourceMetadata(stableIdBytes)))),
    [{ path: '/cards/1/stableId', code: 'duplicate_stable_id' }],
  );

  const printingSlugBytes = bytes(DUPLICATE_CASES.printingSlugCollision);
  assert.deepEqual(
    pathsAndCodes(captureDiagnostics(() => normalizeCards(printingSlugBytes, sourceMetadata(printingSlugBytes)))),
    [{ path: '/cards/1/printingSlugs/0', code: 'duplicate_printing_slug' }],
  );
});

test('DATA-02 rejects invalid source dates hashes and raw-byte hash mismatches', () => {
  assert.deepEqual(
    pathsAndCodes(
      captureDiagnostics(() =>
        normalizeCards(
          VALID_BYTES,
          sourceMetadata(VALID_BYTES, {
            retrievedAt: 'not-a-date',
            effectiveDate: '2026-13-40',
            byteHash: 'sha256:not-a-hash',
          }),
        ),
      ),
    ),
    [
      { path: '/byteHash', code: 'invalid_format' },
      { path: '/effectiveDate', code: 'invalid_format' },
      { path: '/retrievedAt', code: 'invalid_format' },
    ],
  );

  const changedBytes = new Uint8Array(VALID_BYTES);
  const changedIndex = changedBytes.length - 2;
  changedBytes[changedIndex] = (changedBytes[changedIndex] ?? 0) ^ 1;
  assert.deepEqual(
    pathsAndCodes(captureDiagnostics(() => normalizeCards(changedBytes, sourceMetadata(VALID_BYTES)))),
    [{ path: '/byteHash', code: 'byte_hash_mismatch' }],
  );
});

test('DATA-02 valid card count preserves exact cardinality and rejects derived ID collisions atomically', () => {
  const artifact = normalizeCards(VALID_BYTES, sourceMetadata(VALID_BYTES));
  assert.equal(artifact.identity.payload.cards.length, 2);

  const collisionBytes = bytes(DUPLICATE_CASES.stableIdCollision);
  assert.throws(
    () => normalizeCards(collisionBytes, sourceMetadata(collisionBytes)),
    AuthorityValidationError,
  );
});

test.todo('DATA-02 rejects changed input-lock roots before normalization');
test.todo('DATA-02 performs deterministic normalization with networking disabled');

test('DATA-02 reports duplicate JSON object keys with an exact deterministic path', () => {
  const duplicateKeyBytes = new TextEncoder().encode(
    '{"cards":[{"sourceCardId":"duplicate-key","name":"First","name":"Second"}]}',
  );
  assert.deepEqual(
    pathsAndCodes(captureDiagnostics(() => normalizeCards(duplicateKeyBytes, sourceMetadata(duplicateKeyBytes)))),
    [{ path: '/cards/0/name', code: 'duplicate_key' }],
  );
});
