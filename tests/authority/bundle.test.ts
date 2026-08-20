import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, readdir, rm, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import { identityHash, sha256 } from '../../src/authority/hash.ts';
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
import {
  importAuthority,
  runImportAuthorityCommand,
  type ImportAuthorityArguments,
} from '../../src/commands/import-authority.ts';

const encoder = new TextEncoder();
const bundleInputFixture = fileURLToPath(new URL('./fixtures/bundle-input/', import.meta.url));

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
type LockedFile = {
  byteHash: ReturnType<typeof sha256>;
  relativePath: string;
  storageMode: 'manifest-only' | 'stored';
};

type FixtureInputLock = {
  schemaVersion: 1;
  files: LockedFile[];
  inputRootHash: ReturnType<typeof identityHash>;
};

type BuildWorkspace = {
  root: string;
  inputRoot: string;
  outputRoot: string;
  args: ImportAuthorityArguments;
};

async function readFixtureLock(inputRoot: string): Promise<FixtureInputLock> {
  return JSON.parse(await readFile(join(inputRoot, 'input-lock.json'), 'utf8')) as FixtureInputLock;
}

async function createBuildWorkspace(revisionId = 'fixture-revision'): Promise<BuildWorkspace> {
  const root = await mkdtemp(join(tmpdir(), 'sorcery-import-'));
  const inputRoot = join(root, 'input');
  const outputRoot = join(root, 'authority');
  await cp(bundleInputFixture, inputRoot, { recursive: true });
  await mkdir(outputRoot);
  const inputLock = await readFixtureLock(inputRoot);
  return {
    root,
    inputRoot,
    outputRoot,
    args: {
      inputRoot,
      inputLockPath: 'input-lock.json',
      expectedInputRootHash: inputLock.inputRootHash,
      outputRoot,
      revisionId,
    },
  };
}

async function refreshInputLock(inputRoot: string): Promise<FixtureInputLock> {
  const lock = await readFixtureLock(inputRoot);
  const files = await Promise.all(
    lock.files.map(async (entry) => ({
      ...entry,
      byteHash: sha256(await readFile(join(inputRoot, entry.relativePath))),
    })),
  );
  files.sort((left, right) => left.relativePath < right.relativePath ? -1 : left.relativePath > right.relativePath ? 1 : 0);
  const updated: FixtureInputLock = {
    schemaVersion: 1,
    files,
    inputRootHash: identityHash(files),
  };
  await writeFile(join(inputRoot, 'input-lock.json'), canonicalJson(updated));
  return updated;
}

async function setCardStorage(inputRoot: string, approved: boolean): Promise<FixtureInputLock> {
  const path = join(inputRoot, 'sources.json');
  const document = JSON.parse(await readFile(path, 'utf8')) as {
    sources: Record<string, unknown>[];
  };
  const source = document.sources.find((entry) => entry.sourceId === 'source:cards-synthetic');
  assert.ok(source);
  source.licenseStatus = approved ? 'approved' : 'permission-required';
  source.storageMode = 'stored';
  source.storagePolicy = 'permitted';
  source.relativePath = 'raw/cards.raw.json';
  delete source.durableLocator;
  delete source.acquisitionProcedureHash;
  await writeFile(path, canonicalJson(document as never));

  const lock = await readFixtureLock(inputRoot);
  const cards = lock.files.find((entry) => entry.relativePath === 'cards.raw.json');
  assert.ok(cards);
  cards.storageMode = 'stored';
  await writeFile(join(inputRoot, 'input-lock.json'), canonicalJson(lock));
  return refreshInputLock(inputRoot);
}

