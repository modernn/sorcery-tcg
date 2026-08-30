import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, readFileSync, readdirSync, realpathSync } from 'node:fs';
import { basename, dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import {
  CanonicalJsonError,
  canonicalJson,
  parseJsonWithDuplicateKeyCheck,
  type JsonValue,
} from '../src/authority/canonical-json.ts';
import {
  verifyPrivateSourceSet,
  type PrivateAuthoritySourceEntry,
} from '../src/authority/private-source-set.ts';

type PrivateLock = Readonly<{
  primaryRoot: string;
  backupRoot: string;
  entries: readonly PrivateAuthoritySourceEntry[];
  sourceSetRootHash: string;
  rulebookAcquisitionEvidence?: Readonly<{ privateLocatorEvidence?: string }>;
}>;
type CandidateSurface = 'reachable-history' | 'worktree' | 'index' | 'package';
type Candidate = Readonly<{ path: string; bytes: Buffer; surface: CandidateSurface }>;
type PackResult = Readonly<{ files?: readonly Readonly<{ path?: string }>[] }>;
type Locator = Readonly<{ bytes: Buffer; caseInsensitive: boolean }>;
type SemanticIndex = Map<string, string[]>;
type RawFingerprintIndex = Readonly<{
  fingerprints: BigUint64Array;
  fingerprintBloom: Uint8Array;
  fingerprintBloomMask: number;
  sources: readonly Buffer[];
  sourceFingerprintStarts: readonly number[];
  sourceStride: number;
  ordinalBits: number;
  fingerprintMode: 'edges' | 'rolling';
  fingerprintBytes: number;
  minimumMatchBytes: number;
}>;
type InspectionState = {
  candidateBytes: number;
  collisionComparisons: number;
  decodedBytes: number;
  decodedStrings: number;
};
type PreparationBudget = { reservedBytes: number };
type PrivateRetentionBudget = { normalizedBytes: number; semanticBytes: number };
type PrivateEvidence = Readonly<{
  privateHashes: ReadonlySet<string>;
  rawIndex: RawFingerprintIndex;
  normalizedIndex: RawFingerprintIndex;
  semanticIndex: SemanticIndex;
  locators: readonly Locator[];
  state: InspectionState;
}>;

export type PrivateInspectionSource = Readonly<{
  bytes: Uint8Array;
  kind: 'binary' | 'html' | 'json';
}>;

const MIN_PROTECTED_EXCERPT_BYTES = 32;
const RAW_FINGERPRINT_BYTES = 24;
const RAW_FINGERPRINT_STRIDE = MIN_PROTECTED_EXCERPT_BYTES - RAW_FINGERPRINT_BYTES + 1;
const MIN_NORMALIZED_TEXT_BYTES = 96;
const MIN_SEMANTIC_RECORD_BYTES = 96;
const MAX_PRIVATE_BYTES = 128_000_000;
const MAX_PRIVATE_FINGERPRINTS = 128_000_000;
const MAX_PRIVATE_PREPARATION_BYTES = 768 * 1024 * 1024;
// Four MiB covers the official text-only HTML corpus with case-fold growth while preserving two-lock headroom.
const MAX_PRIVATE_NORMALIZED_HTML_BYTES = 4 * 1024 * 1024;
// Ninety-six MiB accommodates the canonical card root plus retained nested records within the two-lock budget.
const MAX_PRIVATE_SEMANTIC_BYTES = 96 * 1024 * 1024;
const MAX_CANDIDATE_BYTES = 64_000_000;
const MAX_CANDIDATE_WORK_BYTES = 1_073_741_824;
const MAX_DECODED_BYTES = 64_000_000;
const MAX_DECODED_STRINGS = 100_000;
// The fail-closed cap retains 4.6x headroom over the 21,713-comparison production baseline.
const MAX_COLLISION_COMPARISONS = 100_000;
const MAX_DECODE_DEPTH = 4;
const MAX_SEMANTIC_SUBTREES = 200_000;
const MAX_COMMAND_BUFFER = 268_435_456;
const MAX_COMMAND_MILLISECONDS = 120_000;
const HASH_BASE_A = 16_777_619;
const HASH_BASE_B = 2_246_822_519;
const PUBLIC_PROVENANCE_VALUE_HASHES = new Set([
  '3b199aa2163bc7a7e2d8ef516d9121e517a4969ed23bc4a06c4ba431febc75de',
  '16a25c7198ec8d34bad13e0c00c8e5f82f74c33f7f5789c86b3d28556c6e7544',
  '58cdb645b9874da1e86634e50903f346bf66a121b1fdedf8217ba25f1a1a51f8',
  '7e7a9cb3fb2bb612b68073ecec741350f675c464f39aa54cc07fb2579ed0a491',
  '8d05aa6430da3512f1f599d16f0b31ec6832a47298e4551a0434af9fa995ee21',
  'a4f69b48b155539df89ebd9a1e12cb41de9d5fb6a30e06d0452a62a4c1b5fca3',
  'be87caa83c92bf30aa7789ea0686fb232a442eb53fcd3655a2bf864fe08b069d',
  'ee06a5b6402cb919ede6e8acb348a4d40aa8f64a29edd143cb4292a87e47deae',
  'ffbceac6cdd294322c4470b46a705a7d77b78726dafd0b5b7ad938d684d090d5',
]);
const EMBEDDED_PUBLIC_PROVENANCE_VALUES = [
  Buffer.from('Sorcery: Contested Realm December 2025 Rulebook Update', 'utf8'),
  Buffer.from('Sorcery: Contested Realm Card Updates 2025', 'utf8'),
] as const;
const MAX_PUBLIC_PROVENANCE_LITERAL_BYTES = 256;

class BoundaryViolation extends Error {}

function parseArguments(arguments_: readonly string[]): Readonly<{ repositoryRoot: string; lockPaths: readonly string[] }> {
  let values: ReturnType<typeof parseArgs>['values'];
  try {
    ({ values } = parseArgs({
      args: [...arguments_],
      allowPositionals: false,
      strict: true,
      options: {
        'repository-root': { type: 'string' },
        lock: { type: 'string', multiple: true },
      },
    }));
  } catch {
    throw new BoundaryViolation('Usage: --repository-root <path> --lock <path> [--lock <path> ...]');
  }
  const repositoryRoot = values['repository-root'];
  const locks = values.lock;
  if (
    typeof repositoryRoot !== 'string' ||
    !Array.isArray(locks) ||
    locks.length === 0 ||
    !locks.every((lockPath): lockPath is string => typeof lockPath === 'string')
  ) {
    throw new BoundaryViolation('Usage: --repository-root <path> --lock <path> [--lock <path> ...]');
  }
  const root = resolve(repositoryRoot);
  return {
    repositoryRoot: root,
    lockPaths: locks.map((lockPath) => isAbsolute(lockPath) ? resolve(lockPath) : resolve(root, lockPath)),
  };
}

function commandBytes(
  command: string,
  arguments_: readonly string[],
  cwd: string,
  input?: string | Uint8Array,
): Buffer {
  try {
    return execFileSync(command, arguments_, {
      cwd,
      encoding: null,
      input,
      killSignal: 'SIGKILL',
      maxBuffer: MAX_COMMAND_BUFFER,
      timeout: MAX_COMMAND_MILLISECONDS,
      windowsHide: true,
    });
  } catch {
    throw new BoundaryViolation(`Boundary command failed [${command}:${arguments_[0] ?? 'unknown'}].`);
  }
}

function reachableHistoryCandidates(repositoryRoot: string): readonly Candidate[] {
  const candidates: Candidate[] = [];
  const objects = commandBytes(
    'git',
    ['rev-list', '--objects', '--branches', '--remotes', '--tags'],
    repositoryRoot,
  ).toString('utf8');
  const records = objects.split(/\r?\n/).flatMap((record) => {
    if (record === '') return [];
    const separator = record.indexOf(' ');
    return separator < 0 ? [] : [{ objectId: record.slice(0, separator), path: record.slice(separator + 1) }];
  });
  if (records.length === 0) return candidates;
  const batch = commandBytes(
    'git',
    ['cat-file', '--batch'],
    repositoryRoot,
    records.map(({ objectId }) => objectId).join('\n') + '\n',
  );
  let offset = 0;
  for (const record of records) {
    const headerEnd = batch.indexOf(0x0a, offset);
    if (headerEnd < 0) throw new BoundaryViolation('Could not parse Git history batch header.');
    const match = /^([0-9a-f]+) ([a-z]+) (\d+)$/.exec(batch.subarray(offset, headerEnd).toString('ascii'));
    if (match === null || match[1] !== record.objectId) {
      throw new BoundaryViolation('Could not parse Git history batch record.');
    }
    const size = Number.parseInt(match[3]!, 10);
    const contentStart = headerEnd + 1;
    const contentEnd = contentStart + size;
    if (!Number.isSafeInteger(size) || contentEnd >= batch.length || batch[contentEnd] !== 0x0a) {
      throw new BoundaryViolation('Could not parse Git history batch content.');
    }
    if (match[2] === 'blob') {
      candidates.push({
        path: record.path,
        bytes: Buffer.from(batch.subarray(contentStart, contentEnd)),
        surface: 'reachable-history',
      });
    }
    offset = contentEnd + 1;
  }
  return candidates;
}

function indexCandidates(repositoryRoot: string): readonly Candidate[] {
  const candidates: Candidate[] = [];
  const index = commandBytes('git', ['ls-files', '--stage', '-z'], repositoryRoot).toString('utf8');
  for (const record of index.split('\0')) {
    if (record === '') continue;
    const match = /^\d+ ([0-9a-f]+) \d+\t([\s\S]+)$/.exec(record);
    if (match === null) throw new BoundaryViolation('Could not parse a Git index blob record.');
    candidates.push({
      path: match[2]!,
      bytes: commandBytes('git', ['cat-file', 'blob', match[1]!], repositoryRoot),
      surface: 'index',
    });
  }
  return candidates;
}

function confinedPath(root: string, path: string, label: string): string {
  const absolute = resolve(root, path);
  const outside = relative(root, absolute);
  if (outside === '..' || outside.startsWith('..' + sep) || isAbsolute(outside)) {
    throw new BoundaryViolation(label + ' escaped its allowed root.');
  }
  return absolute;
}

function worktreeCandidates(repositoryRoot: string): readonly Candidate[] {
  const candidates: Candidate[] = [];
  const files = commandBytes(
    'git',
    ['ls-files', '--cached', '--others', '--exclude-standard', '-z'],
    repositoryRoot,
  ).toString('utf8');
  for (const path of files.split('\0').filter(Boolean)) {
    const absolute = confinedPath(repositoryRoot, path, 'Git worktree path');
    let metadata: ReturnType<typeof lstatSync>;
    try {
      metadata = lstatSync(absolute);
    } catch {
      continue;
    }
    if (metadata.isSymbolicLink()) throw new BoundaryViolation('Git worktree candidate is a symbolic link.');
    if (!metadata.isFile()) continue;
    const real = realpathSync(absolute);
    confinedPath(realpathSync(repositoryRoot), real, 'Git worktree real path');
    candidates.push({ path, bytes: readFileSync(real), surface: 'worktree' });
  }
  return candidates;
}

function packageCandidates(repositoryRoot: string): readonly Candidate[] {
  const pnpmCommand = process.platform === 'win32' ? (process.env.ComSpec ?? 'cmd.exe') : 'pnpm';
  const pnpmArguments =
    process.platform === 'win32'
      ? ['/d', '/s', '/c', 'pnpm.cmd', 'pack', '--dry-run', '--json']
      : ['pack', '--dry-run', '--json'];
  const parsed = JSON.parse(
    execFileSync(pnpmCommand, pnpmArguments, {
      cwd: repositoryRoot,
      encoding: 'utf8',
      maxBuffer: MAX_COMMAND_BUFFER,
      windowsHide: true,
    }),
  ) as PackResult | readonly PackResult[];
  const results = Array.isArray(parsed) ? parsed : [parsed];
  const candidates: Candidate[] = [];
  for (const result of results) {
    if (!Array.isArray(result.files)) throw new BoundaryViolation('Could not parse pnpm dry-run package files.');
    for (const file of result.files) {
      if (typeof file.path !== 'string') throw new BoundaryViolation('Could not parse a pnpm dry-run package path.');
      const path = confinedPath(repositoryRoot, file.path, 'pnpm dry-run path');
      candidates.push({ path: file.path, bytes: readFileSync(path), surface: 'package' });
    }
  }
  return candidates;
}

function sha256Hex(bytes: Uint8Array): string {
  return createHash('sha256').update(bytes).digest('hex');
}

function multiplyAdd(hash: number, base: number, byte: number): number {
  return (Math.imul(hash, base) + byte) >>> 0;
}

function rotateLeft32(value: number, distance: number): number {
  return ((value << distance) | (value >>> (32 - distance))) >>> 0;
}

function avalanche32(value: number): number {
  value = Math.imul(value ^ (value >>> 16), 0x7feb_352d) >>> 0;
  value = Math.imul(value ^ (value >>> 15), 0x846c_a68b) >>> 0;
  return (value ^ (value >>> 16)) >>> 0;
}

function strongRawFingerprints(bytes: Buffer, offset: number): readonly [number, number] {
  let first = 0x243f_6a88;
  let second = 0x85a3_08d3;
  for (let wordIndex = 0; wordIndex < 6; wordIndex += 1) {
    const word = bytes.readUInt32LE(offset + wordIndex * 4);
    first = rotateLeft32(Math.imul(first ^ word, 0x9e37_79b1) >>> 0, 13);
    second = rotateLeft32(Math.imul(second ^ word, 0x85eb_ca77) >>> 0, 15);
  }
  return [avalanche32(first), avalanche32(second)];
}

function power(base: number, exponent: number): number {
  let value = 1;
  for (let index = 0; index < exponent; index += 1) value = Math.imul(value, base) >>> 0;
  return value;
}

function fingerprintWindowCount(byteLength: number, fingerprintBytes: number, stride: number): number {
  return byteLength < fingerprintBytes
    ? 0
    : Math.floor((byteLength - fingerprintBytes) / stride) + 1;
}

function fingerprintBloomBytes(count: number): number {
  return 2 ** Math.min(30, Math.max(3, Math.ceil(Math.log2(Math.max(1, count * 8))))) / 8;
}

function reservePrivatePreparationBytes(budget: PreparationBudget, bytes: number): void {
  if (!Number.isSafeInteger(bytes) || bytes < 0) {
    throw new BoundaryViolation('Private preparation estimate is invalid.');
  }
  const reservedBytes = budget.reservedBytes + bytes;
  if (!Number.isSafeInteger(reservedBytes) || reservedBytes > MAX_PRIVATE_PREPARATION_BYTES) {
    throw new BoundaryViolation('Private preparation allocation exceeded the fixed limit.');
  }
  budget.reservedBytes = reservedBytes;
}

export function verifyPrivatePreparationBudgetForTest(estimates: readonly number[]): void {
  const budget: PreparationBudget = { reservedBytes: 0 };
  for (const bytes of estimates) reservePrivatePreparationBytes(budget, bytes);
}

function windowFingerprints(
  bytes: Buffer,
  minimumBytes: number,
  visit: (first: number, second: number, offset: number, firstProbe?: number) => void,
  stride = 1,
  mode: RawFingerprintIndex['fingerprintMode'] = 'rolling',
): void {
  if (bytes.length < minimumBytes) return;
  if (mode === 'edges') {
    for (let offset = 0; offset <= bytes.length - minimumBytes; offset += stride) {
      const head = bytes.readUInt32LE(offset);
      const middle = bytes.readUInt32LE(offset + 10);
      const tail = bytes.readUInt32LE(offset + minimumBytes - 4);
      const firstSeed = (head ^ ((tail << 13) | (tail >>> 19)) ^ ((middle << 7) | (middle >>> 25))) >>> 0;
      const firstProbe = Math.imul(firstSeed ^ (firstSeed >>> 16), 0x7feb_352d) >>> 0;
      const [first, second] = strongRawFingerprints(bytes, offset);
      visit(first, second, offset, firstProbe);
    }
    return;
  }
  const windowPowerA = power(HASH_BASE_A, minimumBytes - 1);
  const windowPowerB = power(HASH_BASE_B, minimumBytes - 1);
  let first = 0;
  let second = 0;
  for (let index = 0; index < minimumBytes; index += 1) {
    first = multiplyAdd(first, HASH_BASE_A, bytes[index]!);
    second = multiplyAdd(second, HASH_BASE_B, bytes[index]!);
  }
  visit(first, second, 0);
  for (let offset = 1; offset <= bytes.length - minimumBytes; offset += 1) {
    const previous = bytes[offset - 1]!;
    const next = bytes[offset + minimumBytes - 1]!;
    first = multiplyAdd(
      (first - Math.imul(previous, windowPowerA)) >>> 0,
      HASH_BASE_A,
      next,
    );
    second = multiplyAdd(
      (second - Math.imul(previous, windowPowerB)) >>> 0,
      HASH_BASE_B,
      next,
    );
    if (offset % stride === 0) visit(first, second, offset);
  }
}

function normalizedVisibleText(bytes: Buffer, maximumOutputBytes = MAX_PRIVATE_BYTES): Buffer | null {
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
  if (Buffer.byteLength(text, 'utf8') > MAX_PRIVATE_BYTES) {
    throw new BoundaryViolation('Text normalization exceeded the fixed byte limit.');
  }
  const withoutHidden = text
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/<(script|style|head|noscript|template|svg)\b[^>]*>[\s\S]*?<\/\1\s*>/gi, ' ')
    .replace(/<[^>]*>/g, ' ');
  const decoded = withoutHidden
    .replace(/&#(?:x([0-9a-f]+)|([0-9]+));/gi, (_match, hexadecimal: string, decimal: string) => {
      const codePoint = Number.parseInt(hexadecimal ?? decimal, hexadecimal === undefined ? 10 : 16);
      return Number.isInteger(codePoint) &&
        codePoint >= 0 &&
        codePoint <= 0x10ffff &&
        !(codePoint >= 0xd800 && codePoint <= 0xdfff)
        ? String.fromCodePoint(codePoint)
        : ' ';
    })
    .replace(/&(amp|apos|gt|lt|nbsp|quot);/gi, (_match, name: string) => {
      const values: Readonly<Record<string, string>> = {
        amp: '&',
        apos: "'",
        gt: '>',
        lt: '<',
        nbsp: ' ',
        quot: '"',
      };
      return values[name.toLowerCase()]!;
    })
    .toLowerCase()
    .replace(/\s+/gu, ' ')
    .trim();
  if (Buffer.byteLength(decoded, 'utf8') > maximumOutputBytes) {
    throw new BoundaryViolation('Text normalization exceeded the fixed byte limit.');
  }
  return Buffer.from(decoded, 'utf8');
}

