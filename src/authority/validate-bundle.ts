import { lstat, open, readdir, realpath, stat } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep, win32 } from 'node:path';

import { canonicalJson, type JsonValue } from './canonical-json.ts';
import { sha256 } from './hash.ts';
import {
  AuthorityValidationError,
  DEFAULT_AUTHORITY_JSON_LIMITS,
  parseAuthorityJson,
  sortDiagnostics,
  validateFormatArtifact,
  validateNormalizedCardSnapshot,
  validateAuthorityBundle as validateBundleSchema,
  validateCanonicalArtifact,
  type ArtifactKind,
  type AuthorityBundle,
  type Diagnostic,
  type Hash,
  type IdentityDocument,
  type SourceRecord,
  type SourceRef,
} from './schemas.ts';

const MAX_AUTHORITY_BYTES = 32_000_000;
const MAX_AUTHORITY_FILES = 512;
const MAX_AUTHORITY_GRAPH_DEPTH = 64;
const MAX_AUTHORITY_REFERENCES = 4_096;
const MAX_AUTHORITY_DIAGNOSTICS = DEFAULT_AUTHORITY_JSON_LIMITS.maxDiagnostics;

type ExpectedBundle = Readonly<{ stableId: string; contentHash: string }>;

export type AuthorityKind = 'rulebook' | 'format' | 'codex' | 'faq' | 'card-update' | 'card-data';

export type AuthorityPrecedenceRecord = Readonly<{
  source: SourceRecord;
  authorityKind: AuthorityKind;
  topic: string;
  scope: string | null;
  supersedes: readonly string[];
}>;

export type ResolutionResult =
  | Readonly<{
      status: 'resolved';
      reason: null;
      winning: SourceRef;
      contending: readonly SourceRef[];
      superseded: readonly SourceRef[];
      provenance: readonly SourceRef[];
    }>
  | Readonly<{
      status: 'unsupported';
      reason:
        | 'no-official-authority'
        | 'unclear-date'
        | 'unclear-scope'
        | 'mixed-topic'
        | 'broken-supersession'
        | 'invalid-supersession-order'
        | 'ambiguous-supersession'
        | 'equal-rank';
      winning: null;
      contending: readonly SourceRef[];
      superseded: readonly SourceRef[];
      provenance: readonly SourceRef[];
    }>;

export type ValidatedPrecedenceResolution = Readonly<{
  topic: string;
  scope: string | null;
  resolution: ResolutionResult;
}>;

export type ValidatedAuthorityBundle = Readonly<{
  bundle: AuthorityBundle;
  resolvedBundlePath: string;
  storedBytesRehashed: readonly SourceRef[];
  manifestByteBindingsVerified: readonly SourceRef[];
  manifestDeclarationsBound: readonly SourceRef[];
  precedenceResolutions: readonly ValidatedPrecedenceResolution[];
}>;

type GraphNode = Readonly<{
  artifactKind: ArtifactKind;
  stableId: string;
  contentHash: Hash;
  identity: IdentityDocument<JsonValue>;
  path: string;
}>;

function fail(diagnostics: readonly Diagnostic[]): never {
  throw new AuthorityValidationError(sortDiagnostics(diagnostics).slice(0, MAX_AUTHORITY_DIAGNOSTICS));
}

function isErrno(error: unknown, code: string): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    (error as { code?: unknown }).code === code
  );
}

function pathError(code: string, message: string): AuthorityValidationError {
  return new AuthorityValidationError([{ path: '', code, message }]);
}

type BoundedReadResult =
  | Readonly<{ status: 'ok'; bytes: Uint8Array; resolvedPath: string }>
  | Readonly<{ status: 'not-file' }>
  | Readonly<{ status: 'too-large' }>
  | Readonly<{ status: 'unreadable' }>;

function normalizedFilesystemPath(value: string): string {
  const normalized = resolve(value);
  return process.platform === 'win32' ? normalized.toLowerCase() : normalized;
}

function isWithin(root: string, candidate: string): boolean {
  const fromRoot = relative(normalizedFilesystemPath(root), normalizedFilesystemPath(candidate));
  return fromRoot === '' || (fromRoot !== '..' && !fromRoot.startsWith('..' + sep) && !isAbsolute(fromRoot));
}

async function lexicalAuthorityPath(
  root: string,
  candidate: string,
): Promise<Readonly<{ resolvedRoot: string; lexicalCandidate: string }>> {
  const segments = candidate.split('/');
  if (
    candidate.length === 0 ||
    candidate.includes('\\') ||
    candidate.includes('\0') ||
    isAbsolute(candidate) ||
    win32.isAbsolute(candidate) ||
    segments.some((segment) => segment === '' || segment === '.' || segment === '..')
  ) {
    throw pathError('path_escape', 'path must be a confined slash-separated relative path');
  }

  let resolvedRoot: string;
  try {
    resolvedRoot = await realpath(root);
  } catch (error: unknown) {
    if (isErrno(error, 'ENOENT')) throw pathError('path_not_found', 'authority root does not exist');
    throw pathError('path_unreadable', 'authority root cannot be resolved');
  }

  const lexicalCandidate = resolve(resolvedRoot, ...segments);
  if (!isWithin(resolvedRoot, lexicalCandidate)) {
    throw pathError('path_escape', 'path escapes the configured authority root');
  }
  return { resolvedRoot, lexicalCandidate };
}

