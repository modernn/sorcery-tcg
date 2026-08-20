import { createHash } from 'node:crypto';
import { lstat, open, readdir, realpath, stat, type FileHandle } from 'node:fs/promises';
import { isAbsolute, posix, relative, resolve, sep } from 'node:path';

import type { JsonValue } from './canonical-json.ts';
import { identityHash } from './hash.ts';

export const PRIVATE_AUTHORITY_SOURCE_PATHS = Object.freeze([
  'rulebook/rulebook-current.pdf',
  'formats/constructed-current.html',
  'codex/codex-current.html',
  'codex/faqs-current.html',
  'codex/changelog-current.html',
  'updates/card-updates-2025.html',
  'cards/cards.raw.json',
] as const);

export type PrivateAuthoritySourceEntry = Readonly<{
  relativePath: (typeof PRIVATE_AUTHORITY_SOURCE_PATHS)[number];
  url: string;
  retrievedAt: string;
  effectiveDate: string | null;
  mediaType: 'application/pdf' | 'text/html' | 'application/json';
  byteLength: number;
  byteHash: `sha256:${string}`;
}>;

export type PrivateSourceSetOptions = Readonly<{
  primaryRoot: string;
  backupRoot: string;
  repositoryRoot: string;
  entries: readonly PrivateAuthoritySourceEntry[];
}>;

export type PrivateSourceSetVerificationResult = Readonly<{
  entries: readonly PrivateAuthoritySourceEntry[];
  sourceSetRootHash: `sha256:${string}`;
}>;

export class PrivateSourceSetVerificationError extends Error {
  readonly code: string;
  readonly path: string;

  constructor(path: string, code: string, message: string) {
    super(`${code} at ${path || '<root>'}: ${message}`);
    this.name = 'PrivateSourceSetVerificationError';
    this.code = code;
    this.path = path;
  }
}

type RootLabel = 'primary' | 'backup';
type RootInfo = Readonly<{ realPath: string; identity: string }>;
type FileEvidence = Readonly<{
  byteLength: number;
  byteHash: `sha256:${string}`;
  identity: string;
  jsonBytes: Buffer | null;
}>;

const OFFICIAL_HOSTS = new Set([
  'sorcerytcg.com',
  'www.sorcerytcg.com',
  'api.sorcerytcg.com',
  'curiosa.io',
  'www.curiosa.io',
]);
const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
const UTC_TIMESTAMP_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/;
const ISO_DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/;
const WINDOWS_RESERVED_SEGMENT = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?$/i;
const MAX_CARD_BYTES = 10_000_000;
const MAX_OTHER_BYTES = 256 * 1024 * 1024;
const MAX_SOURCE_SET_BYTES = 768 * 1024 * 1024;
const ENTRY_FIELDS = Object.freeze([
  'relativePath',
  'url',
  'retrievedAt',
  'effectiveDate',
  'mediaType',
  'byteLength',
  'byteHash',
] as const);
const SOURCE_PATH_SET = new Set<string>(PRIVATE_AUTHORITY_SOURCE_PATHS);

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

const EXPECTED_DIRECTORIES = Object.freeze(
  [...new Set(PRIVATE_AUTHORITY_SOURCE_PATHS.flatMap((sourcePath) => {
    const segments = sourcePath.split('/');
    return segments.slice(0, -1).map((_, index) => segments.slice(0, index + 1).join('/'));
  }))].sort(compareText),
);

function fail(path: string, code: string, message: string): never {
  throw new PrivateSourceSetVerificationError(path, code, message);
}

function pathKey(value: string): string {
  return process.platform === 'win32' ? value.toLowerCase() : value;
}

function normalizedFilesystemPath(value: string): string {
  return pathKey(resolve(value));
}

function isWithin(parent: string, candidate: string): boolean {
  const difference = relative(normalizedFilesystemPath(parent), normalizedFilesystemPath(candidate));
  return difference === '' || (!isAbsolute(difference) && difference !== '..' && !difference.startsWith(`..${sep}`));
}

function identityOf(value: Readonly<{ dev: bigint; ino: bigint }>, errorPath: string): string {
  if (value.dev === 0n || value.ino === 0n) {
    return fail(errorPath, 'identity_unavailable', 'device and inode identity must be nonzero');
  }
  return `${value.dev}:${value.ino}`;
}

