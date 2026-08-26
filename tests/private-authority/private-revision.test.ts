import assert from 'node:assert/strict';
import { cp, mkdir, mkdtemp, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
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
  createCanonicalArtifact,
  type AuthorityBundle,
  type CanonicalArtifact,
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

async function refreshInputLock(root: string): Promise<InputLock> {
  const prior = JSON.parse(await readFile(join(root, 'input-lock.json'), 'utf8')) as InputLock;
  const files = await Promise.all(prior.files.map(async (entry) => ({
    ...entry,
    byteHash: sha256(await readFile(join(root, entry.relativePath))),
  })));
  const updated: InputLock = { schemaVersion: 1, files, inputRootHash: identityHash(files) };
  await writeFile(join(root, 'input-lock.json'), canonicalJson(updated));
  return updated;
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

async function writeArtifactCompanion(
  revisionRoot: string,
  artifact: CanonicalArtifact<JsonValue>,
  artifacts: readonly CanonicalArtifact<JsonValue>[],
): Promise<void> {
  if (artifact.identity.artifactKind === 'source-manifest') {
    await writeFile(join(revisionRoot, 'sources.json'), canonicalJson(artifact));
  } else if (artifact.identity.artifactKind === 'card-snapshot') {
    await writeFile(join(revisionRoot, 'cards.normalized.json'), canonicalJson(artifact));
  } else if (artifact.identity.artifactKind === 'format') {
    await writeFile(join(revisionRoot, 'formats.json'), canonicalJson({
      formats: artifacts.filter((entry) => entry.identity.artifactKind === 'format'),
    }));
  }
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

test('live private roots independently reproduce the exact selected revision without mutation', async () => {
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
    assert.equal(primary.bundleRootHash, receipt.bundleRootHash);
    assert.deepEqual(await treeMap(primary.revisionPath), await treeMap(backup.revisionPath));
    await validateAuthorityBundle(primaryOutput, `${REVISION_ID}/bundle.json`, {
      stableId: primary.bundleId,
      contentHash: primary.bundleRootHash,
    });
    await validateAuthorityBundle(backupOutput, `${REVISION_ID}/bundle.json`, {
      stableId: backup.bundleId,
      contentHash: backup.bundleRootHash,
    });
    await validateAuthorityBundle(SELECTED_REVISION, 'bundle.json', {
      stableId: STABLE_ID,
      contentHash: receipt.bundleRootHash,
    });

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

    for (const [label, mutate, expectedCode] of [
      ['derived-source', (document: { sources: SourceRecord[] }) => {
        const derived = document.sources.find((source) => source.sourceId.startsWith('source:derived-format-input-')) as unknown as {
          byteHash: Hash;
        };
        derived.byteHash = ZERO_HASH;
      }, 'format_source_mismatch'],
      ['derived-parent', (document: { sources: SourceRecord[] }) => {
        const derived = document.sources.find((source) => source.sourceId.startsWith('source:derived-format-input-')) as unknown as {
          derivation: SourceRecord['derivation'];
        };
        derived.derivation = { ...derived.derivation, parentByteHashes: [ZERO_HASH] };
      }, 'missing_derivation_parent'],
      ['source-manifest', (document: { sources: Array<SourceRecord & { unexpected?: boolean }> }) => {
        document.sources[0]!.unexpected = true;
      }, 'unrecognized_key'],
    ] as const) {
      const input = join(sandbox, `input-${label}-tamper`);
      await cp(primaryInput, input, { recursive: true });
      const sourcePath = join(input, 'sources.json');
      const document = JSON.parse(await readFile(sourcePath, 'utf8')) as { sources: SourceRecord[] };
      mutate(document as never);
      await writeFile(sourcePath, canonicalJson(document));
      const refreshed = await refreshInputLock(input);
      await expectAuthorityFailure(() => importAuthority(importArgs(
        input,
        refreshed,
        join(sandbox, `output-${label}-tamper`),
        `${label}-tamper`,
      )), expectedCode);
    }

    const selectedBundle = JSON.parse(await readFile(join(SELECTED_REVISION, 'bundle.json'), 'utf8')) as AuthorityBundle;
    const selectedExpected = { stableId: STABLE_ID, contentHash: receipt.bundleRootHash };
    for (const [label, file] of [
      ['source-manifest', 'sources.json'],
      ['normalized-cards', 'cards.normalized.json'],
      ['format-artifact', 'formats.json'],
    ] as const) {
      const caseRoot = join(sandbox, `revision-${label}`);
      const copyRoot = join(caseRoot, 'copy');
      await cp(SELECTED_REVISION, copyRoot, { recursive: true });
      const path = join(copyRoot, file);
      await writeFile(path, Buffer.concat([await readFile(path), Buffer.from(' ')]));
      await expectAuthorityFailure(
        () => validateAuthorityBundle(caseRoot, 'copy/bundle.json', selectedExpected),
        'companion_content_mismatch',
      );
    }

    const identityRoot = join(sandbox, 'revision-bundle-identity');
    await cp(SELECTED_REVISION, join(identityRoot, 'copy'), { recursive: true });
    const identityBundle = structuredClone(selectedBundle) as unknown as Record<string, unknown>;
    (identityBundle.identity as Record<string, unknown>).stableId = 'bundle:tampered';
    await writeFile(join(identityRoot, 'copy', 'bundle.json'), canonicalJson(identityBundle as never));
    await expectAuthorityFailure(
      () => validateAuthorityBundle(identityRoot, 'copy/bundle.json', selectedExpected),
      'content_hash_mismatch',
    );

    const refLocations: Array<Readonly<{ artifactIndex: number | null; referenceIndex: number }>> = [];
    selectedBundle.identity.sourceRefs.forEach((_, referenceIndex) => {
      refLocations.push({ artifactIndex: null, referenceIndex });
    });
    selectedBundle.identity.payload.artifacts.forEach((artifact, artifactIndex) => {
      artifact.identity.sourceRefs.forEach((_, referenceIndex) => {
        refLocations.push({ artifactIndex, referenceIndex });
      });
    });
    for (const [caseIndex, location] of refLocations.entries()) {
      const caseRoot = join(sandbox, `revision-source-ref-${caseIndex}`);
      const copyRoot = join(caseRoot, 'copy');
      await cp(SELECTED_REVISION, copyRoot, { recursive: true });
      let tamperedBundle: AuthorityBundle;
      if (location.artifactIndex === null) {
        tamperedBundle = createCanonicalArtifact({
          ...selectedBundle.identity,
          sourceRefs: selectedBundle.identity.sourceRefs.map((reference, index) =>
            index === location.referenceIndex ? { ...reference, byteHash: ZERO_HASH } : reference),
        }) as AuthorityBundle;
      } else {
        const original = selectedBundle.identity.payload.artifacts[location.artifactIndex]!;
        const tamperedArtifact = createCanonicalArtifact({
          ...original.identity,
          sourceRefs: original.identity.sourceRefs.map((reference, index) =>
            index === location.referenceIndex ? { ...reference, byteHash: ZERO_HASH } : reference),
        });
        const artifacts = selectedBundle.identity.payload.artifacts.map((artifact, index) =>
          index === location.artifactIndex ? tamperedArtifact : artifact);
        tamperedBundle = createCanonicalArtifact({
          ...selectedBundle.identity,
          parentRefs: selectedBundle.identity.parentRefs.map((reference) =>
            reference.stableId === tamperedArtifact.identity.stableId
              ? { ...reference, contentHash: tamperedArtifact.contentHash }
              : reference),
          payload: { ...selectedBundle.identity.payload, artifacts },
        }) as AuthorityBundle;
        await writeArtifactCompanion(copyRoot, tamperedArtifact, artifacts);
      }
      await writeFile(join(copyRoot, 'bundle.json'), canonicalJson(tamperedBundle));
      await expectAuthorityFailure(
        () => validateAuthorityBundle(caseRoot, 'copy/bundle.json', {
          stableId: tamperedBundle.identity.stableId,
          contentHash: tamperedBundle.contentHash,
        }),
        'source_ref_hash_mismatch',
      );
    }

    const cycleRoot = join(sandbox, 'revision-cycle');
    const cycleCopy = join(cycleRoot, 'copy');
    await cp(SELECTED_REVISION, cycleCopy, { recursive: true });
    const first = selectedBundle.identity.payload.artifacts[0]!;
    const second = selectedBundle.identity.payload.artifacts[1]!;
    const cycleFirst = createCanonicalArtifact({
      ...first.identity,
      parentRefs: [{
        artifactKind: second.identity.artifactKind,
        stableId: second.identity.stableId,
        contentHash: second.contentHash,
      }],
    });
    const cycleSecond = createCanonicalArtifact({
      ...second.identity,
      parentRefs: [{
        artifactKind: first.identity.artifactKind,
        stableId: first.identity.stableId,
        contentHash: first.contentHash,
      }],
    });
    const cycleArtifacts = selectedBundle.identity.payload.artifacts.map((artifact) =>
      artifact.identity.stableId === first.identity.stableId ? cycleFirst
        : artifact.identity.stableId === second.identity.stableId ? cycleSecond
          : artifact);
    const cycleBundle = createCanonicalArtifact({
      ...selectedBundle.identity,
      parentRefs: selectedBundle.identity.parentRefs.map((reference) => {
        const replacement = cycleArtifacts.find((artifact) => artifact.identity.stableId === reference.stableId)!;
        return { ...reference, contentHash: replacement.contentHash };
      }),
      payload: { ...selectedBundle.identity.payload, artifacts: cycleArtifacts },
    }) as AuthorityBundle;
    await writeArtifactCompanion(cycleCopy, cycleFirst, cycleArtifacts);
    await writeArtifactCompanion(cycleCopy, cycleSecond, cycleArtifacts);
    await writeFile(join(cycleCopy, 'bundle.json'), canonicalJson(cycleBundle));
    await expectAuthorityFailure(
      () => validateAuthorityBundle(cycleRoot, 'copy/bundle.json', {
        stableId: cycleBundle.identity.stableId,
        contentHash: cycleBundle.contentHash,
      }),
      'reference_cycle',
    );
  } finally {
    assert.deepEqual(await treeMap(SELECTED_REVISION), selectedBefore);
    await cleanTemp(sandbox);
  }
});
