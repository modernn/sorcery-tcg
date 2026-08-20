import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import { sha256 } from '../../src/authority/hash.ts';
import {
  AuthorityValidationError,
  createCanonicalArtifact,
  validateSourceRecord,
  type Diagnostic,
  type SourceRecord,
  type SourceRef,
} from '../../src/authority/schemas.ts';
import {
  resolveAuthorityPrecedence,
  validateAuthorityBundle,
  type AuthorityPrecedenceRecord,
} from '../../src/authority/validate-bundle.ts';

const encoder = new TextEncoder();

function ref(source: SourceRecord): SourceRef {
  return { sourceId: source.sourceId, byteHash: source.byteHash };
}

function manifestSource(
  sourceId: string,
  overrides: Partial<SourceRecord> = {},
): SourceRecord {
  return validateSourceRecord({
    sourceId,
    url: `https://curiosa.io/codex/${sourceId.slice('source:'.length)}`,
    authorityClass: 'official',
    retrievedAt: '2026-08-20T00:00:00Z',
    effectiveDate: '2026-01-01',
    mediaType: 'text/html',
    byteHash: sha256(encoder.encode(sourceId)),
    derivation: { method: 'verbatim', parentByteHashes: [], notes: 'Synthetic fixture' },
    licenseStatus: 'reference-only',
    storageMode: 'manifest-only',
    storagePolicy: 'manifest-only',
    durableLocator: `urn:${sha256(encoder.encode(sourceId))}`,
    acquisitionProcedureHash: null,
    ...overrides,
  });
}

function record(
  source: SourceRecord,
  authorityKind: AuthorityPrecedenceRecord['authorityKind'],
  overrides: Partial<Omit<AuthorityPrecedenceRecord, 'source' | 'authorityKind'>> = {},
): AuthorityPrecedenceRecord {
  return {
    source,
    authorityKind,
    topic: 'movement',
    scope: null,
    supersedes: [],
    ...overrides,
  };
}

function precedenceArtifact(entry: AuthorityPrecedenceRecord) {
  return createCanonicalArtifact({
    artifactKind: entry.authorityKind === 'format' ? 'format' : 'rule',
    stableId: `rule:${entry.source.sourceId.slice('source:'.length)}`,
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [ref(entry.source)],
    payload: {
      precedence: {
        topic: entry.topic,
        authorityKind: entry.authorityKind,
        scope: entry.scope,
        supersedes: entry.supersedes,
      },
    },
  });
}

function bundleOf(entries: readonly AuthorityPrecedenceRecord[]) {
  const sources = [...new Map(entries.map((entry) => [entry.source.sourceId, entry.source])).values()];
  return createCanonicalArtifact({
    artifactKind: 'bundle',
    stableId: 'bundle:precedence-fixture',
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: sources.map(ref),
    payload: {
      effectiveDate: '2026-08-20',
      precedencePolicyVersion: 1,
      inputRootHash: null,
      sources,
      artifacts: entries.map(precedenceArtifact),
    },
  });
}

async function captureDiagnostics(run: () => Promise<unknown>): Promise<readonly Diagnostic[]> {
  try {
    await run();
  } catch (error: unknown) {
    if (error instanceof AuthorityValidationError) return error.diagnostics;
    throw error;
  }
  return assert.fail('expected AuthorityValidationError');
}

async function materialize(bundle: ReturnType<typeof bundleOf>) {
  const root = await mkdtemp(join(tmpdir(), 'sorcery-bundle-'));
  await writeFile(join(root, 'bundle.json'), canonicalJson(bundle));
  return {
    root,
    expected: { stableId: bundle.identity.stableId, contentHash: bundle.contentHash },
  };
}

test('DATA-01 resolves current official authority and retains winning source references', () => {
  const oldRulebook = manifestSource('source:rulebook-2025', { effectiveDate: '2025-12-19' });
  const currentRulebook = manifestSource('source:rulebook-2026', { effectiveDate: '2026-07-15' });
  const resolution = resolveAuthorityPrecedence(
    [record(oldRulebook, 'rulebook'), record(currentRulebook, 'rulebook')],
    null,
    '2026-08-20',
  );
  assert.equal(resolution.status, 'resolved');
  assert.deepEqual(resolution.winning, ref(currentRulebook));
  assert.deepEqual(resolution.superseded, [ref(oldRulebook)]);
});

test('DATA-01 resolves explicit official supersession by effective date', () => {
  const oldFaq = manifestSource('source:faq-old', { effectiveDate: '2026-01-01' });
  const update = manifestSource('source:card-update-current', { effectiveDate: '2026-07-15' });
  const resolution = resolveAuthorityPrecedence(
    [
      record(oldFaq, 'faq', { scope: 'card:synthetic', topic: 'card-text' }),
      record(update, 'card-update', {
        scope: 'card:synthetic',
        topic: 'card-text',
        supersedes: [oldFaq.sourceId],
      }),
    ],
    'card:synthetic',
    '2026-08-20',
  );
  assert.equal(resolution.status, 'resolved');
  assert.deepEqual(resolution.winning, ref(update));
  assert.deepEqual(resolution.superseded, [ref(oldFaq)]);
});

test('DATA-01 applies explicitly selected scoped overlays only in scope', () => {
  const base = manifestSource('source:constructed-base', { effectiveDate: '2025-12-19' });
  const overlay = manifestSource('source:gothic-overlay', { effectiveDate: '2026-06-01' });
  const entries = [
    record(base, 'rulebook', { topic: 'deck-size' }),
    record(overlay, 'format', { topic: 'deck-size', scope: 'format:gothic' }),
  ];
  assert.deepEqual(resolveAuthorityPrecedence(entries, null, '2026-08-20').winning, ref(base));
  assert.deepEqual(
    resolveAuthorityPrecedence(entries, 'format:gothic', '2026-08-20').winning,
    ref(overlay),
  );
});