function reservePrivateRetentionBytes(
  budget: PrivateRetentionBudget,
  field: keyof PrivateRetentionBudget,
  bytes: number,
  maximumBytes: number,
  category: 'normalized HTML' | 'semantic JSON',
): void {
  if (!Number.isSafeInteger(bytes) || bytes < 0 || !Number.isSafeInteger(maximumBytes) || maximumBytes < 0) {
    throw new BoundaryViolation('Private retention estimate is invalid.');
  }
  const retainedBytes = budget[field] + bytes;
  if (!Number.isSafeInteger(retainedBytes) || retainedBytes > maximumBytes) {
    throw new BoundaryViolation(`Private ${category} retention exceeded the fixed limit.`);
  }
  budget[field] = retainedBytes;
}

function addPrivateNormalizedHtml(
  bytes: Buffer,
  normalizedTextBuffers: Map<string, Buffer>,
  retentionBudget: PrivateRetentionBudget,
  maximumBytes = MAX_PRIVATE_NORMALIZED_HTML_BYTES,
): void {
  const normalized = normalizedVisibleText(bytes, maximumBytes);
  if (normalized === null || normalized.length < MIN_PROTECTED_EXCERPT_BYTES) return;
  const fingerprint = sha256Hex(normalized);
  if (normalizedTextBuffers.has(fingerprint)) return;
  reservePrivateRetentionBytes(
    retentionBudget,
    'normalizedBytes',
    normalized.length,
    maximumBytes,
    'normalized HTML',
  );
  normalizedTextBuffers.set(fingerprint, normalized);
}

