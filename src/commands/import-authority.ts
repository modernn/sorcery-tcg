import { lstat, mkdir, mkdtemp, readdir, realpath, rename, rm, writeFile } from 'node:fs/promises';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash, sha256 } from '../authority/hash.ts';
import { normalizeCards } from '../authority/normalize-cards.ts';
import {
  AuthorityValidationError,
  DEFAULT_AUTHORITY_JSON_LIMITS,
  createCanonicalArtifact,
  parseAuthorityJson,
  sortDiagnostics,
  validateFormatArtifact,
  validateSourceRecord,
  type CanonicalArtifact,
  type Diagnostic,
  type FormatArtifact,
  type Hash,
  type SourceRecord,
  type SourceRef,
} from '../authority/schemas.ts';
import {
  readBoundedWithinAuthorityRoot,
  validateAuthorityBundle,
} from '../authority/validate-bundle.ts';

const MAX_INPUT_FILES = 16;
const MAX_INPUT_BYTES = 32_000_000;
const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
const REVISION_PATTERN = /^[a-z0-9][a-z0-9._-]{0,99}$/;
const REQUIRED_INPUTS = ['cards.raw.json', 'formats.json', 'sources.json'] as const;
const OFFICIAL_CARD_API_URL = 'https://api.sorcerytcg.com/api/cards';

type LockedFile = Readonly<{
  byteHash: Hash;
  relativePath: string;
  storageMode: 'manifest-only' | 'stored';
}>;

type InputLock = Readonly<{
  schemaVersion: 1;
  files: readonly LockedFile[];
  inputRootHash: Hash;
}>;

export type ImportAuthorityArguments = Readonly<{
  inputRoot: string;
  inputLockPath: string;
  expectedInputRootHash: Hash;
  outputRoot: string;
  revisionId: string;
}>;

export type ImportAuthorityResult = Readonly<{
  revisionPath: string;
  bundleId: string;
  verifiedInputRootHash: Hash;
  bundleRootHash: Hash;
}>;

export type ImportAuthorityHooks = Readonly<{
  afterWrite?: (relativePath: string) => void | Promise<void>;
  onCandidate?: (result: Omit<ImportAuthorityResult, 'revisionPath' | 'bundleId'>) => void;
}>;

type CommandIo = Readonly<{
  stdout: (line: string) => void;
  stderr: (line: string) => void;
}>;

function fail(path: string, code: string, message: string): never {
  throw new AuthorityValidationError([{ path, code, message }]);
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function asRecord(value: JsonValue, path: string): Readonly<Record<string, JsonValue>> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    fail(path, 'invalid_type', 'expected an object');
  }
  return value as Readonly<Record<string, JsonValue>>;
}

function exactKeys(record: Readonly<Record<string, JsonValue>>, expected: readonly string[], path: string): void {
  const actual = Object.keys(record).sort(compareText);
  const wanted = [...expected].sort(compareText);
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail(path, 'unrecognized_key', 'object must contain only the documented keys');
  }
}

function prefixValidation(error: unknown, prefix: string): never {
  if (!(error instanceof AuthorityValidationError)) throw error;
  throw new AuthorityValidationError(
    error.diagnostics.map((diagnostic) => ({ ...diagnostic, path: prefix + diagnostic.path })),
  );
}

