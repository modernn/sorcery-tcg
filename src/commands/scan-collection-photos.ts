import { spawn } from 'node:child_process';
import { readFile, readdir, realpath, mkdir, stat, writeFile } from 'node:fs/promises';
import { dirname, extname, isAbsolute, join, relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { sha256 } from '../authority/hash.ts';
import {
  validateCanonicalArtifact,
  validateNormalizedCardSnapshot,
  type NormalizedCardSnapshot,
} from '../authority/schemas.ts';
import {
  buildScanReport,
  createScanBinding,
  detectorOutputSchema,
  reviewManifestSchema,
  savedCollectionSchema,
  validateReviewManifest,
  validateSavedCollection,
  validateScanCatalog,
  type CollectionMode,
  type DetectorOutput,
  type ReviewManifest,
  type SavedCollection,
} from '../collection/photo-collection.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const COLLECTION_ROOT = join(REPOSITORY_ROOT, '.local', 'collection');
const AUTHORITY_ROOT = join(REPOSITORY_ROOT, '.local', 'authority');
const DETECTOR_PATH = join(REPOSITORY_ROOT, 'scripts', 'scan-card-photos.py');
const IMAGE_EXTENSIONS = new Set(['.bmp', '.jpeg', '.jpg', '.png', '.ppm', '.tif', '.tiff', '.webp']);
const MAX_JSON_BYTES = 10_000_000;
const MAX_IMAGE_BYTES = 100_000_000;
const MAX_DETECTOR_OUTPUT_BYTES = 16_000_000;

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

type CommandIo = Readonly<{
  stdout: (line: string) => void;
  stderr: (line: string) => void;
}>;

type Grid = Readonly<{ columns: number; rows: number }>;

type ScanArguments = Readonly<{
  authority: string;
  catalog: string;
  photos: string;
  saved: string;
  review: string | null;
  report: string;
  collectionOut: string;
  mode: CollectionMode;
  grid: Grid | null;
  confidenceThreshold: number;
  marginThreshold: number;
  minimumCardAreaRatio: number;
  maximumCardAreaRatio: number;
  python: string;
  timeoutMs: number;
}>;

class CollectionScanError extends Error {
  readonly code: string;
  readonly path: string;

  constructor(path: string, code: string, message: string) {
    super(message);
    this.name = 'CollectionScanError';
    this.path = path;
    this.code = code;
  }
}

function fail(path: string, code: string, message: string): never {
  throw new CollectionScanError(path, code, message);
}

function numeric(value: string, path: string, minimum: number, maximum: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < minimum || parsed > maximum) {
    fail(path, 'invalid_number', `value must be between ${minimum} and ${maximum}`);
  }
  return parsed;
}

function parseGrid(value: string): Grid {
  const match = /^(\d{1,2})x(\d{1,2})$/i.exec(value);
  if (!match) fail('/arguments/grid', 'invalid_grid', 'grid must use COLUMNSxROWS');
  const columns = Number(match[1]);
  const rows = Number(match[2]);
  if (columns < 1 || columns > 20 || rows < 1 || rows > 20) {
    fail('/arguments/grid', 'invalid_grid', 'grid dimensions must be between 1 and 20');
  }
  return { columns, rows };
}

const OPTIONS = {
  authority: { type: 'string' },
  catalog: { type: 'string' },
  photos: { type: 'string' },
  saved: { type: 'string' },
  review: { type: 'string' },
  report: { type: 'string' },
  'collection-out': { type: 'string' },
  mode: { type: 'string', default: 'add' },
  grid: { type: 'string' },
  threshold: { type: 'string', default: '0.12' },
  margin: { type: 'string', default: '0.03' },
  'min-area': { type: 'string', default: '0.01' },
  'max-area': { type: 'string', default: '0.70' },
  python: { type: 'string', default: 'python' },
  'timeout-ms': { type: 'string', default: '60000' },
} as const;