function buildRawFingerprintIndex(
  privateBuffers: readonly Buffer[],
  fingerprintBytes: number,
  minimumMatchBytes = fingerprintBytes,
  stride = 1,
  fingerprintMode: RawFingerprintIndex['fingerprintMode'] = 'rolling',
): RawFingerprintIndex {
  let fingerprintCount = 0;
  let privateBytes = 0;
  const sourceFingerprintStarts = [0];
  for (const bytes of privateBuffers) {
    privateBytes += bytes.length;
    fingerprintCount += fingerprintWindowCount(bytes.length, fingerprintBytes, stride);
    sourceFingerprintStarts.push(fingerprintCount);
  }
  if (privateBytes > MAX_PRIVATE_BYTES || fingerprintCount > MAX_PRIVATE_FINGERPRINTS) {
    throw new BoundaryViolation('Private fingerprint work exceeded the fixed limit.');
  }
  const ordinalBits = Math.max(1, Math.ceil(Math.log2(Math.max(2, fingerprintCount))));
  if (ordinalBits >= 64) throw new BoundaryViolation('Private fingerprint index exceeded its fixed width.');
  const ordinalShift = BigInt(ordinalBits);
  const fingerprintBloom = new Uint8Array(fingerprintBloomBytes(fingerprintCount));
  const fingerprintBloomBits = fingerprintBloom.length * 8;
  const fingerprintBloomMask = fingerprintBloomBits - 1;
  const fingerprints = new BigUint64Array(fingerprintCount);
  let index = 0;
  for (const bytes of privateBuffers) {
    windowFingerprints(bytes, fingerprintBytes, (first, second, _offset, firstProbe = first) => {
      const firstBit = firstProbe & fingerprintBloomMask;
      const secondBit = second & fingerprintBloomMask;
      fingerprintBloom[firstBit >>> 3] = fingerprintBloom[firstBit >>> 3]! | (1 << (firstBit & 7));
      fingerprintBloom[secondBit >>> 3] = fingerprintBloom[secondBit >>> 3]! | (1 << (secondBit & 7));
      const fingerprint = (BigInt(first) << 32n) | BigInt(second);
      fingerprints[index] = ((fingerprint >> ordinalShift) << ordinalShift) | BigInt(index);
      index += 1;
    }, stride, fingerprintMode);
  }
  const protectedFingerprints = fingerprints.subarray(0, index);
  protectedFingerprints.sort();
  return {
    fingerprints: protectedFingerprints,
    fingerprintBloom,
    fingerprintBloomMask,
    sources: privateBuffers,
    sourceFingerprintStarts,
    sourceStride: stride,
    ordinalBits,
    fingerprintMode,
    fingerprintBytes,
    minimumMatchBytes,
  };
}

