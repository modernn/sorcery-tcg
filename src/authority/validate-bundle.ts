import { readFile, realpath, stat } from 'node:fs/promises';
import { isAbsolute, relative, resolve, sep, win32 } from 'node:path';

import type { JsonValue } from './canonical-json.ts';
import { sha256 } from './hash.ts';
import {
  AuthorityValidationError,
  DEFAULT_AUTHORITY_JSON_LIMITS,
  parseAuthorityJson,
  sortDiagnostics,
  validateAuthorityBundle as validateBundleSchema,
  validateCanonicalArtifact,
  type ArtifactKind,
  type AuthorityBundle,
  type Diagnostic,
  type Hash,
  type IdentityDocument,
  type SourceRef,
} from './schemas.ts';

const MAX_AUTHORITY_BYTES = 32_000_000;
const MAX_AUTHORITY_FILES = 512;
const MAX_AUTHORITY_GRAPH_DEPTH = 64;
const MAX_AUTHORITY_REFERENCES = 4_096;
const MAX_AUTHORITY_DIAGNOSTICS = DEFAULT_AUTHORITY_JSON_LIMITS.maxDiagnostics;

type ExpectedBundle = Readonly<{ stableId: string; contentHash: string }>;

export type ValidatedAuthorityBundle = Readonly<{
  bundle: AuthorityBundle;
  resolvedBundlePath: string;
  storedBytesRehashed: readonly SourceRef[];
  manifestBindingsVerified: readonly SourceRef[];
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

function isWithin(root: string, candidate: string): boolean {
  const fromRoot = relative(root, candidate);
  return fromRoot === '' || (fromRoot !== '..' && !fromRoot.startsWith('..' + sep) && !isAbsolute(fromRoot));
}

export async function resolveWithinAuthorityRoot(root: string, candidate: string): Promise<string> {
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
  const nodes = new Map<string, GraphNode>();
  const rootNode = asGraphNode(bundle.identity, bundle.contentHash, '/identity');
  nodes.set(rootNode.stableId, rootNode);

  bundle.identity.payload.artifacts.forEach((artifact, index) => {
    const path = `/identity/payload/artifacts/${index}`;
    try {
      validateCanonicalArtifact(artifact);
    } catch (error: unknown) {
      appendValidationDiagnostics(diagnostics, error, path);
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
  );
  if (referenceCount > MAX_AUTHORITY_REFERENCES) {
    diagnostics.push({
      path: '/identity',
      code: 'max_references',
      message: 'authority graph exceeds the fixed reference limit',
    });
  }

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

async function readStoredSources(
  root: string,
  bundleBytes: Uint8Array,
  bundle: AuthorityBundle,
): Promise<readonly SourceRef[]> {
  const stored = bundle.identity.payload.sources.filter((source) => source.storageMode === 'stored');
  if (stored.length + 1 > MAX_AUTHORITY_FILES) {
    fail([
      {
        path: '/identity/payload/sources',
        code: 'max_files',
        message: 'authority bundle exceeds the fixed stored-file limit',
      },
    ]);
  }

  let totalBytes = bundleBytes.byteLength;
  const diagnostics: Diagnostic[] = [];
  const rehashed: SourceRef[] = [];
  for (const source of stored) {
    const index = bundle.identity.payload.sources.indexOf(source);
    const path = `/identity/payload/sources/${index}`;
    let resolvedPath: string;
    try {
      resolvedPath = await resolveWithinAuthorityRoot(root, source.relativePath);
    } catch (error: unknown) {
      appendValidationDiagnostics(diagnostics, error, path + '/relativePath');
      continue;
    }

    let fileSize: number;
    try {
      const metadata = await stat(resolvedPath);
      if (!metadata.isFile()) {
        diagnostics.push({
          path: path + '/relativePath',
          code: 'path_not_file',
          message: 'stored source path must resolve to a regular file',
        });
        continue;
      }
      fileSize = metadata.size;
    } catch {
      diagnostics.push({
        path: path + '/relativePath',
        code: 'path_unreadable',
        message: 'stored source cannot be inspected',
      });
      continue;
    }

    if (totalBytes + fileSize > MAX_AUTHORITY_BYTES) {
      diagnostics.push({
        path: path + '/relativePath',
        code: 'max_bytes',
        message: 'authority bundle exceeds the fixed aggregate byte limit',
      });
      continue;
    }

    let bytes: Uint8Array;
    try {
      bytes = await readFile(resolvedPath);
    } catch {
      diagnostics.push({
        path: path + '/relativePath',
        code: 'path_unreadable',
        message: 'stored source cannot be read',
      });
      continue;
    }
    totalBytes += bytes.byteLength;
    if (totalBytes > MAX_AUTHORITY_BYTES) {
      diagnostics.push({
        path: path + '/relativePath',
        code: 'max_bytes',
        message: 'authority bundle exceeds the fixed aggregate byte limit',
      });
      continue;
    }

    if (sha256(bytes) !== source.byteHash) {
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
  const resolvedBundlePath = await resolveWithinAuthorityRoot(root, bundlePath);
  const metadata = await stat(resolvedBundlePath);
  if (!metadata.isFile()) throw pathError('path_not_file', 'bundle path must resolve to a regular file');
  if (metadata.size > DEFAULT_AUTHORITY_JSON_LIMITS.maxBytes) {
    fail([{ path: '', code: 'max_bytes', message: 'bundle exceeds the fixed byte limit' }]);
  }

  const bundleBytes = await readFile(resolvedBundlePath);
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
  const storedBytesRehashed = [...(await readStoredSources(root, bundleBytes, bundle))].sort(compareSourceRefs);
  const manifestBindingsVerified = bundle.identity.payload.sources
    .filter((source) => source.storageMode === 'manifest-only')
    .map((source) => ({ sourceId: source.sourceId, byteHash: source.byteHash }))
    .sort(compareSourceRefs);

  return Object.freeze({
    bundle,
    resolvedBundlePath,
    storedBytesRehashed: Object.freeze(storedBytesRehashed),
    manifestBindingsVerified: Object.freeze(manifestBindingsVerified),
  });
}