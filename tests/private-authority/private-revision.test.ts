import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, readdir, rename, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve } from 'node:path';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { identityHash, sha256 } from '../../src/authority/hash.ts';
import {
  PRIVATE_AUTHORITY_SOURCE_PATHS,
  verifyPrivateSourceSet,
  type PrivateAuthoritySourceEntry,
} from '../../src/authority/private-source-set.ts';
import {
  AuthorityValidationError,
  type AuthorityBundle,
  type Hash,
  type SourceRecord,
} from '../../src/authority/schemas.ts';
import { validateAuthorityBundle } from '../../src/authority/validate-bundle.ts';
import { importAuthority, type ImportAuthorityArguments } from '../../src/commands/import-authority.ts';

const REVISION_ID = 'official-2026-08-20';
const STABLE_ID = `bundle:${REVISION_ID}`;
const AGENT_METHOD = 'user-authorized-agent-run-one-shot-powershell';
const LEGACY_METHOD = 'user-run-one-shot-powershell';
const CURRENT_REFERENCE = 'quick-260825-mhh-retry-1';
const FAILED_REFERENCE = 'quick-260825-mhh';
const LOCAL_ROOT = '.local/authority';
const LOCK_PATH = `${LOCAL_ROOT}/locks/${REVISION_ID}/source-set-lock.json`;
const SELECTED_REVISION = `${LOCAL_ROOT}/revisions/${REVISION_ID}`;
const RECEIPT_PATH = `data/authority/receipts/${REVISION_ID}.json`;
const SUMMARY_PATH = '.planning/phases/01-rules-and-data-authority/01-07-SUMMARY.md';
const ZERO_HASH = `sha256:${'0'.repeat(64)}` as Hash;
const FINAL_REVISION_ID = 'official-2026-08-27-v3';
const FINAL_STABLE_ID = `bundle:${FINAL_REVISION_ID}`;
const FINAL_LOCK_PATH = `${LOCAL_ROOT}/locks/${FINAL_REVISION_ID}/source-set-lock.json`;
const FINAL_SELECTED_REVISION = `${LOCAL_ROOT}/revisions/${FINAL_REVISION_ID}`;
const FINAL_RECEIPT_PATH = `data/authority/receipts/${FINAL_REVISION_ID}.json`;
const FINAL_SUMMARY_PATH = '.planning/phases/01-rules-and-data-authority/01-15-SUMMARY.md';
const AUTHORITY_README_PATH = 'data/authority/README.md';
const FINAL_METHOD = 'user-provided-manual-download';
const FINAL_REFERENCE = 'phase-01-20260827-manual-provision-1';
const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');

type Evidence = Record<string, unknown>;
type Receipt = Readonly<{
  acquisitionMethod: string;
  authorizationReference: string;
  revisionId: string;
  stableId: string;
  bundleRootHash: Hash;
  inputRootHash: Hash;
  sourceSetRootHash: Hash;
  attestation: Readonly<Record<string, unknown>>;
}>;

type LiveLock = Readonly<{
  acquisitionMethod: string;
  authorizationReference: string;
  primaryRoot: string;
  backupRoot: string;
  sourceSetRootHash: Hash;
  entries: readonly PrivateAuthoritySourceEntry[];
  operatingAcknowledgment: Readonly<Record<string, unknown>>;
}>;

type InputLock = Readonly<{
  schemaVersion: 1;
  files: readonly Readonly<{
    byteHash: Hash;
    relativePath: 'cards.raw.json' | 'formats.json' | 'sources.json';
    storageMode: 'manifest-only' | 'stored';
  }>[];
  inputRootHash: Hash;
}>;

type TreeEntry = Readonly<{ relativePath: string; byteLength: number; byteHash: Hash }>;
type FileEvidence = Readonly<{ byteLength: number; byteHash: Hash }>;

function exactKeys(value: Evidence, keys: readonly string[]): boolean {
  return JSON.stringify(Object.keys(value).sort()) === JSON.stringify([...keys].sort());
}

function validEvidence(lock: Evidence, consumed?: Evidence): boolean {
  const hasReference = Object.hasOwn(lock, 'authorizationReference');
  if (lock.acquisitionMethod === LEGACY_METHOD) return !hasReference && consumed === undefined;
  if (lock.acquisitionMethod !== AGENT_METHOD || !hasReference) return false;
  const reference = lock.authorizationReference;
  if (reference !== FAILED_REFERENCE && reference !== CURRENT_REFERENCE) return false;
  if (consumed === undefined || !exactKeys(consumed, [
    'schemaVersion',
    'revisionId',
    'acquisitionMethod',
    'authorizationReference',
    'consumedAt',
  ])) return false;
  return consumed.schemaVersion === 1 &&
    consumed.revisionId === REVISION_ID &&
    consumed.acquisitionMethod === lock.acquisitionMethod &&
    consumed.authorizationReference === reference &&
    typeof consumed.consumedAt === 'string';
}