function parseInputLock(bytes: Uint8Array): InputLock {
  const document = asRecord(parseAuthorityJson(bytes, DEFAULT_AUTHORITY_JSON_LIMITS), '/inputLock');
  exactKeys(document, ['schemaVersion', 'files', 'inputRootHash'], '/inputLock');
  if (document.schemaVersion !== 1) fail('/inputLock/schemaVersion', 'invalid_literal', 'schemaVersion must be 1');
  if (!Array.isArray(document.files)) fail('/inputLock/files', 'invalid_type', 'files must be an array');
  if (typeof document.inputRootHash !== 'string' || !HASH_PATTERN.test(document.inputRootHash)) {
    fail('/inputLock/inputRootHash', 'invalid_format', 'inputRootHash must be a lowercase SHA-256 digest');
  }

  const files = document.files.map((value, index): LockedFile => {
    const path = `/inputLock/files/${index}`;
    const entry = asRecord(value, path);
    exactKeys(entry, ['byteHash', 'relativePath', 'storageMode'], path);
    if (typeof entry.relativePath !== 'string' || !REQUIRED_INPUTS.includes(entry.relativePath as never)) {
      fail(path + '/relativePath', 'invalid_input_path', 'input lock contains an unsupported path');
    }
    if (typeof entry.byteHash !== 'string' || !HASH_PATTERN.test(entry.byteHash)) {
      fail(path + '/byteHash', 'invalid_format', 'byteHash must be a lowercase SHA-256 digest');
    }
    if (entry.storageMode !== 'manifest-only' && entry.storageMode !== 'stored') {
      fail(path + '/storageMode', 'invalid_literal', 'storageMode must be manifest-only or stored');
    }
    return {
      byteHash: entry.byteHash as Hash,
      relativePath: entry.relativePath,
      storageMode: entry.storageMode,
    };
  });

  if (
    files.length !== REQUIRED_INPUTS.length ||
    files.some((entry, index) => entry.relativePath !== REQUIRED_INPUTS[index]) ||
    files[1]?.storageMode !== 'stored' ||
    files[2]?.storageMode !== 'stored'
  ) {
    fail('/inputLock/files', 'invalid_input_lock', 'input lock must contain the exact path-ordered command inputs');
  }
  if (new Set(files.map((entry) => entry.relativePath)).size !== files.length) {
    fail('/inputLock/files', 'duplicate_input_path', 'input lock paths must be unique');
  }

  const inputRootHash = identityHash(files);
  if (document.inputRootHash !== inputRootHash) {
    fail('/inputLock/inputRootHash', 'input_root_hash_mismatch', 'input lock root does not match its canonical files array');
  }
  return { schemaVersion: 1, files, inputRootHash };
}

async function readBounded(
  root: string,
  relativePath: string,
  maxBytes: number,
  diagnosticPath: string,
): Promise<Uint8Array> {
  let read: Awaited<ReturnType<typeof readBoundedWithinAuthorityRoot>>;
  try {
    read = await readBoundedWithinAuthorityRoot(root, relativePath, maxBytes);
  } catch (error: unknown) {
    prefixValidation(error, diagnosticPath);
  }
  if (read.status === 'not-file') fail(diagnosticPath, 'path_not_file', 'input path must be a regular file');
  if (read.status === 'too-large') fail(diagnosticPath, 'max_bytes', 'input exceeds the fixed byte limit');
  if (read.status === 'unreadable') fail(diagnosticPath, 'path_unreadable', 'input file cannot be read');
  return read.bytes;
}

async function exactInputFiles(root: string, inputLockPath: string, lock: InputLock): Promise<void> {
  const entries = await readdir(root, { recursive: true, withFileTypes: true });
  if (entries.length > MAX_INPUT_FILES) {
    fail('/inputLock/files', 'max_files', 'input tree exceeds the fixed entry limit');
  }
  if (entries.some((entry) => !entry.isFile() && !entry.isDirectory())) {
    fail('/inputLock/files', 'input_file_set_mismatch', 'input tree contains a symlink or unsupported entry');
  }
  const actual = entries
    .filter((entry) => entry.isFile())
    .map((entry) => relative(root, join(entry.parentPath, entry.name)).replaceAll('\\', '/'))
    .sort(compareText);
  const expected = [inputLockPath, ...lock.files.map((entry) => entry.relativePath)].sort(compareText);
  if (actual.length !== expected.length || actual.some((path, index) => path !== expected[index])) {
    fail('/inputLock/files', 'input_file_set_mismatch', 'input tree file set does not match the reviewed lock');
  }
}

