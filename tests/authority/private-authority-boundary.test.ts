import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import test from 'node:test';

import { PRIVATE_AUTHORITY_SOURCE_PATHS } from '../../src/authority/private-source-set.ts';

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

function run(
  command: string,
  arguments_: readonly string[],
  cwd: string,
): Promise<Readonly<{ code: number | null; stdout: string; stderr: string }>> {
  return new Promise((resolveProcess, reject) => {
    const child = spawn(command, arguments_, { cwd, env: process.env, windowsHide: true });
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8').on('data', (chunk: string) => (stdout += chunk));
    child.stderr.setEncoding('utf8').on('data', (chunk: string) => (stderr += chunk));
    child.once('error', reject);
    child.once('close', (code) => resolveProcess({ code, stdout, stderr }));
  });
}

async function git(fixture: Fixture, ...arguments_: readonly string[]): Promise<void> {
  const result = await run('git', arguments_, fixture.repositoryRoot);
  assert.equal(result.code, 0, result.stderr);
}

async function createFixture(
  privateLocatorEvidence = 'https://private.invalid/rulebook-locator',
): Promise<Fixture> {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-boundary-'));
  const repositoryRoot = join(sandbox, 'repository');
  const primaryRoot = join(repositoryRoot, '.local', 'authority', 'inputs', 'synthetic', 'primary');
  const backupRoot = join(sandbox, 'backup');
  const lockPath = join(repositoryRoot, '.local', 'authority', 'locks', 'synthetic', 'source-set-lock.json');
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
  const entries = [];
  await mkdir(repositoryRoot, { recursive: true });
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
    const primaryPath = join(primaryRoot, ...relativePath.split('/'));
    const backupPath = join(backupRoot, ...relativePath.split('/'));
    await mkdir(dirname(primaryPath), { recursive: true });
    await mkdir(dirname(backupPath), { recursive: true });
    await writeFile(primaryPath, bytes);
    await writeFile(backupPath, bytes);
    entries.push({
      relativePath,
      url: `https://official.invalid/${relativePath}`,
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
  await mkdir(dirname(lockPath), { recursive: true });
  await writeFile(
    lockPath,
    JSON.stringify({
      primaryRoot,
      backupRoot,
      entries,
      sourceSetRootHash: `sha256:${'1'.repeat(64)}`,
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
    sourceBytes,
    protectedCard,
    publicProvenanceMarker,
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

function reorderedProtectedCard(fixture: Fixture): string {
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

async function cleanupFixture(fixture: Fixture): Promise<void> {
  const sandbox = resolve(fixture.sandbox);
  assert.equal(dirname(sandbox), resolve(tmpdir()));
  assert.match(basename(sandbox), /^sorcery-private-boundary-/);
  await rm(sandbox, { recursive: true, force: true });
}

async function runGate(fixture: Fixture): Promise<Readonly<{ code: number | null; stdout: string; stderr: string }>> {
  return run(
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

test('private bytes locators excerpts paths and artwork fail without disclosing content', async (context) => {
  const firstSource = PRIVATE_AUTHORITY_SOURCE_PATHS[0];
  const cases: readonly Readonly<{
    name: string;
    path: string;
    bytes: (fixture: Fixture) => string | Buffer;
    mode: 'commit' | 'stage' | 'untracked' | 'stage-force';
    category: RegExp;
  }>[] = [
    {
      name: 'exact bytes in HEAD under another name',
      path: 'included/exact-copy.bin',
      bytes: (fixture: Fixture) => fixture.sourceBytes.get(firstSource)!,
      mode: 'commit',
      category: /exact-private-bytes/i,
    },
    {
      name: 'exact bytes staged under another name',
      path: 'included/staged-copy.dat',
      bytes: (fixture: Fixture) => fixture.sourceBytes.get(firstSource)!,
      mode: 'stage',
      category: /exact-private-bytes/i,
    },
    {
      name: 'private locator',
      path: 'included/locator.txt',
      bytes: (fixture: Fixture) => `do not publish ${fixture.primaryRoot}`,
      mode: 'commit',
      category: /private-locator/i,
    },
    {
      name: 'source-derived excerpt marker',
      path: 'included/excerpt.bin',
      bytes: (fixture: Fixture) => fixture.sourceBytes.get(firstSource)!.subarray(32, 64),
      mode: 'commit',
      category: /source-derived/i,
    },
    {
      name: 'artwork extension',
      path: 'included/artwork.png',
      bytes: () => 'synthetic non-image payload',
      mode: 'commit',
      category: /artwork-path/i,
    },
    {
      name: 'artwork binary signature',
      path: 'included/payload.bin',
      bytes: () => Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00]),
      mode: 'commit',
      category: /artwork-signature/i,
    },
    {
      name: 'forbidden private candidate path',
      path: '.local/authority/package-leak.txt',
      bytes: () => 'synthetic candidate',
      mode: 'stage-force',
      category: /forbidden-private-path/i,
    },
  ] as const;
  for (const scenario of cases) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      try {
        if (scenario.mode === 'stage-force') {
          const path = join(fixture.repositoryRoot, ...scenario.path.split('/'));
          await mkdir(dirname(path), { recursive: true });
          await writeFile(path, scenario.bytes(fixture));
          await git(fixture, 'add', '-f', scenario.path);
        } else {
          await addCandidate(fixture, scenario.path, scenario.bytes(fixture), scenario.mode);
        }
        const result = await runGate(fixture);
        assert.notEqual(result.code, 0);
        assert.match(result.stderr, scenario.category);
        assert.equal(result.stderr.includes(fixture.primaryRoot), false);
        assert.equal(result.stderr.includes(fixture.backupRoot), false);
        assert.equal(result.stderr.includes('private.invalid'), false);
      } finally {
        await cleanupFixture(fixture);
      }
    });
  }
});

test('every pnpm dry-run package file is inspected even when untracked', async () => {
  const fixture = await createFixture();
  try {
    await addCandidate(
      fixture,
      'included/untracked-copy.bin',
      fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[1])!,
      'untracked',
    );
    const result = await runGate(fixture);
    assert.notEqual(result.code, 0);
    assert.match(result.stderr, /exact-private-bytes/i);
  } finally {
    await cleanupFixture(fixture);
  }
});

test('arbitrary offset raw excerpts and normalized semantic derivatives fail', async (context) => {
  const cases: readonly Readonly<{
    name: string;
    candidate: (fixture: Fixture) => string | Buffer;
    category: RegExp;
  }>[] = [
    {
      name: 'arbitrary unsampled offset',
      candidate: (fixture) => fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!.subarray(137, 169),
      category: /source-derived/i,
    },
    {
      name: 'reordered partial card record',
      candidate: reorderedProtectedCard,
      category: /semantic|source-derived/i,
    },
    {
      name: 'normalized visible HTML text',
      candidate: () =>
        'VISIBLE private heading 1 alpha beta gamma secret passage 1 visible-boundary-text visible-boundary-text visible-boundary-text',
      category: /semantic|source-derived/i,
    },
  ];
  for (const scenario of cases) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      try {
        await addCandidate(fixture, 'included/' + scenario.name.replaceAll(' ', '-') + '.txt', scenario.candidate(fixture));
        const result = await runGate(fixture);
        assert.notEqual(result.code, 0);
        assert.match(result.stderr, scenario.category);
        assert.equal(result.stderr.includes(fixture.primaryRoot), false);
      } finally {
        await cleanupFixture(fixture);
      }
    });
  }
});