export async function resolveWithinAuthorityRoot(root: string, candidate: string): Promise<string> {
  const { resolvedRoot, lexicalCandidate } = await lexicalAuthorityPath(root, candidate);
  let resolvedCandidate: string;
  try {
    resolvedCandidate = await realpath(lexicalCandidate);
  } catch (error: unknown) {
    if (isErrno(error, 'ENOENT')) throw pathError('path_not_found', 'authority path does not exist');
    throw pathError('path_unreadable', 'authority path cannot be resolved');
  }

  if (!isWithin(resolvedRoot, resolvedCandidate)) {
    throw pathError('path_escape', 'resolved path escapes the configured authority root');
  }
  return resolvedCandidate;
}

function sameIdentity(
  left: Readonly<{ dev: bigint; ino: bigint }>,
  right: Readonly<{ dev: bigint; ino: bigint }>,
): boolean {
  return left.dev === right.dev && left.ino === right.ino;
}

function requireIdentity(value: Readonly<{ dev: bigint; ino: bigint }>): void {
  if (value.dev === 0n || value.ino === 0n) {
    throw pathError('filesystem_identity_unavailable', 'file identity is unavailable');
  }
}

export async function readBoundedWithinAuthorityRoot(
  root: string,
  candidate: string,
  maxBytes: number,
): Promise<BoundedReadResult> {
  const { resolvedRoot, lexicalCandidate } = await lexicalAuthorityPath(root, candidate);
  let linkMetadata: Awaited<ReturnType<typeof lstat>>;
  try {
    linkMetadata = await lstat(lexicalCandidate, { bigint: true });
  } catch (error: unknown) {
    if (isErrno(error, 'ENOENT')) throw pathError('path_not_found', 'authority path does not exist');
    throw pathError('path_unreadable', 'authority path cannot be inspected');
  }
  requireIdentity(linkMetadata);
  if (linkMetadata.isSymbolicLink()) {
    throw pathError('filesystem_alias', 'authority file may not be a symlink or junction');
  }
  if (!linkMetadata.isFile()) return { status: 'not-file' };

  let resolvedCandidate: string;
  let pathMetadata: Awaited<ReturnType<typeof stat>>;
  try {
    resolvedCandidate = await realpath(lexicalCandidate);
    pathMetadata = await stat(lexicalCandidate, { bigint: true });
  } catch {
    throw pathError('path_unreadable', 'authority path cannot be resolved');
  }
  requireIdentity(pathMetadata);
  if (!isWithin(resolvedRoot, resolvedCandidate)) {
    throw pathError('path_escape', 'resolved path escapes the configured authority root');
  }
  if (
    normalizedFilesystemPath(lexicalCandidate) !== normalizedFilesystemPath(resolvedCandidate) ||
    !sameIdentity(linkMetadata, pathMetadata)
  ) {
    throw pathError('filesystem_alias', 'authority file must equal its verified real path');
  }

  let handle: Awaited<ReturnType<typeof open>>;
  try {
    handle = await open(lexicalCandidate, 'r');
  } catch {
    return { status: 'unreadable' };
  }
  try {
    const openedMetadata = await handle.stat({ bigint: true });
    requireIdentity(openedMetadata);
    if (!openedMetadata.isFile() || !sameIdentity(pathMetadata, openedMetadata)) {
      throw pathError('filesystem_alias', 'opened authority file differs from the verified path');
    }
    if (openedMetadata.size > BigInt(maxBytes)) return { status: 'too-large' };

    const chunks: Uint8Array[] = [];
    const buffer = Buffer.allocUnsafe(Math.min(65_536, maxBytes + 1));
    let total = 0;
    while (true) {
      const allowance = Math.min(buffer.byteLength, maxBytes + 1 - total);
      const { bytesRead } = await handle.read(buffer, 0, allowance, null);
      if (bytesRead === 0) break;
      total += bytesRead;
      if (total > maxBytes) return { status: 'too-large' };
      chunks.push(Buffer.from(buffer.subarray(0, bytesRead)));
    }

    let afterOpenMetadata: Awaited<ReturnType<typeof handle.stat>>;
    let afterPathMetadata: Awaited<ReturnType<typeof lstat>>;
    try {
      afterOpenMetadata = await handle.stat({ bigint: true });
      afterPathMetadata = await lstat(lexicalCandidate, { bigint: true });
    } catch {
      throw pathError('file_changed', 'authority file changed during verification');
    }
    requireIdentity(afterOpenMetadata);
    requireIdentity(afterPathMetadata);
    if (
      afterPathMetadata.isSymbolicLink() ||
      !sameIdentity(openedMetadata, afterOpenMetadata) ||
      !sameIdentity(openedMetadata, afterPathMetadata) ||
      openedMetadata.size !== afterOpenMetadata.size ||
      openedMetadata.size !== BigInt(total)
    ) {
      throw pathError('file_changed', 'authority file changed during verification');
    }
    return { status: 'ok', bytes: Buffer.concat(chunks, total), resolvedPath: resolvedCandidate };
  } catch (error: unknown) {
    if (error instanceof AuthorityValidationError) throw error;
    return { status: 'unreadable' };
  } finally {
    await handle.close().catch(() => undefined);
  }
}

