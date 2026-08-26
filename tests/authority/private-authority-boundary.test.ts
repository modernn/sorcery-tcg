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

type Fixture = Readonly<{
  sandbox: string;
  repositoryRoot: string;
  primaryRoot: string;
  backupRoot: string;
  lockPath: string;
  sourceBytes: ReadonlyMap<string, Buffer>;
}>;

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

async function createFixture(): Promise<Fixture> {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-boundary-'));
  const repositoryRoot = join(sandbox, 'repository');
  const primaryRoot = join(repositoryRoot, '.local', 'authority', 'inputs', 'synthetic', 'primary');
  const backupRoot = join(sandbox, 'backup');
  const lockPath = join(repositoryRoot, '.local', 'authority', 'locks', 'synthetic', 'source-set-lock.json');
  const sourceBytes = new Map<string, Buffer>();
  const entries = [];
  await mkdir(repositoryRoot, { recursive: true });
  for (const [sourceIndex, relativePath] of PRIVATE_AUTHORITY_SOURCE_PATHS.entries()) {
    const bytes = Buffer.alloc(96);
    for (let index = 0; index < bytes.length; index += 1) bytes[index] = 65 + ((sourceIndex * 7 + index) % 26);
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
        privateLocatorEvidence: 'https://private.invalid/rulebook-locator',
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
  const fixture = { sandbox, repositoryRoot, primaryRoot, backupRoot, lockPath, sourceBytes };
  await git(fixture, 'init', '--quiet');
  await git(fixture, 'config', 'user.email', 'boundary@example.invalid');
  await git(fixture, 'config', 'user.name', 'Boundary Fixture');
  await git(fixture, 'add', '.gitignore', 'package.json', 'safe-metadata.json');
  await git(fixture, 'commit', '--quiet', '-m', 'safe fixture');
  return fixture;
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
      category: /source-derived-marker/i,
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

test('boundary verifier source remains standard-library only', async () => {
  const source = await readFile(SCRIPT_PATH, 'utf8');
  assert.doesNotMatch(source, /from ['"](?!node:)[^./]/);
});
