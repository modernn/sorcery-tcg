import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { cp, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import { identityHash, sha256 } from '../../src/authority/hash.ts';
import { normalizeCards } from '../../src/authority/normalize-cards.ts';
import {
  AuthorityValidationError,
  type Diagnostic,
  type SourceMetadata,
} from '../../src/authority/schemas.ts';
import { importAuthority } from '../../src/commands/import-authority.ts';

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

test('DATA-02 normalizes the same pinned synthetic card input byte-identically on repeated runs', () => {
  const metadata = sourceMetadata(VALID_BYTES);
  const first = normalizeCards(VALID_BYTES, metadata);
  const second = normalizeCards(VALID_BYTES, metadata);

  assert.deepEqual(second, first);
  assert.equal(canonicalJson(second), canonicalJson(first));
  assert.equal(second.contentHash, first.contentHash);
});

test('DATA-02 reordered object properties preserve normalized canonical card bytes and payload hashes', () => {
  const parsed = JSON.parse(new TextDecoder().decode(VALID_BYTES)) as {
    cards: Array<Record<string, unknown>>;
  };
  const reorderedBytes = bytes({
    cards: [...parsed.cards]
      .reverse()
      .map((card) => Object.fromEntries(Object.entries(card).reverse())),
  });
  const original = normalizeCards(VALID_BYTES, sourceMetadata(VALID_BYTES));
  const reordered = normalizeCards(reorderedBytes, sourceMetadata(reorderedBytes));

  assert.equal(canonicalJson(reordered.identity.payload), canonicalJson(original.identity.payload));
  assert.equal(identityHash(reordered.identity.payload), identityHash(original.identity.payload));
  assert.deepEqual(
    reordered.identity.payload.cards.map(({ stableId }) => stableId),
    original.identity.payload.cards.map(({ stableId }) => stableId),
  );
});

test('DATA-02 separates raw byte identity from normalized semantic artifact identity', () => {
  const formattingBytes = new Uint8Array(VALID_BYTES.length + 1);
  formattingBytes.set(VALID_BYTES);
  formattingBytes[formattingBytes.length - 1] = 0x20;

  const original = normalizeCards(VALID_BYTES, sourceMetadata(VALID_BYTES));
  const reformatted = normalizeCards(formattingBytes, sourceMetadata(formattingBytes));
  assert.notEqual(reformatted.identity.sourceRefs[0]?.byteHash, original.identity.sourceRefs[0]?.byteHash);
  assert.equal(identityHash(reformatted.identity.payload), identityHash(original.identity.payload));
  assert.notEqual(reformatted.contentHash, original.contentHash);

  const semanticInput = JSON.parse(new TextDecoder().decode(VALID_BYTES)) as {
    cards: Array<Record<string, unknown>>;
  };
  const firstCard = semanticInput.cards[0];
  assert.ok(firstCard);
  firstCard.name = 'Synthetic Semantic Change';
  const semanticBytes = bytes(semanticInput);
  const semantic = normalizeCards(semanticBytes, sourceMetadata(semanticBytes));
  assert.notEqual(identityHash(semantic.identity.payload), identityHash(original.identity.payload));
  assert.notEqual(semantic.contentHash, original.contentHash);
});

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

test('DATA-02 rejects changed input-lock roots before normalization', async () => {
  const root = await mkdtemp(join(tmpdir(), 'sorcery-card-root-'));
  const inputRoot = join(root, 'input');
  const outputRoot = join(root, 'authority');
  try {
    await cp(new URL('./fixtures/bundle-input/', import.meta.url), inputRoot, { recursive: true });
    await mkdir(outputRoot);
    await writeFile(join(inputRoot, 'cards.raw.json'), '{not-json');
    let diagnostics: readonly Diagnostic[] = [];
    try {
      await importAuthority({
        inputRoot,
        inputLockPath: 'input-lock.json',
        expectedInputRootHash: `sha256:${'0'.repeat(64)}`,
        outputRoot,
        revisionId: 'card-root-mismatch',
      });
      assert.fail('expected AuthorityValidationError');
    } catch (error: unknown) {
      assert.ok(error instanceof AuthorityValidationError);
      diagnostics = error.diagnostics;
    }
    assert.deepEqual(pathsAndCodes(diagnostics), [
      { path: '/expectedInputRootHash', code: 'unexpected_input_root_hash' },
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
test('DATA-02 performs deterministic normalization with network clock and randomness disabled', () => {
  const originalFetch = globalThis.fetch;
  const originalNow = Date.now;
  const originalRandom = Math.random;
  const forbidden = (): never => {
    throw new Error('non-deterministic dependency accessed');
  };

  globalThis.fetch = forbidden as typeof fetch;
  Date.now = forbidden;
  Math.random = forbidden;
  try {
    assert.doesNotThrow(() => normalizeCards(VALID_BYTES, sourceMetadata(VALID_BYTES)));
  } finally {
    globalThis.fetch = originalFetch;
    Date.now = originalNow;
    Math.random = originalRandom;
  }
});

test('DATA-02 returns a defensive deeply frozen artifact that cannot change after hashing', () => {
  const mutableBytes = new Uint8Array(VALID_BYTES);
  const mutableMetadata = sourceMetadata(mutableBytes);
  const artifact = normalizeCards(mutableBytes, mutableMetadata);
  const canonicalBefore = canonicalJson(artifact);

  mutableBytes[0] = (mutableBytes[0] ?? 0) ^ 1;
  (mutableMetadata as unknown as { retrievedAt: string }).retrievedAt = '1999-01-01T00:00:00Z';
  assert.equal(canonicalJson(artifact), canonicalBefore);

  const firstCard = artifact.identity.payload.cards[0];
  assert.ok(firstCard);
  assert.ok(Object.isFrozen(artifact));
  assert.ok(Object.isFrozen(artifact.identity));
  assert.ok(Object.isFrozen(artifact.identity.sourceRefs));
  assert.ok(Object.isFrozen(artifact.identity.payload));
  assert.ok(Object.isFrozen(artifact.identity.payload.cards));
  assert.ok(Object.isFrozen(firstCard));
  assert.ok(Object.isFrozen(firstCard.elements));
  assert.ok(Object.isFrozen(firstCard.printingSlugs));
  assert.throws(() => {
    (firstCard as unknown as { name: string }).name = 'Tampered';
  }, TypeError);
  assert.equal(canonicalJson(artifact), canonicalBefore);
});

test('DATA-02 reports duplicate JSON object keys with an exact deterministic path', () => {
  const duplicateKeyBytes = new TextEncoder().encode(
    '{"cards":[{"sourceCardId":"duplicate-key","name":"First","name":"Second"}]}',
  );
  assert.deepEqual(
    pathsAndCodes(captureDiagnostics(() => normalizeCards(duplicateKeyBytes, sourceMetadata(duplicateKeyBytes)))),
    [{ path: '/cards/0/name', code: 'duplicate_key' }],
  );
});