function toSourceRef(entry: AuthorityPrecedenceRecord): SourceRef {
  return { sourceId: entry.source.sourceId, byteHash: entry.source.byteHash };
}

function unsupportedResolution(
  reason: Extract<ResolutionResult, { status: 'unsupported' }>['reason'],
  contending: readonly AuthorityPrecedenceRecord[],
  superseded: readonly AuthorityPrecedenceRecord[],
  provenance: readonly SourceRef[],
): ResolutionResult {
  return Object.freeze({
    status: 'unsupported',
    reason,
    winning: null,
    contending: Object.freeze(contending.map(toSourceRef).sort(compareSourceRefs)),
    superseded: Object.freeze(superseded.map(toSourceRef).sort(compareSourceRefs)),
    provenance: Object.freeze([...provenance].sort(compareSourceRefs)),
  });
}

function precedenceRank(entry: AuthorityPrecedenceRecord, scope: string | null): number {
  if (scope !== null && entry.scope === scope) {
    if (scope.startsWith('card:') && (entry.authorityKind === 'card-update' || entry.authorityKind === 'card-data')) {
      return 90;
    }
    if (entry.authorityKind === 'format') return 80;
    if (entry.authorityKind === 'codex' || entry.authorityKind === 'faq') return 70;
  }
  if (entry.authorityKind === 'card-update' || entry.authorityKind === 'card-data') return 60;
  if (entry.authorityKind === 'codex' || entry.authorityKind === 'faq') return 50;
  return 40;
}

function isStrictDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const parsed = new Date(`${value}T00:00:00.000Z`);
  return !Number.isNaN(parsed.getTime()) && parsed.toISOString().slice(0, 10) === value;
}

export function resolveAuthorityPrecedence(
  records: readonly AuthorityPrecedenceRecord[],
  scope: string | null,
  effectiveAt: string,
): ResolutionResult {
  const provenance = records
    .filter((entry) => entry.source.authorityClass !== 'official')
    .map(toSourceRef)
    .sort(compareSourceRefs);
  const official = records.filter((entry) => entry.source.authorityClass === 'official');
  if (new Set(official.map((entry) => entry.topic)).size > 1) {
    return unsupportedResolution('mixed-topic', official, [], provenance);
  }
  if (!isStrictDate(effectiveAt)) {
    return unsupportedResolution('unclear-date', official, [], provenance);
  }

  const scoped = official.filter((entry) => entry.scope === null || entry.scope === scope);
  if (
    scoped.some(
      (entry) => entry.source.effectiveDate === null || !isStrictDate(entry.source.effectiveDate),
    )
  ) {
    return unsupportedResolution('unclear-date', scoped, [], provenance);
  }
  if (
    scoped.some(
      (entry) =>
        (entry.authorityKind === 'card-update' || entry.authorityKind === 'card-data') &&
        (entry.scope === null || !entry.scope.startsWith('card:')),
    )
  ) {
    return unsupportedResolution('unclear-scope', scoped, [], provenance);
  }

  const applicable = scoped.filter((entry) => entry.source.effectiveDate! <= effectiveAt);
  if (applicable.length === 0) {
    return unsupportedResolution('no-official-authority', [], [], provenance);
  }

  const applicableIds = new Set(applicable.map((entry) => entry.source.sourceId));
  if (applicable.some((entry) => entry.supersedes.some((sourceId) => !applicableIds.has(sourceId)))) {
    return unsupportedResolution('broken-supersession', applicable, [], provenance);
  }
  const supersessionCount = applicable.reduce((count, entry) => count + entry.supersedes.length, 0);
  if (supersessionCount > MAX_AUTHORITY_REFERENCES || applicableIds.size !== applicable.length) {
    return unsupportedResolution('ambiguous-supersession', applicable, [], provenance);
  }

  const applicableById = new Map(applicable.map((entry) => [entry.source.sourceId, entry]));
  const supersessionState = new Map<string, 0 | 1 | 2>();
  let invalidSupersessionGraph = false;
  function visitSupersession(entry: AuthorityPrecedenceRecord, depth: number): void {
    supersessionState.set(entry.source.sourceId, 1);
    for (const targetId of entry.supersedes) {
      const target = applicableById.get(targetId)!;
      if (supersessionState.get(targetId) === 1 || depth >= MAX_AUTHORITY_GRAPH_DEPTH) {
        invalidSupersessionGraph = true;
        continue;
      }
      if (supersessionState.get(targetId) !== 2) visitSupersession(target, depth + 1);
    }
    supersessionState.set(entry.source.sourceId, 2);
  }
  for (const entry of applicable) {
    if (supersessionState.get(entry.source.sourceId) === undefined) visitSupersession(entry, 0);
  }
  if (invalidSupersessionGraph) {
    return unsupportedResolution('ambiguous-supersession', applicable, [], provenance);
  }
  if (
    applicable.some((entry) =>
      entry.supersedes.some((sourceId) =>
        entry.source.effectiveDate! <= applicableById.get(sourceId)!.source.effectiveDate!
      )
    )
  ) {
    return unsupportedResolution('invalid-supersession-order', applicable, [], provenance);
  }

  const supersededIds = new Set(applicable.flatMap((entry) => entry.supersedes));
  const viable = applicable.filter((entry) => !supersededIds.has(entry.source.sourceId));
  if (viable.length === 0) {
    return unsupportedResolution('ambiguous-supersession', applicable, [], provenance);
  }

  const highestRank = Math.max(...viable.map((entry) => precedenceRank(entry, scope)));
  const highest = viable.filter((entry) => precedenceRank(entry, scope) === highestRank);
  const newestDate = highest.reduce(
    (latest, entry) => (entry.source.effectiveDate! > latest ? entry.source.effectiveDate! : latest),
    '',
  );
  const contenders = highest.filter((entry) => entry.source.effectiveDate === newestDate);
  const displaced = applicable.filter((entry) => !contenders.includes(entry));
  if (contenders.length !== 1) {
    return unsupportedResolution('equal-rank', contenders, displaced, provenance);
  }

  return Object.freeze({
    status: 'resolved',
    reason: null,
    winning: toSourceRef(contenders[0]!),
    contending: Object.freeze([]),
    superseded: Object.freeze(displaced.map(toSourceRef).sort(compareSourceRefs)),
    provenance: Object.freeze(provenance),
  });
}