function errorCode(error: unknown): string | undefined {
  return typeof error === 'object' && error !== null && 'code' in error && typeof error.code === 'string'
    ? error.code
    : undefined;
}

async function filesystemCall<T>(errorPath: string, operation: () => Promise<T>): Promise<T> {
  try {
    return await operation();
  } catch (error: unknown) {
    if (error instanceof PrivateSourceSetVerificationError) throw error;
    const code = errorCode(error);
    if (code === 'ENOENT' || code === 'ENOTDIR') {
      return fail(errorPath, 'path_not_found', 'required filesystem path does not exist');
    }
    return fail(errorPath, 'filesystem_error', `filesystem operation failed${code === undefined ? '' : ` (${code})`}`);
  }
}

function isCanonicalRelativePath(value: unknown): value is string {
  if (typeof value !== 'string' || value === '' || value === '.' || value === '..') return false;
  if (isAbsolute(value) || /^[a-z]:\//i.test(value) || value.includes('\\')) return false;
  if (posix.normalize(value) !== value || value.startsWith('/') || value.endsWith('/')) return false;
  const segments = value.split('/');
  return segments.every(
    (segment) =>
      segment !== '' &&
      segment !== '.' &&
      segment !== '..' &&
      !WINDOWS_RESERVED_SEGMENT.test(segment) &&
      !/[. ]$/.test(segment),
  );
}

function isUtcTimestamp(value: string): boolean {
  if (!UTC_TIMESTAMP_PATTERN.test(value)) return false;
  const parsed = new Date(value);
  return !Number.isNaN(parsed.valueOf()) && parsed.toISOString().slice(0, 19) === value.slice(0, 19);
}

function isIsoDate(value: string): boolean {
  if (!ISO_DATE_PATTERN.test(value)) return false;
  const parsed = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(parsed.valueOf()) && parsed.toISOString().slice(0, 10) === value;
}

function expectedMediaType(relativePath: string): PrivateAuthoritySourceEntry['mediaType'] {
  if (relativePath.endsWith('.pdf')) return 'application/pdf';
  if (relativePath.endsWith('.json')) return 'application/json';
  return 'text/html';
}

function valueAt(input: object, field: (typeof ENTRY_FIELDS)[number], index: number): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(input, field);
  if (descriptor === undefined) return fail(`/entries/${index}/${field}`, 'missing_field', `${field} is required`);
  if (!descriptor.enumerable || !('value' in descriptor)) {
    return fail(`/entries/${index}/${field}`, 'invalid_entry', `${field} must be an enumerable data property`);
  }
  return descriptor.value;
}