async function readLockedInputs(root: string, lock: InputLock): Promise<ReadonlyMap<string, Uint8Array>> {
  const inputs = new Map<string, Uint8Array>();
  let totalBytes = 0;
  for (const [index, entry] of lock.files.entries()) {
    const bytes = await readBounded(
      root,
      entry.relativePath,
      Math.min(DEFAULT_AUTHORITY_JSON_LIMITS.maxBytes, MAX_INPUT_BYTES - totalBytes),
      `/inputLock/files/${index}/relativePath`,
    );
    totalBytes += bytes.byteLength;
    if (sha256(bytes) !== entry.byteHash) {
      fail(`/inputLock/files/${index}/byteHash`, 'input_byte_hash_mismatch', 'input bytes do not match the reviewed lock');
    }
    inputs.set(entry.relativePath, bytes);
  }
  return inputs;
}

function parseSources(bytes: Uint8Array): readonly SourceRecord[] {
  const document = asRecord(parseAuthorityJson(bytes, DEFAULT_AUTHORITY_JSON_LIMITS), '/sources');
  exactKeys(document, ['sources'], '/sources');
  if (!Array.isArray(document.sources)) fail('/sources', 'invalid_type', 'sources must be an array');
  const sources = document.sources.map((value, index) => {
    try {
      return validateSourceRecord(value);
    } catch (error: unknown) {
      prefixValidation(error, `/sources/${index}`);
    }
  });
  if (new Set(sources.map((source) => source.sourceId)).size !== sources.length) {
    fail('/sources', 'duplicate_source_id', 'source IDs must be unique');
  }
  return sources.sort((left, right) => compareText(left.sourceId, right.sourceId));
}

function sourceRef(source: SourceRecord): SourceRef {
  return { sourceId: source.sourceId, byteHash: source.byteHash };
}

function parseFormats(
  bytes: Uint8Array,
  sources: readonly SourceRecord[],
  expectedByteHash: Hash,
): readonly FormatArtifact[] {
  const document = asRecord(parseAuthorityJson(bytes, DEFAULT_AUTHORITY_JSON_LIMITS), '/formats');
  exactKeys(document, ['formats'], '/formats');
  if (!Array.isArray(document.formats)) fail('/formats', 'invalid_type', 'formats must be an array');
  const sourceById = new Map(sources.map((source) => [source.sourceId, source]));
  const formats = document.formats.map((value, index): FormatArtifact => {
    const path = `/formats/${index}`;
    const row = asRecord(value, path);
    exactKeys(row, ['stableId', 'sourceId', 'definition'], path);
    if (typeof row.stableId !== 'string') fail(path + '/stableId', 'invalid_type', 'stableId must be a string');
    if (typeof row.sourceId !== 'string') fail(path + '/sourceId', 'invalid_type', 'sourceId must be a string');
    const source = sourceById.get(row.sourceId);
    if (source === undefined || source.byteHash !== expectedByteHash) {
      fail(path + '/sourceId', 'format_source_mismatch', 'format must reference the exact locked format input');
    }
    try {
      return validateFormatArtifact(createCanonicalArtifact({
        artifactKind: 'format',
        stableId: row.stableId,
        schemaVersion: 1,
        parentRefs: [],
        sourceRefs: [sourceRef(source)],
        payload: row.definition!,
      }));
    } catch (error: unknown) {
      prefixValidation(error, path);
    }
  });
  if (new Set(formats.map((format) => format.identity.stableId)).size !== formats.length) {
    fail('/formats', 'duplicate_stable_id', 'format stable IDs must be unique');
  }
  return formats.sort((left, right) => compareText(left.identity.stableId, right.identity.stableId));
}

function effectiveDate(sources: readonly SourceRecord[]): string {
  return sources
    .map((source) => source.effectiveDate ?? source.retrievedAt.slice(0, 10))
    .sort(compareText)
    .at(-1)!;
}

async function exists(path: string): Promise<boolean> {
  try {
    await lstat(path);
    return true;
  } catch (error: unknown) {
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') return false;
    fail('/revisionId', 'path_unreadable', 'revision target cannot be inspected');
  }
}

async function writeCandidateFile(
  candidateRoot: string,
  relativePath: string,
  value: JsonValue | Uint8Array,
  hooks: ImportAuthorityHooks,
): Promise<void> {
  const path = join(candidateRoot, ...relativePath.split('/'));
  await writeFile(path, value instanceof Uint8Array ? value : canonicalJson(value), { flag: 'wx' });
  await hooks.afterWrite?.(relativePath);
}