function validCurrentPublication(lock: Evidence, consumed: Evidence, receipt: Evidence): boolean {
  return validEvidence(lock, consumed) &&
    lock.acquisitionMethod === AGENT_METHOD &&
    lock.authorizationReference === CURRENT_REFERENCE &&
    receipt.acquisitionMethod === AGENT_METHOD &&
    receipt.authorizationReference === CURRENT_REFERENCE &&
    receipt.revisionId === REVISION_ID &&
    receipt.stableId === STABLE_ID &&
    receipt.reuse !== true &&
    receipt.standingPermission !== true;
}

function consumedFor(reference = CURRENT_REFERENCE): Evidence {
  return {
    schemaVersion: 1,
    revisionId: REVISION_ID,
    acquisitionMethod: AGENT_METHOD,
    authorizationReference: reference,
    consumedAt: '2026-08-25T00:00:00.000Z',
  };
}

function validFinalEvidence(lock: Evidence, summary: string): lock is LiveLock {
  if (
    lock.acquisitionMethod !== FINAL_METHOD ||
    lock.authorizationReference !== FINAL_REFERENCE ||
    typeof lock.primaryRoot !== 'string' ||
    typeof lock.backupRoot !== 'string' ||
    !Array.isArray(lock.entries)
  ) return false;
  const expectedPrimary = resolve(REPOSITORY_ROOT, LOCAL_ROOT, 'inputs', FINAL_REVISION_ID, 'primary');
  const expectedBackup = resolve(REPOSITORY_ROOT, '..', `sorcery-tcg-authority-backup-${FINAL_REVISION_ID}`);
  return resolve(lock.primaryRoot) === expectedPrimary &&
    resolve(lock.backupRoot) === expectedBackup &&
    summary.includes('plan: "15"') &&
    summary.includes('## Self-Check: PASSED');
}

function sourceId(relativePath: string): string {
  const ids: Readonly<Record<string, string>> = {
    'cards/cards.raw.json': 'source:official-cards-2026-08-20',
    'codex/changelog-current.html': 'source:official-codex-changelog-2026-08-20',
    'codex/codex-current.html': 'source:official-codex-2026-08-20',
    'codex/faqs-current.html': 'source:official-faqs-2026-08-20',
    'formats/constructed-current.html': 'source:official-constructed-2026-08-20',
    'rulebook/rulebook-current.pdf': 'source:official-rulebook-2025-12-19',
    'updates/card-updates-2025.html': 'source:official-card-updates-2025',
  };
  const value = ids[relativePath];
  assert.ok(value, 'source path must have a fixed source ID');
  return value;
}

function rawSource(entry: PrivateAuthoritySourceEntry): SourceRecord {
  return {
    sourceId: sourceId(entry.relativePath),
    url: entry.url,
    authorityClass: 'official',
    retrievedAt: entry.retrievedAt,
    effectiveDate: entry.effectiveDate,
    mediaType: entry.mediaType,
    byteHash: entry.byteHash,
    derivation: {
      method: 'verbatim',
      parentByteHashes: [],
      notes: entry.relativePath === 'rulebook/rulebook-current.pdf'
        ? "acquired by the fixed one-shot PowerShell collector under Plan 07's closed evidence pair from the exact official release page; observed filename and retrieval time recorded in the private lock"
        : null,
    },
    licenseStatus: 'permission-required',
    storageMode: 'manifest-only',
    storagePolicy: 'prohibited',
    durableLocator: `urn:${entry.byteHash}`,
    acquisitionProcedureHash: null,
  };
}