function validateEntries(input: unknown): readonly PrivateAuthoritySourceEntry[] {
  if (!Array.isArray(input)) fail('/entries', 'invalid_entries', 'entries must be an array');
  const validated: PrivateAuthoritySourceEntry[] = [];
  const foundPaths = new Set<string>();
  let totalBytes = 0;

  for (let index = 0; index < input.length; index += 1) {
    const candidate: unknown = input[index];
    if (typeof candidate !== 'object' || candidate === null || Array.isArray(candidate)) {
      fail(`/entries/${index}`, 'invalid_entry', 'entry must be an ordinary object');
    }
    if (Object.getPrototypeOf(candidate) !== Object.prototype) {
      fail(`/entries/${index}`, 'invalid_entry', 'entry must use Object.prototype');
    }
    for (const key of Reflect.ownKeys(candidate)) {
      if (typeof key !== 'string' || !(ENTRY_FIELDS as readonly string[]).includes(key)) {
        fail(`/entries/${index}/${String(key)}`, 'unknown_field', 'entry contains an unknown field');
      }
    }

    const relativePath = valueAt(candidate, 'relativePath', index);
    if (!isCanonicalRelativePath(relativePath)) {
      fail(`/entries/${index}/relativePath`, 'invalid_relative_path', 'path must be canonical and slash-separated');
    }
    if (!SOURCE_PATH_SET.has(relativePath)) {
      fail(`/entries/${index}/relativePath`, 'unexpected_source_path', 'path is not in the seven-source allowlist');
    }
    if (foundPaths.has(relativePath)) {
      fail(`/entries/${index}/relativePath`, 'duplicate_source_path', 'source path appears more than once');
    }
    foundPaths.add(relativePath);

    const url = valueAt(candidate, 'url', index);
    if (typeof url !== 'string') fail(`/entries/${index}/url`, 'invalid_url', 'URL must be a string');
    let parsedUrl: URL;
    try {
      parsedUrl = new URL(url);
    } catch {
      fail(`/entries/${index}/url`, 'invalid_url', 'URL must be valid');
    }
    if (parsedUrl.protocol !== 'https:' || parsedUrl.username !== '' || parsedUrl.password !== '') {
      fail(`/entries/${index}/url`, 'invalid_url', 'URL must use HTTPS without credentials');
    }
    if (!OFFICIAL_HOSTS.has(parsedUrl.hostname)) {
      fail(`/entries/${index}/url`, 'unapproved_host', 'URL host is not an audited official host');
    }

    const retrievedAt = valueAt(candidate, 'retrievedAt', index);
    if (typeof retrievedAt !== 'string' || !isUtcTimestamp(retrievedAt)) {
      fail(`/entries/${index}/retrievedAt`, 'invalid_timestamp', 'retrievedAt must be a valid UTC ISO timestamp');
    }
    const effectiveDate = valueAt(candidate, 'effectiveDate', index);
    if (effectiveDate !== null && (typeof effectiveDate !== 'string' || !isIsoDate(effectiveDate))) {
      fail(`/entries/${index}/effectiveDate`, 'invalid_date', 'effectiveDate must be a valid ISO date or null');
    }
    const mediaType = valueAt(candidate, 'mediaType', index);
    if (mediaType !== expectedMediaType(relativePath)) {
      fail(`/entries/${index}/mediaType`, 'invalid_media_type', 'media type does not match the locked source path');
    }
    const byteLength = valueAt(candidate, 'byteLength', index);
    if (typeof byteLength !== 'number' || !Number.isSafeInteger(byteLength) || byteLength < 1) {
      fail(`/entries/${index}/byteLength`, 'invalid_byte_length', 'byteLength must be a positive safe integer');
    }
    const maximum = relativePath === 'cards/cards.raw.json' ? MAX_CARD_BYTES : MAX_OTHER_BYTES;
    if (byteLength > maximum) {
      fail(`/entries/${index}/byteLength`, 'max_file_bytes', 'declared source size exceeds the fixed limit');
    }
    totalBytes += byteLength;
    const byteHash = valueAt(candidate, 'byteHash', index);
    if (typeof byteHash !== 'string' || !HASH_PATTERN.test(byteHash)) {
      fail(`/entries/${index}/byteHash`, 'invalid_hash', 'byteHash must be a lowercase SHA-256 digest');
    }

    validated.push(Object.freeze({
      relativePath: relativePath as PrivateAuthoritySourceEntry['relativePath'],
      url,
      retrievedAt,
      effectiveDate,
      mediaType: mediaType as PrivateAuthoritySourceEntry['mediaType'],
      byteLength,
      byteHash: byteHash as PrivateAuthoritySourceEntry['byteHash'],
    }));
  }

  for (const requiredPath of PRIVATE_AUTHORITY_SOURCE_PATHS) {
    if (!foundPaths.has(requiredPath)) fail('/entries', 'missing_source_path', `missing metadata for ${requiredPath}`);
  }
  if (foundPaths.size !== PRIVATE_AUTHORITY_SOURCE_PATHS.length) {
    fail('/entries', 'invalid_source_count', 'entries must contain exactly seven source paths');
  }
  if (totalBytes > MAX_SOURCE_SET_BYTES) {
    fail('/entries', 'max_total_bytes', 'declared source set exceeds the fixed aggregate limit');
  }
  return Object.freeze(validated.sort((left, right) => compareText(left.relativePath, right.relativePath)));
}