test('DATA-01 records equal-rank and ambiguous official conflicts as unsupported', async () => {
  const first = manifestSource('source:faq-first', { effectiveDate: '2026-07-15' });
  const second = manifestSource('source:faq-second', { effectiveDate: '2026-07-15' });
  const entries = [record(first, 'faq'), record(second, 'faq')];
  const resolution = resolveAuthorityPrecedence(entries, null, '2026-08-20');
  assert.equal(resolution.status, 'unsupported');
  assert.equal(resolution.reason, 'equal-rank');
  assert.deepEqual(resolution.contending, [ref(first), ref(second)]);

  const bundle = bundleOf(entries);
  const materialized = await materialize(bundle);
  try {
    assert.deepEqual(
      (await captureDiagnostics(() =>
        validateAuthorityBundle(materialized.root, 'bundle.json', materialized.expected),
      )).map(({ path, code }) => ({ path, code })),
      [{ path: '/identity/payload/artifacts', code: 'unsupported_precedence' }],
    );
  } finally {
    await rm(materialized.root, { recursive: true, force: true });
  }
});

test('DATA-01 never lets community or external-reference sources become normative', () => {
  const official = manifestSource('source:official-rule', { effectiveDate: '2025-12-19' });
  const community = manifestSource('source:community-newer', {
    url: 'https://example.org/community-newer',
    authorityClass: 'community-provenance',
    effectiveDate: '2026-08-01',
  });
  const resolution = resolveAuthorityPrecedence(
    [record(official, 'rulebook'), record(community, 'card-update')],
    null,
    '2026-08-20',
  );
  assert.equal(resolution.status, 'resolved');
  assert.deepEqual(resolution.winning, ref(official));
  assert.deepEqual(resolution.provenance, [ref(community)]);
});

test('DATA-01 recursively validates a complete bundle offline', async () => {
  const source = manifestSource('source:offline-rule');
  const bundle = bundleOf([record(source, 'rulebook')]);
  const materialized = await materialize(bundle);
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (() => {
    throw new Error('network access forbidden');
  }) as typeof fetch;
  try {
    const validated = await validateAuthorityBundle(materialized.root, 'bundle.json', materialized.expected);
    assert.equal(validated.bundle.contentHash, bundle.contentHash);
    assert.deepEqual(validated.manifestBindingsVerified, [ref(source)]);
  } finally {
    globalThis.fetch = originalFetch;
    await rm(materialized.root, { recursive: true, force: true });
  }
});

test('DATA-01 fails if fetch HTTP HTTPS or net access is attempted', async () => {
  const source = await readFile(new URL('../../src/authority/validate-bundle.ts', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /node:(?:http|https|net)|\bfetch\s*\(/);
});

test.todo('DATA-01 rehashes stored source bytes and rejects one-byte tampering');
test.todo('DATA-01 validates manifest-only locator hash procedure and SourceRef binding honestly');
test.todo('DATA-01 rejects changed expected input-root hashes before parsing inputs');
test.todo('DATA-01 rejects missing extra and one-byte-mismatched input-lock files');
test.todo('DATA-01 records the verified input root and independently calculated bundle root');

test('DATA-01 rejects prohibited publisher PDFs images and raw corpus storage', async () => {
  for (const [id, mediaType, relativePath] of [
    ['publisher-pdf', 'application/pdf', 'raw/rules.pdf'],
    ['publisher-image', 'image/png', 'raw/card.png'],
    ['publisher-corpus', 'application/json', 'raw/cards.json'],
  ] as const) {
    const bytes = encoder.encode(`synthetic-${id}`);
    const source = validateSourceRecord({
      sourceId: `source:${id}`,
      url: `https://sorcerytcg.com/${id}`,
      authorityClass: 'official',
      retrievedAt: '2026-08-20T00:00:00Z',
      effectiveDate: '2026-01-01',
      mediaType,
      byteHash: sha256(bytes),
      derivation: { method: 'verbatim', parentByteHashes: [], notes: 'Synthetic fixture' },
      licenseStatus: 'permission-required',
      storageMode: 'stored',
      storagePolicy: 'permitted',
      relativePath,
    });
    const bundle = bundleOf([record(source, 'rulebook')]);
    const materialized = await materialize(bundle);
    try {
      await mkdir(join(materialized.root, 'raw'));
      await writeFile(join(materialized.root, relativePath.slice('raw/'.length)), bytes);
      assert.deepEqual(
        (await captureDiagnostics(() =>
          validateAuthorityBundle(materialized.root, 'bundle.json', materialized.expected),
        )).map(({ path, code }) => ({ path, code })),
        [{ path: '/identity/payload/sources/0/licenseStatus', code: 'storage_prohibited' }],
      );
    } finally {
      await rm(materialized.root, { recursive: true, force: true });
    }
  }
});

test.todo('DATA-01 refuses to overwrite an immutable published revision');
test.todo('DATA-01 removes no published data when an atomic candidate build fails');
test.todo('DATA-01 produces byte-identical bundles from two clean locked-input builds');
test.todo('DATA-01 rejects stored manifest artifact and reference tamper copies');

test('DATA-01 fixture storage policy enforces synthetic clean-room inputs', async () => {
  const policy = (await readFile(new URL('./fixtures/README.md', import.meta.url), 'utf8')).toLowerCase();
  for (const required of ['synthetic', 'pdf', 'images', 'raw api/database corpora', 'gpl-3.0']) {
    assert.ok(policy.includes(required));
  }
});