function addRawFingerprintSegments(
  bytes: Buffer,
  segments: Map<string, Buffer>,
): void {
  if (bytes.length >= MIN_PROTECTED_EXCERPT_BYTES) {
    segments.set(sha256Hex(bytes), bytes);
  }
}

function lowerBoundFingerprint(fingerprints: BigUint64Array, fingerprint: bigint): number {
  let low = 0;
  let high = fingerprints.length;
  while (low < high) {
    const middle = low + Math.floor((high - low) / 2);
    if (fingerprints[middle]! < fingerprint) low = middle + 1;
    else high = middle;
  }
  return low;
}

function containsProtectedWindow(
  bytes: Buffer,
  indexes: readonly RawFingerprintIndex[],
  candidate: Candidate,
  state: InspectionState,
): boolean {
  if (indexes.length === 0) return false;
  const fingerprintBytes = indexes[0]!.fingerprintBytes;
  const minimumMatchBytes = indexes[0]!.minimumMatchBytes;
  const fingerprintMode = indexes[0]!.fingerprintMode;
  if (indexes.some((index) =>
    index.fingerprintBytes !== fingerprintBytes ||
    index.minimumMatchBytes !== minimumMatchBytes ||
    index.fingerprintMode !== fingerprintMode
  )) {
    throw new BoundaryViolation('Private fingerprint indexes are incompatible.');
  }
  let matched = false;
  const leadingBytes = minimumMatchBytes - fingerprintBytes;
  const inspectFingerprint = (
    first: number,
    second: number,
    offset: number,
    firstProbe = first,
  ): void => {
    if (matched) return;
    let fingerprint: bigint | undefined;
    for (const index of indexes) {
      const firstBit = firstProbe & index.fingerprintBloomMask;
      const secondBit = second & index.fingerprintBloomMask;
      if (
        (index.fingerprintBloom[firstBit >>> 3]! & (1 << (firstBit & 7))) === 0 ||
        (index.fingerprintBloom[secondBit >>> 3]! & (1 << (secondBit & 7))) === 0
      ) continue;
      fingerprint ??= (BigInt(first) << 32n) | BigInt(second);
      const ordinalShift = BigInt(index.ordinalBits);
      const ordinalMask = (1n << ordinalShift) - 1n;
      const prefix = fingerprint >> ordinalShift;
      let recordIndex = lowerBoundFingerprint(index.fingerprints, prefix << ordinalShift);
      const token = bytes.subarray(offset, offset + fingerprintBytes);
      while (
        recordIndex < index.fingerprints.length &&
        index.fingerprints[recordIndex]! >> ordinalShift === prefix
      ) {
        state.collisionComparisons += 1;
        if (state.collisionComparisons > MAX_COLLISION_COMPARISONS) {
          fail('fingerprint-collision-work-limit', candidate);
        }
        const ordinal = Number(index.fingerprints[recordIndex]! & ordinalMask);
        let sourceIndex = 0;
        while (index.sourceFingerprintStarts[sourceIndex + 1]! <= ordinal) sourceIndex += 1;
        const source = index.sources[sourceIndex]!;
        const sourceOffset =
          (ordinal - index.sourceFingerprintStarts[sourceIndex]!) * index.sourceStride;
        if (source.subarray(sourceOffset, sourceOffset + fingerprintBytes).equals(token)) {
          for (let leading = 0; leading <= leadingBytes; leading += 1) {
            const candidateStart = offset - leading;
            const sourceStart = sourceOffset - leading;
            if (
              candidateStart >= 0 &&
              sourceStart >= 0 &&
              candidateStart + minimumMatchBytes <= bytes.length &&
              sourceStart + minimumMatchBytes <= source.length &&
              bytes.subarray(candidateStart, candidateStart + minimumMatchBytes)
                .equals(source.subarray(sourceStart, sourceStart + minimumMatchBytes))
            ) {
              matched = true;
              return;
            }
          }
        }
        recordIndex += 1;
      }
    }
  };
  if (fingerprintMode === 'edges') {
    for (let offset = 0; offset <= bytes.length - fingerprintBytes; offset += 1) {
      const head = bytes.readUInt32LE(offset);
      const middle = bytes.readUInt32LE(offset + 10);
      const tail = bytes.readUInt32LE(offset + fingerprintBytes - 4);
      const firstSeed = (head ^ ((tail << 13) | (tail >>> 19)) ^ ((middle << 7) | (middle >>> 25))) >>> 0;
      const coarseProbe = Math.imul(firstSeed ^ (firstSeed >>> 16), 0x7feb_352d) >>> 0;
      let firstMayMatch = false;
      for (const index of indexes) {
        const bit = coarseProbe & index.fingerprintBloomMask;
        if ((index.fingerprintBloom[bit >>> 3]! & (1 << (bit & 7))) !== 0) {
          firstMayMatch = true;
          break;
        }
      }
      if (!firstMayMatch) continue;
      const [first, second] = strongRawFingerprints(bytes, offset);
      inspectFingerprint(
        first,
        second,
        offset,
        coarseProbe,
      );
      if (matched) break;
    }
  } else {
    windowFingerprints(bytes, fingerprintBytes, inspectFingerprint);
  }
  return matched;
}

