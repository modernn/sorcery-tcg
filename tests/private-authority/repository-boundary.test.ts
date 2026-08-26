import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, extname, isAbsolute, join, relative, resolve } from 'node:path';
import test from 'node:test';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const REVISION_ID = 'official-2026-08-20';
const STABLE_ID = `bundle:${REVISION_ID}`;
const LOCK_PATH = join(REPOSITORY_ROOT, '.local', 'authority', 'locks', REVISION_ID, 'source-set-lock.json');
const INPUT_ROOT = join(REPOSITORY_ROOT, '.local', 'authority', 'build-inputs', REVISION_ID, 'primary');
const SELECTED_REVISION = join(REPOSITORY_ROOT, '.local', 'authority', 'revisions', REVISION_ID);
const RECEIPT_PATH = join(REPOSITORY_ROOT, 'data', 'authority', 'receipts', `${REVISION_ID}.json`);
const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;

type Candidate = Readonly<{ path: string; bytes: Buffer; surface: string }>;
type PackResult = Readonly<{ files?: readonly Readonly<{ path?: string }>[] }>;
type PrivateEntry = Readonly<{ relativePath: string; byteHash: string }>;
type PrivateLock = Readonly<{
  primaryRoot: string;
  backupRoot: string;
  entries: readonly PrivateEntry[];
}>;

function commandBytes(command: string, arguments_: readonly string[]): Buffer {
  return execFileSync(command, arguments_, {
    cwd: REPOSITORY_ROOT,
    encoding: 'buffer',
    maxBuffer: 1_073_741_824,
    windowsHide: true,
  });
}

function sha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex');
}

function relativePath(path: string): string {
  const value = relative(REPOSITORY_ROOT, path).replaceAll('\\', '/');
  assert.ok(value !== '..' && !value.startsWith('../') && !isAbsolute(value));
  return value;
}

function gitBlob(hash: string, path: string, surface: string): Candidate {
  return { path, bytes: commandBytes('git', ['cat-file', 'blob', hash]), surface };
}

function reachableCandidates(): readonly Candidate[] {
  const output = commandBytes('git', ['rev-list', '--objects', '--all']).toString('utf8');
  const candidates: Candidate[] = [];
  for (const line of output.split(/\r?\n/)) {
    if (line === '') continue;
    const separator = line.indexOf(' ');
    if (separator < 0) continue;
    const hash = line.slice(0, separator);
    const path = line.slice(separator + 1);
    if (commandBytes('git', ['cat-file', '-t', hash]).toString('utf8').trim() !== 'blob') continue;
    candidates.push(gitBlob(hash, path, 'reachable-history'));
  }
  return candidates;
}

async function worktreeCandidates(): Promise<readonly Candidate[]> {
  const output = commandBytes('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z']).toString('utf8');
  const candidates: Candidate[] = [];
  for (const path of output.split('\0').filter(Boolean)) {
    const absolute = resolve(REPOSITORY_ROOT, path);
    const metadata = await import('node:fs/promises').then(({ stat }) => stat(absolute).catch(() => null));
    if (metadata?.isFile() === true) {
      candidates.push({ path, bytes: await readFile(absolute), surface: 'worktree' });
    }
  }
  return candidates;
}

function indexCandidates(): readonly Candidate[] {
  const output = commandBytes('git', ['ls-files', '--stage', '-z']).toString('utf8');
  const candidates: Candidate[] = [];
  for (const record of output.split('\0').filter(Boolean)) {
    const match = /^\d+ ([0-9a-f]+) \d+\t([\s\S]+)$/.exec(record);
    assert.ok(match, 'Git index record must be parseable');
    candidates.push(gitBlob(match[1]!, match[2]!, 'index'));
  }
  return candidates;
}

async function packageCandidates(): Promise<readonly Candidate[]> {
  const command = process.platform === 'win32' ? (process.env.ComSpec ?? 'cmd.exe') : 'pnpm';
  const arguments_ = process.platform === 'win32'
    ? ['/d', '/s', '/c', 'pnpm.cmd', 'pack', '--dry-run', '--json']
    : ['pack', '--dry-run', '--json'];
  const parsed = JSON.parse(execFileSync(command, arguments_, {
    cwd: REPOSITORY_ROOT,
    encoding: 'utf8',
    maxBuffer: 1_073_741_824,
    windowsHide: true,
  })) as PackResult | readonly PackResult[];
  const results = Array.isArray(parsed) ? parsed : [parsed];
  const candidates: Candidate[] = [];
  for (const result of results) {
    assert.ok(Array.isArray(result.files), 'pnpm package file list must be present');
    for (const file of result.files) {
      assert.equal(typeof file.path, 'string');
      const absolute = resolve(REPOSITORY_ROOT, file.path!);
      candidates.push({ path: file.path!, bytes: await readFile(absolute), surface: 'package' });
    }
  }
  return candidates;
}