async function inspectRoot(input: unknown, field: 'primaryRoot' | 'backupRoot' | 'repositoryRoot'): Promise<RootInfo> {
  const errorPath = `/${field}`;
  if (typeof input !== 'string' || input.trim() === '') fail(errorPath, 'invalid_root', 'root must be a nonempty path');
  const resolved = resolve(input);
  const linkStats = await filesystemCall(errorPath, () => lstat(resolved, { bigint: true }));
  if (linkStats.isSymbolicLink()) fail(errorPath, 'filesystem_alias', 'root may not be a symlink or junction');
  if (!linkStats.isDirectory()) fail(errorPath, 'not_directory', 'root must be an ordinary directory');
  const canonical = await filesystemCall(errorPath, () => realpath(resolved));
  if (normalizedFilesystemPath(resolved) !== normalizedFilesystemPath(canonical)) {
    fail(errorPath, 'filesystem_alias', 'resolved root must equal its realpath');
  }
  const targetStats = await filesystemCall(errorPath, () => stat(canonical, { bigint: true }));
  const linkIdentity = identityOf(linkStats, errorPath);
  const targetIdentity = identityOf(targetStats, errorPath);
  if (linkIdentity !== targetIdentity) fail(errorPath, 'filesystem_alias', 'root identity changed while resolving');
  return { realPath: canonical, identity: targetIdentity };
}

async function scanRoot(root: RootInfo, label: RootLabel): Promise<void> {
  const files = new Set<string>();
  const directories = new Set<string>();
  const expectedFileKeys = new Set(PRIVATE_AUTHORITY_SOURCE_PATHS.map(pathKey));
  const expectedDirectoryKeys = new Set(EXPECTED_DIRECTORIES.map(pathKey));

  async function visit(directory: string, relativeDirectory: string): Promise<void> {
    const entries = await filesystemCall(`/${label}${relativeDirectory === '' ? '' : `/${relativeDirectory}`}`, () =>
      readdir(directory, { withFileTypes: true }),
    );
    entries.sort((left, right) => compareText(left.name, right.name));
    for (const entry of entries) {
      const relativePath = relativeDirectory === '' ? entry.name : `${relativeDirectory}/${entry.name}`;
      const errorPath = `/${label}/${relativePath}`;
      const absolutePath = resolve(directory, entry.name);
      const linkStats = await filesystemCall(errorPath, () => lstat(absolutePath, { bigint: true }));
      if (entry.isSymbolicLink() || linkStats.isSymbolicLink()) {
        fail(errorPath, 'filesystem_alias', 'source tree may not contain symlinks or junctions');
      }
      if (!linkStats.isDirectory() && !linkStats.isFile()) {
        fail(errorPath, 'non_ordinary_file', 'source tree entries must be ordinary files or directories');
      }
      const canonical = await filesystemCall(errorPath, () => realpath(absolutePath));
      if (!isWithin(root.realPath, canonical)) fail(errorPath, 'path_escape', 'source path escaped its root');
      if (normalizedFilesystemPath(absolutePath) !== normalizedFilesystemPath(canonical)) {
        fail(errorPath, 'filesystem_alias', 'source path must equal its realpath');
      }
      const targetStats = await filesystemCall(errorPath, () => stat(canonical, { bigint: true }));
      const linkIdentity = identityOf(linkStats, errorPath);
      const targetIdentity = identityOf(targetStats, errorPath);
      if (linkIdentity !== targetIdentity) {
        fail(errorPath, 'filesystem_alias', 'source path identity changed while resolving');
      }

      if (linkStats.isDirectory()) {
        if (expectedFileKeys.has(pathKey(relativePath))) {
          fail(errorPath, 'non_ordinary_file', 'locked source path must be an ordinary file');
        }
        directories.add(relativePath);
        await visit(absolutePath, relativePath);
      } else {
        if (expectedDirectoryKeys.has(pathKey(relativePath))) {
          fail(errorPath, 'non_ordinary_file', 'locked directory path must be a directory');
        }
        files.add(relativePath);
      }
    }
  }

  await visit(root.realPath, '');
  const fileKeys = new Set([...files].map(pathKey));
  for (const requiredPath of [...PRIVATE_AUTHORITY_SOURCE_PATHS].sort(compareText)) {
    if (!fileKeys.has(pathKey(requiredPath))) fail(`/${label}/${requiredPath}`, 'missing_file', 'required source file is missing');
  }
  const directoryKeys = new Set([...directories].map(pathKey));
  for (const requiredPath of EXPECTED_DIRECTORIES) {
    if (!directoryKeys.has(pathKey(requiredPath))) fail(`/${label}/${requiredPath}`, 'missing_directory', 'required directory is missing');
  }
  const unexpected = [...files, ...directories]
    .filter((candidate) => !expectedFileKeys.has(pathKey(candidate)) && !expectedDirectoryKeys.has(pathKey(candidate)))
    .sort(compareText)[0];
  if (unexpected !== undefined) fail(`/${label}/${unexpected}`, 'unexpected_path', 'source tree contains an unexpected path');
}