async function writeDerivedInput(
  root: string,
  sourceRoot: string,
  entries: readonly PrivateAuthoritySourceEntry[],
): Promise<InputLock> {
  await mkdir(root, { recursive: true });
  const cardsBytes = await readFile(join(sourceRoot, 'cards', 'cards.raw.json'));
  await writeFile(join(root, 'cards.raw.json'), cardsBytes, { flag: 'wx' });

  const formatSourceId = 'source:derived-format-input-official-2026-08-20';
  const formats = {
    formats: [{
      stableId: 'format:constructed-2025-12-19',
      sourceId: formatSourceId,
      definition: {
        name: 'Constructed',
        effectiveDate: '2025-12-19',
        scope: null,
        parentFormatStableId: null,
        avatarCount: 1,
        spellbookMinimum: 60,
        atlasMinimum: 30,
        copyLimits: { ordinary: 4, exceptional: 3, elite: 2, unique: 1 },
      },
    }],
  };
  const formatsBytes = Buffer.from(canonicalJson(formats));
  await writeFile(join(root, 'formats.json'), formatsBytes, { flag: 'wx' });

  const constructed = entries.find(({ relativePath }) => relativePath === 'formats/constructed-current.html');
  assert.ok(constructed, 'constructed source metadata must exist');
  const formatHash = sha256(formatsBytes);
  const formatSource: SourceRecord = {
    sourceId: formatSourceId,
    url: constructed.url,
    authorityClass: 'official',
    retrievedAt: entries.map(({ retrievedAt }) => retrievedAt).sort().at(-1)!,
    effectiveDate: '2025-12-19',
    mediaType: 'application/json',
    byteHash: formatHash,
    derivation: {
      method: 'manual-transcription',
      parentByteHashes: entries
        .filter(({ relativePath }) => relativePath !== 'cards/cards.raw.json')
        .map(({ byteHash }) => byteHash)
        .sort(),
      notes: 'Project format input transcribed from the locked official sources.',
    },
    licenseStatus: 'permission-required',
    storageMode: 'manifest-only',
    storagePolicy: 'prohibited',
    durableLocator: `urn:${formatHash}`,
    acquisitionProcedureHash: null,
  };
  const sourcesBytes = Buffer.from(canonicalJson({
    sources: [...entries.map(rawSource), formatSource].sort((left, right) =>
      left.sourceId < right.sourceId ? -1 : left.sourceId > right.sourceId ? 1 : 0),
  }));
  await writeFile(join(root, 'sources.json'), sourcesBytes, { flag: 'wx' });

  const files: InputLock['files'] = [
    { relativePath: 'cards.raw.json', storageMode: 'manifest-only', byteHash: sha256(cardsBytes) },
    { relativePath: 'formats.json', storageMode: 'stored', byteHash: sha256(formatsBytes) },
    { relativePath: 'sources.json', storageMode: 'stored', byteHash: sha256(sourcesBytes) },
  ];
  const inputLock: InputLock = { schemaVersion: 1, files, inputRootHash: identityHash(files) };
  await writeFile(join(root, 'input-lock.json'), canonicalJson(inputLock), { flag: 'wx' });
  return inputLock;
}

async function treeMap(root: string): Promise<readonly TreeEntry[]> {
  const entries = await readdir(root, { recursive: true, withFileTypes: true });
  const files = entries.filter((entry) => entry.isFile()).map((entry) => {
    const path = join(entry.parentPath, entry.name);
    return { path, relativePath: relative(root, path).replaceAll('\\', '/') };
  });
  const mapped = await Promise.all(files.map(async ({ path, relativePath }) => {
    const bytes = await readFile(path);
    return { relativePath, byteLength: bytes.byteLength, byteHash: sha256(bytes) };
  }));
  return mapped.sort((left, right) =>
    left.relativePath < right.relativePath ? -1 : left.relativePath > right.relativePath ? 1 : 0);
}

async function fileEvidence(path: string): Promise<FileEvidence> {
  const bytes = await readFile(path);
  return { byteLength: bytes.byteLength, byteHash: sha256(bytes) };
}

async function pathExists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch (error: unknown) {
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') return false;
    throw error;
  }
}

async function installWriteOnce(candidate: string, target: string): Promise<void> {
  const candidateMap = await treeMap(candidate);
  if (await pathExists(target)) {
    assert.deepEqual(await treeMap(target), candidateMap, 'Write-once revision has different content.');
    return;
  }
  await mkdir(dirname(target), { recursive: true });
  await rename(candidate, target);
  assert.deepEqual(await treeMap(target), candidateMap);
}

function finalReceipt(lock: LiveLock, bundle: AuthorityBundle, revisionFileMapHash: Hash): JsonValue {
  const payload = bundle.identity.payload as unknown as { inputRootHash: Hash };
  return {
    schemaVersion: 1,
    acquisitionMethod: FINAL_METHOD,
    authorizationReference: FINAL_REFERENCE,
    revisionId: FINAL_REVISION_ID,
    stableId: FINAL_STABLE_ID,
    bundleRootHash: bundle.contentHash,
    inputRootHash: payload.inputRootHash,
    revisionFileMapHash,
    sourceSetRootHash: lock.sourceSetRootHash,
    sourceSets: { primary: 'fresh-private-primary', backup: 'independent-fresh-private-backup' },
    sources: lock.entries,
    gitContainsCorpus: false,
    attestation: {
      scope: 'private-local-noncommercial',
      publisherPermission: false,
      redistribution: false,
      release: false,
      hosting: false,
      thirdPartyUpload: false,
      publicApi: false,
      recurringAcquisition: false,
      artwork: false,
      commercialUse: false,
    },
  } as unknown as JsonValue;
}