function appendValidationDiagnostics(diagnostics: Diagnostic[], error: unknown, prefix: string): void {
  if (!(error instanceof AuthorityValidationError)) throw error;
  for (const diagnostic of error.diagnostics) {
    diagnostics.push({ ...diagnostic, path: prefix + diagnostic.path });
  }
}

function asGraphNode(
  identity: IdentityDocument<JsonValue>,
  contentHash: Hash,
  path: string,
): GraphNode {
  return {
    artifactKind: identity.artifactKind,
    stableId: identity.stableId,
    contentHash,
    identity,
    path,
  };
}

function validateGraph(bundle: AuthorityBundle): void {
  const diagnostics: Diagnostic[] = [];
  const sourceById = new Map(bundle.identity.payload.sources.map((source) => [source.sourceId, source]));
  const sourceHashes = new Set(bundle.identity.payload.sources.map((source) => source.byteHash));
  const sourceIndexByHash = new Map(
    bundle.identity.payload.sources.map((source, index) => [source.byteHash, index]),
  );
  const nodes = new Map<string, GraphNode>();
  const rootNode = asGraphNode(bundle.identity, bundle.contentHash, '/identity');
  nodes.set(rootNode.stableId, rootNode);

  bundle.identity.payload.artifacts.forEach((artifact, index) => {
    const path = `/identity/payload/artifacts/${index}`;
    try {
      if (artifact.identity.artifactKind === 'format') validateFormatArtifact(artifact);
      else validateCanonicalArtifact(artifact);
    } catch (error: unknown) {
      appendValidationDiagnostics(diagnostics, error, path);
    }
    if (artifact.identity.artifactKind === 'card-snapshot') {
      try {
        validateNormalizedCardSnapshot(artifact.identity.payload);
      } catch (error: unknown) {
        appendValidationDiagnostics(diagnostics, error, path + '/identity/payload');
      }
    }

    const node = asGraphNode(artifact.identity, artifact.contentHash, path + '/identity');
    if (nodes.has(node.stableId)) {
      diagnostics.push({
        path: path + '/identity/stableId',
        code: 'duplicate_stable_id',
        message: 'artifact stable ID duplicates another graph node',
      });
    } else {
      nodes.set(node.stableId, node);
    }
  });

  const referenceCount = [...nodes.values()].reduce(
    (count, node) => count + node.identity.parentRefs.length + node.identity.sourceRefs.length,
    0,
  ) + bundle.identity.payload.sources.reduce(
    (count, source) => count + source.derivation.parentByteHashes.length,
    0,
  );
  if (referenceCount > MAX_AUTHORITY_REFERENCES) {
    diagnostics.push({
      path: '/identity',
      code: 'max_references',
      message: 'authority graph exceeds the fixed reference limit',
    });
  }

  bundle.identity.payload.sources.forEach((source, sourceIndex) => {
    const path = `/identity/payload/sources/${sourceIndex}/derivation/parentByteHashes`;
    const parents = source.derivation.parentByteHashes;
    const foundParents = new Set<Hash>();
    parents.forEach((parentHash, parentIndex) => {
      const parentPath = `${path}/${parentIndex}`;
      if (foundParents.has(parentHash)) {
        diagnostics.push({
          path: parentPath,
          code: 'duplicate_derivation_parent',
          message: 'derivation parent byte hash duplicates an earlier edge',
        });
      }
      foundParents.add(parentHash);
      if (parentHash === source.byteHash) {
        diagnostics.push({
          path: parentPath,
          code: 'self_derivation',
          message: 'a source cannot derive from its own byte hash',
        });
      } else if (!sourceHashes.has(parentHash)) {
        diagnostics.push({
          path: parentPath,
          code: 'missing_derivation_parent',
          message: 'derivation parent hash does not resolve to another source record',
        });
      }
    });
  });

  const derivationState = new Map<Hash, 0 | 1 | 2>();
  function visitDerivation(sourceIndex: number, depth: number): void {
    const source = bundle.identity.payload.sources[sourceIndex]!;
    derivationState.set(source.byteHash, 1);
    source.derivation.parentByteHashes.forEach((parentHash, parentIndex) => {
      if (parentHash === source.byteHash) return;
      const parentSourceIndex = sourceIndexByHash.get(parentHash);
      if (parentSourceIndex === undefined) return;
      const path = `/identity/payload/sources/${sourceIndex}/derivation/parentByteHashes/${parentIndex}`;
      if (derivationState.get(parentHash) === 1) {
        diagnostics.push({
          path,
          code: 'derivation_cycle',
          message: 'source byte derivation contains a cycle',
        });
        return;
      }
      if (depth >= MAX_AUTHORITY_GRAPH_DEPTH) {
        diagnostics.push({
          path,
          code: 'max_graph_depth',
          message: 'authority graph exceeds the fixed depth limit',
        });
        return;
      }
      if (derivationState.get(parentHash) !== 2) visitDerivation(parentSourceIndex, depth + 1);
    });
    derivationState.set(source.byteHash, 2);
  }

  bundle.identity.payload.sources.forEach((source, index) => {
    if (derivationState.get(source.byteHash) === undefined) visitDerivation(index, 0);
  });

  const referencedSources = new Set<string>();
  for (const node of nodes.values()) {
    node.identity.parentRefs.forEach((reference, index) => {
      const path = `${node.path}/parentRefs/${index}`;
      const target = nodes.get(reference.stableId);
      if (target === undefined) {
        diagnostics.push({
          path: path + '/stableId',
          code: 'missing_parent_ref',
          message: 'parent reference does not resolve to an artifact',
        });
        return;
      }
      if (target.artifactKind !== reference.artifactKind) {
        diagnostics.push({
          path: path + '/artifactKind',
          code: 'parent_ref_kind_mismatch',
          message: 'parent reference artifact kind does not match its target',
        });
      }
      if (target.contentHash !== reference.contentHash) {
        diagnostics.push({
          path: path + '/contentHash',
          code: 'parent_ref_hash_mismatch',
          message: 'parent reference content hash does not match its target',
        });
      }
    });

    node.identity.sourceRefs.forEach((reference, index) => {
      const path = `${node.path}/sourceRefs/${index}`;
      const target = sourceById.get(reference.sourceId);
      if (target === undefined) {
        diagnostics.push({
          path: path + '/sourceId',
          code: 'missing_source_ref',
          message: 'source reference does not resolve to a source record',
        });
        return;
      }
      referencedSources.add(reference.sourceId);
      if (target.byteHash !== reference.byteHash) {
        diagnostics.push({
          path: path + '/byteHash',
          code: 'source_ref_hash_mismatch',
          message: 'source reference byte hash does not match its source record',
        });
      }
    });
  }

  bundle.identity.payload.sources.forEach((source, index) => {
    if (!referencedSources.has(source.sourceId)) {
      diagnostics.push({
        path: `/identity/payload/sources/${index}/sourceId`,
        code: 'unreferenced_source',
        message: 'source record is not bound by any SourceRef',
      });
    }
  });

  const state = new Map<string, 0 | 1 | 2>();
  function visit(node: GraphNode, depth: number): void {
    state.set(node.stableId, 1);
    node.identity.parentRefs.forEach((reference, index) => {
      const target = nodes.get(reference.stableId);
      if (target === undefined) return;
      const path = `${node.path}/parentRefs/${index}`;
      if (state.get(target.stableId) === 1) {
        diagnostics.push({
          path,
          code: 'reference_cycle',
          message: 'artifact parent references contain a cycle',
        });
        return;
      }
      if (depth >= MAX_AUTHORITY_GRAPH_DEPTH) {
        diagnostics.push({
          path,
          code: 'max_graph_depth',
          message: 'authority graph exceeds the fixed depth limit',
        });
        return;
      }
      if (state.get(target.stableId) !== 2) visit(target, depth + 1);
    });
    state.set(node.stableId, 2);
  }

  for (const node of nodes.values()) {
    if (state.get(node.stableId) === undefined) visit(node, 0);
  }

  if (diagnostics.length > 0) fail(diagnostics);
}

