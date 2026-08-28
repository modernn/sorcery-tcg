import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import test from 'node:test';

import {
  PRIVATE_AUTHORITY_SOURCE_PATHS,
  verifyPrivateSourceSet,
  type PrivateAuthoritySourceEntry,
} from '../../src/authority/private-source-set.ts';
import {
  createPrivateCandidateInspectorForTest,
  type PrivateInspectionSource,
} from '../../scripts/verify-private-authority-boundary.ts';
import { runBounded } from '../helpers/bounded-process.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const SCRIPT_PATH = join(REPOSITORY_ROOT, 'scripts', 'verify-private-authority-boundary.ts');
const SYNTHETIC_PUBLIC_PROVENANCE_URL = 'https://official.invalid/formats/constructed-current.html';

type Fixture = Readonly<{
  sandbox: string;
  repositoryRoot: string;
  primaryRoot: string;
  backupRoot: string;
  lockPath: string;
  sourceBytes: ReadonlyMap<string, Buffer>;
  protectedCard: Readonly<Record<string, unknown>>;
  publicProvenanceMarker: string;
}>;

type Surface = 'reachable-history' | 'worktree' | 'index' | 'package';
type PrivateData = Pick<Fixture, 'protectedCard' | 'publicProvenanceMarker' | 'sourceBytes'>;

async function git(fixture: Fixture, ...arguments_: readonly string[]): Promise<void> {
  const result = await runBounded('git', arguments_, fixture.repositoryRoot);
  assert.equal(result.code, 0, result.stderr);
}

let privateDataPromise: Promise<PrivateData> | undefined;

function privateData(): Promise<PrivateData> {
  privateDataPromise ??= (async () => {
    const collector = await readFile(join(REPOSITORY_ROOT, 'scripts', 'collect-private-authority.ps1'), 'utf8');
    const publicProvenanceMarker = [...collector.matchAll(/^\s*sourceMarker\s*=\s*'([^']+)'/gm)]
      .map((match) => match[1]!)
      .find((value) => Buffer.byteLength(value, 'utf8') >= 32);
    assert.ok(publicProvenanceMarker !== undefined, 'tracked public provenance marker missing');
    const sourceBytes = new Map<string, Buffer>();
    const protectedCard = {
      name: 'Synthetic Boundary Sentinel',
      guardian: {
        rulesText: 'A deliberately private synthetic card record used only for boundary verification.',
        threshold: { air: 1, earth: 0, fire: 0, water: 0 },
      },
      sets: [{ variants: [{ finish: 'standard', slug: 'synthetic-boundary-sentinel' }] }],
    } as const;
    for (const [sourceIndex, relativePath] of PRIVATE_AUTHORITY_SOURCE_PATHS.entries()) {
      const canonicalLink = relativePath === 'formats/constructed-current.html'
        ? '<link rel="canonical" href="' + SYNTHETIC_PUBLIC_PROVENANCE_URL + '">'
        : '';
      const bytes = relativePath.endsWith('.json')
        ? Buffer.from(JSON.stringify([protectedCard, { name: 'Second Synthetic Record', ordinal: sourceIndex }]), 'utf8')
        : relativePath.endsWith('.html')
          ? Buffer.from(
              '<!doctype html><html><head>' + canonicalLink + '<style>.hidden { display: none }</style></head><body>' +
                '<h1>Visible Private Heading ' + sourceIndex + '</h1><p>Alpha   Beta\nGamma Secret Passage ' +
                sourceIndex + ' ' + 'visible-boundary-text '.repeat(8) + '</p><h2>' +
                publicProvenanceMarker + '</h2></body></html>',
              'utf8',
            )
          : Buffer.from(
              'Synthetic source ' + sourceIndex + ': ' +
                Array.from(
                  { length: 90 },
                  (_, index) => 'segment-' + sourceIndex + '-' + String(index).padStart(3, '0') + ';',
                ).join(''),
              'utf8',
            );
      sourceBytes.set(relativePath, bytes);
    }
    return { protectedCard, publicProvenanceMarker, sourceBytes };
  })();
  return privateDataPromise;
}