function sameIdentity(
  left: Readonly<{ dev: bigint; ino: bigint }>,
  right: Readonly<{ dev: bigint; ino: bigint }>,
): boolean {
  return left.dev === right.dev && left.ino === right.ino;
}

async function readFileEvidence(
  root: RootInfo,
  label: RootLabel,
  entry: PrivateAuthoritySourceEntry,
): Promise<FileEvidence> {
  const errorPath = `/${label}/${entry.relativePath}`;
  const absolutePath = resolve(root.realPath, ...entry.relativePath.split('/'));
  if (!isWithin(root.realPath, absolutePath)) fail(errorPath, 'path_escape', 'source path escaped its root');
  const linkStats = await filesystemCall(errorPath, () => lstat(absolutePath, { bigint: true }));
  if (linkStats.isSymbolicLink()) fail(errorPath, 'filesystem_alias', 'source file may not be a symlink or junction');
  if (!linkStats.isFile()) fail(errorPath, 'non_ordinary_file', 'source path must be an ordinary file');
  const canonical = await filesystemCall(errorPath, () => realpath(absolutePath));
  if (!isWithin(root.realPath, canonical)) fail(errorPath, 'path_escape', 'source file escaped its root');
  if (normalizedFilesystemPath(absolutePath) !== normalizedFilesystemPath(canonical)) {
    fail(errorPath, 'filesystem_alias', 'source file must equal its realpath');
  }
  const pathStats = await filesystemCall(errorPath, () => stat(absolutePath, { bigint: true }));
  const identity = identityOf(pathStats, errorPath);
  if (!sameIdentity(linkStats, pathStats)) fail(errorPath, 'filesystem_alias', 'source file identity changed while resolving');

  let handle: FileHandle | undefined;
  try {
    handle = await filesystemCall(errorPath, () => open(absolutePath, 'r'));
    const openedStats = await filesystemCall(errorPath, () => handle!.stat({ bigint: true }));
    identityOf(openedStats, errorPath);
    if (!openedStats.isFile() || !sameIdentity(pathStats, openedStats)) {
      fail(errorPath, 'filesystem_alias', 'opened source identity differs from the verified path');
    }
    const maximum = entry.relativePath === 'cards/cards.raw.json' ? MAX_CARD_BYTES : MAX_OTHER_BYTES;
    if (openedStats.size > BigInt(maximum)) fail(errorPath, 'max_file_bytes', 'source file exceeds the fixed limit');

    const hash = createHash('sha256');
    const jsonChunks: Buffer[] | null = entry.relativePath === 'cards/cards.raw.json' ? [] : null;
    let byteLength = 0;
    try {
      for await (const chunk of handle.createReadStream({ autoClose: false })) {
        const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk as Uint8Array);
        byteLength += bytes.byteLength;
        if (byteLength > maximum) fail(errorPath, 'max_file_bytes', 'source file exceeds the fixed limit');
        hash.update(bytes);
        jsonChunks?.push(bytes);
      }
    } catch (error: unknown) {
      if (error instanceof PrivateSourceSetVerificationError) throw error;
      fail(errorPath, 'filesystem_error', 'source file could not be read completely');
    }

    const afterOpenStats = await filesystemCall(errorPath, () => handle!.stat({ bigint: true }));
    const afterPathStats = await filesystemCall(errorPath, () => stat(absolutePath, { bigint: true }));
    if (
      !sameIdentity(openedStats, afterOpenStats) ||
      !sameIdentity(openedStats, afterPathStats) ||
      openedStats.size !== afterOpenStats.size ||
      openedStats.size !== BigInt(byteLength)
    ) {
      fail(errorPath, 'file_changed', 'source file changed during verification');
    }
    const byteHash = `sha256:${hash.digest('hex')}` as const;
    if (byteLength !== entry.byteLength) {
      fail(`${errorPath}/byteLength`, 'byte_length_mismatch', 'source byte length does not match metadata');
    }
    if (byteHash !== entry.byteHash) {
      fail(`${errorPath}/byteHash`, 'byte_hash_mismatch', 'source byte hash does not match metadata');
    }
    return {
      byteLength,
      byteHash,
      identity,
      jsonBytes: jsonChunks === null ? null : Buffer.concat(jsonChunks, byteLength),
    };
  } finally {
    await handle?.close();
  }
}