function parseArguments(argv: readonly string[]): ScanArguments {
  try {
    const { values, tokens } = parseArgs({
      args: [...argv],
      options: OPTIONS,
      strict: true,
      allowPositionals: false,
      tokens: true,
    });
    const seen = new Set<string>();
    for (const token of tokens) {
      if (token.kind !== 'option') continue;
      if (seen.has(token.name)) fail('/arguments', 'duplicate_argument', 'provide each option at most once');
      seen.add(token.name);
    }
    for (const required of ['authority', 'catalog', 'photos', 'saved', 'report', 'collection-out'] as const) {
      if (!values[required]) fail(`/arguments/${required}`, 'missing_argument', `--${required} is required`);
    }
    if (values.mode !== 'add' && values.mode !== 'replace') {
      fail('/arguments/mode', 'invalid_mode', 'mode must be add or replace');
    }
    const confidenceThreshold = numeric(values.threshold, '/arguments/threshold', 0, 1);
    const marginThreshold = numeric(values.margin, '/arguments/margin', 0, 1);
    const minimumCardAreaRatio = numeric(values['min-area'], '/arguments/min-area', 0.0001, 0.5);
    const maximumCardAreaRatio = numeric(values['max-area'], '/arguments/max-area', 0.01, 1);
    if (minimumCardAreaRatio >= maximumCardAreaRatio) {
      fail('/arguments', 'invalid_area_range', 'minimum card area must be less than maximum card area');
    }
    return {
      authority: values.authority!,
      catalog: values.catalog!,
      photos: values.photos!,
      saved: values.saved!,
      review: values.review ?? null,
      report: values.report!,
      collectionOut: values['collection-out']!,
      mode: values.mode,
      grid: values.grid ? parseGrid(values.grid) : null,
      confidenceThreshold,
      marginThreshold,
      minimumCardAreaRatio,
      maximumCardAreaRatio,
      python: values.python,
      timeoutMs: Math.floor(numeric(values['timeout-ms'], '/arguments/timeout-ms', 1_000, 300_000)),
    };
  } catch (error: unknown) {
    if (error instanceof CollectionScanError) throw error;
    return fail('/arguments', 'invalid_arguments', 'invalid collection scan arguments');
  }
}

function isWithin(root: string, candidate: string): boolean {
  const path = relative(root, candidate);
  return path === '' || (!path.startsWith('..') && !isAbsolute(path));
}

async function existingPrivatePath(root: string, candidate: string, path: string): Promise<string> {
  let resolvedRoot: string;
  let resolvedCandidate: string;
  try {
    [resolvedRoot, resolvedCandidate] = await Promise.all([realpath(root), realpath(resolve(candidate))]);
  } catch {
    return fail(path, 'path_unreadable', 'private-local input cannot be resolved');
  }
  if (!isWithin(resolvedRoot, resolvedCandidate)) {
    fail(path, 'path_outside_private_root', 'input must remain inside its private-local root');
  }
  return resolvedCandidate;
}

async function privateOutputPath(candidate: string, path: string): Promise<string> {
  await mkdir(COLLECTION_ROOT, { recursive: true });
  const resolvedRoot = await realpath(COLLECTION_ROOT);
  const target = resolve(candidate);
  if (!isWithin(resolve(COLLECTION_ROOT), target)) {
    fail(path, 'path_outside_private_root', 'output must remain inside the private-local collection root');
  }
  let parent: string;
  try {
    parent = await realpath(dirname(target));
  } catch {
    return fail(path, 'path_unreadable', 'output parent directory must already exist');
  }
  if (!isWithin(resolvedRoot, parent)) {
    fail(path, 'path_outside_private_root', 'output parent must remain inside the private-local collection root');
  }
  try {
    await stat(target);
    return fail(path, 'output_exists', 'output already exists; choose a new path');
  } catch (error: unknown) {
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') return target;
    if (error instanceof CollectionScanError) throw error;
    return fail(path, 'path_unreadable', 'output path cannot be inspected');
  }
}

async function boundedRead(path: string, maximumBytes: number, diagnosticPath: string): Promise<Uint8Array> {
  let metadata: Awaited<ReturnType<typeof stat>>;
  try {
    metadata = await stat(path);
  } catch {
    return fail(diagnosticPath, 'path_unreadable', 'input cannot be inspected');
  }
  if (!metadata.isFile()) fail(diagnosticPath, 'path_not_file', 'input must be a regular file');
  if (metadata.size > maximumBytes) fail(diagnosticPath, 'max_bytes', 'input exceeds its byte limit');
  try {
    return await readFile(path);
  } catch {
    return fail(diagnosticPath, 'path_unreadable', 'input cannot be read');
  }
}

async function privateJson(root: string, candidate: string, path: string): Promise<unknown> {
  const resolved = await existingPrivatePath(root, candidate, path);
  const bytes = await boundedRead(resolved, MAX_JSON_BYTES, path);
  return parseJsonWithDuplicateKeyCheck(Buffer.from(bytes).toString('utf8'));
}