function publicProvenanceFreeSegments(bytes: Buffer): readonly Buffer[] {
  const segments: Buffer[] = [];
  let segmentStart = 0;
  const isAsciiWord = (byte: number | undefined): boolean =>
    byte !== undefined &&
    ((byte >= 0x30 && byte <= 0x39) ||
      (byte >= 0x41 && byte <= 0x5a) ||
      (byte >= 0x61 && byte <= 0x7a) ||
      byte === 0x5f);
  for (let index = 0; index < bytes.length; index += 1) {
    const embedded = EMBEDDED_PUBLIC_PROVENANCE_VALUES.find((value) =>
      !isAsciiWord(bytes[index - 1]) &&
      bytes.subarray(index, index + value.length).equals(value) &&
      !isAsciiWord(bytes[index + value.length]),
    );
    if (embedded !== undefined) {
      segments.push(bytes.subarray(segmentStart, index));
      segmentStart = index + embedded.length;
      index = segmentStart - 1;
      continue;
    }
    if (bytes.subarray(index, index + 8).toString('ascii') === 'https://') {
      const searchLimit = Math.min(bytes.length, index + MAX_PUBLIC_PROVENANCE_LITERAL_BYTES);
      let closing = index + 8;
      while (
        closing < searchLimit &&
        ![0x09, 0x0a, 0x0d, 0x20, 0x22, 0x27, 0x29, 0x3e, 0x5d, 0x60].includes(bytes[closing]!)
      ) closing += 1;
      const literal = bytes.subarray(index, closing);
      if (PUBLIC_PROVENANCE_VALUE_HASHES.has(sha256Hex(literal))) {
        segments.push(bytes.subarray(segmentStart, index));
        segmentStart = closing;
        index = closing - 1;
        continue;
      }
    }
    const delimiter = bytes[index];
    if (delimiter !== 0x22 && delimiter !== 0x27 && delimiter !== 0x60) continue;
    const searchLimit = Math.min(bytes.length, index + MAX_PUBLIC_PROVENANCE_LITERAL_BYTES + 2);
    for (let closing = index + 1; closing < searchLimit; closing += 1) {
      if (bytes[closing] !== delimiter) continue;
      let backslashes = 0;
      for (let cursor = closing - 1; cursor > index && bytes[cursor] === 0x5c; cursor -= 1) backslashes += 1;
      if (backslashes % 2 !== 0) continue;
      const literal = bytes.subarray(index + 1, closing);
      if (PUBLIC_PROVENANCE_VALUE_HASHES.has(sha256Hex(literal))) {
        segments.push(bytes.subarray(segmentStart, index));
        segmentStart = closing + 1;
        index = closing;
      }
      break;
    }
  }
  segments.push(bytes.subarray(segmentStart));
  return segments;
}

function containsProtectedContent(
  bytes: Buffer,
  indexes: readonly RawFingerprintIndex[],
  candidate: Candidate,
  state: InspectionState,
): boolean {
  if (PUBLIC_PROVENANCE_VALUE_HASHES.has(sha256Hex(bytes))) return false;
  return publicProvenanceFreeSegments(bytes).some((segment) =>
    containsProtectedWindow(segment, indexes, candidate, state)
  );
}

function semanticFingerprint(canonical: string): string {
  return sha256Hex(Buffer.from(canonical, 'utf8'));
}

function addSemanticValue(
  value: JsonValue,
  semanticIndex: SemanticIndex,
  work: { subtrees: number },
  retentionBudget: PrivateRetentionBudget,
  maximumBytes: number,
): void {
  if (value === null || typeof value !== 'object') return;
  work.subtrees += 1;
  if (work.subtrees > MAX_SEMANTIC_SUBTREES) {
    throw new BoundaryViolation('Private semantic fingerprint work exceeded the fixed limit.');
  }
  const canonical = canonicalJson(value);
  const canonicalBytes = Buffer.byteLength(canonical, 'utf8');
  if (canonicalBytes >= MIN_SEMANTIC_RECORD_BYTES) {
    const fingerprint = semanticFingerprint(canonical);
    const matches = semanticIndex.get(fingerprint) ?? [];
    if (!matches.includes(canonical)) {
      reservePrivateRetentionBytes(
        retentionBudget,
        'semanticBytes',
        canonicalBytes,
        maximumBytes,
        'semantic JSON',
      );
      matches.push(canonical);
    }
    semanticIndex.set(fingerprint, matches);
  }
  if (Array.isArray(value)) {
    for (const child of value) {
      addSemanticValue(child, semanticIndex, work, retentionBudget, maximumBytes);
    }
  } else {
    for (const child of Object.values(value)) {
      addSemanticValue(child, semanticIndex, work, retentionBudget, maximumBytes);
    }
  }
}

function addPrivateJsonSemantics(
  bytes: Buffer,
  semanticIndex: SemanticIndex,
  retentionBudget: PrivateRetentionBudget,
  maximumBytes = MAX_PRIVATE_SEMANTIC_BYTES,
): void {
  let value: JsonValue;
  try {
    value = parseJsonWithDuplicateKeyCheck(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    throw new BoundaryViolation('Private JSON could not be fingerprinted safely.');
  }
  addSemanticValue(value, semanticIndex, { subtrees: 0 }, retentionBudget, maximumBytes);
}

export function verifyPrivateNormalizedRetentionForTest(
  sources: readonly Uint8Array[],
  maximumBytes: number,
): number {
  const retentionBudget: PrivateRetentionBudget = { normalizedBytes: 0, semanticBytes: 0 };
  const normalizedTextBuffers = new Map<string, Buffer>();
  reservePrivateRetentionBytes(retentionBudget, 'normalizedBytes', 0, maximumBytes, 'normalized HTML');
  for (const source of sources) {
    addPrivateNormalizedHtml(Buffer.from(source), normalizedTextBuffers, retentionBudget, maximumBytes);
  }
  return retentionBudget.normalizedBytes;
}

export function verifyPrivateSemanticRetentionForTest(
  sources: readonly Uint8Array[],
  maximumBytes: number,
): number {
  const retentionBudget: PrivateRetentionBudget = { normalizedBytes: 0, semanticBytes: 0 };
  const semanticIndex: SemanticIndex = new Map();
  reservePrivateRetentionBytes(retentionBudget, 'semanticBytes', 0, maximumBytes, 'semantic JSON');
  for (const source of sources) {
    addPrivateJsonSemantics(Buffer.from(source), semanticIndex, retentionBudget, maximumBytes);
  }
  return retentionBudget.semanticBytes;
}

function inspectJsonSemantics(bytes: Buffer, semanticIndex: SemanticIndex): boolean {
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes).trim();
  } catch {
    return false;
  }
  if (!(text.startsWith('{') || text.startsWith('['))) return false;
  let value: JsonValue;
  try {
    value = parseJsonWithDuplicateKeyCheck(text);
  } catch (error: unknown) {
    if (
      error instanceof CanonicalJsonError &&
      ['max_depth', 'max_nodes', 'max_string_bytes'].includes(error.code)
    ) {
      throw new BoundaryViolation('Candidate semantic work exceeded the fixed limit.');
    }
    return false;
  }
  let matched = false;
  const inspect = (child: JsonValue, work: { subtrees: number }): void => {
    if (matched || child === null || typeof child !== 'object') return;
    work.subtrees += 1;
    if (work.subtrees > MAX_SEMANTIC_SUBTREES) {
      throw new BoundaryViolation('Candidate semantic work exceeded the fixed limit.');
    }
    const canonical = canonicalJson(child);
    if (Buffer.byteLength(canonical, 'utf8') >= MIN_SEMANTIC_RECORD_BYTES) {
      const matches = semanticIndex.get(semanticFingerprint(canonical));
      if (matches?.includes(canonical) === true) {
        matched = true;
        return;
      }
    }
    if (Array.isArray(child)) {
      for (const nested of child) inspect(nested, work);
    } else {
      for (const nested of Object.values(child)) inspect(nested, work);
    }
  };
  inspect(value, { subtrees: 0 });
  return matched;
}