function validateCardJson(bytes: Buffer, errorPath: string): void {
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return fail(errorPath, 'invalid_json', 'card JSON must be valid UTF-8');
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text) as unknown;
  } catch {
    return fail(errorPath, 'invalid_json', 'card source must contain valid JSON');
  }
  if (typeof parsed !== 'object' || parsed === null) {
    fail(errorPath, 'invalid_json', 'card JSON must have an object or array root');
  }
}

export async function verifyPrivateSourceSet(
  options: PrivateSourceSetOptions,
): Promise<PrivateSourceSetVerificationResult> {
  const entries = validateEntries(options.entries);
  const primary = await inspectRoot(options.primaryRoot, 'primaryRoot');
  const backup = await inspectRoot(options.backupRoot, 'backupRoot');
  const repository = await inspectRoot(options.repositoryRoot, 'repositoryRoot');

  if (isWithin(primary.realPath, backup.realPath) || isWithin(backup.realPath, primary.realPath)) {
    fail('/backupRoot', 'root_overlap', 'primary and backup roots must not be equal or nested');
  }
  if (primary.identity === backup.identity) {
    fail('/backupRoot', 'root_overlap', 'primary and backup roots must have distinct filesystem identities');
  }
  if (isWithin(repository.realPath, backup.realPath) || isWithin(backup.realPath, repository.realPath)) {
    fail('/backupRoot', 'backup_inside_repository', 'backup root must be independent of the repository');
  }

  await scanRoot(primary, 'primary');
  await scanRoot(backup, 'backup');

  const identities = new Map<string, string>();
  let primaryBytes = 0;
  let backupBytes = 0;
  for (const entry of entries) {
    const primaryEvidence = await readFileEvidence(primary, 'primary', entry);
    const primaryLinkedTo = identities.get(primaryEvidence.identity);
    if (primaryLinkedTo !== undefined) {
      fail(`/primary/${entry.relativePath}`, 'linked_file', `source file shares device/inode identity with ${primaryLinkedTo}`);
    }
    identities.set(primaryEvidence.identity, `/primary/${entry.relativePath}`);
    const backupEvidence = await readFileEvidence(backup, 'backup', entry);
    const linkedTo = identities.get(backupEvidence.identity);
    if (linkedTo !== undefined) {
      fail(`/backup/${entry.relativePath}`, 'linked_file', `source file shares device/inode identity with ${linkedTo}`);
    }
    identities.set(backupEvidence.identity, `/backup/${entry.relativePath}`);
    if (
      primaryEvidence.byteLength !== backupEvidence.byteLength ||
      primaryEvidence.byteHash !== backupEvidence.byteHash
    ) {
      fail(`/backup/${entry.relativePath}`, 'backup_mismatch', 'backup bytes do not match the primary source');
    }
    primaryBytes += primaryEvidence.byteLength;
    backupBytes += backupEvidence.byteLength;
    if (primaryBytes > MAX_SOURCE_SET_BYTES) fail('/primary', 'max_total_bytes', 'primary source set exceeds the aggregate limit');
    if (backupBytes > MAX_SOURCE_SET_BYTES) fail('/backup', 'max_total_bytes', 'backup source set exceeds the aggregate limit');
    if (entry.relativePath === 'cards/cards.raw.json') {
      validateCardJson(primaryEvidence.jsonBytes!, `/primary/${entry.relativePath}`);
      validateCardJson(backupEvidence.jsonBytes!, `/backup/${entry.relativePath}`);
    }
  }

  const rootDocument = { schemaVersion: 1, entries } as unknown as JsonValue;
  return Object.freeze({
    entries,
    sourceSetRootHash: identityHash(rootDocument),
  });
}