async function cleanLocalCandidate(root: string): Promise<void> {
  const resolved = resolve(root);
  assert.equal(dirname(resolved), resolve(LOCAL_ROOT), 'selection cleanup must stay directly under the private authority root');
  assert.match(basename(resolved), /^\.plan-01-13-selection-/);
  await rm(resolved, { recursive: true, force: true });
}

function installNetworkSentinels(
  http: typeof import('node:http'),
  https: typeof import('node:https'),
  net: typeof import('node:net'),
): Readonly<{ calls: () => number; restore: () => void }> {
  let count = 0;
  const blocked = (): never => {
    count += 1;
    throw new Error('network access is forbidden in final authority construction');
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
  assert.equal(dirname(resolved), resolve(tmpdir()), 'cleanup root must be directly under the system temp directory');
  assert.match(basename(resolved), /^sorcery-private-revision-/);
  await rm(resolved, { recursive: true, force: true });
}

function importArgs(inputRoot: string, inputLock: InputLock, outputRoot: string, revisionId: string): ImportAuthorityArguments {
  return {
    inputRoot,
    inputLockPath: 'input-lock.json',
    expectedInputRootHash: inputLock.inputRootHash,
    outputRoot,
    revisionId,
  };
}

async function expectAuthorityFailure(run: () => Promise<unknown>, code?: string): Promise<void> {
  await assert.rejects(run, (error: unknown) => {
    assert.ok(error instanceof AuthorityValidationError);
    assert.ok(error.diagnostics.length > 0 && error.diagnostics.length <= 100);
    if (code !== undefined) assert.ok(error.diagnostics.some((diagnostic) => diagnostic.code === code));
    return true;
  });
}

test('publication evidence accepts only the two closed structures and exact current pair', () => {
  assert.equal(validEvidence({ acquisitionMethod: LEGACY_METHOD }), true);
  assert.equal(validEvidence(
    { acquisitionMethod: AGENT_METHOD, authorizationReference: CURRENT_REFERENCE },
    consumedFor(),
  ), true);

  const validLock = { acquisitionMethod: AGENT_METHOD, authorizationReference: CURRENT_REFERENCE };
  const validReceipt = {
    acquisitionMethod: AGENT_METHOD,
    authorizationReference: CURRENT_REFERENCE,
    revisionId: REVISION_ID,
    stableId: STABLE_ID,
  };
  assert.equal(validCurrentPublication(validLock, consumedFor(), validReceipt), true);

  const invalid: readonly [Evidence, Evidence | undefined, Evidence][] = [
    [{ acquisitionMethod: LEGACY_METHOD, authorizationReference: undefined }, undefined, validReceipt],
    [{ acquisitionMethod: AGENT_METHOD, authorizationReference: FAILED_REFERENCE }, consumedFor(FAILED_REFERENCE), validReceipt],
    [{ acquisitionMethod: AGENT_METHOD, authorizationReference: CURRENT_REFERENCE }, undefined, validReceipt],
    [{ acquisitionMethod: AGENT_METHOD, authorizationReference: CURRENT_REFERENCE.toUpperCase() }, consumedFor(), validReceipt],
    [{ acquisitionMethod: AGENT_METHOD, authorizationReference: ` ${CURRENT_REFERENCE}` }, consumedFor(), validReceipt],
    [{ acquisitionMethod: AGENT_METHOD, authorizationReference: `${CURRENT_REFERENCE} ` }, consumedFor(), validReceipt],
    [{ acquisitionMethod: AGENT_METHOD, authorizationReference: 'quick-arbitrary' }, consumedFor(), validReceipt],
    [{ acquisitionMethod: LEGACY_METHOD }, consumedFor(), validReceipt],
    [validLock, { ...consumedFor(), revisionId: 'wrong' }, validReceipt],
    [validLock, { ...consumedFor(), extra: true }, validReceipt],
    [validLock, consumedFor(), { ...validReceipt, reuse: true }],
    [validLock, consumedFor(), { ...validReceipt, standingPermission: true }],
  ];
  for (const [lock, consumed, receipt] of invalid) {
    assert.equal(validCurrentPublication(lock, consumed ?? {}, receipt), false);
  }
});

test('fresh v3 roots build final v3 candidates while historical evidence remains immutable', async () => {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-revision-'));
  const summary = await readFile(FINAL_SUMMARY_PATH, 'utf8');
  const lock = JSON.parse(await readFile(FINAL_LOCK_PATH, 'utf8')) as Evidence;
  assert.ok(validFinalEvidence(lock, summary), 'Fresh v3 publication evidence is invalid.');
  for (const invalid of [
    { ...lock, acquisitionMethod: AGENT_METHOD },
    { ...lock, authorizationReference: CURRENT_REFERENCE },
    { ...lock, primaryRoot: `${lock.primaryRoot}-stale` },
    { ...lock, backupRoot: lock.primaryRoot },
  ]) {
    assert.equal(validFinalEvidence(invalid, summary), false);
  }
  assert.equal(validFinalEvidence(lock, summary.replace('## Self-Check: PASSED', '')), false);

  const historicalLock = JSON.parse(await readFile(LOCK_PATH, 'utf8')) as LiveLock;
  const freshBefore = {
    primary: await treeMap(lock.primaryRoot),
    backup: await treeMap(lock.backupRoot),
    lock: await fileEvidence(FINAL_LOCK_PATH),
  };
  const historicalBefore = {
    primary: await treeMap(historicalLock.primaryRoot),
    backup: await treeMap(historicalLock.backupRoot),
    lock: await fileEvidence(LOCK_PATH),
    authorizations: await treeMap(`${LOCAL_ROOT}/authorizations`),
    revision: await treeMap(SELECTED_REVISION),
    receipt: await fileEvidence(RECEIPT_PATH),
  };
  assert.deepEqual(freshBefore.primary, freshBefore.backup);

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
    assert.equal(verified.sourceSetRootHash, lock.sourceSetRootHash);
    assert.equal(verified.entries.length, PRIVATE_AUTHORITY_SOURCE_PATHS.length);

    const primaryInput = join(sandbox, 'primary-input');
    const backupInput = join(sandbox, 'backup-input');
    const primaryLock = await writeDerivedInput(primaryInput, lock.primaryRoot, verified.entries);
    const backupLock = await writeDerivedInput(backupInput, lock.backupRoot, verified.entries);
    assert.deepEqual(await treeMap(primaryInput), await treeMap(backupInput));

    const primary = await importAuthority(importArgs(
      primaryInput,
      primaryLock,
      join(sandbox, 'primary-output'),
      FINAL_REVISION_ID,
    ));
    const backup = await importAuthority(importArgs(
      backupInput,
      backupLock,
      join(sandbox, 'backup-output'),
      FINAL_REVISION_ID,
    ));
    assert.equal(primary.bundleId, FINAL_STABLE_ID);
    assert.equal(backup.bundleId, FINAL_STABLE_ID);
    assert.equal(primary.verifiedInputRootHash, backup.verifiedInputRootHash);
    assert.equal(primary.bundleRootHash, backup.bundleRootHash);
    assert.deepEqual(await treeMap(primary.revisionPath), await treeMap(backup.revisionPath));

    const primaryBundle = JSON.parse(await readFile(join(primary.revisionPath, 'bundle.json'), 'utf8')) as AuthorityBundle;
    const backupBundle = JSON.parse(await readFile(join(backup.revisionPath, 'bundle.json'), 'utf8')) as AuthorityBundle;
    const independentlyRecomputedRoot = identityHash(primaryBundle.identity as unknown as JsonValue);
    assert.equal(primary.bundleRootHash, independentlyRecomputedRoot);
    assert.equal(identityHash(backupBundle.identity as unknown as JsonValue), independentlyRecomputedRoot);
    await validateAuthorityBundle(dirname(primary.revisionPath), `${FINAL_REVISION_ID}/bundle.json`, {
      stableId: FINAL_STABLE_ID,
      contentHash: independentlyRecomputedRoot,
    });
    await validateAuthorityBundle(dirname(backup.revisionPath), `${FINAL_REVISION_ID}/bundle.json`, {
      stableId: FINAL_STABLE_ID,
      contentHash: independentlyRecomputedRoot,
    });
    assert.equal(sentinel.calls(), 0, 'Final v3 candidate construction attempted network access.');
  } finally {
    sentinel.restore();
    assert.deepEqual(await treeMap(lock.primaryRoot), freshBefore.primary);
    assert.deepEqual(await treeMap(lock.backupRoot), freshBefore.backup);
    assert.deepEqual(await fileEvidence(FINAL_LOCK_PATH), freshBefore.lock);
    assert.deepEqual(await treeMap(historicalLock.primaryRoot), historicalBefore.primary);
    assert.deepEqual(await treeMap(historicalLock.backupRoot), historicalBefore.backup);
    assert.deepEqual(await fileEvidence(LOCK_PATH), historicalBefore.lock);
    assert.deepEqual(await treeMap(`${LOCAL_ROOT}/authorizations`), historicalBefore.authorizations);
    assert.deepEqual(await treeMap(SELECTED_REVISION), historicalBefore.revision);
    assert.deepEqual(await fileEvidence(RECEIPT_PATH), historicalBefore.receipt);
    await cleanTemp(sandbox);
  }
});

