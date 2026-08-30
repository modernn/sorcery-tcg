import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import test from 'node:test';

import { runBounded } from '../helpers/bounded-process.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const REVISION_ID = 'official-2026-08-20';
const STABLE_ID = `bundle:${REVISION_ID}`;
const LOCK_PATH = join(REPOSITORY_ROOT, '.local', 'authority', 'locks', REVISION_ID, 'source-set-lock.json');
const INPUT_ROOT = join(REPOSITORY_ROOT, '.local', 'authority', 'build-inputs', REVISION_ID, 'primary');
const BOUNDARY_SCRIPT = join(REPOSITORY_ROOT, 'scripts', 'verify-private-authority-boundary.ts');
const FINAL_REVISION_ID = 'official-2026-08-27-v3';
const FINAL_LOCK_PATH = join(REPOSITORY_ROOT, '.local', 'authority', 'locks', FINAL_REVISION_ID, 'source-set-lock.json');
const FINAL_RECEIPT_PATH = join(REPOSITORY_ROOT, 'data', 'authority', 'receipts', `${FINAL_REVISION_ID}.json`);

type PrivateEntry = Readonly<{ relativePath: string; byteHash: string }>;
type PrivateLock = Readonly<{
  primaryRoot: string;
  backupRoot: string;
  entries: readonly PrivateEntry[];
}>;

function sha256(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex');
}

function relativePath(path: string): string {
  const value = relative(REPOSITORY_ROOT, path).replaceAll('\\', '/');
  assert.ok(value !== '..' && !value.startsWith('../') && !isAbsolute(value));
  return value;
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

test('only the fixed offline manual importer exists and no authority acquisition or API surface is added', async () => {
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
    assert.doesNotMatch(name + ':' + command, /(?:fetch|poll|scrape|crawl|latest)/i);
  }

  const sourceFiles = (await readdir(join(REPOSITORY_ROOT, 'src'), { recursive: true, withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith('.ts'))
    .map((entry) => join(entry.parentPath, entry.name));
  for (const path of sourceFiles) {
    const source = await readFile(path, 'utf8');
    const candidatePath = relativePath(path);
    assert.doesNotMatch(source, /collect-private-authority/i, candidatePath);
    // The local gameplay prototypes intentionally provide browser transport, not authority acquisition.
    if (!candidatePath.startsWith('src/prototype/')) {
      assert.doesNotMatch(source, /(?:globalThis\.)?fetch\s*\(|createServer\s*\(|\.listen\s*\(/i, candidatePath);
    }
    assert.doesNotMatch(source, /(?:mutable[-_ ]?latest|latest[-_ ]?(?:bundle|revision|cards))/i, candidatePath);
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
    '.local/authority/manual-inbox/official-2026-08-27-v3',
    'ImportManualInbox',
    'AcknowledgePrivateUseRisk',
    'user-provided-manual-download',
    'phase-01-20260827-manual-provision-1',
  ]) assert.ok(collectorLower.includes(required.toLowerCase()));
  assert.equal(/InvocationName[^\r\n]+['"]\.['"]/i.test(collector), true, 'collector dot-source guard missing');
  assert.doesNotMatch(
    collector,
    /\b(?:HttpClient|HttpWebRequest|Invoke-WebRequest|Invoke-RestMethod|WebClient|TcpClient|Start-BitsTransfer|Start-Process|curl(?:\.exe)?|fetch|Socket|Invoke-PrivateAuthorityTransport|Invoke-BoundedHttpToFile|New-PrivateAuthorityAcquisitionContext)\b|node:(?:http|https|net|tls)|Start-Sleep|while\s*\(\s*\$true|\.png|\.jpe?g/i,
    'manual importer exposes network, retry, polling, or artwork-acquisition surface',
  );


  const collectorTests = await readFile(
    join(REPOSITORY_ROOT, 'tests', 'authority', 'private-authority-collector.test.ts'),
    'utf8',
  );
  for (const required of ['manual intake', 'without transport', 'linked', 'production', 'acknowledgment']) {
    assert.ok(
      collectorTests.toLowerCase().includes(required),
      'manual-intake suite missing ' + required + ' evidence',
    );
  }
});

test('historical and final v3 locks pass one production boundary scan', async () => {
  await Promise.all([
    readFile(LOCK_PATH, 'utf8'),
    readFile(FINAL_LOCK_PATH, 'utf8'),
    readFile(FINAL_RECEIPT_PATH, 'utf8'),
  ]);
  const lockArguments = [
    '--lock', LOCK_PATH,
    '--lock', FINAL_LOCK_PATH,
  ];
  assert.equal(lockArguments[1], LOCK_PATH, 'historical lock is not exercised');
  assert.equal(lockArguments[3], FINAL_LOCK_PATH, 'final v3 lock is not exercised');
  const result = await runBounded(
    process.execPath,
    [BOUNDARY_SCRIPT, '--repository-root', REPOSITORY_ROOT, ...lockArguments],
    REPOSITORY_ROOT,
    { maxBuffer: 268_435_456, timeout: 180_000 },
  );
  assert.equal(result.code, 0, 'production private authority boundary scanner failed');
  assert.equal(result.stdout, 'Private authority boundary verified.\n');
  assert.equal(result.stderr, '');
});
