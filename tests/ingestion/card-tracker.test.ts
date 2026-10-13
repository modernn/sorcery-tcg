import assert from 'node:assert/strict';
import { lstat, mkdir, mkdtemp, readFile, realpath, readdir, rename, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { DatabaseSync } from 'node:sqlite';

import { canonicalJson, parseJsonWithDuplicateKeyCheck } from '../../src/authority/canonical-json.ts';
import { identityHash, sha256 } from '../../src/authority/hash.ts';
import { normalizeCards } from '../../src/authority/normalize-cards.ts';
import { createCanonicalArtifact } from '../../src/authority/schemas.ts';
import { readBoundedWithinAuthorityRoot, sameBoundedFileIdentity } from '../../src/authority/validate-bundle.ts';
import { auditCardReadiness } from '../../src/catalog/card-readiness.ts';
import { assertExactSourceHash, CARD_TRACKER_SCHEMA, engineSourceHash, expireStaleCardValidationAnnotations } from '../../src/catalog/card-tracker.ts';

const sourceBytes = Buffer.from(JSON.stringify({ cards: [{
  sourceCardId: 'candidate-a', name: 'Synthetic Candidate', cardType: 'minion', elements: ['fire'], rarity: 'ordinary',
  manaCost: 1, attack: 1, defense: 1, life: null, thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  rulesText: '', subtypes: ['Mortal'], printingSlugs: ['synthetic-candidate'], releasedAt: null,
}] }));

async function readinessFixture(multipleCards = false): Promise<{ root: string; authorityRoot: string; packetPath: string; dbPath: string; cardId: string; candidate: Record<string, unknown> }> {
  const root = await mkdtemp(join(tmpdir(), 'card-readiness-'));
  const authorityRoot = join(root, '.local/authority');
  const revisionId = 'synthetic-readiness';
  const sourceFeed = JSON.parse(sourceBytes.toString('utf8')) as { cards: Array<Record<string, unknown>> };
  const fixtureSourceBytes = multipleCards ? Buffer.from(JSON.stringify({ cards: [
    sourceFeed.cards[0], { ...sourceFeed.cards[0], sourceCardId: 'candidate-b', name: 'Synthetic Candidate B',
      printingSlugs: ['synthetic-candidate-b'] },
  ] })) : sourceBytes;
  const normalized = normalizeCards(fixtureSourceBytes, {
    sourceId: 'source:synthetic-readiness', url: 'https://example.invalid/cards', authorityClass: 'community-provenance',
    retrievedAt: '2026-01-01T00:00:00Z', effectiveDate: '2026-01-01', mediaType: 'application/json',
    byteHash: sha256(fixtureSourceBytes), derivation: { method: 'verbatim', parentByteHashes: [], notes: 'synthetic' },
    licenseStatus: 'reference-only', storageMode: 'manifest-only', storagePolicy: 'prohibited',
    durableLocator: `urn:${sha256(fixtureSourceBytes)}`, acquisitionProcedureHash: null,
  });
  const cards = normalized.identity.payload.cards;
  const card = cards[0]!;
  const fact = { cardType: 'minion', elements: ['fire'], manaCost: 1, attack: 1, defense: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 }, subtypes: ['Mortal'], ordinary: true };
  const candidates = (multipleCards ? cards : [card]).map((item) => ({ cardId: item.stableId,
    sourceCardHash: identityHash(item as never), facts: fact,
    review: { entireRulesText: true, proofs: ['synthetic complete source review'] } }));
  const candidate = candidates[0]!;
  const format = createCanonicalArtifact({ artifactKind: 'format', stableId: 'format:synthetic-constructed', schemaVersion: 1,
    parentRefs: [], sourceRefs: [{ sourceId: 'source:synthetic-readiness', byteHash: sha256(fixtureSourceBytes) }], payload: {
      name: 'Constructed', effectiveDate: '2026-01-01', scope: null, parentFormatStableId: null, avatarCount: 1,
      spellbookMinimum: 0, atlasMinimum: 0, copyLimits: { ordinary: 4, exceptional: 3, elite: 2, unique: 1 },
    } });
  const revisionRoot = join(authorityRoot, 'revisions', revisionId);
  const catalogRoot = join(authorityRoot, 'catalog');
  await mkdir(revisionRoot, { recursive: true }); await mkdir(catalogRoot, { recursive: true });
  await mkdir(join(authorityRoot, 'scenarios'), { recursive: true });
  await mkdir(join(authorityRoot, 'codex'), { recursive: true });
  await writeFile(join(authorityRoot, 'scenarios/vanilla-constructed.json'), canonicalJson({ revisionId }));
  await writeFile(join(revisionRoot, 'cards.normalized.json'), canonicalJson(normalized));
  await writeFile(join(revisionRoot, 'formats.json'), canonicalJson({ formats: [format] }));
  await writeFile(join(authorityRoot, 'codex/official-codex.snapshot.json'), canonicalJson({ entries: [] }));
  await writeFile(join(root, 'Cargo.lock'), 'synthetic lock');
  await mkdir(join(root, 'crates/sorcery-engine/src'), { recursive: true });
  await mkdir(join(root, 'crates/sorcery-engine/tests'), { recursive: true });
  await writeFile(join(root, 'crates/sorcery-engine/Cargo.toml'), '[package]');
  await writeFile(join(root, 'crates/sorcery-engine/src/lib.rs'), '');
  const dbPath = join(catalogRoot, 'card-tracker.sqlite3');
  const db = new DatabaseSync(dbPath);
  db.exec(CARD_TRACKER_SCHEMA);
  const codexHash = sha256(Buffer.from(await readFile(join(authorityRoot, 'codex/official-codex.snapshot.json'))));
  db.prepare('INSERT INTO tracker_meta(key,value) VALUES (?,?)').run('authorityHash', normalized.contentHash);
  db.prepare('INSERT INTO tracker_meta(key,value) VALUES (?,?)').run('revisionId', revisionId);
  db.prepare('INSERT INTO tracker_meta(key,value) VALUES (?,?)').run('codexSnapshotHash', codexHash);
  candidates.forEach((row, index) => {
    const sourceCard = cards[index]!;
    db.prepare('INSERT INTO cards VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,1)').run(sourceCard.stableId, sourceCard.officialSourceId, sourceCard.name,
      sourceCard.cardType, canonicalJson(sourceCard.elements), '{}', sourceCard.rulesText, row.sourceCardHash, normalized.contentHash, revisionId,
      'unbound', null, null);
    db.prepare('INSERT INTO card_status VALUES (?,?,?,?,?,?,?,?,?,?,?)').run(sourceCard.stableId, 'remaining', 'reviewed',
      'current-native-validation-accepted', 1, 'source-reviewed', row.sourceCardHash, '[]', '[]', null, '{}');
    db.prepare('INSERT INTO manual_card_status(card_id) VALUES (?)').run(sourceCard.stableId);
  });
  db.close();
  await writeFile(join(catalogRoot, 'registry.json'), 'synthetic registry sentinel');
  await writeFile(join(catalogRoot, 'validation-jobs.json'), 'synthetic job sentinel');
  for (const [path, contents] of [['evidence/source.txt', 'reviewed'], ['evidence/runtime.txt', 'qualified'], ['evidence/card.txt', 'scenario']] as const) {
    await mkdir(join(authorityRoot, path.slice(0, path.lastIndexOf('/'))), { recursive: true });
    await writeFile(join(authorityRoot, path), contents);
  }
  const writeJson = async (path: string, value: unknown): Promise<{ path: string; sha256: string }> => {
    const bytes = Buffer.from(canonicalJson(value as never));
    await writeFile(join(authorityRoot, path), bytes);
    return { path, sha256: sha256(bytes) };
  };
  const candidateRef = await writeJson('candidates.json', { schemaVersion: 1, authorityHash: normalized.contentHash, revisionId,
    cards: candidates });
  const executablePath = await realpath('/bin/true');
  const runtimeRef = await writeJson('runtime.json', { schemaVersion: 1, kind: 'runtime-qualification', status: 'passed',
    engineSourceHash: await engineSourceHash(root), executable: { path: executablePath, sha256: sha256(await readFile(executablePath)) },
    evidence: [{ path: 'evidence/runtime.txt', sha256: sha256(Buffer.from('qualified')) }] });
  const packetCards = await Promise.all(candidates.map(async (row, index) => ({ cardId: row.cardId,
    contextReview: await writeJson(`context-${index}.json`, { schemaVersion: 1, kind: 'card-context-review', cardId: row.cardId,
      reviewer: 'Synthetic reviewer', authorityHash: normalized.contentHash, authorityRevision: revisionId, codexSnapshotHash: codexHash,
      sourceCardHash: row.sourceCardHash, candidateFactsHash: identityHash(fact as never), completeContext: true,
      requirements: [], unresolved: [], evidence: [{ path: 'evidence/source.txt', sha256: sha256(Buffer.from('reviewed')) }] }),
    sharedProofs: [], validation: null })));
  const packetPath = 'catalog/readiness-packet.json';
  await writeJson(packetPath, { schemaVersion: 1, candidateBindings: candidateRef, runtime: runtimeRef,
    cards: packetCards });
  return { root, authorityRoot, packetPath, dbPath, cardId: card.stableId, candidate };
}