test('official-2026-08-27-v3 safe receipt write-once selection receipt root validates', async () => {
  const candidateRoot = await mkdtemp(join(resolve(LOCAL_ROOT), '.plan-01-13-selection-'));
  const summary = await readFile(FINAL_SUMMARY_PATH, 'utf8');
  const lock = JSON.parse(await readFile(FINAL_LOCK_PATH, 'utf8')) as Evidence;
  assert.ok(validFinalEvidence(lock, summary), 'Fresh v3 publication evidence is invalid.');
  assert.equal(validFinalEvidence(JSON.parse(await readFile(LOCK_PATH, 'utf8')) as Evidence, summary), false);
  const historicalLock = JSON.parse(await readFile(LOCK_PATH, 'utf8')) as LiveLock;
  const freshBefore = {
    primary: await treeMap(lock.primaryRoot),
    backup: await treeMap(lock.backupRoot),
    lock: await fileEvidence(FINAL_LOCK_PATH),
  };
  const historicalBefore = {
    primary: await treeMap(historicalLock.primaryRoot),
    backup: await treeMap(historicalLock.backupRoot),
    lock: await fileEvidence(LOCK_PATH),
    authorizations: await treeMap(`${LOCAL_ROOT}/authorizations`),
    revision: await treeMap(SELECTED_REVISION),
    receipt: await fileEvidence(RECEIPT_PATH),
  };
  const selectedBefore = await pathExists(FINAL_SELECTED_REVISION)
    ? await treeMap(FINAL_SELECTED_REVISION)
    : null;

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
    assert.equal(verified.sourceSetRootHash, lock.sourceSetRootHash);
    const input = join(candidateRoot, 'input');
    const inputLock = await writeDerivedInput(input, lock.primaryRoot, verified.entries);
    const candidate = await importAuthority(importArgs(
      input,
      inputLock,
      join(candidateRoot, 'build'),
      FINAL_REVISION_ID,
    ));
    const candidateBundle = JSON.parse(await readFile(join(candidate.revisionPath, 'bundle.json'), 'utf8')) as AuthorityBundle;
    const selectedRoot = identityHash(candidateBundle.identity as unknown as JsonValue);
    assert.equal(candidate.bundleRootHash, selectedRoot);
    await validateAuthorityBundle(dirname(candidate.revisionPath), `${FINAL_REVISION_ID}/bundle.json`, {
      stableId: FINAL_STABLE_ID,
      contentHash: selectedRoot,
    });
    await installWriteOnce(candidate.revisionPath, FINAL_SELECTED_REVISION);

    const installedMap = await treeMap(FINAL_SELECTED_REVISION);
    const installedBundle = JSON.parse(await readFile(join(FINAL_SELECTED_REVISION, 'bundle.json'), 'utf8')) as AuthorityBundle;
    const expectedReceipt = finalReceipt(lock, installedBundle, identityHash(installedMap));
    const receiptText = await readFile(FINAL_RECEIPT_PATH, 'utf8');
    const receipt = JSON.parse(receiptText) as JsonValue;
    assert.equal(receiptText, canonicalJson(receipt));
    assert.deepEqual(receipt, expectedReceipt);
    for (const privateValue of [lock.primaryRoot, lock.backupRoot]) {
      assert.equal(receiptText.includes(privateValue), false, 'Safe receipt contains a private root.');
    }
    for (const forbidden of ['primaryRoot', 'backupRoot', 'privateLocatorEvidence', 'rulebookAcquisitionEvidence', 'operatingAcknowledgment']) {
      assert.equal(receiptText.includes(`"${forbidden}"`), false, 'Safe receipt contains a private-only field.');
    }

    const readme = await readFile(AUTHORITY_README_PATH, 'utf8');
    const selection = readme.split(/\r?\n\r?\n/)[1] ?? '';
    assert.ok(selection.includes(FINAL_STABLE_ID) && selection.includes(`receipts/${FINAL_REVISION_ID}.json`));
    assert.equal(selection.includes(STABLE_ID), false);
    assert.equal(/bundle:(?:latest|official-\d{4}-\d{2}-\d{2})(?!-v3)/.test(selection), false);
    for (const statement of ['private', 'local', 'noncommercial', 'no-redistribution', 'release', 'hosting', 'upload', 'artwork', 'public API', 'recurring acquisition', 'legal permission']) {
      assert.ok(readme.toLowerCase().includes(statement.toLowerCase()), `README is missing the ${statement} boundary.`);
    }

    const receiptRecord = receipt as unknown as { bundleRootHash: Hash };
    await validateAuthorityBundle(FINAL_SELECTED_REVISION, 'bundle.json', {
      stableId: FINAL_STABLE_ID,
      contentHash: receiptRecord.bundleRootHash,
    });
    await expectAuthorityFailure(() => validateAuthorityBundle(FINAL_SELECTED_REVISION, 'bundle.json', {
      stableId: STABLE_ID,
      contentHash: receiptRecord.bundleRootHash,
    }));
    await assert.rejects(() => stat(`${LOCAL_ROOT}/revisions/latest`), { code: 'ENOENT' });

    const matching = join(candidateRoot, 'matching-candidate');
    await cp(FINAL_SELECTED_REVISION, matching, { recursive: true });
    await installWriteOnce(matching, FINAL_SELECTED_REVISION);
    const different = join(candidateRoot, 'different-candidate');
    await cp(FINAL_SELECTED_REVISION, different, { recursive: true });
    await writeFile(join(different, 'unexpected.json'), '{}', { flag: 'wx' });
    await assert.rejects(
      () => installWriteOnce(different, FINAL_SELECTED_REVISION),
      /Write-once revision has different content/,
    );
    assert.deepEqual(await treeMap(FINAL_SELECTED_REVISION), installedMap);
    assert.equal(sentinel.calls(), 0, 'Final v3 selection attempted network access.');
  } finally {
    sentinel.restore();
    assert.deepEqual(await treeMap(lock.primaryRoot), freshBefore.primary);
    assert.deepEqual(await treeMap(lock.backupRoot), freshBefore.backup);
    assert.deepEqual(await fileEvidence(FINAL_LOCK_PATH), freshBefore.lock);
    assert.deepEqual(await treeMap(historicalLock.primaryRoot), historicalBefore.primary);
    assert.deepEqual(await treeMap(historicalLock.backupRoot), historicalBefore.backup);
    assert.deepEqual(await fileEvidence(LOCK_PATH), historicalBefore.lock);
    assert.deepEqual(await treeMap(`${LOCAL_ROOT}/authorizations`), historicalBefore.authorizations);
    assert.deepEqual(await treeMap(SELECTED_REVISION), historicalBefore.revision);
    assert.deepEqual(await fileEvidence(RECEIPT_PATH), historicalBefore.receipt);
    if (selectedBefore !== null) assert.deepEqual(await treeMap(FINAL_SELECTED_REVISION), selectedBefore);
    await cleanLocalCandidate(candidateRoot);
  }
});

