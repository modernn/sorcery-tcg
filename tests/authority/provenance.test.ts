import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { sha256 } from '../../src/authority/hash.ts';
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
  type SourceRecord,
} from '../../src/authority/schemas.ts';
import {
  resolveWithinAuthorityRoot,
  validateAuthorityBundle as validateBundleGraph,
} from '../../src/authority/validate-bundle.ts';

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

async function captureAsyncDiagnostics(run: () => Promise<unknown>): Promise<readonly Diagnostic[]> {
  try {
    await run();
  } catch (error: unknown) {
    if (error instanceof AuthorityValidationError) return error.diagnostics;
    throw error;
  }
  return assert.fail('expected AuthorityValidationError');
}

type ProvenanceFixture = Readonly<{
  bundleStableId: string;
  storedBytes: string;
  storedSource: Record<string, JsonValue>;
  manifestSource: Record<string, JsonValue>;
}>;

async function loadFixture(name: string): Promise<ProvenanceFixture> {
  return JSON.parse(
    await readFile(new URL(`./fixtures/${name}`, import.meta.url), 'utf8'),
  ) as ProvenanceFixture;
}

function sourceRef(source: SourceRecord) {
  return { sourceId: source.sourceId, byteHash: source.byteHash } as const;
}

function artifactRef(artifact: ReturnType<typeof createCanonicalArtifact>) {
  return {
    artifactKind: artifact.identity.artifactKind,
    stableId: artifact.identity.stableId,
    contentHash: artifact.contentHash,
  } as const;
}

function buildFixtureBundle(fixture: ProvenanceFixture) {
  const stored = validateSourceRecord(fixture.storedSource);
  const manifest = validateSourceRecord(fixture.manifestSource);
  const base = createCanonicalArtifact({
    artifactKind: 'rule',
    stableId: 'rule:synthetic-base',
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [sourceRef(stored)],
    payload: { title: 'Synthetic base rule' },
  });
  const derived = createCanonicalArtifact({
    artifactKind: 'rule',
    stableId: 'rule:synthetic-derived',
    schemaVersion: 1,
    parentRefs: [artifactRef(base)],
    sourceRefs: [sourceRef(manifest)],
    payload: { title: 'Synthetic derived rule' },
  });
  return createCanonicalArtifact({
    artifactKind: 'bundle',
    stableId: fixture.bundleStableId,
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [sourceRef(stored), sourceRef(manifest)],
    payload: {
      effectiveDate: '2026-08-20',
      precedencePolicyVersion: 1,
      inputRootHash: null,
      sources: [stored, manifest],
      artifacts: [base, derived],
    },
  });
}

async function materializeFixture(name = 'provenance-valid.json') {
  const fixture = await loadFixture(name);
  const root = await mkdtemp(join(tmpdir(), 'sorcery-authority-'));
  await mkdir(join(root, 'raw'));
  await writeFile(join(root, 'raw', 'rules.json'), fixture.storedBytes);
  const bundle = buildFixtureBundle(fixture);
  await writeFile(join(root, 'bundle.json'), canonicalJson(bundle));
  return {
    root,
    fixture,
    bundle,
    expected: { stableId: bundle.identity.stableId, contentHash: bundle.contentHash },
  };
}

async function writeBundle(root: string, bundle: ReturnType<typeof buildFixtureBundle>): Promise<void> {
  await writeFile(join(root, 'bundle.json'), canonicalJson(bundle));
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

test('DATA-03 rejects missing broken and duplicate parent and source references with sorted diagnostics', async () => {
  const materialized = await materializeFixture();
  try {
    const [base, derived] = materialized.bundle.identity.payload.artifacts;
    assert.ok(base);
    assert.ok(derived);
    const broken = createCanonicalArtifact({
      ...derived.identity,
      parentRefs: [{ ...artifactRef(base), stableId: 'rule:missing-parent' }],
      sourceRefs: [{ sourceId: 'source:missing-source', byteHash: HASH_B }],
    });
    const bundle = createCanonicalArtifact({
      ...materialized.bundle.identity,
      payload: { ...materialized.bundle.identity.payload, artifacts: [base, broken] },
    });
    await writeBundle(materialized.root, bundle);
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', {
          stableId: bundle.identity.stableId,
          contentHash: bundle.contentHash,
        }),
      )).map(({ path, code }) => ({ path, code })),
      [
        {
          path: '/identity/payload/artifacts/1/identity/parentRefs/0/stableId',
          code: 'missing_parent_ref',
        },
        {
          path: '/identity/payload/artifacts/1/identity/sourceRefs/0/sourceId',
          code: 'missing_source_ref',
        },
      ],
    );

    const manifest = materialized.bundle.identity.payload.sources[1]!;
    assert.deepEqual(
      captureDiagnostics(() =>
        validateCanonicalArtifact({
          ...derived,
          identity: {
            ...derived.identity,
            parentRefs: [artifactRef(base), artifactRef(base)],
            sourceRefs: [sourceRef(manifest), sourceRef(manifest)],
          },
        }),
      ).map(({ path, code }) => ({ path, code })),
      [
        { path: '/identity/parentRefs/1/stableId', code: 'duplicate_parent_ref' },
        { path: '/identity/sourceRefs/1/sourceId', code: 'duplicate_source_ref' },
      ],
    );
  } finally {
    await rm(materialized.root, { recursive: true, force: true });
  }
});