test('exact declared public provenance literals pass while one-byte extensions and private prose fail', async (context) => {
  const cases = [
    { name: 'exact public metadata', candidate: (fixture: Fixture) => '`' + fixture.publicProvenanceMarker + '`', passes: true },
    {
      name: 'embedded exact public metadata',
      candidate: (fixture: Fixture) => '<title>' + fixture.publicProvenanceMarker + '</title>',
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
      candidate: (fixture: Fixture) => '`' + fixture.publicProvenanceMarker + 'x`',
      passes: false,
    },
    {
      name: 'embedded one-byte-extended public metadata',
      candidate: (fixture: Fixture) => '<title>' + fixture.publicProvenanceMarker + 'x</title>',
      passes: false,
    },
    {
      name: 'one-byte-extended Markdown provenance URL',
      candidate: () => '[CITED: ' + SYNTHETIC_PUBLIC_PROVENANCE_URL + 'x]',
      passes: false,
    },
    {
      name: 'private prose',
      candidate: (fixture: Fixture) => fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!.subarray(137, 169),
      passes: false,
    },
  ] as const;
  for (const scenario of cases) {
    await context.test(scenario.name, async () => {
      const fixture = await createFixture();
      try {
        await addCandidate(fixture, 'included/' + scenario.name.replaceAll(' ', '-') + '.txt', scenario.candidate(fixture));
        const result = await runGate(fixture);
        if (scenario.passes) {
          assert.equal(result.code, 0, result.stderr);
          assert.equal(result.stdout, 'Private authority boundary verified.\n');
        } else {
          assert.notEqual(result.code, 0);
          assert.match(result.stderr, /source-derived/i);
        }
      } finally {
        await cleanupFixture(fixture);
      }
    });
  }
});