export async function importAuthority(
  args: ImportAuthorityArguments,
  hooks: ImportAuthorityHooks = {},
): Promise<ImportAuthorityResult> {
  if (!REVISION_PATTERN.test(args.revisionId)) {
    fail('/revisionId', 'invalid_revision_id', 'revisionId must be a confined lowercase path segment');
  }
  if (!HASH_PATTERN.test(args.expectedInputRootHash)) {
    fail('/expectedInputRootHash', 'invalid_format', 'expected input root must be a lowercase SHA-256 digest');
  }

  let inputRoot: string;
  try {
    inputRoot = await realpath(args.inputRoot);
  } catch {
    fail('/input', 'path_unreadable', 'input root cannot be resolved');
  }
  const lock = parseInputLock(
    await readBounded(
      inputRoot,
      args.inputLockPath,
      DEFAULT_AUTHORITY_JSON_LIMITS.maxBytes,
      '/inputLockPath',
    ),
  );
  if (lock.inputRootHash !== args.expectedInputRootHash) {
    fail('/expectedInputRootHash', 'unexpected_input_root_hash', 'input root does not match the independently expected hash');
  }
  await exactInputFiles(inputRoot, args.inputLockPath, lock);
  const inputs = await readLockedInputs(inputRoot, lock);
  const cardsBytes = inputs.get('cards.raw.json')!;
  const formatsBytes = inputs.get('formats.json')!;
  const sourcesBytes = inputs.get('sources.json')!;

  const sources = parseSources(sourcesBytes);
  const cardsLock = lock.files[0]!;
  const cardSources = sources.filter((source) => source.byteHash === cardsLock.byteHash);
  if (
    cardSources.length !== 1 ||
    cardSources[0]!.authorityClass !== 'official' ||
    cardSources[0]!.url !== OFFICIAL_CARD_API_URL ||
    cardSources[0]!.mediaType !== 'application/json'
  ) {
    fail('/sources', 'card_source_mismatch', 'locked card bytes must come from the audited official card API');
  }
  const cardSource = cardSources[0]!;
  if (cardSource.storageMode !== cardsLock.storageMode) {
    fail('/sources', 'storage_mode_mismatch', 'card source storage mode must match the reviewed input lock');
  }
  sources.forEach((source, index) => {
    if (source.storageMode !== 'stored') return;
    if (source !== cardSource || source.relativePath !== 'raw/cards.raw.json') {
      fail(`/sources/${index}/relativePath`, 'storage_prohibited', 'only the fixed approved raw card path may be stored');
    }
    if (source.licenseStatus !== 'approved') {
      fail(`/sources/${index}/licenseStatus`, 'storage_prohibited', 'stored source bytes require approved permission');
    }
  });

  const cards = normalizeCards(cardsBytes, cardSource);
  const formats = parseFormats(formatsBytes, sources, lock.files[1]!.byteHash);
  const sourceManifest = createCanonicalArtifact({
    artifactKind: 'source-manifest',
    stableId: `source-manifest:${args.revisionId}`,
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: sources.map(sourceRef),
    payload: { sources },
  });
  const artifacts: CanonicalArtifact<JsonValue>[] = [sourceManifest, cards, ...formats]
    .sort((left, right) => compareText(left.identity.stableId, right.identity.stableId));
  const bundle = createCanonicalArtifact({
    artifactKind: 'bundle',
    stableId: `bundle:${args.revisionId}`,
    schemaVersion: 1,
    parentRefs: artifacts.map((artifact) => ({
      artifactKind: artifact.identity.artifactKind,
      stableId: artifact.identity.stableId,
      contentHash: artifact.contentHash,
    })),
    sourceRefs: sources.map(sourceRef),
    payload: {
      effectiveDate: effectiveDate(sources),
      precedencePolicyVersion: 1,
      inputRootHash: lock.inputRootHash,
      sources,
      artifacts,
    },
  });

  await mkdir(args.outputRoot, { recursive: true });
  const outputRoot = await realpath(args.outputRoot);
  const revisionPath = join(outputRoot, args.revisionId);
  if (await exists(revisionPath)) fail('/revisionId', 'revision_exists', 'authority revision already exists');

  const candidate = await mkdtemp(join(outputRoot, `.${args.revisionId}.tmp-`));
  let published = false;
  try {
    await writeCandidateFile(candidate, 'sources.json', sourceManifest, hooks);
    await writeCandidateFile(candidate, 'formats.json', { formats }, hooks);
    await writeCandidateFile(candidate, 'cards.normalized.json', cards, hooks);
    await writeCandidateFile(candidate, 'bundle.json', bundle, hooks);
    if (cardSource.storageMode === 'stored') {
      await mkdir(join(candidate, 'raw'));
      await writeCandidateFile(candidate, 'raw/cards.raw.json', cardsBytes, hooks);
    }

    hooks.onCandidate?.({
      verifiedInputRootHash: lock.inputRootHash,
      bundleRootHash: bundle.contentHash,
    });
    await validateAuthorityBundle(
      outputRoot,
      `${basename(candidate)}/bundle.json`,
      { stableId: bundle.identity.stableId, contentHash: bundle.contentHash },
    );
    if (await exists(revisionPath)) fail('/revisionId', 'revision_exists', 'authority revision already exists');
    await rename(candidate, revisionPath);
    published = true;
  } catch (error: unknown) {
    if (error instanceof AuthorityValidationError) throw error;
    fail('/candidate', 'candidate_write_failed', 'candidate build or atomic publication failed');
  } finally {
    if (!published && dirname(candidate) === outputRoot && basename(candidate).startsWith(`.${args.revisionId}.tmp-`)) {
      await rm(candidate, { recursive: true, force: true });
    }
  }

  return Object.freeze({
    revisionPath,
    bundleId: bundle.identity.stableId,
    verifiedInputRootHash: lock.inputRootHash,
    bundleRootHash: bundle.contentHash,
  });
}