test('DATA-03 rejects reference cycles before returning a partial graph', async () => {
  const materialized = await materializeFixture();
  try {
    const cycle = JSON.parse(
      await readFile(new URL('./fixtures/provenance-cycle.json', import.meta.url), 'utf8'),
    ) as { first: string; second: string };
    const stored = materialized.bundle.identity.payload.sources[0]!;
    const first = createCanonicalArtifact({
      artifactKind: 'rule',
      stableId: cycle.first,
      schemaVersion: 1,
      parentRefs: [{ artifactKind: 'rule', stableId: cycle.second, contentHash: HASH_A }],
      sourceRefs: [sourceRef(stored)],
      payload: { title: 'Cycle first' },
    });
    const second = createCanonicalArtifact({
      artifactKind: 'rule',
      stableId: cycle.second,
      schemaVersion: 1,
      parentRefs: [{ artifactKind: 'rule', stableId: cycle.first, contentHash: HASH_B }],
      sourceRefs: [sourceRef(stored)],
      payload: { title: 'Cycle second' },
    });
    const bundle = createCanonicalArtifact({
      ...materialized.bundle.identity,
      payload: { ...materialized.bundle.identity.payload, artifacts: [first, second] },
    });
    await writeBundle(materialized.root, bundle);
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', {
          stableId: bundle.identity.stableId,
          contentHash: bundle.contentHash,
        }),
      )).map(({ path, code }) => ({ path, code })),
      [
        {
          path: '/identity/payload/artifacts/0/identity/parentRefs/0/contentHash',
          code: 'parent_ref_hash_mismatch',
        },
        {
          path: '/identity/payload/artifacts/1/identity/parentRefs/0',
          code: 'reference_cycle',
        },
        {
          path: '/identity/payload/artifacts/1/identity/parentRefs/0/contentHash',
          code: 'parent_ref_hash_mismatch',
        },
      ],
    );
  } finally {
    await rm(materialized.root, { recursive: true, force: true });
  }
});

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

test('DATA-03 rejects bundle traversal absolute paths missing files and stored-source junction escape', async () => {
  const materialized = await materializeFixture();
  const outside = await mkdtemp(join(tmpdir(), 'sorcery-authority-outside-'));
  try {
    for (const candidate of ['../bundle.json', join(materialized.root, 'bundle.json')]) {
      assert.deepEqual(
        (await captureAsyncDiagnostics(() => resolveWithinAuthorityRoot(materialized.root, candidate))).map(
          ({ path, code }) => ({ path, code }),
        ),
        [{ path: '', code: 'path_escape' }],
      );
    }
    await rm(join(materialized.root, 'raw'), { recursive: true, force: true });
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', materialized.expected),
      )).map(({ path, code }) => ({ path, code })),
      [{ path: '/identity/payload/sources/0/relativePath', code: 'path_not_found' }],
    );

    await writeFile(join(outside, 'rules.json'), materialized.fixture.storedBytes);
    await symlink(outside, join(materialized.root, 'raw'), 'junction');
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', materialized.expected),
      )).map(({ path, code }) => ({ path, code })),
      [{ path: '/identity/payload/sources/0/relativePath', code: 'path_escape' }],
    );
  } finally {
    await rm(materialized.root, { recursive: true, force: true });
    await rm(outside, { recursive: true, force: true });
  }
});

test('DATA-03 rehashes stored bytes and reports manifest-only bindings without reading absent bytes', async () => {
  const materialized = await materializeFixture();
  try {
    const validated = await validateBundleGraph(materialized.root, 'bundle.json', materialized.expected);
    assert.deepEqual(validated.storedBytesRehashed, [sourceRef(materialized.bundle.identity.payload.sources[0]!)]);
    assert.deepEqual(validated.manifestBindingsVerified, [
      sourceRef(materialized.bundle.identity.payload.sources[1]!),
    ]);

    const tampered = await loadFixture('provenance-tampered.json');
    await writeFile(join(materialized.root, 'raw', 'rules.json'), tampered.storedBytes);
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', materialized.expected),
      )).map(({ path, code }) => ({ path, code })),
      [{ path: '/identity/payload/sources/0/byteHash', code: 'stored_byte_hash_mismatch' }],
    );
  } finally {
    await rm(materialized.root, { recursive: true, force: true });
  }
});