function hasArtworkSignature(bytes: Buffer): boolean {
  return bytes.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) ||
    (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) ||
    bytes.subarray(0, 6).toString('ascii') === 'GIF87a' ||
    bytes.subarray(0, 6).toString('ascii') === 'GIF89a' ||
    (bytes.subarray(0, 4).toString('ascii') === 'RIFF' && bytes.subarray(8, 12).toString('ascii') === 'WEBP') ||
    /^\s*<svg\b/i.test(bytes.subarray(0, 256).toString('utf8'));
}

function sourceMarkers(bytes: Buffer): readonly Buffer[] {
  if (bytes.length < 64) return [];
  const markerLength = 48;
  const offsets = [0, Math.floor((bytes.length - markerLength) / 2), bytes.length - markerLength];
  return [...new Set(offsets)].map((offset) => bytes.subarray(offset, offset + markerLength));
}

function inspectCandidate(
  candidate: Candidate,
  exactPrivateHashes: ReadonlySet<string>,
  markers: readonly Buffer[],
  privateLocators: readonly Buffer[],
): void {
  const path = candidate.path.replaceAll('\\', '/');
  const fail = (category: string): never => {
    throw new Error(`boundary:${category}:${candidate.surface}:${path}`);
  };
  if (path === '.local/authority' || path.startsWith('.local/authority/')) fail('private-path');
  if (/\.(?:avif|bmp|gif|ico|jpe?g|png|svg|tiff?|webp)$/i.test(path)) fail('artwork-path');
  if (hasArtworkSignature(candidate.bytes)) fail('artwork-signature');
  if (exactPrivateHashes.has(sha256(candidate.bytes))) fail('exact-private-bytes');
  if (markers.some((marker) => candidate.bytes.includes(marker))) fail('private-content-marker');
  if (privateLocators.some((locator) => candidate.bytes.includes(locator))) fail('private-locator');

  const prefix = candidate.bytes.subarray(0, 512).toString('utf8');
  if (candidate.bytes.length > 1_000 && /^(?:%PDF-|\s*<!doctype html|\s*<html\b)/i.test(prefix)) {
    fail('publisher-document');
  }
  if (
    candidate.bytes.length > 500_000 &&
    extname(path).toLowerCase() === '.json' &&
    /"(?:sourceCardId|printingSlugs|rulesText)"/.test(prefix + candidate.bytes.subarray(-512).toString('utf8'))
  ) fail('full-card-corpus');
  if (
    candidate.bytes.length > 100_000 &&
    /\.(?:json|jsonl|csv|sqlite|db)$/i.test(path) &&
    /(?:contested-realms|spells\.bar|sorcery-registry)/i.test(candidate.bytes.toString('utf8'))
  ) fail('copied-community-corpus');
}

function installNetworkSentinels(
  http: typeof import('node:http'),
  https: typeof import('node:https'),
  net: typeof import('node:net'),
): Readonly<{ calls: () => number; restore: () => void }> {
  let count = 0;
  const blocked = (): never => {
    count += 1;
    throw new Error('network access is forbidden in private authority validation');
  };
  const originalFetch = globalThis.fetch;
  const httpDefault = ((http as unknown as { default?: Record<string, unknown> }).default ?? http) as unknown as Record<string, unknown>;
  const httpsDefault = ((https as unknown as { default?: Record<string, unknown> }).default ?? https) as unknown as Record<string, unknown>;
  const netDefault = ((net as unknown as { default?: Record<string, unknown> }).default ?? net) as unknown as Record<string, unknown>;
  const originals = {
    httpRequest: httpDefault.request,
    httpGet: httpDefault.get,
    httpsRequest: httpsDefault.request,
    httpsGet: httpsDefault.get,
    netConnect: netDefault.connect,
    netCreateConnection: netDefault.createConnection,
  };
  globalThis.fetch = blocked as typeof fetch;
  httpDefault.request = blocked;
  httpDefault.get = blocked;
  httpsDefault.request = blocked;
  httpsDefault.get = blocked;
  netDefault.connect = blocked;
  netDefault.createConnection = blocked;
  return {
    calls: () => count,
    restore: () => {
      globalThis.fetch = originalFetch;
      httpDefault.request = originals.httpRequest;
      httpDefault.get = originals.httpGet;
      httpsDefault.request = originals.httpsRequest;
      httpsDefault.get = originals.httpsGet;
      netDefault.connect = originals.netConnect;
      netDefault.createConnection = originals.netCreateConnection;
    },
  };
}