async function loadAuthority(candidate: string): Promise<NormalizedCardSnapshot> {
  const input = await privateJson(AUTHORITY_ROOT, candidate, '/authority');
  const artifact = validateCanonicalArtifact(input);
  if (artifact.identity.artifactKind !== 'card-snapshot') {
    fail('/authority', 'invalid_artifact_kind', 'authority input must be a normalized card snapshot artifact');
  }
  return validateNormalizedCardSnapshot(artifact.identity.payload);
}

async function listPhotos(directory: string): Promise<readonly Readonly<{
  photoId: string;
  path: string;
  byteHash: `sha256:${string}`;
}>[]> {
  const root = await existingPrivatePath(COLLECTION_ROOT, directory, '/photos');
  const metadata = await stat(root);
  if (!metadata.isDirectory()) fail('/photos', 'path_not_directory', 'photos input must be a directory');
  const entries = (await readdir(root, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && IMAGE_EXTENSIONS.has(extname(entry.name).toLowerCase()))
    .sort((left, right) => compareText(left.name, right.name));
  if (entries.length === 0 || entries.length > 100) {
    fail('/photos', 'invalid_photo_count', 'photos directory must contain between 1 and 100 supported images');
  }
  const photos = await Promise.all(entries.map(async (entry, index) => {
    const path = await existingPrivatePath(COLLECTION_ROOT, join(root, entry.name), `/photos/${index}`);
    const bytes = await boundedRead(path, MAX_IMAGE_BYTES, `/photos/${index}`);
    return {
      photoId: `photo-${String(index + 1).padStart(4, '0')}`,
      path,
      byteHash: sha256(bytes),
    };
  }));
  if (new Set(photos.map(({ byteHash }) => byteHash)).size !== photos.length) {
    fail('/photos', 'duplicate_photo', 'photos directory contains duplicate image bytes');
  }
  return photos;
}

async function detectorReferences(
  catalogPath: string,
  catalog: ReturnType<typeof validateScanCatalog>,
): Promise<readonly Readonly<{ labelId: string; path: string }>[]> {
  const root = dirname(catalogPath);
  const references: Array<Readonly<{ labelId: string; path: string }>> = [];
  for (const [entryIndex, entry] of catalog.entries()) {
    for (const [imageIndex, relativePath] of entry.referenceImages.entries()) {
      const path = await existingPrivatePath(
        COLLECTION_ROOT,
        join(root, ...relativePath.split('/')),
        `/catalog/entries/${entryIndex}/referenceImages/${imageIndex}`,
      );
      if (!IMAGE_EXTENSIONS.has(extname(path).toLowerCase())) {
        fail(`/catalog/entries/${entryIndex}/referenceImages/${imageIndex}`, 'unsupported_image', 'unsupported reference image type');
      }
      await boundedRead(path, MAX_IMAGE_BYTES, `/catalog/entries/${entryIndex}/referenceImages/${imageIndex}`);
      references.push({ labelId: entry.labelId, path });
    }
  }
  return references;
}

async function runDetector(
  executable: string,
  request: JsonValue,
  timeoutMs: number,
): Promise<DetectorOutput> {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(executable, [DETECTOR_PATH], {
      cwd: REPOSITORY_ROOT,
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    });
    const stdout: Buffer[] = [];
    let stdoutBytes = 0;
    let settled = false;
    const finish = (error: Error | null, output?: DetectorOutput): void => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) rejectPromise(error);
      else resolvePromise(output!);
    };
    const timer = setTimeout(() => {
      child.kill();
      finish(new Error('detector timeout'));
    }, timeoutMs);
    child.on('error', () => finish(new Error('detector unavailable')));
    child.stdout.on('data', (chunk: Buffer) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > MAX_DETECTOR_OUTPUT_BYTES) {
        child.kill();
        finish(new Error('detector output limit'));
        return;
      }
      stdout.push(chunk);
    });
    child.stderr.resume();
    child.stdin.on('error', () => finish(new Error('detector input failed')));
    child.on('close', (code) => {
      if (code !== 0) return finish(new Error('detector failed'));
      try {
        const parsed = parseJsonWithDuplicateKeyCheck(Buffer.concat(stdout).toString('utf8'));
        finish(null, detectorOutputSchema.parse(parsed));
      } catch {
        finish(new Error('detector returned invalid output'));
      }
    });
    child.stdin.end(canonicalJson(request));
  });
}

