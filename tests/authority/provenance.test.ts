import assert from 'node:assert/strict';
import test from 'node:test';

import {
  AuthorityValidationError,
  createCanonicalArtifact,
  isNormativeSource,
  parseAuthorityJson,
  sortDiagnostics,
  validateAuthorityBundle,
  validateCanonicalArtifact,
  validateFormatArtifact,
  validateNormalizedCard,
  validateRawCard,
  validateSourceRecord,
  type AuthorityJsonLimits,
  type Diagnostic,
  type Hash,
  type IdentityDocument,
} from '../../src/authority/schemas.ts';
import type { JsonValue } from '../../src/authority/canonical-json.ts';

const HASH_A = ('sha256:' + 'a'.repeat(64)) as Hash;
const HASH_B = ('sha256:' + 'b'.repeat(64)) as Hash;

function storedSource(overrides: Record<string, JsonValue> = {}): Record<string, JsonValue> {
  return {
    sourceId: 'source:official-rules-2025',
    url: 'https://sorcerytcg.com/rules/2025',
    authorityClass: 'official',
    retrievedAt: '2026-08-20T00:00:00Z',
    effectiveDate: '2025-12-19',
    mediaType: 'application/json',
    byteHash: HASH_A,
    derivation: { method: 'verbatim', parentByteHashes: [], notes: null },
    licenseStatus: 'approved',
    storageMode: 'stored',
    storagePolicy: 'permitted',
    relativePath: 'raw/rules.json',
    ...overrides,
  };
}

function manifestSource(overrides: Record<string, JsonValue> = {}): Record<string, JsonValue> {
  return {
    sourceId: 'source:community-example',
    url: 'https://example.org/sorcery/example',
    authorityClass: 'community-provenance',
    retrievedAt: '2026-08-20T00:00:00Z',
    effectiveDate: null,
    mediaType: 'text/html',
    byteHash: HASH_B,
    derivation: {
      method: 'manual-transcription',
      parentByteHashes: [],
      notes: 'Synthetic test record',
    },
    licenseStatus: 'reference-only',
    storageMode: 'manifest-only',
    storagePolicy: 'manifest-only',
    durableLocator: 'https://archive.example.org/revisions/example-1',
    acquisitionProcedureHash: null,
    ...overrides,
  };
}