async function cleanTemp(root: string): Promise<void> {
  const resolved = resolve(root);
  assert.equal(dirname(resolved), resolve(tmpdir()));
  assert.match(basename(resolved), /^sorcery-private-network-/);
  await rm(resolved, { recursive: true, force: true });
}

test('authority helper adapter import and validation remain offline under network sentinels', async () => {
  const http = await import('node:http');
  const https = await import('node:https');
  const net = await import('node:net');
  const sentinel = installNetworkSentinels(http, https, net);
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-network-'));
  try {
    const [
      { verifyPrivateSourceSet },
      { importAuthority },
      { validateAuthorityBundle },
      { adaptOfficialCardApiSnapshot },
      { normalizeCards },
      { identityHash },
    ] = await Promise.all([
      import('../../src/authority/private-source-set.ts'),
      import('../../src/commands/import-authority.ts'),
      import('../../src/authority/validate-bundle.ts'),
      import('../../src/authority/official-card-api-adapter.ts'),
      import('../../src/authority/normalize-cards.ts'),
      import('../../src/authority/hash.ts'),
    ]);
    const lock = JSON.parse(await readFile(LOCK_PATH, 'utf8')) as PrivateLock & {
      sourceSetRootHash: string;
      entries: readonly PrivateEntry[];
    };
    const verified = await verifyPrivateSourceSet({
      primaryRoot: lock.primaryRoot,
      backupRoot: lock.backupRoot,
      repositoryRoot: REPOSITORY_ROOT,
      entries: lock.entries as never,
    });
    assert.equal(verified.sourceSetRootHash, lock.sourceSetRootHash);

    const inputLock = JSON.parse(await readFile(join(INPUT_ROOT, 'input-lock.json'), 'utf8')) as {
      inputRootHash: `sha256:${string}`;
    };
    const outputRoot = join(sandbox, 'output');
    const imported = await importAuthority({
      inputRoot: INPUT_ROOT,
      inputLockPath: 'input-lock.json',
      expectedInputRootHash: inputLock.inputRootHash,
      outputRoot,
      revisionId: REVISION_ID,
    });
    assert.equal(imported.bundleId, STABLE_ID);
    const validated = await validateAuthorityBundle(imported.revisionPath, 'bundle.json', {
      stableId: imported.bundleId,
      contentHash: imported.bundleRootHash,
    });
    assert.equal(identityHash(validated.bundle.identity), imported.bundleRootHash);

    const cardsBytes = await readFile(join(INPUT_ROOT, 'cards.raw.json'));
    const sources = JSON.parse(await readFile(join(INPUT_ROOT, 'sources.json'), 'utf8')) as {
      sources: readonly Readonly<{ byteHash: string; mediaType: string }>[];
    };
    const source = sources.sources.find((entry) => entry.byteHash === inputLockHash(cardsBytes));
    assert.ok(source && source.mediaType === 'application/json');
    const parsed = JSON.parse(cardsBytes.toString('utf8')) as unknown;
    const adapted = adaptOfficialCardApiSnapshot(parsed);
    const normalized = normalizeCards(cardsBytes, source as never);
    assert.equal(adapted.cards.length, 1_100);
    assert.equal(adapted.cards.length, normalized.identity.payload.cards.length);
    assert.equal(normalized.identity.sourceRefs[0]?.byteHash, inputLockHash(cardsBytes));
    assert.equal(sentinel.calls(), 0);
  } finally {
    sentinel.restore();
    await cleanTemp(sandbox);
  }
});

function inputLockHash(bytes: Uint8Array): `sha256:${string}` {
  return `sha256:${sha256(bytes)}`;
}

