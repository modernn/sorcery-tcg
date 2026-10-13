import { constants } from 'node:fs';
import { access, lstat, realpath } from 'node:fs/promises';
import { basename, isAbsolute, join, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { z } from 'zod';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { loadPrivateCardSnapshot } from '../authority/private-cards.ts';
import { identityHash, sha256 } from '../authority/hash.ts';
import { readBoundedWithinAuthorityRoot, sameBoundedFileIdentity } from '../authority/validate-bundle.ts';
import { mergeReviewedCardBindings, reviewedCardBindingsSchema } from '../ingestion/reviewed-card-bindings.ts';
import { engineSourceHash } from './card-tracker.ts';

const hash = z.string().regex(/^sha256:[a-f0-9]{64}$/u);
const privatePath = z.string().min(1).max(512).refine((value) => !value.startsWith('/') && !value.includes('\\')
  && value.split('/').every((part) => part !== '' && part !== '.' && part !== '..'), 'path must be authority-relative');
const ref = z.strictObject({ path: privatePath, sha256: hash });
const packetSchema = z.strictObject({
  schemaVersion: z.literal(1), candidateBindings: ref, runtime: z.nullable(ref),
  cards: z.array(z.strictObject({ cardId: z.string().min(1).max(256), contextReview: z.nullable(ref),
    sharedProofs: z.array(ref).max(10_000), validation: z.nullable(ref) })).min(1).max(10_000),
});
const contextSchema = z.strictObject({
  schemaVersion: z.literal(1), kind: z.literal('card-context-review'), cardId: z.string().min(1).max(256),
  reviewer: z.string().max(256).refine((value) => value.trim().length > 0), authorityHash: hash, authorityRevision: z.string().min(1).max(128),
  codexSnapshotHash: hash, sourceCardHash: hash, candidateFactsHash: hash, completeContext: z.literal(true),
  requirements: z.array(z.strictObject({ sliceId: z.string().min(1).max(256), sourceHash: hash })).max(10_000),
  unresolved: z.array(z.string().max(2048)).max(10_000), evidence: z.array(ref).min(1).max(128),
});
const runtimeSchema = z.strictObject({
  schemaVersion: z.literal(1), kind: z.literal('runtime-qualification'), status: z.enum(['passed', 'failed']),
  engineSourceHash: hash, executable: z.strictObject({ path: z.string().min(1).max(4096), sha256: hash }),
  evidence: z.array(ref).min(1).max(128),
});
const proofSchema = z.strictObject({
  schemaVersion: z.literal(1), kind: z.literal('shared-rule-proof'), status: z.enum(['passed', 'failed']),
  cardId: z.string().min(1).max(256), authorityHash: hash, codexSnapshotHash: hash, sourceCardHash: hash,
  candidateFactsHash: hash, sliceId: z.string().min(1).max(256), sliceHash: hash, engineSourceHash: hash,
  executableSha256: hash, evidence: z.array(ref).min(1).max(128),
});
const validationSchema = z.strictObject({
  schemaVersion: z.literal(1), kind: z.literal('card-validation'), status: z.enum(['passed', 'failed']),
  cardId: z.string().min(1).max(256), dependencyFingerprint: hash, evidence: z.array(ref).min(1).max(128),
});

type Ref = z.infer<typeof ref>;
type Slice = { sliceId: string; sourceHash: string; sourceCardHash: string; requirementId: string; cardId: string };
type CardResult = { cardId: string; status: 'blocked' | 'candidate-ready' | 'currently-validated'; reasons: string[];
  dependencyFingerprint: string | null; admission: { bindingStatus: string; factsMatch: boolean } };
const MAX_ARTIFACT = 16 * 1024 * 1024;
const MAX_EXECUTABLE = 1_073_741_824;

function record(value: JsonValue, label: string): Record<string, JsonValue> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new Error(`${label} must be an object`);
  return value as Record<string, JsonValue>;
}

function string(value: JsonValue, label: string): string {
  if (typeof value !== 'string' || value.length === 0) throw new Error(`${label} must be nonempty text`);
  return value;
}

async function readPrivateJson<T>(root: string, reference: Ref, schema: z.ZodType<T>, limit = MAX_ARTIFACT): Promise<{ value: T; hash: string }> {
  const read = await readBoundedWithinAuthorityRoot(root, reference.path, limit);
  if (read.status !== 'ok' || sha256(read.bytes) !== reference.sha256) throw new Error(`invalid or stale artifact: ${reference.path}`);
  return { value: schema.parse(parseJsonWithDuplicateKeyCheck(Buffer.from(read.bytes).toString('utf8'), { validateCanonical: false })), hash: reference.sha256 };
}