function identity(payload: JsonValue = { title: 'Synthetic rule' }): IdentityDocument<JsonValue> {
  return {
    artifactKind: 'rule',
    stableId: 'rule:synthetic-one',
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [{ sourceId: 'source:official-rules-2025', byteHash: HASH_A }],
    payload,
  };
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

test('DATA-03 accepts a strict artifact envelope with stable ID schema version provenance and content hash', () => {
  const artifact = createCanonicalArtifact(identity());
  assert.deepEqual(validateCanonicalArtifact(artifact), artifact);
  assert.equal(artifact.identity.stableId, 'rule:synthetic-one');
  assert.equal(artifact.identity.schemaVersion, 1);
  assert.match(artifact.contentHash, /^sha256:[0-9a-f]{64}$/);
});

test('DATA-03 accepts stored sources only with their strict stored-byte shape', () => {
  const source = validateSourceRecord(storedSource());
  assert.equal(source.storageMode, 'stored');
  assert.equal(source.relativePath, 'raw/rules.json');
  assert.deepEqual(
    captureDiagnostics(() => validateSourceRecord(storedSource({ durableLocator: 'https://example.org/a' }))).map(
      ({ path, code }) => ({ path, code }),
    ),
    [{ path: '/durableLocator', code: 'unrecognized_key' }],
  );
});

test('DATA-03 accepts manifest-only sources only with locator or procedure fingerprint and expected byte hash', () => {
  const source = validateSourceRecord(manifestSource());
  assert.equal(source.storageMode, 'manifest-only');
  assert.equal(source.byteHash, HASH_B);
  assert.equal('relativePath' in source, false);
  assert.deepEqual(
    captureDiagnostics(() =>
      validateSourceRecord(manifestSource({ durableLocator: null, acquisitionProcedureHash: null })),
    ).map(({ path, code }) => ({ path, code })),
    [{ path: '/durableLocator', code: 'custom' }],
  );
});

test('DATA-03 accepts reviewed community and external sources as non-normative provenance', () => {
  const community = validateSourceRecord(manifestSource());
  const external = validateSourceRecord(
    manifestSource({
      sourceId: 'source:external-example',
      authorityClass: 'external-reference',
      url: 'https://reference.example/sorcery',
    }),
  );
  assert.equal(isNormativeSource(community), false);
  assert.equal(isNormativeSource(external), false);
});

test('DATA-03 prevents non-official authority classes from winning precedence', () => {
  assert.equal(isNormativeSource(validateSourceRecord(storedSource())), true);
  assert.equal(isNormativeSource(validateSourceRecord(manifestSource())), false);
  assert.deepEqual(
    captureDiagnostics(() => validateSourceRecord(storedSource({ url: 'https://example.org/not-official' }))).map(
      ({ path, code }) => ({ path, code }),
    ),
    [{ path: '/url', code: 'custom' }],
  );
});

test('DATA-03 rejects unknown and missing envelope fields with exact JSON Pointer paths', () => {
  const artifact = createCanonicalArtifact(identity());
  const withUnknown = { ...artifact, identity: { ...artifact.identity, 'bad/key~': true } };
  assert.deepEqual(
    captureDiagnostics(() => validateCanonicalArtifact(withUnknown)).map(({ path, code }) => ({ path, code })),
    [{ path: '/identity/bad~1key~0', code: 'unrecognized_key' }],
  );

  const missingStableId = { ...artifact.identity } as Record<string, JsonValue>;
  delete missingStableId.stableId;
  assert.deepEqual(
    captureDiagnostics(() => validateCanonicalArtifact({ ...artifact, identity: missingStableId })).map(
      ({ path, code }) => ({ path, code }),
    ),
    [{ path: '/identity/stableId', code: 'invalid_type' }],
  );
});

test('DATA-03 rejects unsupported schema versions and invalid dates or hashes', () => {
  const artifact = createCanonicalArtifact(identity());
  assert.deepEqual(
    captureDiagnostics(() =>
      validateCanonicalArtifact({ ...artifact, identity: { ...artifact.identity, schemaVersion: 2 } }),
    ).map(({ path, code }) => ({ path, code })),
    [{ path: '/identity/schemaVersion', code: 'invalid_value' }],
  );

  assert.deepEqual(
    captureDiagnostics(() =>
      validateSourceRecord(
        storedSource({ retrievedAt: 'today', effectiveDate: '2026-02-30', byteHash: 'sha256:ABC' }),
      ),
    )
      .map(({ path, code }) => ({ path, code }))
      .sort((left, right) => left.path.localeCompare(right.path)),
    [
      { path: '/byteHash', code: 'invalid_format' },
      { path: '/effectiveDate', code: 'invalid_format' },
      { path: '/retrievedAt', code: 'invalid_format' },
    ],
  );
});

test('DATA-03 rejects duplicate stable IDs and source IDs in authority bundles', () => {
  const rule = createCanonicalArtifact(identity());
  const bundle = createCanonicalArtifact({
    artifactKind: 'bundle',
    stableId: 'bundle:synthetic-one',
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [{ sourceId: 'source:official-rules-2025', byteHash: HASH_A }],
    payload: {
      effectiveDate: '2026-08-20',
      precedencePolicyVersion: 1,
      inputRootHash: null,
      sources: [storedSource(), storedSource()],
      artifacts: [rule, rule],
    },
  });
  assert.deepEqual(
    captureDiagnostics(() => validateAuthorityBundle(bundle)).map(({ path, code }) => ({ path, code })),
    [
      { path: '/identity/payload/artifacts/1/identity/stableId', code: 'duplicate_stable_id' },
      { path: '/identity/payload/sources/1/sourceId', code: 'duplicate_source_id' },
    ],
  );
});

test('DATA-03 rejects artifact tampering and recomputed-hash mismatch', () => {
  const artifact = createCanonicalArtifact(identity());
  assert.deepEqual(
    captureDiagnostics(() =>
      validateCanonicalArtifact({
        ...artifact,
        identity: { ...artifact.identity, payload: { title: 'Tampered' } },
      }),
    ).map(({ path, code }) => ({ path, code })),
    [{ path: '/contentHash', code: 'content_hash_mismatch' }],
  );
  assert.deepEqual(
    captureDiagnostics(() => validateCanonicalArtifact({ ...artifact, contentHash: HASH_B })).map(
      ({ path, code }) => ({ path, code }),
    ),
    [{ path: '/contentHash', code: 'content_hash_mismatch' }],
  );
});

test.todo('DATA-03 rejects missing or broken parent and source references');
test.todo('DATA-03 rejects reference cycles');

test('DATA-03 rejects relative traversal and absolute stored-source paths', () => {
  for (const relativePath of ['../outside.json', '/absolute.json', 'C:\\absolute.json', 'raw\\rules.json']) {
    assert.deepEqual(
      captureDiagnostics(() => validateSourceRecord(storedSource({ relativePath }))).map(({ path, code }) => ({
        path,
        code,
      })),
      [{ path: '/relativePath', code: 'custom' }],
    );
  }
});

test.todo('DATA-03 rejects stored-source symlink escape');
test.todo('DATA-03 rehashes stored source bytes during offline validation');
test.todo('DATA-03 rejects manifest-only locator procedure hash byte hash and SourceRef tampering');

test('DATA-03 bounds input bytes records nesting depth and diagnostic count', () => {
  const limits: AuthorityJsonLimits = { maxBytes: 256, maxDepth: 3, maxRecords: 5, maxDiagnostics: 2 };
  const encoder = new TextEncoder();
  for (const input of ['"' + 'a'.repeat(256) + '"', '[[[[null]]]]', '[1,2,3,4,5,6]']) {
    const found = captureDiagnostics(() => parseAuthorityJson(encoder.encode(input), limits));
    assert.ok(found.length <= limits.maxDiagnostics);
  }
  assert.deepEqual(
    captureDiagnostics(() => parseAuthorityJson(encoder.encode('{"safe":1,"safe":2}'), limits)).map(
      ({ path, code }) => ({ path, code }),
    ),
    [{ path: '/safe', code: 'duplicate_key' }],
  );
});

test('DATA-03 sorts diagnostics deterministically by JSON Pointer path code and message', () => {
  const input: readonly Diagnostic[] = [
    { path: '/z', code: 'b', message: 'a' },
    { path: '/a', code: 'b', message: 'z' },
    { path: '/a', code: 'a', message: 'z' },
    { path: '/a', code: 'a', message: 'a' },
  ];
  assert.deepEqual(sortDiagnostics(input), [
    { path: '/a', code: 'a', message: 'a' },
    { path: '/a', code: 'a', message: 'z' },
    { path: '/a', code: 'b', message: 'z' },
    { path: '/z', code: 'b', message: 'a' },
  ]);
});

test('DATA-03 schema exports cover strict raw cards normalized cards formats sources artifacts and bundles', () => {
  assert.equal(
    validateRawCard({
      sourceCardId: 'official-card-1',
      name: 'Synthetic Adept',
      cardType: 'minion',
      elements: ['earth'],
      rarity: 'ordinary',
      manaCost: 1,
      attack: 1,
      defense: 1,
      rulesText: '',
      printingSlugs: ['synthetic-adept-alpha'],
      releasedAt: '2025-12-19',
    }).sourceCardId,
    'official-card-1',
  );

  assert.equal(
    validateNormalizedCard({
      stableId: 'card:synthetic-adept',
      officialSourceId: 'official-card-1',
      name: 'Synthetic Adept',
      cardType: 'minion',
      elements: ['earth'],
      rarity: 'ordinary',
      manaCost: 1,
      attack: 1,
      defense: 1,
      rulesText: '',
      printingSlugs: ['synthetic-adept-alpha'],
    }).stableId,
    'card:synthetic-adept',
  );

  const format = createCanonicalArtifact({
    artifactKind: 'format',
    stableId: 'format:constructed-2025',
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [{ sourceId: 'source:official-rules-2025', byteHash: HASH_A }],
    payload: {
      name: 'Constructed',
      effectiveDate: '2025-12-19',
      scope: null,
      parentFormatStableId: null,
      avatarCount: 1,
      spellbookMinimum: 60,
      atlasMinimum: 30,
      copyLimits: { ordinary: 4, exceptional: 3, elite: 2, unique: 1 },
    },
  });
  assert.equal(validateFormatArtifact(format).identity.artifactKind, 'format');

  const bundle = createCanonicalArtifact({
    artifactKind: 'bundle',
    stableId: 'bundle:synthetic-valid',
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [{ sourceId: 'source:official-rules-2025', byteHash: HASH_A }],
    payload: {
      effectiveDate: '2026-08-20',
      precedencePolicyVersion: 1,
      inputRootHash: null,
      sources: [storedSource()],
      artifacts: [createCanonicalArtifact(identity())],
    },
  });
  assert.equal(validateAuthorityBundle(bundle).identity.artifactKind, 'bundle');
});