type ParsedPrecedence = Readonly<{
  records: readonly AuthorityPrecedenceRecord[];
  diagnostics: readonly Diagnostic[];
}>;

const AUTHORITY_KINDS = new Set<AuthorityKind>([
  'rulebook',
  'format',
  'codex',
  'faq',
  'card-update',
  'card-data',
]);

function asJsonRecord(value: JsonValue): Readonly<Record<string, JsonValue>> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? (value as Readonly<Record<string, JsonValue>>)
    : null;
}

function parseBundlePrecedence(bundle: AuthorityBundle): ParsedPrecedence {
  const diagnostics: Diagnostic[] = [];
  const records: AuthorityPrecedenceRecord[] = [];
  const sourceById = new Map(bundle.identity.payload.sources.map((source) => [source.sourceId, source]));

  bundle.identity.payload.artifacts.forEach((artifact, index) => {
    const payload = asJsonRecord(artifact.identity.payload);
    if (payload === null || !('precedence' in payload)) return;
    const path = `/identity/payload/artifacts/${index}/identity/payload/precedence`;
    const precedence = asJsonRecord(payload.precedence);
    const keys = precedence === null ? [] : Object.keys(precedence).sort();
    const validKeys = keys.join('|') === 'authorityKind|scope|supersedes|topic';
    const authorityKind = precedence?.authorityKind;
    const topic = precedence?.topic;
    const scope = precedence?.scope;
    const supersedes = precedence?.supersedes;
    const validSupersedes =
      Array.isArray(supersedes) &&
      supersedes.length <= 100 &&
      supersedes.every((sourceId) => typeof sourceId === 'string' && sourceId.startsWith('source:')) &&
      new Set(supersedes).size === supersedes.length;
    const sourceReference = artifact.identity.sourceRefs.length === 1 ? artifact.identity.sourceRefs[0] : undefined;
    const source = sourceReference === undefined ? undefined : sourceById.get(sourceReference.sourceId);

    if (
      !validKeys ||
      typeof authorityKind !== 'string' ||
      !AUTHORITY_KINDS.has(authorityKind as AuthorityKind) ||
      typeof topic !== 'string' ||
      topic.length === 0 ||
      topic.length > 200 ||
      !(scope === null || (typeof scope === 'string' && scope.length > 0 && scope.length <= 200)) ||
      !validSupersedes ||
      source === undefined
    ) {
      diagnostics.push({
        path,
        code: 'invalid_precedence_record',
        message: 'precedence records must be strict, bounded, and bound to exactly one source',
      });
      return;
    }

    records.push({
      source,
      authorityKind: authorityKind as AuthorityKind,
      topic,
      scope,
      supersedes: supersedes as string[],
    });
  });

  return { records, diagnostics };
}