async function verifyEvidence(root: string, refs: readonly Ref[]): Promise<string[]> {
  if (refs.length === 0) throw new Error('evidence references must be nonempty');
  const hashes: string[] = [];
  for (const item of refs) {
    const read = await readBoundedWithinAuthorityRoot(root, item.path, MAX_ARTIFACT);
    if (read.status !== 'ok' || read.bytes.byteLength === 0 || sha256(read.bytes) !== item.sha256) {
      throw new Error(`missing, empty, or stale evidence: ${item.path}`);
    }
    hashes.push(item.sha256);
  }
  return hashes;
}

async function executableHash(path: string): Promise<string> {
  if (!isAbsolute(path) || resolve(path) !== path) throw new Error('executable path must be canonical and absolute');
  if (await realpath(path) !== path) throw new Error('executable path must be canonical and absolute');
  const read = await readBoundedWithinAuthorityRoot('/', path.slice(1), MAX_EXECUTABLE);
  if (read.status !== 'ok') throw new Error('executable must be a bounded ordinary file');
  if (await realpath(path) !== path) throw new Error('executable path must be canonical and absolute');
  await access(path, constants.X_OK);
  const metadata = await lstat(path, { bigint: true });
  if (metadata.isSymbolicLink() || !metadata.isFile() || (metadata.mode & 0o111n) === 0n
    || !sameBoundedFileIdentity(read.identity, metadata)) throw new Error('runtime executable path changed after hashing');
  const bytes = Buffer.from(read.bytes);
  if (bytes.byteLength < 4 || bytes.subarray(0, 4).toString('hex') !== '7f454c46') throw new Error('runtime executable must be an ELF binary');
  return sha256(bytes);
}

function compare(a: string, b: string): number { return a < b ? -1 : a > b ? 1 : 0; }

export type CardReadinessReport = Readonly<{
  schemaVersion: 1; engineSourceHash: string; codexSnapshotHash: string; authorityHash: string; authorityRevision: string;
  limitations: readonly string[]; cards: readonly CardResult[];
}>;