async function createFixture(
  privateLocatorEvidence = 'https://private.invalid/rulebook-locator',
): Promise<Fixture> {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-boundary-'));
  const repositoryRoot = join(sandbox, 'repository');
  const primaryRoot = join(repositoryRoot, '.local', 'authority', 'inputs', 'synthetic', 'primary');
  const backupRoot = join(sandbox, 'backup');
  const lockPath = join(repositoryRoot, '.local', 'authority', 'locks', 'synthetic', 'source-set-lock.json');
  const data = await privateData();
  const entries: PrivateAuthoritySourceEntry[] = [];
  await mkdir(repositoryRoot, { recursive: true });
  for (const relativePath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
    const bytes = data.sourceBytes.get(relativePath)!;
    const primaryPath = join(primaryRoot, ...relativePath.split('/'));
    const backupPath = join(backupRoot, ...relativePath.split('/'));
    await mkdir(dirname(primaryPath), { recursive: true });
    await mkdir(dirname(backupPath), { recursive: true });
    await writeFile(primaryPath, bytes);
    await writeFile(backupPath, bytes);
    entries.push({
      relativePath,
      url: `https://sorcerytcg.com/${relativePath}`,
      retrievedAt: '2026-08-25T00:00:00.000Z',
      effectiveDate: null,
      mediaType: relativePath.endsWith('.pdf')
        ? 'application/pdf'
        : relativePath.endsWith('.json')
          ? 'application/json'
          : 'text/html',
      byteLength: bytes.length,
      byteHash: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
    });
  }
  const verified = await verifyPrivateSourceSet({ repositoryRoot, primaryRoot, backupRoot, entries });
  await mkdir(dirname(lockPath), { recursive: true });
  await writeFile(
    lockPath,
    JSON.stringify({
      primaryRoot,
      backupRoot,
      entries,
      sourceSetRootHash: verified.sourceSetRootHash,
      rulebookAcquisitionEvidence: {
        privateLocatorEvidence,
      },
    }),
  );
  await writeFile(join(repositoryRoot, '.gitignore'), '.local/authority/\n');
  await writeFile(
    join(repositoryRoot, 'package.json'),
    JSON.stringify({ name: 'boundary-fixture', version: '1.0.0', files: ['safe-metadata.json', 'included/**'] }),
  );
  await writeFile(
    join(repositoryRoot, 'safe-metadata.json'),
    JSON.stringify({
      relativePaths: [...PRIVATE_AUTHORITY_SOURCE_PATHS],
      byteHashes: entries.map(({ byteHash }) => byteHash),
    }),
  );
  const fixture = {
    sandbox,
    repositoryRoot,
    primaryRoot,
    backupRoot,
    lockPath,
    ...data,
  };
  await git(fixture, 'init', '--quiet');
  await git(fixture, 'config', 'user.email', 'boundary@example.invalid');
  await git(fixture, 'config', 'user.name', 'Boundary Fixture');
  await git(fixture, 'add', '.gitignore', 'package.json', 'safe-metadata.json');
  await git(fixture, 'commit', '--quiet', '-m', 'safe fixture');
  return fixture;
}

async function addSurfaceCandidate(
  fixture: Fixture,
  surface: Surface,
  name: string,
  bytes: string | Buffer,
): Promise<void> {
  const relativePath = surface === 'package'
    ? 'included/' + name + '.txt'
    : surface === 'worktree'
      ? 'worktree-only/' + name + '.tmp'
      : surface === 'index'
        ? 'index-only/' + name + '.txt'
        : 'history-only/' + name + '.txt';
  if (surface === 'reachable-history') {
    await addCandidate(fixture, relativePath, bytes, 'commit');
    await rm(join(fixture.repositoryRoot, ...relativePath.split('/')));
    await git(fixture, 'add', '-u', relativePath);
    await git(fixture, 'commit', '--quiet', '-m', 'remove ' + relativePath);
    return;
  }
  await addCandidate(
    fixture,
    relativePath,
    bytes,
    surface === 'index' ? 'stage' : 'untracked',
  );
}

function reorderedProtectedCard(fixture: PrivateData): string {
  const card = fixture.protectedCard as {
    name: string;
    guardian: { rulesText: string; threshold: Record<string, number> };
    sets: readonly unknown[];
  };
  return JSON.stringify({
    sets: card.sets,
    guardian: {
      threshold: {
        water: card.guardian.threshold.water,
        fire: card.guardian.threshold.fire,
        earth: card.guardian.threshold.earth,
        air: card.guardian.threshold.air,
      },
      rulesText: card.guardian.rulesText,
    },
    name: card.name,
  });
}