test('encoded and escaped source excerpts and card records fail on every history worktree index and package surface', async (context) => {
  const surfaces: readonly Surface[] = ['reachable-history', 'worktree', 'index', 'package'];
  for (const surface of surfaces) {
    for (const encoding of ['escaped source', 'escaped card'] as const) {
      await context.test(encoding + ' on ' + surface, async () => {
        const fixture = await createFixture();
        try {
          const raw = encoding === 'escaped source'
            ? fixture.sourceBytes.get(PRIVATE_AUTHORITY_SOURCE_PATHS[0])!.subarray(137, 169).toString('utf8')
            : reorderedProtectedCard(fixture);
          const candidate = encoding === 'escaped source'
            ? 'const protectedExcerpt = ' + JSON.stringify(raw) + ';'
            : "const protectedCard = '" + raw + "';";
          await addSurfaceCandidate(fixture, surface, encoding.replaceAll(' ', '-') + '-' + surface, candidate);
          const result = await runGate(fixture);
          assert.notEqual(result.code, 0, encoding + ' unexpectedly passed on ' + surface);
          assert.match(result.stderr, /source-derived|semantic/i);
          assert.match(result.stderr, new RegExp(surface));
          assert.equal(result.stderr.includes(fixture.primaryRoot), false);
          assert.equal(result.stderr.includes(fixture.backupRoot), false);
          assert.equal(result.stderr.includes('private.invalid'), false);
        } finally {
          await cleanupFixture(fixture);
        }
      });
    }
  }
});

test('locator escaped and case-varied Windows forms fail on every surface', async (context) => {
  const locator = 'C:\\Private\\Authority\\Rulebook';
  const mixedCase = 'c:\\pRIVATE\\AUTHORITY\\rULEBOOK';
  const candidates = {
    'JSON escaped': 'const locator = ' + JSON.stringify(mixedCase) + ';',
    'TypeScript escaped': "const locator = '" + mixedCase.replaceAll('\\', '\\\\') + "';",
    'slash-normalized case-varied': mixedCase.replaceAll('\\', '/'),
  } as const;
  for (const surface of ['reachable-history', 'worktree', 'index', 'package'] as const) {
    for (const [encoding, candidate] of Object.entries(candidates)) {
      await context.test(encoding + ' on ' + surface, async () => {
        const fixture = await createFixture(locator);
        try {
          await addSurfaceCandidate(fixture, surface, 'locator-' + encoding.replaceAll(' ', '-') + '-' + surface, candidate);
          const result = await runGate(fixture);
          assert.notEqual(result.code, 0, encoding + ' unexpectedly passed on ' + surface);
          assert.match(result.stderr, /private-locator/i);
          assert.match(result.stderr, new RegExp(surface));
          assert.equal(result.stderr.includes(locator), false);
          assert.equal(result.stderr.includes(mixedCase), false);
        } finally {
          await cleanupFixture(fixture);
        }
      });
    }
  }
});

test('POSIX locator matching remains exact and case-sensitive', async (context) => {
  const locator = '/Private/Authority/Rulebook';
  await context.test('exact locator fails', async () => {
    const fixture = await createFixture(locator);
    try {
      await addCandidate(fixture, 'included/posix-exact.txt', locator);
      const result = await runGate(fixture);
      assert.notEqual(result.code, 0);
      assert.match(result.stderr, /private-locator/i);
    } finally {
      await cleanupFixture(fixture);
    }
  });
  await context.test('case-varied locator passes', async () => {
    const fixture = await createFixture(locator);
    try {
      await addCandidate(fixture, 'included/posix-case-varied.txt', locator.toLowerCase());
      const result = await runGate(fixture);
      assert.equal(result.code, 0, result.stderr);
      assert.equal(result.stdout, 'Private authority boundary verified.\n');
    } finally {
      await cleanupFixture(fixture);
    }
  });
});

test('boundary verifier source remains standard-library only', async () => {
  const source = await readFile(SCRIPT_PATH, 'utf8');
  assert.doesNotMatch(source, /from ['"](?!node:)[^./]/);
});