test('historical private roots rebuild deterministically while the selected revision remains immutable', async () => {
  const sandbox = await mkdtemp(join(tmpdir(), 'sorcery-private-revision-'));
  const selectedBefore = await treeMap(SELECTED_REVISION);
  try {
    const lock = JSON.parse(await readFile(LOCK_PATH, 'utf8')) as LiveLock;
    const receipt = JSON.parse(await readFile(RECEIPT_PATH, 'utf8')) as Receipt;
    const consumed = JSON.parse(await readFile(
      `${LOCAL_ROOT}/authorizations/${CURRENT_REFERENCE}.consumed.json`,
      'utf8',
    )) as Evidence;
    const summary = await readFile(SUMMARY_PATH, 'utf8');
    assert.equal(validCurrentPublication(lock as unknown as Evidence, consumed, receipt as unknown as Evidence), true);
    assert.ok(summary.includes(AGENT_METHOD) && summary.includes(CURRENT_REFERENCE), 'Plan 07 summary evidence mismatch');
    assert.equal(lock.operatingAcknowledgment.scope, 'private-local-noncommercial');
    assert.equal(lock.operatingAcknowledgment.establishesLegalPermission, false);
    assert.equal(receipt.attestation.scope, 'private-local-noncommercial');
    assert.equal(receipt.attestation.publisherPermission, false);
    assert.equal(receipt.attestation.redistribution, false);
    assert.equal(receipt.attestation.publicApi, false);
    assert.equal(receipt.attestation.artwork, false);
    assert.equal((await stat(lock.primaryRoot)).isDirectory(), true);
    assert.equal((await stat(lock.backupRoot)).isDirectory(), true);
    assert.equal((await stat(SELECTED_REVISION)).isDirectory(), true);

    const verified = await verifyPrivateSourceSet({
      primaryRoot: lock.primaryRoot,
      backupRoot: lock.backupRoot,
      repositoryRoot: '.',
      entries: lock.entries,
    });
    assert.equal(verified.entries.length, PRIVATE_AUTHORITY_SOURCE_PATHS.length);
    assert.equal(verified.sourceSetRootHash, lock.sourceSetRootHash);
    assert.equal(receipt.sourceSetRootHash, lock.sourceSetRootHash);

    const primaryInput = join(sandbox, 'primary-input');
    const backupInput = join(sandbox, 'backup-input');
    const primaryLock = await writeDerivedInput(primaryInput, lock.primaryRoot, verified.entries);
    const backupLock = await writeDerivedInput(backupInput, lock.backupRoot, verified.entries);
    assert.equal(primaryLock.inputRootHash, receipt.inputRootHash);
    assert.deepEqual(await treeMap(primaryInput), await treeMap(backupInput));

    const primaryOutput = join(sandbox, 'primary-output');
    const backupOutput = join(sandbox, 'backup-output');
    const primary = await importAuthority(importArgs(primaryInput, primaryLock, primaryOutput, REVISION_ID));
    const backup = await importAuthority(importArgs(backupInput, backupLock, backupOutput, REVISION_ID));
    assert.equal(primary.bundleId, STABLE_ID);
    assert.equal(backup.bundleId, STABLE_ID);
    assert.equal(primary.verifiedInputRootHash, backup.verifiedInputRootHash);
    assert.equal(primary.bundleRootHash, backup.bundleRootHash);
    assert.deepEqual(await treeMap(primary.revisionPath), await treeMap(backup.revisionPath));
    await validateAuthorityBundle(primaryOutput, `${REVISION_ID}/bundle.json`, {
      stableId: primary.bundleId,
      contentHash: primary.bundleRootHash,
    });
    await validateAuthorityBundle(backupOutput, `${REVISION_ID}/bundle.json`, {
      stableId: backup.bundleId,
      contentHash: backup.bundleRootHash,
    });
    const historicalBundle = JSON.parse(
      await readFile(join(SELECTED_REVISION, 'bundle.json'), 'utf8'),
    ) as AuthorityBundle;
    assert.equal(historicalBundle.identity.stableId, STABLE_ID);
    assert.equal(identityHash(historicalBundle.identity as unknown as JsonValue), receipt.bundleRootHash);

    const rawPrimary = join(sandbox, 'raw-primary');
    const rawBackup = join(sandbox, 'raw-backup');
    await cp(lock.primaryRoot, rawPrimary, { recursive: true });
    await cp(lock.backupRoot, rawBackup, { recursive: true });
    const rawPath = join(rawPrimary, 'cards', 'cards.raw.json');
    const rawBytes = await readFile(rawPath);
    rawBytes[0] = rawBytes[0] === 91 ? 123 : 91;
    await writeFile(rawPath, rawBytes);
    await assert.rejects(() => verifyPrivateSourceSet({
      primaryRoot: rawPrimary,
      backupRoot: rawBackup,
      repositoryRoot: '.',
      entries: lock.entries,
    }));
    await assert.rejects(() => verifyPrivateSourceSet({
      primaryRoot: lock.primaryRoot,
      backupRoot: lock.backupRoot,
      repositoryRoot: '.',
      entries: lock.entries.map((entry, index) => index === 0 ? { ...entry, byteHash: ZERO_HASH } : entry),
    }));
    assert.notEqual(identityHash(verified.entries), ZERO_HASH, 'source-set root tamper must not match');

    const cardsTamper = join(sandbox, 'input-cards-tamper');
    await cp(primaryInput, cardsTamper, { recursive: true });
    await writeFile(join(cardsTamper, 'cards.raw.json'), Buffer.concat([
      await readFile(join(cardsTamper, 'cards.raw.json')),
      Buffer.from(' '),
    ]));
    await expectAuthorityFailure(() => importAuthority(importArgs(
      cardsTamper,
      primaryLock,
      join(sandbox, 'output-cards-tamper'),
      'cards-tamper',
    )), 'input_byte_hash_mismatch');

    const lockTamper = join(sandbox, 'input-lock-tamper');
    await cp(primaryInput, lockTamper, { recursive: true });
    const changedLock = { ...primaryLock, inputRootHash: ZERO_HASH };
    await writeFile(join(lockTamper, 'input-lock.json'), canonicalJson(changedLock));
    await expectAuthorityFailure(() => importAuthority(importArgs(
      lockTamper,
      changedLock,
      join(sandbox, 'output-lock-tamper'),
      'lock-tamper',
    )), 'input_root_hash_mismatch');

  } finally {
    assert.deepEqual(await treeMap(SELECTED_REVISION), selectedBefore);
    await cleanTemp(sandbox);
  }
});