test('DATA-03 rejects manifest locator procedure byte hash SourceRef artifact and expected-root tampering', async () => {
  const materialized = await materializeFixture();
  try {
    const [stored, manifest] = materialized.bundle.identity.payload.sources;
    const [base, derived] = materialized.bundle.identity.payload.artifacts;
    assert.ok(stored);
    assert.ok(manifest);
    assert.ok(base);
    assert.ok(derived);

    for (const changedManifest of [
      { ...manifest, durableLocator: 'urn:sha256:' + 'c'.repeat(64) },
      { ...manifest, acquisitionProcedureHash: HASH_A },
      { ...manifest, byteHash: HASH_A },
    ]) {
      const tampered = {
        ...materialized.bundle,
        identity: {
          ...materialized.bundle.identity,
          payload: { ...materialized.bundle.identity.payload, sources: [stored, changedManifest] },
        },
      };
      await writeFile(join(materialized.root, 'bundle.json'), canonicalJson(tampered));
      assert.deepEqual(
        (await captureAsyncDiagnostics(() =>
          validateBundleGraph(materialized.root, 'bundle.json', materialized.expected),
        )).map(({ path, code }) => ({ path, code })),
        [{ path: '/contentHash', code: 'content_hash_mismatch' }],
      );
    }

    const badBinding = createCanonicalArtifact({
      ...derived.identity,
      sourceRefs: [{ sourceId: manifest.sourceId, byteHash: HASH_A }],
    });
    const bindingBundle = createCanonicalArtifact({
      ...materialized.bundle.identity,
      payload: { ...materialized.bundle.identity.payload, artifacts: [base, badBinding] },
    });
    await writeBundle(materialized.root, bindingBundle);
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', {
          stableId: bindingBundle.identity.stableId,
          contentHash: bindingBundle.contentHash,
        }),
      )).map(({ path, code }) => ({ path, code })),
      [
        {
          path: '/identity/payload/artifacts/1/identity/sourceRefs/0/byteHash',
          code: 'source_ref_hash_mismatch',
        },
      ],
    );

    const artifactTamper = { ...derived, identity: { ...derived.identity, payload: { title: 'Tampered' } } };
    const artifactBundle = createCanonicalArtifact({
      ...materialized.bundle.identity,
      payload: { ...materialized.bundle.identity.payload, artifacts: [base, artifactTamper] },
    });
    await writeBundle(materialized.root, artifactBundle);
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', {
          stableId: artifactBundle.identity.stableId,
          contentHash: artifactBundle.contentHash,
        }),
      )).map(({ path, code }) => ({ path, code })),
      [
        {
          path: '/identity/payload/artifacts/1/contentHash',
          code: 'content_hash_mismatch',
        },
      ],
    );

    await writeBundle(materialized.root, materialized.bundle);
    assert.deepEqual(
      (await captureAsyncDiagnostics(() =>
        validateBundleGraph(materialized.root, 'bundle.json', {
          stableId: 'bundle:wrong',
          contentHash: sha256(new TextEncoder().encode('wrong')),
        }),
      )).map(({ path, code }) => ({ path, code })),
      [
        { path: '/expected/contentHash', code: 'unexpected_content_hash' },
        { path: '/expected/stableId', code: 'unexpected_stable_id' },
      ],
    );
  } finally {
    await rm(materialized.root, { recursive: true, force: true });
  }
});

test('DATA-03 caps recursive graph diagnostics before rejecting the bundle', async () => {
  const materialized = await materializeFixture();
  try {
    const stored = materialized.bundle.identity.payload.sources[0]!;
    const artifacts = Array.from({ length: 120 }, (_, index) =>
      createCanonicalArtifact({
        artifactKind: 'rule',
        stableId: `rule:missing-source-${index}`,
        schemaVersion: 1,
        parentRefs: [],
        sourceRefs: [{ sourceId: `source:missing-${index}`, byteHash: stored.byteHash }],
        payload: { index },
      }),
    );
    const bundle = createCanonicalArtifact({
      ...materialized.bundle.identity,
      payload: { ...materialized.bundle.identity.payload, artifacts },
    });
    await writeBundle(materialized.root, bundle);
    const diagnostics = await captureAsyncDiagnostics(() =>
      validateBundleGraph(materialized.root, 'bundle.json', {
        stableId: bundle.identity.stableId,
        contentHash: bundle.contentHash,
      }),
    );
    assert.equal(diagnostics.length, 100);
    assert.ok(diagnostics.every(({ code }) => code === 'missing_source_ref'));
    assert.deepEqual(diagnostics, sortDiagnostics(diagnostics));
  } finally {
    await rm(materialized.root, { recursive: true, force: true });
  }
});

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