test('only the audited fixed collector exists and no runtime acquisition or public API surface is added', async () => {
  const packageDocument = JSON.parse(await readFile(join(REPOSITORY_ROOT, 'package.json'), 'utf8')) as {
    scripts: Record<string, string>;
    dependencies: Record<string, string>;
    devDependencies: Record<string, string>;
  };
  assert.deepEqual(packageDocument.dependencies, { zod: '4.4.3' });
  assert.deepEqual(Object.keys(packageDocument.devDependencies).sort(), [
    '@types/node',
    'eslint',
    'typescript',
    'typescript-eslint',
  ]);
  for (const [name, command] of Object.entries(packageDocument.scripts)) {
    if (name === 'authority:verify-private') continue;
    assert.doesNotMatch(`${name}:${command}`, /(?:fetch|poll|scrape|crawl|serve|listen|latest)/i);
  }

  const sourceFiles = (await readdir(join(REPOSITORY_ROOT, 'src'), { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith('.ts'))
    .map((entry) => join(entry.parentPath, entry.name));
  for (const path of sourceFiles) {
    const source = await readFile(path, 'utf8');
    assert.doesNotMatch(source, /collect-private-authority|(?:globalThis\.)?fetch\s*\(|createServer\s*\(|\.listen\s*\(/i, relativePath(path));
    assert.doesNotMatch(source, /(?:mutable[-_ ]?latest|latest[-_ ]?(?:bundle|revision|cards))/i, relativePath(path));
  }

  const collectorPath = join(REPOSITORY_ROOT, 'scripts', 'collect-private-authority.ps1');
  const collector = await readFile(collectorPath, 'utf8');
  const collectorLower = collector.toLowerCase();
  for (const required of [
    'rulebook/rulebook-current.pdf',
    'formats/constructed-current.html',
    'codex/codex-current.html',
    'codex/faqs-current.html',
    'codex/changelog-current.html',
    'updates/card-updates-2025.html',
    'cards/cards.raw.json',
    'AcknowledgePrivateUseRisk',
    'CAPTCHA',
    '401',
    '403',
    '429',
  ]) assert.ok(collectorLower.includes(required.toLowerCase()));
  assert.equal(/InvocationName[^\r\n]+['"]\.['"]/i.test(collector), true, 'collector dot-source guard missing');
  assert.equal(
    /(?:Start-Sleep|while\s*\(\s*\$true|\.png|\.jpe?g)/i.test(collector),
    false,
    'collector exposes retry or image-acquisition surface',
  );

  const collectorTests = await readFile(
    join(REPOSITORY_ROOT, 'tests', 'authority', 'private-authority-collector.test.ts'),
    'utf8',
  );
  for (const required of ['dot-sourcing', 'seven', 'artwork', 'retry', 'production']) {
    assert.ok(collectorTests.toLowerCase().includes(required), `collector loopback suite missing ${required} evidence`);
  }
});

test('reachable history worktree index and package contain no private authority bytes or locators', async () => {
  const lock = JSON.parse(await readFile(LOCK_PATH, 'utf8')) as PrivateLock;
  assert.equal(lock.entries.length, 7);
  const exactPrivateHashes = new Set<string>();
  const markers: Buffer[] = [];
  for (const root of [lock.primaryRoot, lock.backupRoot]) {
    for (const entry of lock.entries) {
      assert.match(entry.byteHash, HASH_PATTERN);
      const bytes = await readFile(resolve(root, ...entry.relativePath.split('/')));
      assert.equal(inputLockHash(bytes), entry.byteHash);
      exactPrivateHashes.add(sha256(bytes));
      markers.push(...sourceMarkers(bytes));
    }
  }
  for (const entry of await readdir(SELECTED_REVISION, { recursive: true, withFileTypes: true })) {
    if (!entry.isFile()) continue;
    exactPrivateHashes.add(sha256(await readFile(join(entry.parentPath, entry.name))));
  }
  const privateLocators = [...new Set([
    lock.primaryRoot,
    lock.backupRoot,
    lock.primaryRoot.replaceAll('\\', '/'),
    lock.backupRoot.replaceAll('\\', '/'),
  ])].map((value) => Buffer.from(value));

  const candidates = [
    ...reachableCandidates(),
    ...(await worktreeCandidates()),
    ...indexCandidates(),
    ...(await packageCandidates()),
  ];
  assert.ok(candidates.some(({ surface }) => surface === 'reachable-history'));
  assert.ok(candidates.some(({ surface }) => surface === 'worktree'));
  assert.ok(candidates.some(({ surface }) => surface === 'index'));
  assert.ok(candidates.some(({ surface }) => surface === 'package'));
  for (const candidate of candidates) {
    inspectCandidate(candidate, exactPrivateHashes, markers, privateLocators);
  }
  assert.equal(commandBytes('git', ['ls-files', '.local/authority']).byteLength, 0);

  const receipt = JSON.parse(await readFile(RECEIPT_PATH, 'utf8')) as Record<string, unknown>;
  assert.equal(receipt.gitContainsCorpus, false);
  assert.equal(JSON.stringify(receipt).includes(lock.primaryRoot), false);
  assert.equal(JSON.stringify(receipt).includes(lock.backupRoot), false);
});