async function privateInspector(locator = 'https://private.invalid/rulebook-locator') {
  const data = await privateData();
  const sources: PrivateInspectionSource[] = [...data.sourceBytes].map(([relativePath, bytes]) => ({
    bytes,
    kind: relativePath.endsWith('.json') ? 'json' : relativePath.endsWith('.html') ? 'html' : 'binary',
  }));
  return {
    ...data,
    inspect: createPrivateCandidateInspectorForTest(sources, [locator]),
  };
}

function assertViolation(
  inspect: ReturnType<typeof createPrivateCandidateInspectorForTest>,
  path: string,
  bytes: string | Uint8Array,
  category: RegExp,
  forbidden: readonly string[] = [],
): void {
  assert.throws(() => inspect({ path, bytes }), (error: unknown) => {
    assert.ok(error instanceof Error);
    assert.match(error.message, category);
    const protectedText = typeof bytes === 'string' ? bytes : Buffer.from(bytes).toString('utf8');
    for (const value of new Set([...forbidden, protectedText].filter(Boolean))) {
      assert.equal(error.message.includes(value), false);
    }
    return true;
  });
}

function escapedTemplate(value: string): string {
  return '`' + [...value].map((character) => `\\u{${character.codePointAt(0)!.toString(16)}}`).join('') + '`';
}

async function cleanupFixture(fixture: Fixture): Promise<void> {
  const sandbox = resolve(fixture.sandbox);
  assert.equal(dirname(sandbox), resolve(tmpdir()));
  assert.match(basename(sandbox), /^sorcery-private-boundary-/);
  await rm(sandbox, { recursive: true, force: true });
}

async function runGate(fixture: Fixture): Promise<Readonly<{ code: number | null; stdout: string; stderr: string }>> {
  return runBounded(
    'node',
    [SCRIPT_PATH, '--repository-root', fixture.repositoryRoot, '--lock', fixture.lockPath],
    fixture.repositoryRoot,
  );
}

async function addCandidate(
  fixture: Fixture,
  relativePath: string,
  bytes: string | Buffer,
  mode: 'commit' | 'stage' | 'untracked' = 'commit',
): Promise<void> {
  const path = join(fixture.repositoryRoot, ...relativePath.split('/'));
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, bytes);
  if (mode === 'untracked') return;
  await git(fixture, 'add', relativePath);
  if (mode === 'commit') await git(fixture, 'commit', '--quiet', '-m', `add ${relativePath}`);
}

test('safe relative metadata and hashes pass the Git index and package boundary', async () => {
  const fixture = await createFixture();
  try {
    const result = await runGate(fixture);
    assert.equal(result.code, 0, result.stderr);
    assert.match(result.stdout, /private authority boundary verified/i);
  } finally {
    await cleanupFixture(fixture);
  }
});

test('generic local-file evidence label is not treated as a private locator', async () => {
  const label = ['user-provided', 'manual-local-file'].join('-');
  const fixture = await createFixture(label);
  try {
    await addCandidate(fixture, 'included/generic-evidence-label.txt', label);
    const result = await runGate(fixture);
    assert.equal(result.code, 0, result.stderr);
    assert.equal(result.stdout, 'Private authority boundary verified.\n');
  } finally {
    await cleanupFixture(fixture);
  }
});

test('empty partial duplicate and unknown private locks fail before candidate scanning', async (context) => {
  const mutations = {
    empty: (entries: readonly Record<string, unknown>[]) => entries.slice(0, 0),
    partial: (entries: readonly Record<string, unknown>[]) => entries.slice(0, -1),
    duplicate: (entries: readonly Record<string, unknown>[]) => [...entries, entries[0]!],
    unknown: (entries: readonly Record<string, unknown>[]) => [
      ...entries.slice(0, -1),
      { ...entries.at(-1)!, relativePath: 'unknown/private-source.html' },
    ],
  } as const;
  for (const [name, mutate] of Object.entries(mutations)) {
    await context.test(name, async () => {
      const fixture = await createFixture();
      try {
        const lock = JSON.parse(await readFile(fixture.lockPath, 'utf8')) as {
          entries: readonly Record<string, unknown>[];
        } & Record<string, unknown>;
        await writeFile(fixture.lockPath, JSON.stringify({ ...lock, entries: mutate(lock.entries) }));
        const result = await runGate(fixture);
        assert.notEqual(result.code, 0);
        assert.equal(result.stderr, 'Private authority boundary verification failed.\n');
        for (const privateValue of [
          fixture.primaryRoot,
          fixture.backupRoot,
          'private.invalid',
          fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!.subarray(137, 169).toString('utf8'),
        ]) {
          assert.equal(result.stderr.includes(privateValue), false);
        }
      } finally {
        await cleanupFixture(fixture);
      }
    });
  }
});