function validateBundlePrecedence(bundle: AuthorityBundle): readonly ValidatedPrecedenceResolution[] {
  const parsed = parseBundlePrecedence(bundle);
  if (parsed.diagnostics.length > 0) fail(parsed.diagnostics);

  const byTopic = new Map<string, AuthorityPrecedenceRecord[]>();
  for (const entry of parsed.records) {
    const group = byTopic.get(entry.topic) ?? [];
    group.push(entry);
    byTopic.set(entry.topic, group);
  }

  const diagnostics: Diagnostic[] = [];
  const validated: ValidatedPrecedenceResolution[] = [];
  for (const [topic, entries] of [...byTopic.entries()].sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))) {
    const official = entries.filter((entry) => entry.source.authorityClass === 'official');
    if (official.length === 0) continue;
    const scopes = [...new Set(official.map((entry) => entry.scope))].sort((left, right) => {
      if (left === right) return 0;
      if (left === null) return -1;
      if (right === null) return 1;
      return left < right ? -1 : 1;
    });
    for (const scope of scopes) {
      const resolution = resolveAuthorityPrecedence(entries, scope, bundle.identity.payload.effectiveDate);
      validated.push({ topic, scope, resolution });
      if (resolution.status === 'unsupported') {
        diagnostics.push({
          path: '/identity/payload/artifacts',
          code: 'unsupported_precedence',
          message: `${topic} authority is unsupported: ${resolution.reason}`,
        });
      }
    }
  }

  if (diagnostics.length > 0) fail(diagnostics);
  return validated;
}

function validateStoragePolicy(bundle: AuthorityBundle): void {
  const diagnostics: Diagnostic[] = [];
  bundle.identity.payload.sources.forEach((source, index) => {
    if (source.storageMode === 'stored' && source.licenseStatus !== 'approved') {
      diagnostics.push({
        path: `/identity/payload/sources/${index}/licenseStatus`,
        code: 'storage_prohibited',
        message: 'stored source bytes require explicit approved permission metadata',
      });
    }
  });
  if (diagnostics.length > 0) fail(diagnostics);
}