/** Read-only audit of source-guarded candidates and immutable review/validation receipts. */
export async function auditCardReadiness(repositoryRoot = process.cwd(), packetPath = 'catalog/readiness-packet.json'): Promise<CardReadinessReport> {
  const root = resolve(repositoryRoot);
  const authorityRoot = join(root, '.local', 'authority');
  const rootReal = await realpath(root);
  if (rootReal !== root) throw new Error('repository root must be canonical');
  for (const path of [join(root, '.local'), authorityRoot, join(authorityRoot, 'scenarios'), join(authorityRoot, 'revisions'),
    join(authorityRoot, 'codex'), join(authorityRoot, 'catalog')]) {
    const info = await lstat(path);
    if (info.isSymbolicLink() || !info.isDirectory() || await realpath(path) !== path) throw new Error('private authority directories must be real directories inside the repository');
  }
  const scenarioRead = await readBoundedWithinAuthorityRoot(authorityRoot, 'scenarios/vanilla-constructed.json', MAX_ARTIFACT);
  if (scenarioRead.status !== 'ok') throw new Error('private authority scenario is unavailable');
  const scenario = record(parseJsonWithDuplicateKeyCheck(Buffer.from(scenarioRead.bytes).toString('utf8'), { validateCanonical: false }), 'authority scenario');
  const revisionId = string(scenario.revisionId!, 'authority revision');
  if (basename(revisionId) !== revisionId) throw new Error('private revision ID must be one path segment');
  const revisionRoot = join(authorityRoot, 'revisions', revisionId);
  for (const path of [revisionRoot, join(revisionRoot, 'cards.normalized.json'), join(revisionRoot, 'formats.json')]) {
    const info = await lstat(path);
    if (info.isSymbolicLink() || (path === revisionRoot ? !info.isDirectory() : !info.isFile()) || await realpath(path) !== path) {
      throw new Error('pinned source files must be ordinary files inside the private authority revision');
    }
  }
  const normalizedRead = await readBoundedWithinAuthorityRoot(authorityRoot, `revisions/${revisionId}/cards.normalized.json`, MAX_ARTIFACT);
  const formatsRead = await readBoundedWithinAuthorityRoot(authorityRoot, `revisions/${revisionId}/formats.json`, MAX_ARTIFACT);
  if (normalizedRead.status !== 'ok' || formatsRead.status !== 'ok') throw new Error('pinned authority source is unavailable');
  const packetRead = await readBoundedWithinAuthorityRoot(authorityRoot, packetPath, MAX_ARTIFACT);
  if (packetRead.status !== 'ok') throw new Error('audit packet is unavailable or exceeds fixed size limit');
  const packet = packetSchema.parse(parseJsonWithDuplicateKeyCheck(Buffer.from(packetRead.bytes).toString('utf8'), { validateCanonical: false }));
  const candidate = (await readPrivateJson(authorityRoot, packet.candidateBindings, reviewedCardBindingsSchema)).value;
  if (candidate.cards.some((row) => Object.hasOwn(row, 'replacesFactsHash'))) throw new Error('candidate bindings may not contain replacement metadata');
  const authority = await loadPrivateCardSnapshot(join(authorityRoot, 'scenarios/vanilla-constructed.json'), root);
  if (candidate.authorityHash !== authority.authorityHash || candidate.revisionId !== authority.revisionId) throw new Error('candidate authority identity is stale');
  const candidateIds = candidate.cards.map(({ cardId }) => cardId);
  const packetIds = packet.cards.map(({ cardId }) => cardId);
  if (new Set(candidateIds).size !== candidateIds.length || new Set(packetIds).size !== packetIds.length
    || candidateIds.length !== packetIds.length || candidateIds.some((id) => !packetIds.includes(id))) throw new Error('packet and candidate card identities differ or duplicate');
  const candidateFacts = new Map(candidate.cards.map((row) => [row.cardId, identityHash(row.facts as JsonValue)]));
  const reviewInput = { schemaVersion: 1, authorityHash: candidate.authorityHash, revisionId: candidate.revisionId, cards: candidate.cards };
  mergeReviewedCardBindings(reviewInput, authority, new Map());

  const codexRead = await readBoundedWithinAuthorityRoot(authorityRoot, 'codex/official-codex.snapshot.json', MAX_ARTIFACT);
  if (codexRead.status !== 'ok') throw new Error('pinned Codex snapshot is unavailable');
  parseJsonWithDuplicateKeyCheck(Buffer.from(codexRead.bytes).toString('utf8'), { validateCanonical: false });
  const codexSnapshotHash = sha256(codexRead.bytes);
  const currentEngineHash = await engineSourceHash(root);
  const executable = packet.runtime ? (await readPrivateJson(authorityRoot, packet.runtime, runtimeSchema)).value : null;
  let executableSha256: string | null = null;
  let runtimeReceiptHash: string | null = null;
  if (executable) {
    await verifyEvidence(authorityRoot, executable.evidence);
    executableSha256 = await executableHash(executable.executable.path);
    runtimeReceiptHash = packet.runtime?.sha256 ?? null;
  }
  const dbPath = join(authorityRoot, 'catalog/card-tracker.sqlite3');
  const dbInfo = await lstat(dbPath);
  if (dbInfo.isSymbolicLink() || !dbInfo.isFile() || await realpath(dbPath) !== dbPath) throw new Error('tracker database must be a private ordinary file');
  const db = new DatabaseSync(dbPath, { readOnly: true, enableForeignKeyConstraints: true });
  let metas: Record<string, string>;
  let cardRows: Record<string, unknown>[];
  const slicesByCard = new Map<string, Slice[]>();
  try {
    db.exec('BEGIN');
    metas = Object.fromEntries(db.prepare('SELECT key,value FROM tracker_meta').all().map((r) => [String(r.key), String(r.value)]));
    const ids = packet.cards.map(({ cardId }) => cardId);
    const marks = ids.map(() => '?').join(',');
    cardRows = db.prepare(`SELECT c.card_id AS cardId,c.source_hash AS sourceHash,c.authority_hash AS authorityHash,
      c.authority_revision AS authorityRevision,c.binding_status AS bindingStatus,c.facts_hash AS factsHash,c.facts_json AS factsJson
      FROM cards c WHERE c.is_current=1 AND c.card_id IN (${marks}) ORDER BY c.card_id`).all(...ids);
    const links = db.prepare(`SELECT r.card_id AS cardId,r.slice_id AS sliceId,r.source_card_hash AS sourceCardHash,
      r.is_current AS linkCurrent,s.requirement_id AS requirementId,s.payload_json AS payloadJson,s.source_hash AS sourceHash,
      s.card_id AS sliceCardId,s.is_current AS sliceCurrent FROM card_requirements r LEFT JOIN rule_slices s USING(slice_id)
      WHERE r.card_id IN (${marks}) OR s.card_id IN (${marks}) ORDER BY r.card_id,r.slice_id`).all(...ids, ...ids);
    for (const row of links) {
      if (row.linkCurrent !== 1 && row.sliceCurrent !== 1) continue;
      if (row.linkCurrent !== 1 || row.sliceCurrent !== 1 || row.cardId !== row.sliceCardId) throw new Error('stale or wrong-owner current rule slice');
      if (typeof row.requirementId !== 'string' || typeof row.payloadJson !== 'string' || typeof row.sourceHash !== 'string') throw new Error('current requirement row is incomplete');
      const payload = String(row.payloadJson);
      if (sha256(Buffer.from(payload)) !== row.sourceHash) throw new Error(`rule slice byte hash mismatch: ${String(row.sliceId)}`);
      const list = slicesByCard.get(String(row.cardId)) ?? [];
      list.push({ cardId: String(row.cardId), sliceId: String(row.sliceId), sourceHash: String(row.sourceHash),
        sourceCardHash: String(row.sourceCardHash), requirementId: String(row.requirementId) });
      slicesByCard.set(String(row.cardId), list);
    }
    const currentSlices = db.prepare(`SELECT s.card_id AS cardId,s.slice_id AS sliceId,s.requirement_id AS requirementId,
      s.payload_json AS payloadJson,s.source_hash AS sourceHash,r.card_id AS linkCardId,r.source_card_hash AS sourceCardHash,
      r.is_current AS linkCurrent FROM rule_slices s LEFT JOIN card_requirements r
        ON r.slice_id=s.slice_id AND r.card_id=s.card_id
      WHERE s.card_id IN (${marks}) AND s.is_current=1 ORDER BY s.card_id,s.slice_id`).all(...ids);
    for (const row of currentSlices) {
      if (row.linkCurrent !== 1 || row.linkCardId !== row.cardId) throw new Error('current rule slice is missing its exact card requirement link');
    }
    db.exec('COMMIT');
  } catch (error) { db.exec('ROLLBACK'); throw error; }
  finally { db.close(); }

  if (metas.authorityHash !== authority.authorityHash || metas.revisionId !== authority.revisionId || metas.codexSnapshotHash !== codexSnapshotHash) {
    throw new Error('tracker metadata is stale against pinned authority or Codex');
  }
  const dbCards = new Map(cardRows.map((row) => [String(row.cardId), row]));
  const sourceCards = new Map(authority.cards.map((card) => [card.stableId, card]));
  const results: CardResult[] = [];
  for (const packetCard of [...packet.cards].sort((a, b) => compare(a.cardId, b.cardId))) {
    const { cardId } = packetCard;
    const dbCard = dbCards.get(cardId);
    const source = sourceCards.get(cardId);
    if (!dbCard || !source) throw new Error(`current card/source row missing: ${cardId}`);
    const sourceCardHash = identityHash(source as unknown as JsonValue);
    if (dbCard.sourceHash !== sourceCardHash || dbCard.authorityHash !== authority.authorityHash || dbCard.authorityRevision !== authority.revisionId) throw new Error(`current source identity mismatch: ${cardId}`);
    const factsHash = candidateFacts.get(cardId)!;
    const slices = (slicesByCard.get(cardId) ?? []).sort((a, b) => compare(a.sliceId, b.sliceId));
    if (new Set(slices.map(({ sliceId }) => sliceId)).size !== slices.length) throw new Error(`duplicate current requirement slice: ${cardId}`);
    if (slices.some((slice) => slice.cardId !== cardId || slice.sourceCardHash !== sourceCardHash)) throw new Error(`invalid current requirement source: ${cardId}`);
    const requirements = slices.map(({ sliceId, sourceHash }) => ({ sliceId, sourceHash }));
    const reasons = new Set<string>();
    let contextHash: string | null = null;
    let context: z.infer<typeof contextSchema> | null = null;
    if (packetCard.contextReview === null) reasons.add('context_review_missing');
    else {
      const read = await readPrivateJson(authorityRoot, packetCard.contextReview, contextSchema);
      contextHash = read.hash; context = read.value;
      await verifyEvidence(authorityRoot, context.evidence);
      if (context.cardId !== cardId || context.authorityHash !== authority.authorityHash || context.authorityRevision !== authority.revisionId
        || context.codexSnapshotHash !== codexSnapshotHash || context.sourceCardHash !== sourceCardHash || context.candidateFactsHash !== factsHash) reasons.add('context_identity_stale');
      if (context.unresolved.length) reasons.add('context_unresolved');
      if (new Set(context.requirements.map(({ sliceId }) => sliceId)).size !== context.requirements.length) throw new Error(`duplicate context requirement: ${cardId}`);
      const contextRequirements = [...context.requirements].sort((a, b) => compare(a.sliceId, b.sliceId));
      if (canonicalJson(contextRequirements as unknown as JsonValue) !== canonicalJson(requirements as unknown as JsonValue)) reasons.add('context_requirements_stale');
    }
    const proofHashes: string[] = [];
    const proofBySlice = new Map<string, z.infer<typeof proofSchema>>();
    for (const proofRef of packetCard.sharedProofs) {
      const read = await readPrivateJson(authorityRoot, proofRef, proofSchema);
      await verifyEvidence(authorityRoot, read.value.evidence);
      if (read.value.cardId !== cardId) throw new Error(`shared proof has conflicting card identity: ${cardId}`);
      proofHashes.push(read.hash);
      if (proofBySlice.has(read.value.sliceId)) throw new Error(`duplicate shared proof slice: ${cardId}/${read.value.sliceId}`);
      proofBySlice.set(read.value.sliceId, read.value);
    }
    if ([...proofBySlice.keys()].some((id) => !requirements.some((req) => req.sliceId === id))) throw new Error(`unexpected shared proof slice for ${cardId}`);
    if (context) {
      for (const slice of requirements) {
        const proof = proofBySlice.get(slice.sliceId);
        if (!proof) { reasons.add('shared_proof_missing'); continue; }
        if (proof.cardId !== cardId || proof.authorityHash !== authority.authorityHash || proof.codexSnapshotHash !== codexSnapshotHash
          || proof.sourceCardHash !== sourceCardHash || proof.candidateFactsHash !== factsHash || proof.sliceHash !== slice.sourceHash
          || proof.engineSourceHash !== currentEngineHash || proof.executableSha256 !== executableSha256) reasons.add('shared_proof_stale');
        else if (proof.status !== 'passed') reasons.add('shared_proof_failed');
      }
    }
    if (!executable) reasons.add('runtime_missing');
    else {
      if (executable.engineSourceHash !== currentEngineHash || executableSha256 !== executable.executable.sha256) reasons.add('runtime_stale');
      if (executable.status !== 'passed') reasons.add('runtime_failed');
    }
    const dependencyFingerprint = contextHash && executable && executableSha256 && runtimeReceiptHash && !reasons.has('context_identity_stale')
      ? identityHash({ schema: 'card-validation-deps/1', authorityHash: authority.authorityHash, authorityRevision: authority.revisionId,
        codexSnapshotHash, cardId, sourceCardHash, candidateFactsHash: factsHash, requirements,
        engineSourceHash: currentEngineHash, executable: { path: executable.executable.path, sha256: executableSha256 },
        runtimeReceiptHash, contextReviewHash: contextHash, sharedProofHashes: proofHashes.sort(compare) } as unknown as JsonValue)
      : null;
    let validation: z.infer<typeof validationSchema> | null = null;
    if (packetCard.validation) {
      validation = (await readPrivateJson(authorityRoot, packetCard.validation, validationSchema)).value;
      await verifyEvidence(authorityRoot, validation.evidence);
    }
    const admissionFacts = dbCard.factsJson ? JSON.parse(String(dbCard.factsJson)) as JsonValue : null;
    const factsMatch = admissionFacts !== null && canonicalJson(admissionFacts) === canonicalJson(candidate.cards.find((c) => c.cardId === cardId)!.facts as JsonValue);
    const prevalidationReady = reasons.size === 0 && dependencyFingerprint !== null;
    if (!prevalidationReady) { /* codes above describe exact blockers */ }
    else if (!validation) reasons.add('card_validation_missing');
    else if (validation.cardId !== cardId || validation.dependencyFingerprint !== dependencyFingerprint) reasons.add('card_validation_stale');
    else if (validation.status !== 'passed') reasons.add('card_validation_failed');
    const currentlyValidated = prevalidationReady && validation?.status === 'passed' && validation.cardId === cardId
      && validation.dependencyFingerprint === dependencyFingerprint;
    results.push({ cardId, status: currentlyValidated ? 'currently-validated' : prevalidationReady ? 'candidate-ready' : 'blocked',
      reasons: [...reasons].sort(compare), dependencyFingerprint,
      admission: { bindingStatus: String(dbCard.bindingStatus), factsMatch } });
  }
  return { schemaVersion: 1, engineSourceHash: currentEngineHash, codexSnapshotHash, authorityHash: authority.authorityHash,
    authorityRevision: authority.revisionId, limitations: ['Runtime qualification is for the recorded host environment; OS and shared-library closure and cross-platform compatibility are not proven.'],
    cards: results };
}