const IMPORT_OPTIONS = {
  input: { type: 'string' },
  'input-lock': { type: 'string' },
  'expected-input-root-hash': { type: 'string' },
  'output-root': { type: 'string' },
  'revision-id': { type: 'string' },
} as const;

function parseArguments(argv: readonly string[]): ImportAuthorityArguments {
  try {
    const { values, tokens } = parseArgs({
      args: [...argv],
      options: IMPORT_OPTIONS,
      strict: true,
      allowPositionals: false,
      tokens: true,
    });
    const parsed = {
      inputRoot: values.input ?? '',
      inputLockPath: values['input-lock'] ?? '',
      expectedInputRootHash: values['expected-input-root-hash'] ?? '',
      outputRoot: values['output-root'] ?? '',
      revisionId: values['revision-id'] ?? '',
    };
    if (tokens.length !== 5 || Object.values(parsed).some((value) => value.length === 0)) {
      fail('/arguments', 'invalid_arguments', 'provide each documented import argument exactly once');
    }
    return { ...parsed, expectedInputRootHash: parsed.expectedInputRootHash as Hash };
  } catch (error: unknown) {
    if (error instanceof AuthorityValidationError) throw error;
    return fail('/arguments', 'invalid_arguments', 'provide each documented import argument exactly once');
  }
}

const defaultIo: CommandIo = {
  stdout: (line) => process.stdout.write(line + '\n'),
  stderr: (line) => process.stderr.write(line + '\n'),
};

export async function runImportAuthorityCommand(
  argv: readonly string[],
  io: CommandIo = defaultIo,
): Promise<number> {
  try {
    const args = parseArguments(argv);
    await importAuthority(args, {
      onCandidate: (candidate) => io.stdout(canonicalJson({ status: 'candidate', ...candidate })),
    });
    return 0;
  } catch (error: unknown) {
    const diagnostics: readonly Diagnostic[] = error instanceof AuthorityValidationError
      ? error.diagnostics
      : [{ path: '', code: 'internal_error', message: 'authority import failed' }];
    for (const diagnostic of sortDiagnostics(diagnostics)) io.stderr(canonicalJson(diagnostic));
    return 1;
  }
}

if (process.argv[1] !== undefined && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  process.exitCode = await runImportAuthorityCommand(process.argv.slice(2));
}