async function validateImportedRevisionFiles(
  revisionRoot: string,
  bundleBytes: Uint8Array,
  bundle: AuthorityBundle,
): Promise<number> {
  if (bundle.identity.payload.inputRootHash === null) return bundleBytes.byteLength;

  const diagnostics: Diagnostic[] = [];
  const sourceManifests = bundle.identity.payload.artifacts.filter(
    (artifact) => artifact.identity.artifactKind === 'source-manifest',
  );
  const cardSnapshots = bundle.identity.payload.artifacts.filter(
    (artifact) => artifact.identity.artifactKind === 'card-snapshot',
  );
  const formats = bundle.identity.payload.artifacts
    .filter((artifact) => artifact.identity.artifactKind === 'format')
    .sort((left, right) => left.identity.stableId < right.identity.stableId ? -1 : left.identity.stableId > right.identity.stableId ? 1 : 0);
  if (sourceManifests.length !== 1 || cardSnapshots.length !== 1 || formats.length === 0) {
    fail([{
      path: '/identity/payload/artifacts',
      code: 'invalid_imported_artifacts',
      message: 'imported revisions require one source manifest, one card snapshot, and at least one format',
    }]);
  }
  const sourceManifest = sourceManifests[0]!;
  const cardSnapshot = cardSnapshots[0]!;
  if (
    canonicalJson(sourceManifest.identity.payload) !==
    canonicalJson({ sources: bundle.identity.payload.sources })
  ) {
    diagnostics.push({
      path: '/identity/payload/artifacts',
      code: 'source_manifest_mismatch',
      message: 'source manifest payload does not match bundle sources',
    });
  }
  try {
    validateNormalizedCardSnapshot(cardSnapshot.identity.payload);
  } catch (error: unknown) {
    appendValidationDiagnostics(diagnostics, error, '/identity/payload/artifacts');
  }
  formats.forEach((format, index) => {
    try {
      validateFormatArtifact(format);
    } catch (error: unknown) {
      appendValidationDiagnostics(diagnostics, error, `/identity/payload/artifacts/${index}`);
    }
  });
  if (diagnostics.length > 0) fail(diagnostics);

  const entries = await readdir(revisionRoot, { recursive: true, withFileTypes: true });
  if (entries.length > MAX_AUTHORITY_FILES) {
    fail([{
      path: '/files',
      code: 'max_files',
      message: 'authority revision exceeds the fixed file-entry limit',
    }]);
  }
  if (entries.some((entry) => !entry.isFile() && !entry.isDirectory())) {
    fail([{
      path: '/files',
      code: 'unexpected_file_type',
      message: 'authority revision contains a symlink or unsupported entry',
    }]);
  }
  const actualFiles = entries
    .filter((entry) => entry.isFile())
    .map((entry) => relative(revisionRoot, resolve(entry.parentPath, entry.name)).replaceAll('\\', '/'))
    .sort();
  const expectedFiles = [
    'bundle.json',
    'cards.normalized.json',
    'formats.json',
    'sources.json',
    ...bundle.identity.payload.sources
      .filter((source) => source.storageMode === 'stored')
      .map((source) => source.relativePath),
  ].sort();
  if (
    actualFiles.length !== expectedFiles.length ||
    actualFiles.some((path, index) => path !== expectedFiles[index])
  ) {
    fail([{
      path: '/files',
      code: 'unexpected_file_set',
      message: 'authority revision files do not match the imported bundle manifest',
    }]);
  }

  const expectedCompanions = [
    ['sources.json', sourceManifest],
    ['formats.json', { formats }],
    ['cards.normalized.json', cardSnapshot],
  ] as const;
  let totalBytes = bundleBytes.byteLength;
  for (const [relativePath, expected] of expectedCompanions) {
    let read: BoundedReadResult;
    try {
      read = await readBoundedWithinAuthorityRoot(
        revisionRoot,
        relativePath,
        MAX_AUTHORITY_BYTES - totalBytes,
      );
    } catch (error: unknown) {
      appendValidationDiagnostics(diagnostics, error, `/files/${relativePath}`);
      continue;
    }
    if (read.status !== 'ok') {
      diagnostics.push({
        path: `/files/${relativePath}`,
        code: read.status === 'too-large' ? 'max_bytes' : 'path_unreadable',
        message: 'canonical companion file cannot be read within the fixed byte limit',
      });
      continue;
    }
    totalBytes += read.bytes.byteLength;
    if (!Buffer.from(read.bytes).equals(Buffer.from(canonicalJson(expected as JsonValue), 'utf8'))) {
      diagnostics.push({
        path: `/files/${relativePath}`,
        code: 'companion_content_mismatch',
        message: 'canonical companion file does not match the bundle artifact',
      });
    }
  }
  if (diagnostics.length > 0) fail(diagnostics);
  return totalBytes;
}