const defaultIo: CommandIo = {
  stdout: (line) => process.stdout.write(line + '\n'),
  stderr: (line) => process.stderr.write(line + '\n'),
};

export async function runCollectionPhotoScanCommand(
  argv: readonly string[],
  io: CommandIo = defaultIo,
): Promise<number> {
  try {
    const args = parseArguments(argv);
    await mkdir(COLLECTION_ROOT, { recursive: true });
    const [authority, catalogInput, savedInput, reviewInput, photos, reportPath, collectionOutPath] = await Promise.all([
      loadAuthority(args.authority),
      privateJson(COLLECTION_ROOT, args.catalog, '/catalog'),
      privateJson(COLLECTION_ROOT, args.saved, '/saved'),
      args.review
        ? privateJson(COLLECTION_ROOT, args.review, '/review')
        : Promise.resolve(null),
      listPhotos(args.photos),
      privateOutputPath(args.report, '/report'),
      privateOutputPath(args.collectionOut, '/collectionOut'),
    ]);
    const catalogPath = await existingPrivatePath(COLLECTION_ROOT, args.catalog, '/catalog');
    if (reportPath === collectionOutPath) {
      fail('/arguments', 'duplicate_output_path', 'report and collection outputs must use different paths');
    }
    const catalog = validateScanCatalog(catalogInput, authority);
    const saved: SavedCollection = validateSavedCollection(savedCollectionSchema.parse(savedInput), authority);
    const references = await detectorReferences(catalogPath, catalog);
    const detector = await runDetector(args.python, {
      schemaVersion: 1,
      photos: photos.map(({ photoId, path }) => ({ photoId, path })),
      references,
      options: {
        confidenceThreshold: args.confidenceThreshold,
        marginThreshold: args.marginThreshold,
        minimumCardAreaRatio: args.minimumCardAreaRatio,
        maximumCardAreaRatio: args.maximumCardAreaRatio,
        grid: args.grid,
      },
    }, args.timeoutMs);
    const expectedPhotoIds = photos.map(({ photoId }) => photoId);
    const actualPhotoIds = detector.photos.map(({ photoId }) => photoId);
    if (canonicalJson(actualPhotoIds) !== canonicalJson(expectedPhotoIds)) {
      fail('/detector/photos', 'photo_set_mismatch', 'detector did not return the exact requested photo set');
    }
    if (detector.photos.flatMap(({ detections }) => detections).length === 0) {
      fail('/detector/photos', 'no_cards_detected', 'no cards were detected; adjust the grid or area calibration');
    }
    const photoHashes = photos.map(({ photoId, byteHash }) => ({ photoId, byteHash }));
    const review: ReviewManifest = reviewInput
      ? validateReviewManifest(reviewInput, authority)
      : reviewManifestSchema.parse({
          schemaVersion: 1,
          scanBinding: createScanBinding(detector, photoHashes, catalog),
          decisions: [],
        });
    const report = buildScanReport({
      detector,
      catalog,
      snapshot: authority,
      savedCollection: saved,
      review,
      mode: args.mode,
      photoHashes,
    });
    await writeFile(reportPath, canonicalJson(report as unknown as JsonValue), { flag: 'wx', flush: true });
    let collectionWritten = false;
    if (report.proposedCollection) {
      await writeFile(collectionOutPath, canonicalJson(report.proposedCollection), { flag: 'wx', flush: true });
      collectionWritten = true;
    }
    io.stdout(canonicalJson({
      status: report.unresolvedDetectionIds.length === 0 ? 'reconciled' : 'review-required',
      detected: report.detections.length,
      confirmed: report.detections.filter(({ disposition }) => disposition === 'confirmed').length,
      dismissed: report.detections.filter(({ disposition }) => disposition === 'dismissed').length,
      unresolved: report.unresolvedDetectionIds.length,
      collectionWritten,
    }));
    return 0;
  } catch (error: unknown) {
    const diagnostic = error instanceof CollectionScanError
      ? { path: error.path, code: error.code, message: error.message }
      : { path: '', code: 'collection_scan_failed', message: 'collection photo scan failed' };
    io.stderr(canonicalJson(diagnostic));
    return 1;
  }
}

if (process.argv[1] !== undefined && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  process.exitCode = await runCollectionPhotoScanCommand(process.argv.slice(2));
}