function decodeStringLiteral(
  text: string,
  start: number,
): Readonly<{ value: string; end: number }> | null {
  const quote = text[start];
  if (quote !== "'" && quote !== '"' && quote !== '`') return null;
  let value = '';
  for (let index = start + 1; index < text.length; index += 1) {
    const character = text[index]!;
    if (character === quote) return { value, end: index + 1 };
    if ((character === '\n' || character === '\r') && quote !== '`') return null;
    if (quote === '`' && character === '$' && text[index + 1] === '{') return null;
    if (character !== '\\') {
      value += character;
      continue;
    }
    const escape = text[index + 1];
    if (escape === undefined) return null;
    index += 1;
    const simple: Readonly<Record<string, string>> = {
      '"': '"',
      "'": "'",
      '`': '`',
      '\\': '\\',
      '/': '/',
      b: '\b',
      f: '\f',
      n: '\n',
      r: '\r',
      t: '\t',
      v: '\v',
      '0': '\0',
    };
    if (escape in simple) {
      value += simple[escape]!;
      continue;
    }
    if (escape === '\n') continue;
    if (escape === '\r') {
      if (text[index + 1] === '\n') index += 1;
      continue;
    }
    if (escape === 'x') {
      const digits = text.slice(index + 1, index + 3);
      if (!/^[0-9a-f]{2}$/i.test(digits)) return null;
      value += String.fromCodePoint(Number.parseInt(digits, 16));
      index += 2;
      continue;
    }
    if (escape === 'u') {
      if (text[index + 1] === '{') {
        const closing = text.indexOf('}', index + 2);
        if (closing < 0) return null;
        const digits = text.slice(index + 2, closing);
        if (!/^[0-9a-f]{1,6}$/i.test(digits)) return null;
        const codePoint = Number.parseInt(digits, 16);
        if (codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) return null;
        value += String.fromCodePoint(codePoint);
        index = closing;
        continue;
      }
      const digits = text.slice(index + 1, index + 5);
      if (!/^[0-9a-f]{4}$/i.test(digits)) return null;
      value += String.fromCharCode(Number.parseInt(digits, 16));
      index += 4;
      continue;
    }
    return null;
  }
  return null;
}

function hasArtworkSignature(bytes: Buffer): boolean {
  return (
    bytes.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) ||
    (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) ||
    bytes.subarray(0, 6).toString('ascii') === 'GIF87a' ||
    bytes.subarray(0, 6).toString('ascii') === 'GIF89a' ||
    (bytes.subarray(0, 4).toString('ascii') === 'RIFF' && bytes.subarray(8, 12).toString('ascii') === 'WEBP') ||
    bytes.subarray(0, 2).toString('ascii') === 'BM' ||
    bytes.subarray(0, 4).equals(Buffer.from([0x00, 0x00, 0x01, 0x00])) ||
    bytes.subarray(0, 4).equals(Buffer.from([0x49, 0x49, 0x2a, 0x00])) ||
    bytes.subarray(0, 4).equals(Buffer.from([0x4d, 0x4d, 0x00, 0x2a])) ||
    bytes.subarray(4, 12).toString('ascii') === 'ftypavif' ||
    /^\s*<svg\b/i.test(bytes.subarray(0, 256).toString('utf8'))
  );
}

function fail(category: string, candidate: Candidate, redactPath = false): never {
  const candidateLabel = redactPath
    ? 'candidate-' + sha256Hex(Buffer.from(candidate.path, 'utf8')).slice(0, 16)
    : candidate.path;
  throw new BoundaryViolation(
    'Private authority boundary violation [' +
      category +
      '] [' +
      candidate.surface +
      ']: ' +
      candidateLabel,
  );
}

function inspectContent(
  candidate: Candidate,
  bytes: Buffer,
  privateHashes: readonly ReadonlySet<string>[],
  rawIndexes: readonly RawFingerprintIndex[],
  normalizedIndexes: readonly RawFingerprintIndex[],
  semanticIndexes: readonly SemanticIndex[],
  locators: readonly Locator[],
  depth: number,
  state: InspectionState,
): void {
  const byteHash = sha256Hex(bytes);
  if (privateHashes.some((hashes) => hashes.has(byteHash))) fail('exact-private-bytes', candidate);
  const normalized = normalizedVisibleText(bytes);
  if (normalized !== null && containsProtectedWindow(normalized, normalizedIndexes, candidate, state)) {
    fail('source-derived-normalized-text', candidate);
  }
  if (containsProtectedContent(bytes, rawIndexes, candidate, state)) fail('source-derived-content', candidate);
  if (semanticIndexes.some((index) => inspectJsonSemantics(bytes, index))) {
    fail('semantic-private-content', candidate);
  }
  if (matchesLocator(bytes, locators)) fail('private-locator', candidate);

  const prefix = bytes.subarray(0, 512).toString('utf8');
  if (depth === 0
    && bytes.length > 1_000
    && /^(?:%PDF-|\s*<!doctype html|\s*<html\b)/i.test(prefix)) {
    fail('publisher-document', candidate);
  }
  if (
    bytes.length > 500_000 &&
    extname(candidate.path).toLowerCase() === '.json' &&
    /"(?:sourceCardId|printingSlugs|rulesText)"/.test(
      prefix + bytes.subarray(Math.max(0, bytes.length - 512)).toString('utf8'),
    )
  ) {
    fail('full-card-corpus', candidate);
  }
  if (
    bytes.length > 100_000 &&
    /\.(?:json|jsonl|csv|sqlite|db)$/i.test(candidate.path) &&
    /(?:contested-realms|spells\.bar|sorcery-registry)/i.test(bytes.toString('utf8'))
  ) {
    fail('copied-community-corpus', candidate);
  }

  if (depth >= MAX_DECODE_DEPTH) return;
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return;
  }
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] !== "'" && text[index] !== '"' && text[index] !== '`') continue;
    const decoded = decodeStringLiteral(text, index);
    if (decoded === null) continue;
    state.decodedStrings += 1;
    if (state.decodedStrings > MAX_DECODED_STRINGS) fail('decoded-string-work-limit', candidate);
    const decodedBytes = Buffer.from(decoded.value, 'utf8');
    state.decodedBytes += decodedBytes.length;
    if (state.decodedBytes > MAX_DECODED_BYTES) fail('decoded-byte-work-limit', candidate);
    if (decodedBytes.length > 0) {
      inspectContent(
        candidate,
        decodedBytes,
        privateHashes,
        rawIndexes,
        normalizedIndexes,
        semanticIndexes,
        locators,
        depth + 1,
        state,
      );
    }
    index = decoded.end - 1;
  }
}

function inspectCandidate(
  candidate: Candidate,
  privateHashes: readonly ReadonlySet<string>[],
  rawIndexes: readonly RawFingerprintIndex[],
  normalizedIndexes: readonly RawFingerprintIndex[],
  semanticIndexes: readonly SemanticIndex[],
  locators: readonly Locator[],
  state: InspectionState,
): void {
  state.decodedBytes = 0;
  state.decodedStrings = 0;
  if (candidate.bytes.length > MAX_CANDIDATE_BYTES) fail('candidate-size-limit', candidate);
  state.candidateBytes += candidate.bytes.length;
  if (state.candidateBytes > MAX_CANDIDATE_WORK_BYTES) fail('candidate-byte-work-limit', candidate);
  const normalizedPath = candidate.path.replaceAll('\\', '/');
  if (normalizedPath === '.local/authority' || normalizedPath.startsWith('.local/authority/')) {
    fail('forbidden-private-path', candidate);
  }
  const pathBytes = Buffer.from(normalizedPath, 'utf8');
  const normalizedPathBytes = normalizedVisibleText(pathBytes);
  if (containsProtectedContent(pathBytes, rawIndexes, candidate, state)) {
    fail('source-derived-path', candidate, true);
  }
  if (
    normalizedPathBytes !== null &&
    containsProtectedWindow(normalizedPathBytes, normalizedIndexes, candidate, state)
  ) {
    fail('source-derived-normalized-path', candidate, true);
  }
  if (matchesLocator(pathBytes, locators)) fail('private-locator-path', candidate, true);
  if (/\.(?:avif|bmp|gif|ico|jpe?g|png|svg|tiff?|webp)$/i.test(normalizedPath)) {
    fail('artwork-path', candidate);
  }
  if (hasArtworkSignature(candidate.bytes)) fail('artwork-signature', candidate);
  inspectContent(
    candidate,
    candidate.bytes,
    privateHashes,
    rawIndexes,
    normalizedIndexes,
    semanticIndexes,
    locators,
    0,
    state,
  );
}