async function readStoredSources(
  root: string,
  initialBytes: number,
  bundle: AuthorityBundle,
): Promise<readonly SourceRef[]> {
  const stored = bundle.identity.payload.sources.filter((source) => source.storageMode === 'stored');
  if (stored.length + (bundle.identity.payload.inputRootHash === null ? 1 : 4) > MAX_AUTHORITY_FILES) {
    fail([
      {
        path: '/identity/payload/sources',
        code: 'max_files',
        message: 'authority bundle exceeds the fixed stored-file limit',
      },
    ]);
  }

  let totalBytes = initialBytes;
  const diagnostics: Diagnostic[] = [];
  const rehashed: SourceRef[] = [];
  for (const source of stored) {
    const index = bundle.identity.payload.sources.indexOf(source);
    const path = `/identity/payload/sources/${index}`;
    let read: BoundedReadResult;
    try {
      read = await readBoundedWithinAuthorityRoot(
        root,
        source.relativePath,
        MAX_AUTHORITY_BYTES - totalBytes,
      );
    } catch (error: unknown) {
      appendValidationDiagnostics(diagnostics, error, path + '/relativePath');
      continue;
    }
    if (read.status !== 'ok') {
      diagnostics.push({
        path: path + '/relativePath',
        code:
          read.status === 'not-file'
            ? 'path_not_file'
            : read.status === 'too-large'
              ? 'max_bytes'
              : 'path_unreadable',
        message:
          read.status === 'not-file'
            ? 'stored source path must resolve to a regular file'
            : read.status === 'too-large'
              ? 'authority bundle exceeds the fixed aggregate byte limit'
              : 'stored source cannot be read',
      });
      continue;
    }
    totalBytes += read.bytes.byteLength;

    if (sha256(read.bytes) !== source.byteHash) {
      diagnostics.push({
        path: path + '/byteHash',
        code: 'stored_byte_hash_mismatch',
        message: 'stored source bytes do not match byteHash',
      });
      continue;
    }
    rehashed.push({ sourceId: source.sourceId, byteHash: source.byteHash });
  }

  if (diagnostics.length > 0) fail(diagnostics);
  return rehashed;
}

function compareSourceRefs(left: SourceRef, right: SourceRef): number {
  return left.sourceId < right.sourceId ? -1 : left.sourceId > right.sourceId ? 1 : 0;
}

export async function validateAuthorityBundle(
  root: string,
  bundlePath: string,
  expected: ExpectedBundle,
): Promise<ValidatedAuthorityBundle> {
  const bundleRead = await readBoundedWithinAuthorityRoot(
    root,
    bundlePath,
    DEFAULT_AUTHORITY_JSON_LIMITS.maxBytes,
  );
  if (bundleRead.status === 'not-file') {
    throw pathError('path_not_file', 'bundle path must resolve to a regular file');
  }
  if (bundleRead.status === 'too-large') {
    fail([{ path: '', code: 'max_bytes', message: 'bundle exceeds the fixed byte limit' }]);
  }
  if (bundleRead.status === 'unreadable') {
    throw pathError('path_unreadable', 'bundle cannot be read');
  }

  const resolvedBundlePath = bundleRead.resolvedPath;
  const bundleBytes = bundleRead.bytes;
  const parsed = parseAuthorityJson(bundleBytes, DEFAULT_AUTHORITY_JSON_LIMITS);
  const bundle = validateBundleSchema(parsed);

  const expectedDiagnostics: Diagnostic[] = [];
  if (bundle.identity.stableId !== expected.stableId) {
    expectedDiagnostics.push({
      path: '/expected/stableId',
      code: 'unexpected_stable_id',
      message: 'bundle stable ID does not match the independently expected ID',
    });
  }
  if (bundle.contentHash !== expected.contentHash) {
    expectedDiagnostics.push({
      path: '/expected/contentHash',
      code: 'unexpected_content_hash',
      message: 'bundle content hash does not match the independently expected hash',
    });
  }
  if (expectedDiagnostics.length > 0) fail(expectedDiagnostics);

  validateGraph(bundle);
  validateStoragePolicy(bundle);
  const precedenceResolutions = validateBundlePrecedence(bundle);
  const revisionRoot = dirname(resolvedBundlePath);
  const validatedBytes = await validateImportedRevisionFiles(revisionRoot, bundleBytes, bundle);
  const storedBytesRehashed = [
    ...(await readStoredSources(revisionRoot, validatedBytes, bundle)),
  ].sort(compareSourceRefs);
  const manifestSources = bundle.identity.payload.sources.filter(
    (source) => source.storageMode === 'manifest-only',
  );
  const manifestByteBindingsVerified = manifestSources
    .filter((source) => source.durableLocator?.startsWith('urn:sha256:') === true)
    .map((source) => ({ sourceId: source.sourceId, byteHash: source.byteHash }))
    .sort(compareSourceRefs);
  const manifestDeclarationsBound = manifestSources
    .filter((source) => source.durableLocator?.startsWith('urn:sha256:') !== true)
    .map((source) => ({ sourceId: source.sourceId, byteHash: source.byteHash }))
    .sort(compareSourceRefs);

  return Object.freeze({
    bundle,
    resolvedBundlePath,
    storedBytesRehashed: Object.freeze(storedBytesRehashed),
    manifestByteBindingsVerified: Object.freeze(manifestByteBindingsVerified),
    manifestDeclarationsBound: Object.freeze(manifestDeclarationsBound),
    precedenceResolutions: Object.freeze(precedenceResolutions),
  });
}
