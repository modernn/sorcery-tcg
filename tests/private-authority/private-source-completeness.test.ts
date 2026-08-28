import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, readdir, realpath } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';
import test from 'node:test';

import {
  PRIVATE_AUTHORITY_SOURCE_PATHS,
  verifyPrivateSourceSet,
  type PrivateAuthoritySourceEntry,
} from '../../src/authority/private-source-set.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const REVISION_ID = 'official-2026-08-27-v3';
const LOCK_PATH = join(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'locks',
  REVISION_ID,
  'source-set-lock.json',
);
const INBOX_ROOT = join(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'manual-inbox',
  REVISION_ID,
);
const COLLECTOR_PATH = join(REPOSITORY_ROOT, 'scripts', 'collect-private-authority.ps1');
const MANUAL_METHOD = 'user-provided-manual-download';
const MANUAL_REFERENCE = 'phase-01-20260827-manual-provision-1';
const EXPECTED_SOURCES = Object.freeze({
  'rulebook/rulebook-current.pdf': {
    url: 'https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update',
    mediaType: 'application/pdf',
  },
  'formats/constructed-current.html': {
    url: 'https://sorcerytcg.com/constructed',
    mediaType: 'text/html',
  },
  'codex/codex-current.html': {
    url: 'https://curiosa.io/codex',
    mediaType: 'text/html',
  },
  'codex/faqs-current.html': {
    url: 'https://curiosa.io/faqs',
    mediaType: 'text/html',
  },
  'codex/changelog-current.html': {
    url: 'https://curiosa.io/codex/changelog',
    mediaType: 'text/html',
  },
  'updates/card-updates-2025.html': {
    url: 'https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025',
    mediaType: 'text/html',
  },
  'cards/cards.raw.json': {
    url: 'https://api.sorcerytcg.com/api/cards',
    mediaType: 'application/json',
  },
} as const);

type PrivateLock = Readonly<{
  schemaVersion: number;
  acquisitionMethod: string;
  authorizationReference: string;
  primaryRoot: string;
  backupRoot: string;
  entries: readonly PrivateAuthoritySourceEntry[];
  sourceSetRootHash: `sha256:${string}`;
  operatingAcknowledgment: Readonly<{
    scope: string;
    noRedistributionReleaseHostingUploadOrArtwork: boolean;
    apiTermsRobotsConflictAndPrivateUseRiskAccepted: boolean;
    establishesLegalPermission: boolean;
    stopOnBlockedStatusCaptchaOrPublisherObjection: boolean;
    retryOrEvasion: boolean;
  }>;
}>;

type TreeEntry = Readonly<{
  relativePath: string;
  byteLength: number;
  byteHash: string;
}>;

type FileEvidence = Readonly<{ byteLength: number; byteHash: string }>;