test('exact private bytes fail on every repository and package surface without disclosure', async (context) => {
  for (const surface of ['reachable-history', 'worktree', 'index', 'package'] as const) {
    await context.test(surface, async () => {
      const fixture = await createFixture();
      try {
        await addSurfaceCandidate(
          fixture,
          surface,
          'exact-private-copy-' + surface,
          fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!,
        );
        const result = await runGate(fixture);
        assert.notEqual(result.code, 0);
        assert.match(result.stderr, /exact-private-bytes/i);
        assert.match(result.stderr, new RegExp(surface));
        for (const privateValue of [
          fixture.primaryRoot,
          fixture.backupRoot,
          'private.invalid',
          fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!.subarray(137, 169).toString('utf8'),
        ]) {
          assert.equal(result.stderr.includes(privateValue), false);
        }
      } finally {
        await cleanupFixture(fixture);
      }
    });
  }
});

test('pure detector rejects private content paths and artwork without disclosure', async (context) => {
  const fixture = await privateInspector();
  const firstSource = fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!;
  const pathExcerpt = firstSource.subarray(137, 169).toString('utf8');
  const cases = [
    { name: 'exact private bytes', path: 'candidate.bin', bytes: firstSource, category: /exact-private-bytes/i },
    {
      name: 'private locator',
      path: 'candidate.txt',
      bytes: 'do not publish https://private.invalid/rulebook-locator',
      category: /private-locator/i,
    },
    { name: 'source excerpt', path: 'candidate.bin', bytes: firstSource.subarray(32, 64), category: /source-derived/i },
    { name: 'arbitrary offset', path: 'candidate.bin', bytes: firstSource.subarray(137, 169), category: /source-derived/i },
    { name: 'reordered card', path: 'candidate.json', bytes: reorderedProtectedCard(fixture), category: /semantic|source-derived/i },
    {
      name: 'normalized HTML',
      path: 'candidate.txt',
      bytes: 'VISIBLE private heading 1 alpha beta gamma secret passage 1 visible-boundary-text visible-boundary-text visible-boundary-text',
      category: /semantic|source-derived/i,
    },
    { name: 'artwork extension', path: 'artwork.png', bytes: 'synthetic non-image payload', category: /artwork-path/i },
    {
      name: 'artwork signature',
      path: 'payload.bin',
      bytes: Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00]),
      category: /artwork-signature/i,
    },
    {
      name: 'private candidate path',
      path: '.local/authority/package-leak.txt',
      bytes: 'synthetic candidate',
      category: /forbidden-private-path/i,
    },
    {
      name: 'source excerpt in candidate path',
      path: 'prefix-' + pathExcerpt + '-suffix.txt',
      bytes: 'safe candidate content',
      category: /source-derived-path/i,
      forbidden: [pathExcerpt],
    },
    {
      name: 'private locator in candidate path',
      path: 'prefix-https://private.invalid/rulebook-locator-suffix.txt',
      bytes: 'safe candidate content',
      category: /private-locator-path/i,
      forbidden: ['https://private.invalid/rulebook-locator'],
    },
  ] as const;
  for (const scenario of cases) {
    await context.test(scenario.name, () => {
      assertViolation(
        fixture.inspect,
        scenario.path,
        scenario.bytes,
        scenario.category,
        [
          'https://private.invalid/rulebook-locator',
          ...('forbidden' in scenario ? scenario.forbidden : []),
        ],
      );
    });
  }
});