export function createPrivateCandidateInspectorForTest(
  sources: readonly PrivateInspectionSource[],
  locatorTexts: readonly string[] = [],
): (candidate: Readonly<{ path: string; bytes: string | Uint8Array }>) => void {
  const privateHashes = new Set<string>();
  const rawFingerprintBuffers = new Map<string, Buffer>();
  const normalizedTextBuffers = new Map<string, Buffer>();
  const semanticIndex: SemanticIndex = new Map();
  const retentionBudget: PrivateRetentionBudget = { normalizedBytes: 0, semanticBytes: 0 };
  for (const source of sources) {
    const bytes = Buffer.from(source.bytes);
    privateHashes.add(sha256Hex(bytes));
    addRawFingerprintSegments(bytes, rawFingerprintBuffers);
    if (source.kind === 'json') addPrivateJsonSemantics(bytes, semanticIndex, retentionBudget);
    if (source.kind === 'html') {
      addPrivateNormalizedHtml(bytes, normalizedTextBuffers, retentionBudget);
    }
  }
  const rawIndex = buildRawFingerprintIndex(
    [...rawFingerprintBuffers.values()],
    RAW_FINGERPRINT_BYTES,
    MIN_PROTECTED_EXCERPT_BYTES,
    RAW_FINGERPRINT_STRIDE,
    'edges',
  );
  const normalizedIndex = buildRawFingerprintIndex([...normalizedTextBuffers.values()], MIN_NORMALIZED_TEXT_BYTES);
  const locators = buildLocators(locatorTexts);
  return ({ path, bytes }): void => {
    inspectCandidate(
      { path, bytes: typeof bytes === 'string' ? Buffer.from(bytes, 'utf8') : Buffer.from(bytes), surface: 'worktree' },
      [privateHashes],
      [rawIndex],
      [normalizedIndex],
      [semanticIndex],
      locators,
      { candidateBytes: 0, collisionComparisons: 0, decodedBytes: 0, decodedStrings: 0 },
    );
  };
}

function confinedPrivateFilePath(root: string, relativePath: string): string {
  const rootReal = realpathSync(root);
  const candidate = confinedPath(rootReal, join(...relativePath.split('/')), 'Private source path');
  const metadata = lstatSync(candidate);
  if (metadata.isSymbolicLink()) {
    throw new BoundaryViolation('Private source path is a symbolic link.');
  }
  if (!metadata.isFile()) throw new BoundaryViolation('Private source path is not a file.');
  const real = realpathSync(candidate);
  confinedPath(rootReal, real, 'Private source real path');
  return real;
}

function readConfinedPrivateFile(root: string, relativePath: string): Buffer {
  return readFileSync(confinedPrivateFilePath(root, relativePath));
}

function readPrivateLock(lockPath: string): PrivateLock {
  const lock = JSON.parse(readFileSync(lockPath, 'utf8')) as PrivateLock;
  if (typeof lock.primaryRoot !== 'string' || typeof lock.backupRoot !== 'string' || !Array.isArray(lock.entries)) {
    throw new BoundaryViolation('Private lock is missing required boundary metadata.');
  }
  return lock;
}

function fingerprintAllocationEstimate(
  byteLengths: readonly number[],
  fingerprintBytes: number,
  stride: number,
): number {
  const privateBytes = byteLengths.reduce((sum, bytes) => sum + bytes, 0);
  const fingerprints = byteLengths.reduce(
    (sum, bytes) => sum + fingerprintWindowCount(bytes, fingerprintBytes, stride),
    0,
  );
  if (
    !Number.isSafeInteger(privateBytes) ||
    !Number.isSafeInteger(fingerprints) ||
    privateBytes > MAX_PRIVATE_BYTES ||
    fingerprints > MAX_PRIVATE_FINGERPRINTS
  ) {
    throw new BoundaryViolation('Private fingerprint work exceeded the fixed limit.');
  }
  return fingerprints * BigUint64Array.BYTES_PER_ELEMENT +
    fingerprintBloomBytes(fingerprints) +
    (byteLengths.length + 1) * Float64Array.BYTES_PER_ELEMENT;
}

function privatePreparationEstimate(repositoryRoot: string, lockPath: string): number {
  const lock = readPrivateLock(lockPath);
  const primaryRoot = isAbsolute(lock.primaryRoot) ? lock.primaryRoot : resolve(repositoryRoot, lock.primaryRoot);
  const backupRoot = isAbsolute(lock.backupRoot) ? lock.backupRoot : resolve(repositoryRoot, lock.backupRoot);
  const primaryBytes: number[] = [];
  let htmlSources = 0;
  const budget: PreparationBudget = { reservedBytes: 0 };
  for (const entry of lock.entries) {
    if (typeof entry.relativePath !== 'string' || typeof entry.byteHash !== 'string') {
      throw new BoundaryViolation('Private lock contains an invalid source entry.');
    }
    const primarySize = lstatSync(confinedPrivateFilePath(primaryRoot, entry.relativePath)).size;
    const backupSize = lstatSync(confinedPrivateFilePath(backupRoot, entry.relativePath)).size;
    reservePrivatePreparationBytes(budget, primarySize + backupSize);
    primaryBytes.push(primarySize);
    if (entry.relativePath.endsWith('.html')) htmlSources += 1;
  }
  reservePrivatePreparationBytes(
    budget,
    fingerprintAllocationEstimate(primaryBytes, RAW_FINGERPRINT_BYTES, RAW_FINGERPRINT_STRIDE),
  );
  reservePrivatePreparationBytes(
    budget,
    MAX_PRIVATE_NORMALIZED_HTML_BYTES,
  );
  reservePrivatePreparationBytes(
    budget,
    fingerprintAllocationEstimate(
      Array.from(
        { length: Math.max(1, htmlSources) },
        (_, index) => index === 0 ? MAX_PRIVATE_NORMALIZED_HTML_BYTES : 0,
      ),
      MIN_NORMALIZED_TEXT_BYTES,
      1,
    ),
  );
  reservePrivatePreparationBytes(budget, MAX_PRIVATE_SEMANTIC_BYTES);
  return budget.reservedBytes;
}

function addSelectedRevisionEvidence(
  repositoryRoot: string,
  lockPath: string,
  privateHashes: Set<string>,
  semanticIndex: SemanticIndex,
  retentionBudget: PrivateRetentionBudget,
): void {
  const revisionId = basename(dirname(lockPath));
  if (!/^[a-z0-9][a-z0-9._-]{0,99}$/.test(revisionId)) return;
  const revisionRoot = resolve(repositoryRoot, '.local', 'authority', 'revisions', revisionId);
  if (!existsSync(revisionRoot)) return;
  const revisionReal = realpathSync(revisionRoot);
  for (const entry of readdirSync(revisionReal, { recursive: true, withFileTypes: true })) {
    if (entry.isSymbolicLink()) throw new BoundaryViolation('Private revision contains a symbolic link.');
    if (!entry.isFile()) continue;
    const absolute = confinedPath(revisionReal, join(entry.parentPath, entry.name), 'Private revision path');
    const bytes = readFileSync(absolute);
    const hash = sha256Hex(bytes);
    privateHashes.add(hash);
    const path = relative(revisionReal, absolute).replaceAll('\\', '/');
    if (path === 'cards.normalized.json') {
      addPrivateJsonSemantics(bytes, semanticIndex, retentionBudget);
    }
  }
}