function sameValue(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

async function treeMap(root: string): Promise<readonly TreeEntry[]> {
  try {
    const entries = await readdir(root, { recursive: true, withFileTypes: true });
    const files = entries
      .filter((entry) => entry.isFile())
      .map((entry) => join(entry.parentPath, entry.name))
      .sort();
    const mapped: TreeEntry[] = [];
    for (const path of files) {
      const bytes = await readFile(path);
      mapped.push({
        relativePath: relative(root, path).replaceAll('\\', '/'),
        byteLength: bytes.byteLength,
        byteHash: 'sha256:' + createHash('sha256').update(bytes).digest('hex'),
      });
    }
    return mapped;
  } catch {
    assert.fail('Fresh v3 private tree snapshot failed.');
  }
}

async function fileEvidence(path: string): Promise<FileEvidence> {
  try {
    const bytes = await readFile(path);
    return {
      byteLength: bytes.byteLength,
      byteHash: 'sha256:' + createHash('sha256').update(bytes).digest('hex'),
    };
  } catch {
    assert.fail('Fresh v3 private lock snapshot failed.');
  }
}

async function readPrivateLock(): Promise<PrivateLock> {
  try {
    return JSON.parse(await readFile(LOCK_PATH, 'utf8')) as PrivateLock;
  } catch {
    assert.fail('Fresh v3 private lock is unreadable.');
  }
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
test('fresh v3 primary and backup completeness', async () => {
  const lock = await readPrivateLock();
  assert.ok(
    lock.schemaVersion === 1 &&
      lock.acquisitionMethod === MANUAL_METHOD &&
      lock.authorizationReference === MANUAL_REFERENCE,
    'Fresh v3 manual lock identity is invalid.',
  );
  const expectedPrimary = join(
    REPOSITORY_ROOT,
    '.local',
    'authority',
    'inputs',
    REVISION_ID,
    'primary',
  );
  const expectedBackup = resolve(
    REPOSITORY_ROOT,
    '..',
    'sorcery-tcg-authority-backup-' + REVISION_ID,
  );
  assert.ok(
    resolve(lock.primaryRoot) === expectedPrimary && resolve(lock.backupRoot) === expectedBackup,
    'Fresh v3 root identity is invalid.',
  );
  try {
    assert.ok(
      await realpath(lock.primaryRoot) !== await realpath(lock.backupRoot),
      'Fresh v3 roots are not independent.',
    );
  } catch {
    assert.fail('Fresh v3 root identity verification failed.');
  }
  assert.ok(
    sameValue(
      lock.entries.map(({ relativePath }) => relativePath).sort(),
      [...PRIVATE_AUTHORITY_SOURCE_PATHS].sort(),
    ),
    'Fresh v3 source path set is invalid.',
  );
  for (const entry of lock.entries) {
    const expected = EXPECTED_SOURCES[entry.relativePath];
    assert.ok(
      entry.url === expected.url && entry.mediaType === expected.mediaType,
      'Fresh v3 public source descriptor is invalid.',
    );
  }
  assert.ok(
    sameValue(lock.operatingAcknowledgment, {
      scope: 'private-local-noncommercial',
      noRedistributionReleaseHostingUploadOrArtwork: true,
      apiTermsRobotsConflictAndPrivateUseRiskAccepted: true,
      establishesLegalPermission: false,
      stopOnBlockedStatusCaptchaOrPublisherObjection: true,
      retryOrEvasion: false,
    }),
    'Fresh v3 operating acknowledgment is invalid.',
  );

  const inboxBefore = await treeMap(INBOX_ROOT);
  const primaryBefore = await treeMap(lock.primaryRoot);
  const backupBefore = await treeMap(lock.backupRoot);
  const lockBefore = await fileEvidence(LOCK_PATH);
  assert.ok(
    sameValue(inboxBefore, primaryBefore) && sameValue(primaryBefore, backupBefore),
    'Fresh v3 source trees differ.',
  );

  const http = await import('node:http');
  const https = await import('node:https');
  const net = await import('node:net');
  const sentinel = installNetworkSentinels(http, https, net);
  try {
    const verified = await verifyPrivateSourceSet({
      primaryRoot: lock.primaryRoot,
      backupRoot: lock.backupRoot,
      repositoryRoot: REPOSITORY_ROOT,
      entries: lock.entries,
    });
    assert.ok(
      verified.sourceSetRootHash === lock.sourceSetRootHash &&
        verified.entries.length === PRIVATE_AUTHORITY_SOURCE_PATHS.length,
      'Fresh v3 source-set verification result is invalid.',
    );
    assert.ok(sentinel.calls() === 0, 'Fresh v3 in-process verification attempted network access.');
  } catch {
    assert.fail('Fresh v3 in-process verification failed.');
  } finally {
    sentinel.restore();
  }

  const shared = spawnSync(
    'pwsh',
    [
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      COLLECTOR_PATH,
      '-VerifyExistingRoots',
      '-LockPath',
      LOCK_PATH,
    ],
    {
      cwd: REPOSITORY_ROOT,
      encoding: 'utf8',
      maxBuffer: 1_048_576,
      timeout: 30_000,
      windowsHide: true,
    },
  );
  assert.ok(
    shared.status === 0 && shared.signal === null,
    'Fresh v3 shared verification process failed.',
  );
  assert.ok(
    shared.stdout.replaceAll('\r\n', '\n') === 'Private authority existing roots verified.\n' &&
      shared.stderr === '',
    'Fresh v3 shared verification output was not sanitized.',
  );
  assert.ok(
    sameValue(await treeMap(INBOX_ROOT), inboxBefore) &&
      sameValue(await treeMap(lock.primaryRoot), primaryBefore) &&
      sameValue(await treeMap(lock.backupRoot), backupBefore) &&
      sameValue(await fileEvidence(LOCK_PATH), lockBefore),
    'Fresh v3 evidence changed during verification.',
  );
});