test('pure detector allows exact public provenance while rejecting extensions and private prose', async (context) => {
  const fixture = await privateInspector();
  const cases = [
    { name: 'exact public metadata', candidate: (fixture: PrivateData) => '`' + fixture.publicProvenanceMarker + '`', passes: true },
    {
      name: 'embedded exact public metadata',
      candidate: (fixture: PrivateData) => '<title>' + fixture.publicProvenanceMarker + '</title>',
      passes: true,
    },
    {
      name: 'common semantic fragment',
      candidate: () => JSON.stringify({ water: 0, fire: 0, earth: 0, air: 1 }, null, 2),
      passes: true,
    },
    {
      name: 'exact Markdown provenance URL',
      candidate: () => '[CITED: ' + SYNTHETIC_PUBLIC_PROVENANCE_URL + ']',
      passes: true,
    },
    {
      name: 'one-byte-extended public metadata',
      candidate: (fixture: PrivateData) => '`' + fixture.publicProvenanceMarker + 'x`',
      passes: false,
    },
    {
      name: 'embedded one-byte-extended public metadata',
      candidate: (fixture: PrivateData) => '<title>' + fixture.publicProvenanceMarker + 'x</title>',
      passes: false,
    },
    {
      name: 'one-byte-extended Markdown provenance URL',
      candidate: () => '[CITED: ' + SYNTHETIC_PUBLIC_PROVENANCE_URL + 'x]',
      passes: false,
    },
    {
      name: 'private prose',
      candidate: (fixture: PrivateData) => fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!.subarray(137, 169),
      passes: false,
    },
  ] as const;
  for (const scenario of cases) {
    await context.test(scenario.name, () => {
      const candidate = scenario.candidate(fixture);
      if (scenario.passes) assert.doesNotThrow(() => fixture.inspect({ path: 'candidate.txt', bytes: candidate }));
      else assertViolation(fixture.inspect, 'candidate.txt', candidate, /source-derived/i);
    });
  }
});

test('pure detector rejects encoded and escaped private content', async (context) => {
  const fixture = await privateInspector();
  const sourceExcerpt = fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!.subarray(137, 169).toString('utf8');
  const protectedCard = reorderedProtectedCard(fixture);
  const cases = [
    { name: 'escaped source', candidate: 'const protectedExcerpt = ' + JSON.stringify(sourceExcerpt) + ';', category: /source-derived/i },
    { name: 'escaped card', candidate: "const protectedCard = '" + protectedCard + "';", category: /source-derived|semantic/i },
    { name: 'backtick escaped source', candidate: escapedTemplate(sourceExcerpt), category: /source-derived/i },
    { name: 'backtick escaped card', candidate: escapedTemplate(protectedCard), category: /source-derived|semantic/i },
    {
      name: 'backtick escaped locator',
      candidate: escapedTemplate('https://private.invalid/rulebook-locator'),
      category: /private-locator/i,
    },
  ] as const;
  for (const scenario of cases) {
    await context.test(scenario.name, () => {
      assertViolation(fixture.inspect, 'candidate.ts', scenario.candidate, scenario.category, [sourceExcerpt, protectedCard]);
    });
  }
});

test('pure detector rejects escaped and case-varied Windows locators', async (context) => {
  const locator = 'C:\\Private\\Authority\\Rulebook';
  const mixedCase = 'c:\\pRIVATE\\AUTHORITY\\rULEBOOK';
  const fixture = await privateInspector(locator);
  const candidates = {
    'JSON escaped': 'const locator = ' + JSON.stringify(mixedCase) + ';',
    'TypeScript escaped': "const locator = '" + mixedCase.replaceAll('\\', '\\\\') + "';",
    'slash-normalized case-varied': mixedCase.replaceAll('\\', '/'),
  } as const;
  for (const [encoding, candidate] of Object.entries(candidates)) {
    await context.test(encoding, () => {
      assertViolation(fixture.inspect, 'candidate.ts', candidate, /private-locator/i, [locator, mixedCase]);
    });
  }
});

test('pure detector keeps POSIX locator matching exact and case-sensitive', async (context) => {
  const locator = '/Private/Authority/Rulebook';
  const fixture = await privateInspector(locator);
  await context.test('exact locator fails', () => {
    assertViolation(fixture.inspect, 'candidate.txt', locator, /private-locator/i, [locator]);
  });
  await context.test('case-varied locator passes', () => {
    assert.doesNotThrow(() => fixture.inspect({ path: 'candidate.txt', bytes: locator.toLowerCase() }));
  });
});

test('boundary verifier source remains standard-library only', async () => {
  const source = await readFile(SCRIPT_PATH, 'utf8');
  assert.doesNotMatch(source, /from ['"](?!node:)[^./]/);
});