function lowercaseAscii(bytes: Buffer): Buffer {
  const folded = Buffer.from(bytes);
  for (let index = 0; index < folded.length; index += 1) {
    if (folded[index]! >= 0x41 && folded[index]! <= 0x5a) folded[index]! += 0x20;
  }
  return folded;
}

function matchesLocator(bytes: Buffer, locators: readonly Locator[]): boolean {
  const folded = locators.some(({ caseInsensitive }) => caseInsensitive)
    ? lowercaseAscii(bytes)
    : null;
  return locators.some(({ bytes: locator, caseInsensitive }) =>
    (caseInsensitive ? folded! : bytes).includes(locator),
  );
}

function buildLocators(values: readonly string[]): readonly Locator[] {
  const locators = new Map<string, Locator>();
  for (const value of values) {
    const caseInsensitive = /^[a-z]:[\\/]/i.test(value) || /^(?:\\\\|\/\/)[^\\/]/.test(value);
    for (const form of new Set([
      value,
      value.replaceAll('\\', '/'),
      value.replaceAll('/', '\\'),
    ])) {
      const json = JSON.stringify(form);
      const representations = [
        form,
        json,
        "'" + json.slice(1, -1).replaceAll("'", "\\'") + "'",
      ];
      for (const representation of representations) {
        const bytes = Buffer.from(representation, 'utf8');
        const locator = caseInsensitive ? lowercaseAscii(bytes) : bytes;
        locators.set(String(caseInsensitive) + ':' + locator.toString('hex'), {
          bytes: locator,
          caseInsensitive,
        });
      }
    }
  }
  return [...locators.values()];
}

async function preparePrivateEvidence(repositoryRoot: string, lockPath: string): Promise<PrivateEvidence> {
  const privateHashes = new Set<string>();
  const rawFingerprintBuffers = new Map<string, Buffer>();
  const normalizedTextBuffers = new Map<string, Buffer>();
  const semanticIndex: SemanticIndex = new Map();
  const retentionBudget: PrivateRetentionBudget = { normalizedBytes: 0, semanticBytes: 0 };
  const locatorTexts: string[] = [];
  const localFileEvidence = ['user-provided', 'manual-local-file'].join('-');
  const lock = readPrivateLock(lockPath);
  const verified = await verifyPrivateSourceSet({
    repositoryRoot,
    primaryRoot: lock.primaryRoot,
    backupRoot: lock.backupRoot,
    entries: lock.entries,
  });
  if (verified.sourceSetRootHash !== lock.sourceSetRootHash) {
    throw new BoundaryViolation('Private lock source-set root does not match verified evidence.');
  }
  for (const [rootIndex, root] of [lock.primaryRoot, lock.backupRoot].entries()) {
    for (const entry of verified.entries) {
      if (typeof entry.relativePath !== 'string' || typeof entry.byteHash !== 'string') {
        throw new BoundaryViolation('Private lock contains an invalid source entry.');
      }
      const bytes = readConfinedPrivateFile(root, entry.relativePath);
      const hash = sha256Hex(bytes);
      if (entry.byteHash !== 'sha256:' + hash) {
        throw new BoundaryViolation('Private lock source hash does not match its bytes.');
      }
      privateHashes.add(hash);
      if (rootIndex === 0) addRawFingerprintSegments(bytes, rawFingerprintBuffers);
      if (rootIndex === 0 && entry.relativePath.endsWith('.json')) {
        addPrivateJsonSemantics(bytes, semanticIndex, retentionBudget);
      }
      if (rootIndex === 0 && entry.relativePath.endsWith('.html')) {
        addPrivateNormalizedHtml(bytes, normalizedTextBuffers, retentionBudget);
      }
    }
  }
  addSelectedRevisionEvidence(repositoryRoot, lockPath, privateHashes, semanticIndex, retentionBudget);
  const privateLocatorEvidence = lock.rulebookAcquisitionEvidence?.privateLocatorEvidence;
  locatorTexts.push(lock.primaryRoot, lock.backupRoot);
  if (typeof privateLocatorEvidence === 'string' && privateLocatorEvidence.length > 0 && privateLocatorEvidence !== localFileEvidence) {
    locatorTexts.push(privateLocatorEvidence);
  }

  return {
    privateHashes,
    rawIndex: buildRawFingerprintIndex(
      [...rawFingerprintBuffers.values()],
      RAW_FINGERPRINT_BYTES,
      MIN_PROTECTED_EXCERPT_BYTES,
      RAW_FINGERPRINT_STRIDE,
      'edges',
    ),
    normalizedIndex: buildRawFingerprintIndex([...normalizedTextBuffers.values()], MIN_NORMALIZED_TEXT_BYTES),
    semanticIndex,
    locators: buildLocators(locatorTexts),
    state: { candidateBytes: 0, collisionComparisons: 0, decodedBytes: 0, decodedStrings: 0 },
  };
}

async function main(): Promise<void> {
  const { repositoryRoot, lockPaths } = parseArguments(process.argv.slice(2));
  const preparationBudget: PreparationBudget = { reservedBytes: 0 };
  for (const lockPath of lockPaths) {
    reservePrivatePreparationBytes(
      preparationBudget,
      privatePreparationEstimate(repositoryRoot, lockPath),
    );
  }
  const evidenceSets: PrivateEvidence[] = [];
  for (const lockPath of lockPaths) evidenceSets.push(await preparePrivateEvidence(repositoryRoot, lockPath));
  const privateHashes = evidenceSets.map((evidence) => evidence.privateHashes);
  const rawIndexes = evidenceSets.map((evidence) => evidence.rawIndex);
  const normalizedIndexes = evidenceSets.map((evidence) => evidence.normalizedIndex);
  const semanticIndexes = evidenceSets.map((evidence) => evidence.semanticIndex);
  const locators = evidenceSets.flatMap((evidence) => evidence.locators);
  const state = evidenceSets[0]!.state;
  for (const enumerate of [
    reachableHistoryCandidates,
    indexCandidates,
    packageCandidates,
    worktreeCandidates,
  ] as const) {
    let candidates: readonly Candidate[];
    try {
      candidates = enumerate(repositoryRoot);
    } catch (error) {
      if (error instanceof BoundaryViolation) throw error;
      throw new BoundaryViolation(`Candidate enumeration failed [${enumerate.name}].`);
    }
    for (const candidate of candidates) {
      inspectCandidate(
        candidate,
        privateHashes,
        rawIndexes,
        normalizedIndexes,
        semanticIndexes,
        locators,
        state,
      );
    }
  }
  process.stdout.write('Private authority boundary verified.\n');
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url))) {
  try {
    await main();
  } catch (error) {
    process.stderr.write(
      `${error instanceof BoundaryViolation ? error.message : 'Private authority boundary verification failed.'}\n`,
    );
    process.exitCode = 1;
  }
}