async function fileMap(root: string): Promise<readonly (readonly [string, string])[]> {
  const files = (await readdir(root, { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile())
    .map((entry) => {
      const path = join(entry.parentPath, entry.name);
      return {
        path,
        relativePath: relative(root, path).replaceAll('\\', '/'),
      };
    });
  const mapped = await Promise.all(
    files.map(async (entry) => [entry.relativePath, (await readFile(entry.path)).toString('base64')] as const),
  );
  return mapped.sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
}

function importArgv(args: ImportAuthorityArguments): string[] {
  return [
    '--input', args.inputRoot,
    '--input-lock', args.inputLockPath,
    '--expected-input-root-hash', args.expectedInputRootHash,
    '--output-root', args.outputRoot,
    '--revision-id', args.revisionId,
  ];
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

test('DATA-01 resolves explicit official superseded authority by effective date', () => {
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

  const undated = manifestSource('source:faq-undated', { effectiveDate: null });
  assert.equal(resolveAuthorityPrecedence([record(undated, 'faq')], null, '2026-08-20').reason, 'unclear-date');
  const unscopedUpdate = manifestSource('source:update-unscoped', { effectiveDate: '2026-07-15' });
  assert.equal(
    resolveAuthorityPrecedence([record(unscopedUpdate, 'card-update')], null, '2026-08-20').reason,
    'unclear-scope',
  );

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

test('DATA-01 remains offline and fails if fetch HTTP HTTPS or net access is attempted', async () => {
  const source = await readFile(new URL('../../src/authority/validate-bundle.ts', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /node:(?:http|https|net)|\bfetch\s*\(/);
});

test.todo('DATA-01 rehashes stored source bytes and rejects one-byte tampering');
test.todo('DATA-01 validates manifest-only locator hash procedure and SourceRef binding honestly');
test('DATA-01 rejects changed expected input-root hashes before parsing inputs', async () => {
  const workspace = await createBuildWorkspace();
  try {
    await writeFile(join(workspace.inputRoot, 'cards.raw.json'), '{not-json');
    const diagnostics = await captureDiagnostics(() =>
      importAuthority({
        ...workspace.args,
        expectedInputRootHash: `sha256:${'0'.repeat(64)}`,
      }),
    );
    assert.deepEqual(
      diagnostics.map(({ path, code }) => ({ path, code })),
      [{ path: '/expectedInputRootHash', code: 'unexpected_input_root_hash' }],
    );
  } finally {
    await rm(workspace.root, { recursive: true, force: true });
  }
});

test('DATA-01 rejects missing extra and one-byte-mismatched input-lock files', async () => {
  const cases = [
    {
      name: 'missing',
      mutate: (workspace: BuildWorkspace) => unlink(join(workspace.inputRoot, 'formats.json')),
      expected: { path: '/inputLock/files', code: 'input_file_set_mismatch' },
    },
    {
      name: 'extra',
      mutate: (workspace: BuildWorkspace) => writeFile(join(workspace.inputRoot, 'extra.json'), '{}'),
      expected: { path: '/inputLock/files', code: 'input_file_set_mismatch' },
    },
    {
      name: 'byte mismatch',
      mutate: async (workspace: BuildWorkspace) => {
        const path = join(workspace.inputRoot, 'cards.raw.json');
        await writeFile(path, Buffer.concat([await readFile(path), Buffer.from(' ')]));
      },
      expected: { path: '/inputLock/files/0/byteHash', code: 'input_byte_hash_mismatch' },
    },
  ] as const;

  for (const fixture of cases) {
    const workspace = await createBuildWorkspace(`fixture-${fixture.name.replace(' ', '-')}`);
    try {
      await fixture.mutate(workspace);
      const diagnostics = await captureDiagnostics(() => importAuthority(workspace.args));
      assert.deepEqual(
        diagnostics.map(({ path, code }) => ({ path, code })),
        [fixture.expected],
        fixture.name,
      );
    } finally {
      await rm(workspace.root, { recursive: true, force: true });
    }
  }
});

test('DATA-01 records the verified input root and independently calculated bundle root', async () => {
  const workspace = await createBuildWorkspace();
  const stdout: string[] = [];
  const stderr: string[] = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (() => {
    throw new Error('network access forbidden');
  }) as typeof fetch;
  try {
    const exitCode = await runImportAuthorityCommand(importArgv(workspace.args), {
      stdout: (line) => stdout.push(line),
      stderr: (line) => stderr.push(line),
    });
    assert.equal(exitCode, 0);
    assert.deepEqual(stderr, []);
    assert.equal(stdout.length, 1);
    const announced = JSON.parse(stdout[0]!) as {
      status: string;
      verifiedInputRootHash: string;
      bundleRootHash: string;
    };
    const lock = await readFixtureLock(workspace.inputRoot);
    assert.equal(announced.status, 'candidate');
    assert.equal(announced.verifiedInputRootHash, lock.inputRootHash);

    const revisionPath = join(workspace.outputRoot, workspace.args.revisionId);
    assert.deepEqual(
      (await fileMap(revisionPath)).map(([path]) => path),
      ['bundle.json', 'cards.normalized.json', 'formats.json', 'sources.json'],
    );
    const bundle = JSON.parse(await readFile(join(revisionPath, 'bundle.json'), 'utf8')) as {
      identity: { stableId: string; payload: { inputRootHash: string } };
      contentHash: string;
    };
    assert.equal(bundle.identity.payload.inputRootHash, lock.inputRootHash);
    assert.equal(bundle.contentHash, announced.bundleRootHash);
    await validateAuthorityBundle(
      workspace.outputRoot,
      `${workspace.args.revisionId}/bundle.json`,
      { stableId: bundle.identity.stableId, contentHash: bundle.contentHash },
    );
  } finally {
    globalThis.fetch = originalFetch;
    await rm(workspace.root, { recursive: true, force: true });
  }
});

test('DATA-01 stores only an explicitly approved fixed raw card path', async () => {
  for (const approved of [true, false]) {
    const workspace = await createBuildWorkspace(`fixture-storage-${approved ? 'approved' : 'blocked'}`);
    try {
      const lock = await setCardStorage(workspace.inputRoot, approved);
      workspace.args = { ...workspace.args, expectedInputRootHash: lock.inputRootHash };
      if (!approved) {
        const diagnostics = await captureDiagnostics(() => importAuthority(workspace.args));
        assert.deepEqual(
          diagnostics.map(({ path, code }) => ({ path, code })),
          [{ path: '/sources/0/licenseStatus', code: 'storage_prohibited' }],
        );
        continue;
      }

      const result = await importAuthority(workspace.args);
      assert.deepEqual(
        (await fileMap(result.revisionPath)).map(([path]) => path),
        ['bundle.json', 'cards.normalized.json', 'formats.json', 'raw/cards.raw.json', 'sources.json'],
      );
      assert.deepEqual(
        await readFile(join(result.revisionPath, 'raw/cards.raw.json')),
        await readFile(join(workspace.inputRoot, 'cards.raw.json')),
      );
    } finally {
      await rm(workspace.root, { recursive: true, force: true });
    }
  }
});

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
      await writeFile(join(materialized.root, relativePath), bytes);
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

test('DATA-01 refuses to overwrite an immutable published revision', async () => {
  const workspace = await createBuildWorkspace();
  try {
    const first = await importAuthority(workspace.args);
    const before = await fileMap(first.revisionPath);
    const diagnostics = await captureDiagnostics(() => importAuthority(workspace.args));
    assert.deepEqual(
      diagnostics.map(({ path, code }) => ({ path, code })),
      [{ path: '/revisionId', code: 'revision_exists' }],
    );
    assert.deepEqual(await fileMap(first.revisionPath), before);
  } finally {
    await rm(workspace.root, { recursive: true, force: true });
  }
});

test('DATA-01 removes no published data when an atomic candidate build fails', async () => {
  const workspace = await createBuildWorkspace();
  try {
    const diagnostics = await captureDiagnostics(() =>
      importAuthority(workspace.args, {
        afterWrite: (relativePath) => {
          if (relativePath === 'sources.json') throw new Error('injected write failure');
        },
      }),
    );
    assert.deepEqual(
      diagnostics.map(({ path, code }) => ({ path, code })),
      [{ path: '/candidate', code: 'candidate_write_failed' }],
    );
    assert.equal(
      (await readdir(workspace.outputRoot)).some(
        (name) => name === workspace.args.revisionId || name.startsWith(`.${workspace.args.revisionId}.tmp-`),
      ),
      false,
    );
  } finally {
    await rm(workspace.root, { recursive: true, force: true });
  }
});

test('DATA-01 produces byte-identical bundles from two clean locked-input builds', async () => {
  const first = await createBuildWorkspace('fixture-repeatable');
  const second = await createBuildWorkspace('fixture-repeatable');
  try {
    const firstResult = await importAuthority(first.args);
    const secondResult = await importAuthority(second.args);
    assert.equal(firstResult.bundleRootHash, secondResult.bundleRootHash);
    assert.equal(firstResult.verifiedInputRootHash, secondResult.verifiedInputRootHash);
    assert.deepEqual(await fileMap(firstResult.revisionPath), await fileMap(secondResult.revisionPath));
  } finally {
    await rm(first.root, { recursive: true, force: true });
    await rm(second.root, { recursive: true, force: true });
  }
});
test.todo('DATA-01 rejects stored manifest artifact and reference tamper copies');

test('DATA-01 fixture storage policy enforces synthetic clean-room inputs', async () => {
  const policy = (await readFile(new URL('./fixtures/README.md', import.meta.url), 'utf8')).toLowerCase();
  for (const required of ['synthetic', 'pdf', 'images', 'raw api/database corpora', 'gpl-3.0']) {
    assert.ok(policy.includes(required));
  }
});