async function writePrivateJson(root: string, path: string, value: unknown): Promise<{ path: string; sha256: string }> {
  const bytes = Buffer.from(canonicalJson(value as never));
  await writeFile(join(root, path), bytes);
  return { path, sha256: sha256(bytes) };
}

test('readiness distinguishes candidate validation from admission flags and validates frozen receipts', async () => {
  const fixture = await readinessFixture();
  try {
    const before = sha256(await readFile(fixture.dbPath));
    const catalogEntries = await readdir(join(fixture.authorityRoot, 'catalog'));
    const sidecarsBefore = catalogEntries.filter((name) => /-(?:wal|shm|journal)$/u.test(name));
    const registryAndJobsBefore = await Promise.all(['registry.json', 'validation-jobs.json']
      .map((name) => readFile(join(fixture.authorityRoot, 'catalog', name))));
    const report = await auditCardReadiness(fixture.root, fixture.packetPath);
    assert.equal(report.cards[0]?.status, 'candidate-ready');
    assert.deepEqual(report.cards[0]?.reasons, ['card_validation_missing']);
    assert.equal(report.cards[0]?.admission.bindingStatus, 'unbound');
    assert.equal(report.cards[0]?.admission.factsMatch, false);
    assert.deepEqual(await auditCardReadiness(fixture.root, fixture.packetPath), report);

    const card = report.cards[0]!;
    const evidencePath = join(fixture.authorityRoot, 'evidence/card.txt');
    const evidenceHash = sha256(await readFile(evidencePath));
    const validation = { schemaVersion: 1, kind: 'card-validation', status: 'passed', cardId: fixture.cardId,
      dependencyFingerprint: card.dependencyFingerprint,
      evidence: [{ path: 'evidence/card.txt', sha256: evidenceHash }] };
    const validationBytes = Buffer.from(canonicalJson(validation as never));
    await writeFile(join(fixture.authorityRoot, 'validation.json'), validationBytes);
    const packetFile = join(fixture.authorityRoot, fixture.packetPath);
    const packet = JSON.parse(await readFile(packetFile, 'utf8')) as { cards: Array<Record<string, unknown>> };
    packet.cards[0]!.validation = { path: 'validation.json', sha256: sha256(validationBytes) };
    await writeFile(packetFile, canonicalJson(packet as never));
    assert.equal((await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]?.status, 'currently-validated');
    const staleValidation = { ...validation, dependencyFingerprint: `sha256:${'f'.repeat(64)}` };
    packet.cards[0]!.validation = await writePrivateJson(fixture.authorityRoot, 'validation.json', staleValidation);
    await writeFile(packetFile, canonicalJson(packet as never));
    const stale = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(stale.status, 'candidate-ready');
    assert.ok(stale.reasons.includes('card_validation_stale'));
    packet.cards[0]!.validation = await writePrivateJson(fixture.authorityRoot, 'validation.json', { ...validation, status: 'failed' });
    await writeFile(packetFile, canonicalJson(packet as never));
    const failed = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(failed.status, 'candidate-ready');
    assert.ok(failed.reasons.includes('card_validation_failed'));
    assert.equal(sha256(await readFile(fixture.dbPath)), before);
    assert.deepEqual(await readdir(join(fixture.authorityRoot, 'catalog')), catalogEntries);
    assert.deepEqual((await readdir(join(fixture.authorityRoot, 'catalog'))).filter((name) => /-(?:wal|shm|journal)$/u.test(name)), sidecarsBefore);
    assert.deepEqual(await Promise.all(['registry.json', 'validation-jobs.json']
      .map((name) => readFile(join(fixture.authorityRoot, 'catalog', name)))), registryAndJobsBefore);

    packet.cards[0]!.contextReview = null;
    const packetWithNoRuntime = JSON.parse(await readFile(packetFile, 'utf8')) as { runtime: unknown; cards: Array<Record<string, unknown>> };
    packetWithNoRuntime.cards[0]!.contextReview = null;
    packetWithNoRuntime.runtime = null;
    await writeFile(packetFile, canonicalJson(packetWithNoRuntime as never));
    const blocked = await auditCardReadiness(fixture.root, fixture.packetPath);
    assert.equal(blocked.cards[0]?.status, 'blocked');
    assert.ok(blocked.cards[0]?.reasons.includes('context_review_missing'));
    assert.ok(blocked.cards[0]?.reasons.includes('runtime_missing'));
    assert.equal(sha256(await readFile(fixture.dbPath)), before);
    assert.deepEqual(await Promise.all(['registry.json', 'validation-jobs.json']
      .map((name) => readFile(join(fixture.authorityRoot, 'catalog', name)))), registryAndJobsBefore);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('readiness rejects packet mismatches, replacement metadata, unsafe paths, and bad evidence', async () => {
  const fixture = await readinessFixture();
  const packetFile = join(fixture.authorityRoot, fixture.packetPath);
  try {
    const packet = JSON.parse(await readFile(packetFile, 'utf8')) as { candidateBindings: { path: string; sha256: string }; cards: Array<Record<string, unknown>> };
    packet.cards.push(structuredClone(packet.cards[0]!));
    await writeFile(packetFile, canonicalJson(packet as never));
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /packet and candidate card identities/u);

    packet.cards.pop();
    const candidatesPath = join(fixture.authorityRoot, packet.candidateBindings.path);
    const candidateFile = JSON.parse(await readFile(candidatesPath, 'utf8')) as { cards: Array<Record<string, unknown>> };
    candidateFile.cards[0]!.replacesFactsHash = `sha256:${'a'.repeat(64)}`;
    packet.candidateBindings = await writePrivateJson(fixture.authorityRoot, 'candidates.json', candidateFile);
    await writeFile(packetFile, canonicalJson(packet as never));
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /may not contain replacement metadata/u);

    packet.candidateBindings.path = '../candidate.json';
    await writeFile(packetFile, canonicalJson(packet as never));
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /authority-relative/u);

    packet.candidateBindings = await writePrivateJson(fixture.authorityRoot, 'candidates.json', {
      schemaVersion: 1,
      authorityHash: JSON.parse(await readFile(candidatesPath, 'utf8')).authorityHash,
      revisionId: 'synthetic-readiness', cards: [{ ...fixture.candidate }],
    });
    await writeFile(packetFile, canonicalJson(packet as never));
    await writeFile(join(fixture.authorityRoot, 'evidence/source.txt'), 'tampered');
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /empty, or stale evidence/u);
    await writeFile(join(fixture.authorityRoot, 'evidence/source.txt'), '');
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /empty, or stale evidence/u);
    await rm(join(fixture.authorityRoot, 'evidence/source.txt'));
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath));

    await writeFile(join(fixture.authorityRoot, 'evidence/source.txt'), 'reviewed');
    await symlink(join(fixture.authorityRoot, 'evidence/source.txt'), join(fixture.authorityRoot, 'evidence/alias.txt'));
    const contextFile = JSON.parse(await readFile(join(fixture.authorityRoot, 'context-0.json'), 'utf8')) as { evidence: Array<{ path: string; sha256: string }> };
    contextFile.evidence[0]!.path = 'evidence/alias.txt';
    const contextRef = await writePrivateJson(fixture.authorityRoot, 'context.json', contextFile);
    packet.cards[0]!.contextReview = contextRef;
    await writeFile(packetFile, canonicalJson(packet as never));
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath));
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('readiness binds each shared proof to the exact current requirement and candidate facts', async () => {
  const fixture = await readinessFixture();
  try {
    const authorityRoot = fixture.authorityRoot;
    const packetFile = join(authorityRoot, fixture.packetPath);
    const packet = JSON.parse(await readFile(packetFile, 'utf8')) as {
      cards: Array<{ contextReview: { path: string; sha256: string }; sharedProofs: Array<{ path: string; sha256: string }> }>;
      candidateBindings: { path: string; sha256: string };
      runtime: { path: string; sha256: string };
    };
    const candidates = JSON.parse(await readFile(join(authorityRoot, packet.candidateBindings.path), 'utf8')) as {
      authorityHash: string; cards: Array<{ cardId: string; sourceCardHash: string; facts: unknown }>;
    };
    const candidate = candidates.cards[0]!;
    const db = new DatabaseSync(fixture.dbPath);
    db.prepare('INSERT INTO capability_groups VALUES (?,?,?,?,?,?,?,?,1)')
      .run('synthetic-group', 'Synthetic', 1, 'synthetic', 0, 0, '{}', `sha256:${'1'.repeat(64)}`);
    const payloadA = '{"z":1,"requirementId":"synthetic-requirement-a"}';
    const payloadB = '{"requirementId":"synthetic-requirement-b","z":1}';
    const sliceHashA = sha256(Buffer.from(payloadA));
    const sliceHashB = sha256(Buffer.from(payloadB));
    assert.equal(sliceHashA, 'sha256:854aa47d1acec724390da0cb72a3b806b26f0c7d140c5f47aed7e84fd85e5dbf');
    assert.notEqual(sliceHashA, sha256(Buffer.from(canonicalJson(JSON.parse(payloadA)))));
    for (const [sliceId, requirementId, body, hash] of [
      ['synthetic-slice-a', 'synthetic-requirement-a', payloadA, sliceHashA],
      ['synthetic-slice-b', 'synthetic-requirement-b', payloadB, sliceHashB],
    ] as const) {
      db.prepare('INSERT INTO rule_slices VALUES (?,?,?,?,?,?,?,?,1)')
        .run(sliceId, requirementId, fixture.cardId, 'synthetic-group', 'unverified-contextual-requirement',
          'synthetic-family', body, hash);
      db.prepare('INSERT INTO card_requirements VALUES (?,?,?,1)').run(fixture.cardId, sliceId, candidate.sourceCardHash);
    }
    db.close();

    const context = JSON.parse(await readFile(join(authorityRoot, packet.cards[0]!.contextReview.path), 'utf8')) as { requirements: unknown[] };
    context.requirements = [
      { sliceId: 'synthetic-slice-b', sourceHash: sliceHashB },
      { sliceId: 'synthetic-slice-a', sourceHash: sliceHashA },
    ];
    packet.cards[0]!.contextReview = await writePrivateJson(authorityRoot, 'context.json', context);
    const runtime = JSON.parse(await readFile(join(authorityRoot, packet.runtime.path), 'utf8')) as {
      engineSourceHash: string; executable: { sha256: string };
    };
    const contextBody = JSON.parse(await readFile(join(authorityRoot, packet.cards[0]!.contextReview.path), 'utf8')) as {
      codexSnapshotHash: string; candidateFactsHash: string;
    };
    const proof = (sliceId: string, sliceHash: string) => ({ schemaVersion: 1, kind: 'shared-rule-proof', status: 'passed', cardId: fixture.cardId,
      authorityHash: candidates.authorityHash, codexSnapshotHash: contextBody.codexSnapshotHash,
      sourceCardHash: candidate.sourceCardHash, candidateFactsHash: contextBody.candidateFactsHash,
      sliceId, sliceHash, engineSourceHash: runtime.engineSourceHash,
      executableSha256: runtime.executable.sha256,
      evidence: [{ path: 'evidence/runtime.txt', sha256: sha256(Buffer.from('qualified')) }] });
    const proofA = proof('synthetic-slice-a', sliceHashA);
    const proofB = proof('synthetic-slice-b', sliceHashB);
    const proofARef = await writePrivateJson(authorityRoot, 'proof-a.json', proofA);
    const proofBRef = await writePrivateJson(authorityRoot, 'proof-b.json', proofB);
    packet.cards[0]!.sharedProofs = [proofARef, proofBRef];
    await writeFile(packetFile, canonicalJson(packet as never));
    const valid = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(valid.status, 'candidate-ready');
    assert.deepEqual(valid.reasons, ['card_validation_missing']);
    const report = await auditCardReadiness(fixture.root, fixture.packetPath);
    packet.cards[0]!.sharedProofs.reverse();
    await writeFile(packetFile, canonicalJson(packet as never));
    assert.deepEqual(await auditCardReadiness(fixture.root, fixture.packetPath), report);
    const reorderedContextRef = await writePrivateJson(authorityRoot, 'context-reordered.json', {
      ...context, requirements: [...context.requirements].reverse(),
    });
    packet.cards[0]!.contextReview = reorderedContextRef;
    await writeFile(packetFile, canonicalJson(packet as never));
    const reorderedRequirements = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(reorderedRequirements.status, 'candidate-ready');
    assert.deepEqual(reorderedRequirements.reasons, ['card_validation_missing']);

    packet.cards[0]!.sharedProofs = [await writePrivateJson(authorityRoot, 'proof-a.json', {
      ...proofA, candidateFactsHash: `sha256:${'e'.repeat(64)}`,
    }), proofBRef];
    await writeFile(packetFile, canonicalJson(packet as never));
    const staleFacts = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(staleFacts.status, 'blocked');
    assert.ok(staleFacts.reasons.includes('shared_proof_stale'));

    const proofRef = packet.cards[0]!.sharedProofs[0]!;
    packet.cards[0]!.sharedProofs = [proofRef, proofRef];
    await writeFile(packetFile, canonicalJson(packet as never));
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /duplicate shared proof slice/u);
    packet.cards[0]!.sharedProofs = [proofRef];

    const duplicateContext = { ...context, requirements: [
      { sliceId: 'synthetic-slice-a', sourceHash: sliceHashA }, { sliceId: 'synthetic-slice-a', sourceHash: sliceHashA },
    ] };
    packet.cards[0]!.contextReview = await writePrivateJson(authorityRoot, 'context.json', duplicateContext);
    await writeFile(packetFile, canonicalJson(packet as never));
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /duplicate context requirement/u);
    packet.cards[0]!.contextReview = await writePrivateJson(authorityRoot, 'context.json', context);
    packet.cards[0]!.sharedProofs = [proofARef, proofBRef];
    await writeFile(packetFile, canonicalJson(packet as never));

    const staleDb = new DatabaseSync(fixture.dbPath);
    staleDb.prepare('UPDATE card_requirements SET source_card_hash=? WHERE card_id=?')
      .run(`sha256:${'f'.repeat(64)}`, fixture.cardId);
    staleDb.close();
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /invalid current requirement source/u);
    const missingLinkDb = new DatabaseSync(fixture.dbPath);
    missingLinkDb.prepare('DELETE FROM card_requirements WHERE card_id=?').run(fixture.cardId);
    missingLinkDb.close();
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /missing its exact card requirement link/u);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('readiness blocks when the preserved executable bytes do not match the runtime receipt', async () => {
  const fixture = await readinessFixture();
  try {
    const packetFile = join(fixture.authorityRoot, fixture.packetPath);
    const packet = JSON.parse(await readFile(packetFile, 'utf8')) as { runtime: { path: string; sha256: string } };
    const runtime = JSON.parse(await readFile(join(fixture.authorityRoot, packet.runtime.path), 'utf8')) as {
      executable: { path: string; sha256: string };
    };
    runtime.executable.path = await realpath('/usr/bin/false');
    packet.runtime = await writePrivateJson(fixture.authorityRoot, 'runtime.json', runtime);
    await writeFile(packetFile, canonicalJson({
      ...JSON.parse(await readFile(packetFile, 'utf8')),
      runtime: packet.runtime,
    } as never));
    const card = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(card.status, 'blocked');
    assert.ok(card.reasons.includes('runtime_stale'));
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('readiness blocks stale context source and runtime engine identities', async () => {
  const fixture = await readinessFixture();
  try {
    const packetFile = join(fixture.authorityRoot, fixture.packetPath);
    const packet = JSON.parse(await readFile(packetFile, 'utf8')) as {
      runtime: { path: string; sha256: string };
      cards: Array<{ contextReview: { path: string; sha256: string } }>;
    };
    const context = JSON.parse(await readFile(join(fixture.authorityRoot, packet.cards[0]!.contextReview.path), 'utf8')) as {
      sourceCardHash: string;
    };
    context.sourceCardHash = `sha256:${'f'.repeat(64)}`;
    packet.cards[0]!.contextReview = await writePrivateJson(fixture.authorityRoot, 'context.json', context);
    await writeFile(packetFile, canonicalJson(packet as never));
    const staleContext = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(staleContext.status, 'blocked');
    assert.ok(staleContext.reasons.includes('context_identity_stale'));

    const runtime = JSON.parse(await readFile(join(fixture.authorityRoot, packet.runtime.path), 'utf8')) as {
      engineSourceHash: string;
    };
    runtime.engineSourceHash = `sha256:${'e'.repeat(64)}`;
    packet.runtime = await writePrivateJson(fixture.authorityRoot, 'runtime.json', runtime) as typeof packet.runtime;
    await writeFile(packetFile, canonicalJson(packet as never));
    const staleRuntime = (await auditCardReadiness(fixture.root, fixture.packetPath)).cards[0]!;
    assert.equal(staleRuntime.status, 'blocked');
    assert.ok(staleRuntime.reasons.includes('context_identity_stale'));
    assert.ok(staleRuntime.reasons.includes('runtime_stale'));
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('readiness rejects a tracker source hash that differs from pinned normalized source', async () => {
  const fixture = await readinessFixture();
  try {
    const db = new DatabaseSync(fixture.dbPath);
    db.prepare('UPDATE cards SET source_hash=? WHERE card_id=?').run(`sha256:${'f'.repeat(64)}`, fixture.cardId);
    db.close();
    await assert.rejects(auditCardReadiness(fixture.root, fixture.packetPath), /current source identity mismatch/u);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('readiness output bytes do not depend on packet card order', async () => {
  const fixture = await readinessFixture(true);
  try {
    const packetFile = join(fixture.authorityRoot, fixture.packetPath);
    const packet = JSON.parse(await readFile(packetFile, 'utf8')) as { cards: unknown[] };
    const before = JSON.stringify(await auditCardReadiness(fixture.root, fixture.packetPath));
    packet.cards.reverse();
    await writeFile(packetFile, canonicalJson(packet as never));
    assert.equal(JSON.stringify(await auditCardReadiness(fixture.root, fixture.packetPath)), before);
  } finally { await rm(fixture.root, { recursive: true, force: true }); }
});

test('large-feed parser keeps duplicate, depth, node, and byte guards', () => {
  assert.throws(() => parseJsonWithDuplicateKeyCheck('{"a":1,"a":2}', { validateCanonical: false }), /duplicate_key/u);
  assert.throws(() => parseJsonWithDuplicateKeyCheck(`${'['.repeat(66)}0${']'.repeat(66)}`, { validateCanonical: false }), /max_depth/u);
  assert.throws(() => parseJsonWithDuplicateKeyCheck('"too-long"', { validateCanonical: false, maxStringBytes: 3 }), /max_string_bytes/u);
  assert.throws(() => parseJsonWithDuplicateKeyCheck('[1,2]', { validateCanonical: false, maxNodes: 2 }), /max_nodes/u);
});

test('bounded file identity detects a same-path replacement after the buffer read', async () => {
  const root = await mkdtemp(join(tmpdir(), 'bounded-identity-'));
  const path = join(root, 'session-json');
  try {
    await writeFile(path, 'old ELF bytes');
    const read = await readBoundedWithinAuthorityRoot(root, 'session-json', 128);
    assert.equal(read.status, 'ok');
    if (read.status !== 'ok') return;
    assert.equal(sameBoundedFileIdentity(read.identity, await lstat(path, { bigint: true })), true);
    await writeFile(path, 'new ELF bytes');
    assert.equal((await lstat(path, { bigint: true })).ino, read.identity.ino);
    assert.equal(sameBoundedFileIdentity(read.identity, await lstat(path, { bigint: true })), false);
    await rename(path, join(root, 'old-session-json'));
    await writeFile(path, 'new ELF bytes');
    assert.notEqual(Buffer.from(read.bytes).toString(), await readFile(path, 'utf8'));
    assert.equal(sameBoundedFileIdentity(read.identity, await lstat(path, { bigint: true })), false);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('tracker source hashes require exact valid identities', () => {
  const hashA = `sha256:${'a'.repeat(64)}`;
  const hashB = `sha256:${'b'.repeat(64)}`;
  assert.doesNotThrow(() => assertExactSourceHash(hashA, hashA, 'synthetic source'));
  assert.throws(() => assertExactSourceHash(hashA, hashB, 'synthetic source'), /source hash mismatch/u);
  assert.throws(() => assertExactSourceHash('not-a-hash', hashB, 'synthetic source'), /SHA-256/u);
});

test('tracker schema preserves null frequencies and manual decisions while constraining exact joins', () => {
  const db = new DatabaseSync(':memory:', { enableForeignKeyConstraints: true });
  try {
    db.exec(CARD_TRACKER_SCHEMA);
    db.prepare('INSERT INTO capability_groups VALUES (?,?,?,?,?,?,?,?,1)')
      .run('group-a', 'Synthetic group', 1, 'synthetic', 1, 0, '{}', 'hash');
    db.prepare('INSERT INTO cards VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,1)')
      .run('card-a', null, 'Synthetic card', 'Unit', '[]', '{}', 'synthetic text', 'source', 'authority', 'revision', 'unbound', null, null);
    db.prepare('INSERT INTO codex_entries VALUES (?,?,?,?,?,?,?,?,1)')
      .run('entry-a', 'Synthetic entry', 'https://example.invalid', null, '[]', '', 'entry-hash', 'snapshot-hash');
    db.prepare('INSERT INTO rule_slices VALUES (?,?,?,?,?,?,?,?,1)')
      .run('slice-a', 'requirement-a', 'card-a', 'group-a', 'unverified-contextual-requirement', 'family-a', '{}', 'slice-hash');
    db.prepare('INSERT INTO card_requirements VALUES (?,?,?,1)')
      .run('card-a', 'slice-a', 'source');
    db.prepare('INSERT INTO codex_refs VALUES (?,?,?,?,?)')
      .run('slice', 'slice-a', 'entry-a', 'context-only', 'shared-family-context');
    db.prepare('INSERT INTO card_status VALUES (?,?,?,?,?,?,?,?,?,?,?)')
      .run('card-a', 'remaining', 'pending', 'blocked', 0, 'source-review-required', null, '[]', '[]', null, '{}');
    const addManual = db.prepare('INSERT INTO manual_card_status(card_id) VALUES (?) ON CONFLICT(card_id) DO NOTHING');
    addManual.run('card-a');
    db.prepare("UPDATE manual_card_status SET status='reviewed',note='synthetic decision' WHERE card_id='card-a'").run();
    addManual.run('card-a');

    assert.equal(db.prepare('SELECT count(*) AS n FROM cards WHERE is_current=1').get()!.n, 1);
    assert.equal(db.prepare('SELECT selected_deck_frequency AS n FROM card_status WHERE card_id=?').get('card-a')!.n, null);
    const manual = db.prepare('SELECT status,note FROM manual_card_status WHERE card_id=?').get('card-a')!;
    assert.equal(manual.status, 'reviewed');
    assert.equal(manual.note, 'synthetic decision');
    assert.throws(() => db.prepare('INSERT INTO codex_refs VALUES (?,?,?,?,?)')
      .run('slice', 'slice-a', 'missing-entry', 'context-only', 'shared-family-context'), /FOREIGN KEY/u);
    assert.equal(db.prepare('PRAGMA foreign_key_check').all().length, 0);
  } finally {
    db.close();
  }
});

test('tracker expires stale acceptance annotations but preserves historical evidence and held work', () => {
  const db = new DatabaseSync(':memory:', { enableForeignKeyConstraints: true });
  try {
    db.exec(CARD_TRACKER_SCHEMA);
    db.prepare('INSERT INTO capability_groups VALUES (?,?,?,?,?,?,?,?,1)')
      .run('group', 'Synthetic group', 1, 'synthetic', 0, 0, '{}', 'group-hash');
    const addCard = (id: string, status = 'current-native-validation-accepted'): void => {
      db.prepare('INSERT INTO cards VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,1)')
        .run(id, null, id, 'minion', '[]', '{}', '', 'source-current', 'authority', 'revision', 'admitted-current', 'facts', '{}');
      db.prepare('INSERT INTO card_status VALUES (?,?,?,?,?,?,?,?,?,?,?)')
        .run(id, 'admitted', 'reviewed', status, 0, 'source-reviewed', 'source-current', '[]', '[]', null, '{}');
    };
    const addSlice = (id: string, suffix = 'a'): string => {
      const slice = `${id}-${suffix}`;
      db.prepare('INSERT INTO rule_slices VALUES (?,?,?,?,?,?,?,?,1)')
        .run(slice, suffix, id, 'group', 'proved', 'synthetic', '{}', 'slice-current');
      db.prepare('INSERT INTO card_requirements VALUES (?,?,?,1)').run(id, slice, 'source-current');
      return slice;
    };
    const addProof = (id: string, slice: string, source = 'source-current', sliceHash = 'slice-current', engine = 'engine-current', evidence: string | null = 'receipt'): void => {
      db.prepare('INSERT INTO proof_metadata VALUES (?,?,?,?,?,?,?,?,?)')
        .run(slice, id, slice, 'synthetic-proof', 'verified', source, sliceHash, engine, evidence);
    };
    for (const id of ['current', 'old-engine', 'old-source', 'old-slice', 'missing-slice', 'no-requirements', 'no-evidence', 'unverified']) addCard(id);
    addCard('held', 'blocked');
    addProof('current', addSlice('current'));
    addProof('current', addSlice('current', 'b'));
    addProof('old-engine', addSlice('old-engine'), 'source-current', 'slice-current', 'engine-old');
    addProof('old-source', addSlice('old-source'), 'source-old');
    addProof('old-slice', addSlice('old-slice'), 'source-current', 'slice-old');
    addProof('missing-slice', addSlice('missing-slice'));
    addSlice('missing-slice', 'b');
    addProof('no-evidence', addSlice('no-evidence'), 'source-current', 'slice-current', 'engine-current', null);
    addProof('unverified', addSlice('unverified'));
    db.exec("UPDATE proof_metadata SET status='referenced-unverified' WHERE card_id='unverified'");
    const evidenceBefore = db.prepare('SELECT * FROM proof_metadata ORDER BY proof_key').all();
    const bindingsBefore = db.prepare('SELECT * FROM cards ORDER BY card_id').all();

    expireStaleCardValidationAnnotations(db, 'engine-current');

    assert.equal(db.prepare("SELECT validation_status FROM card_status WHERE card_id='current'").get()!.validation_status, 'current-native-validation-accepted');
    assert.equal(db.prepare("SELECT validation_status FROM card_status WHERE card_id='held'").get()!.validation_status, 'blocked');
    assert.deepEqual(db.prepare("SELECT card_id FROM card_status WHERE validation_status='awaiting-current-engine-revalidation' AND validation_ready_annotation=1 ORDER BY card_id").all().map((row) => row.card_id),
      ['missing-slice', 'no-evidence', 'no-requirements', 'old-engine', 'old-slice', 'old-source', 'unverified']);
    assert.deepEqual(db.prepare('SELECT * FROM proof_metadata ORDER BY proof_key').all(), evidenceBefore);
    assert.deepEqual(db.prepare('SELECT * FROM cards ORDER BY card_id').all(), bindingsBefore);
    expireStaleCardValidationAnnotations(db, 'engine-current');
    assert.deepEqual(db.prepare('SELECT * FROM proof_metadata ORDER BY proof_key').all(), evidenceBefore);
  } finally {
    db.close();
  }
});
