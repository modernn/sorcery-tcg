import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, readFileSync, readdirSync, realpathSync } from 'node:fs';
import { basename, dirname, extname, isAbsolute, join, relative, resolve, sep } from 'node:path';

import {
  CanonicalJsonError,
  canonicalJson,
  parseJsonWithDuplicateKeyCheck,
  type JsonValue,
} from '../src/authority/canonical-json.ts';

type PrivateEntry = Readonly<{
  relativePath: string;
  byteHash: string;
  url?: string;
  retrievedAt?: string;
  effectiveDate?: string | null;
  mediaType?: string;
}>;
type PrivateLock = Readonly<{
  primaryRoot: string;
  backupRoot: string;
  entries: readonly PrivateEntry[];
  rulebookAcquisitionEvidence?: Readonly<{ privateLocatorEvidence?: string }>;
}>;
type CandidateSurface = 'reachable-history' | 'worktree' | 'index' | 'package';
type Candidate = Readonly<{ path: string; bytes: Buffer; surface: CandidateSurface }>;
type PackResult = Readonly<{ files?: readonly Readonly<{ path?: string }>[] }>;
type Locator = Readonly<{ bytes: Buffer; caseInsensitive: boolean }>;
type SemanticIndex = Map<string, string[]>;
type RawFingerprintIndex = Readonly<{
  fingerprints: BigUint64Array;
  sources: readonly Buffer[];
  minimumBytes: number;
}>;
type InspectionState = {
  candidateBytes: number;
  decodedBytes: number;
  decodedStrings: number;
};

const MIN_PROTECTED_EXCERPT_BYTES = 32;
const MIN_NORMALIZED_TEXT_BYTES = 96;
const MIN_SEMANTIC_RECORD_BYTES = 96;
const MAX_PRIVATE_BYTES = 128_000_000;
const MAX_PRIVATE_FINGERPRINTS = 128_000_000;
const MAX_CANDIDATE_BYTES = 64_000_000;
const MAX_CANDIDATE_WORK_BYTES = 1_073_741_824;
const MAX_DECODED_BYTES = 64_000_000;
const MAX_DECODED_STRINGS = 50_000;
const MAX_DECODE_DEPTH = 4;
const MAX_SEMANTIC_SUBTREES = 200_000;
const MAX_COMMAND_BUFFER = 268_435_456;
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

function parseArguments(arguments_: readonly string[]): Readonly<{ repositoryRoot: string; lockPath: string }> {
  let repositoryRoot: string | undefined;
  let lockPath: string | undefined;
  for (let index = 0; index < arguments_.length; index += 2) {
    const flag = arguments_[index];
    const value = arguments_[index + 1];
    if (value === undefined || (flag !== '--repository-root' && flag !== '--lock')) {
      throw new BoundaryViolation('Usage: --repository-root <path> --lock <path>');
    }
    if (flag === '--repository-root') repositoryRoot = value;
    else lockPath = value;
  }
  if (repositoryRoot === undefined || lockPath === undefined) {
    throw new BoundaryViolation('Usage: --repository-root <path> --lock <path>');
  }
  const root = resolve(repositoryRoot);
  return { repositoryRoot: root, lockPath: isAbsolute(lockPath) ? resolve(lockPath) : resolve(root, lockPath) };
}

function commandBytes(command: string, arguments_: readonly string[], cwd: string): Buffer {
  return execFileSync(command, arguments_, {
    cwd,
    encoding: 'buffer',
    maxBuffer: MAX_COMMAND_BUFFER,
    windowsHide: true,
  });
}

function reachableHistoryCandidates(repositoryRoot: string): readonly Candidate[] {
  const candidates: Candidate[] = [];
  const objects = commandBytes('git', ['rev-list', '--objects', '--all'], repositoryRoot).toString('utf8');
  for (const record of objects.split(/\r?\n/)) {
    if (record === '') continue;
    const separator = record.indexOf(' ');
    if (separator < 0) continue;
    const objectId = record.slice(0, separator);
    const path = record.slice(separator + 1);
    if (commandBytes('git', ['cat-file', '-t', objectId], repositoryRoot).toString('utf8').trim() !== 'blob') continue;
    candidates.push({
      path,
      bytes: commandBytes('git', ['cat-file', 'blob', objectId], repositoryRoot),
      surface: 'reachable-history',
    });
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

function power(base: number, exponent: number): number {
  let value = 1;
  for (let index = 0; index < exponent; index += 1) value = Math.imul(value, base) >>> 0;
  return value;
}

function windowFingerprints(
  bytes: Buffer,
  minimumBytes: number,
  visit: (fingerprint: bigint, offset: number) => void,
): void {
  if (bytes.length < minimumBytes) return;
  const windowPowerA = power(HASH_BASE_A, minimumBytes - 1);
  const windowPowerB = power(HASH_BASE_B, minimumBytes - 1);
  let first = 0;
  let second = 0;
  for (let index = 0; index < minimumBytes; index += 1) {
    first = multiplyAdd(first, HASH_BASE_A, bytes[index]!);
    second = multiplyAdd(second, HASH_BASE_B, bytes[index]!);
  }
  visit((BigInt(first) << 32n) | BigInt(second), 0);
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
    visit((BigInt(first) << 32n) | BigInt(second), offset);
  }
}

function normalizedVisibleText(bytes: Buffer): Buffer | null {
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
  return Buffer.from(decoded, 'utf8');
}

function buildRawFingerprintIndex(
  privateBuffers: readonly Buffer[],
  minimumBytes: number,
): RawFingerprintIndex {
  let fingerprintCount = 0;
  let privateBytes = 0;
  for (const bytes of privateBuffers) {
    privateBytes += bytes.length;
    fingerprintCount += Math.max(0, bytes.length - minimumBytes + 1);
  }
  if (privateBytes > MAX_PRIVATE_BYTES || fingerprintCount > MAX_PRIVATE_FINGERPRINTS) {
    throw new BoundaryViolation('Private fingerprint work exceeded the fixed limit.');
  }
  const fingerprints = new BigUint64Array(fingerprintCount);
  let index = 0;
  for (const bytes of privateBuffers) {
    windowFingerprints(bytes, minimumBytes, (fingerprint) => {
      fingerprints[index] = fingerprint;
      index += 1;
    });
  }
  const protectedFingerprints = fingerprints.subarray(0, index);
  protectedFingerprints.sort();
  return { fingerprints: protectedFingerprints, sources: privateBuffers, minimumBytes };
}

function addRawFingerprintSegments(
  bytes: Buffer,
  segments: Map<string, Buffer>,
): void {
  if (bytes.length >= MIN_PROTECTED_EXCERPT_BYTES) {
    segments.set(sha256Hex(bytes), bytes);
  }
}

function containsFingerprint(fingerprints: BigUint64Array, fingerprint: bigint): boolean {
  let low = 0;
  let high = fingerprints.length;
  while (low < high) {
    const middle = low + Math.floor((high - low) / 2);
    const value = fingerprints[middle]!;
    if (value < fingerprint) low = middle + 1;
    else high = middle;
  }
  return low < fingerprints.length && fingerprints[low] === fingerprint;
}

function containsProtectedWindow(bytes: Buffer, index: RawFingerprintIndex): boolean {
  let matched = false;
  windowFingerprints(bytes, index.minimumBytes, (fingerprint, offset) => {
    if (matched || !containsFingerprint(index.fingerprints, fingerprint)) return;
    const window = bytes.subarray(offset, offset + index.minimumBytes);
    matched = index.sources.some((source) => source.includes(window));
  });
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

function containsProtectedContent(bytes: Buffer, index: RawFingerprintIndex): boolean {
  if (PUBLIC_PROVENANCE_VALUE_HASHES.has(sha256Hex(bytes))) return false;
  return publicProvenanceFreeSegments(bytes).some((segment) => containsProtectedWindow(segment, index));
}

function semanticFingerprint(canonical: string): string {
  return sha256Hex(Buffer.from(canonical, 'utf8'));
}

function addSemanticValue(
  value: JsonValue,
  semanticIndex: SemanticIndex,
  work: { subtrees: number },
): void {
  if (value === null || typeof value !== 'object') return;
  work.subtrees += 1;
  if (work.subtrees > MAX_SEMANTIC_SUBTREES) {
    throw new BoundaryViolation('Private semantic fingerprint work exceeded the fixed limit.');
  }
  const canonical = canonicalJson(value);
  if (Buffer.byteLength(canonical, 'utf8') >= MIN_SEMANTIC_RECORD_BYTES) {
    const fingerprint = semanticFingerprint(canonical);
    const matches = semanticIndex.get(fingerprint) ?? [];
    if (!matches.includes(canonical)) matches.push(canonical);
    semanticIndex.set(fingerprint, matches);
  }
  if (Array.isArray(value)) {
    for (const child of value) addSemanticValue(child, semanticIndex, work);
  } else {
    for (const child of Object.values(value)) addSemanticValue(child, semanticIndex, work);
  }
}

function addPrivateJsonSemantics(bytes: Buffer, semanticIndex: SemanticIndex): void {
  let value: JsonValue;
  try {
    value = parseJsonWithDuplicateKeyCheck(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    throw new BoundaryViolation('Private JSON could not be fingerprinted safely.');
  }
  addSemanticValue(value, semanticIndex, { subtrees: 0 });
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
  if (quote !== "'" && quote !== '"') return null;
  let value = '';
  for (let index = start + 1; index < text.length; index += 1) {
    const character = text[index]!;
    if (character === quote) return { value, end: index + 1 };
    if (character === '\n' || character === '\r') return null;
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

function fail(category: string, candidate: Candidate): never {
  throw new BoundaryViolation(
    'Private authority boundary violation [' +
      category +
      '] [' +
      candidate.surface +
      ']: ' +
      candidate.path,
  );
}

function inspectContent(
  candidate: Candidate,
  bytes: Buffer,
  privateHashes: ReadonlySet<string>,
  rawIndex: RawFingerprintIndex,
  normalizedIndex: RawFingerprintIndex,
  semanticIndex: SemanticIndex,
  locators: readonly Locator[],
  depth: number,
  state: InspectionState,
): void {
  if (privateHashes.has(sha256Hex(bytes))) fail('exact-private-bytes', candidate);
  const normalized = normalizedVisibleText(bytes);
  if (
    normalized !== null &&
    normalized.length >= normalizedIndex.minimumBytes &&
    containsProtectedWindow(normalized, normalizedIndex)
  ) {
    fail('source-derived-normalized-text', candidate);
  }
  if (containsProtectedContent(bytes, rawIndex)) fail('source-derived-content', candidate);
  if (inspectJsonSemantics(bytes, semanticIndex)) fail('semantic-private-content', candidate);
  if (matchesLocator(bytes, locators)) fail('private-locator', candidate);

  const prefix = bytes.subarray(0, 512).toString('utf8');
  if (bytes.length > 1_000 && /^(?:%PDF-|\s*<!doctype html|\s*<html\b)/i.test(prefix)) {
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
    if (text[index] !== "'" && text[index] !== '"') continue;
    const decoded = decodeStringLiteral(text, index);
    if (decoded === null) continue;
    state.decodedStrings += 1;
    if (state.decodedStrings > MAX_DECODED_STRINGS) fail('inspection-work-limit', candidate);
    const decodedBytes = Buffer.from(decoded.value, 'utf8');
    state.decodedBytes += decodedBytes.length;
    if (state.decodedBytes > MAX_DECODED_BYTES) fail('inspection-work-limit', candidate);
    if (decodedBytes.length > 0) {
      inspectContent(
        candidate,
        decodedBytes,
        privateHashes,
        rawIndex,
        normalizedIndex,
        semanticIndex,
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
  privateHashes: ReadonlySet<string>,
  rawIndex: RawFingerprintIndex,
  normalizedIndex: RawFingerprintIndex,
  semanticIndex: SemanticIndex,
  locators: readonly Locator[],
  state: InspectionState,
): void {
  if (candidate.bytes.length > MAX_CANDIDATE_BYTES) fail('inspection-work-limit', candidate);
  state.candidateBytes += candidate.bytes.length;
  if (state.candidateBytes > MAX_CANDIDATE_WORK_BYTES) fail('inspection-work-limit', candidate);
  const normalizedPath = candidate.path.replaceAll('\\', '/');
  if (normalizedPath === '.local/authority' || normalizedPath.startsWith('.local/authority/')) {
    fail('forbidden-private-path', candidate);
  }
  if (/\.(?:avif|bmp|gif|ico|jpe?g|png|svg|tiff?|webp)$/i.test(normalizedPath)) {
    fail('artwork-path', candidate);
  }
  if (hasArtworkSignature(candidate.bytes)) fail('artwork-signature', candidate);
  inspectContent(
    candidate,
    candidate.bytes,
    privateHashes,
    rawIndex,
    normalizedIndex,
    semanticIndex,
    locators,
    0,
    state,
  );
}

function readConfinedPrivateFile(root: string, relativePath: string): Buffer {
  const rootReal = realpathSync(root);
  const candidate = confinedPath(rootReal, join(...relativePath.split('/')), 'Private source path');
  if (lstatSync(candidate).isSymbolicLink()) {
    throw new BoundaryViolation('Private source path is a symbolic link.');
  }
  const real = realpathSync(candidate);
  confinedPath(rootReal, real, 'Private source real path');
  return readFileSync(real);
}

function addSelectedRevisionEvidence(
  repositoryRoot: string,
  lockPath: string,
  privateHashes: Set<string>,
  semanticIndex: SemanticIndex,
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
      addPrivateJsonSemantics(bytes, semanticIndex);
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

function main(): void {
  const { repositoryRoot, lockPath } = parseArguments(process.argv.slice(2));
  const lock = JSON.parse(readFileSync(lockPath, 'utf8')) as PrivateLock;
  if (typeof lock.primaryRoot !== 'string' || typeof lock.backupRoot !== 'string' || !Array.isArray(lock.entries)) {
    throw new BoundaryViolation('Private lock is missing required boundary metadata.');
  }

  const privateHashes = new Set<string>();
  const rawFingerprintBuffers = new Map<string, Buffer>();
  const normalizedTextBuffers = new Map<string, Buffer>();
  const semanticIndex: SemanticIndex = new Map();
  for (const [rootIndex, root] of [lock.primaryRoot, lock.backupRoot].entries()) {
    for (const entry of lock.entries) {
      if (typeof entry.relativePath !== 'string' || typeof entry.byteHash !== 'string') {
        throw new BoundaryViolation('Private lock contains an invalid source entry.');
      }
      const bytes = readConfinedPrivateFile(root, entry.relativePath);
      const hash = sha256Hex(bytes);
      if (entry.byteHash !== 'sha256:' + hash) {
        throw new BoundaryViolation('Private lock source hash does not match its bytes.');
      }
      privateHashes.add(hash);
      if (rootIndex === 0) {
        addRawFingerprintSegments(bytes, rawFingerprintBuffers);
      }
      if (rootIndex === 0 && entry.relativePath.endsWith('.json')) {
        addPrivateJsonSemantics(bytes, semanticIndex);
      }
      if (rootIndex === 0 && entry.relativePath.endsWith('.html')) {
        const normalized = normalizedVisibleText(bytes);
        if (normalized !== null && normalized.length >= MIN_PROTECTED_EXCERPT_BYTES) {
          normalizedTextBuffers.set(sha256Hex(normalized), normalized);
        }
      }
    }
  }
  addSelectedRevisionEvidence(repositoryRoot, lockPath, privateHashes, semanticIndex);

  const rawIndex = buildRawFingerprintIndex([...rawFingerprintBuffers.values()], MIN_PROTECTED_EXCERPT_BYTES);
  const normalizedIndex = buildRawFingerprintIndex([...normalizedTextBuffers.values()], MIN_NORMALIZED_TEXT_BYTES);
  const locatorTexts = [
    lock.primaryRoot,
    lock.backupRoot,
    lock.rulebookAcquisitionEvidence?.privateLocatorEvidence,
  ].filter((value): value is string => typeof value === 'string' && value.length > 0);
  const locators = buildLocators(locatorTexts);
  const state: InspectionState = { candidateBytes: 0, decodedBytes: 0, decodedStrings: 0 };
  for (const enumerate of [
    reachableHistoryCandidates,
    indexCandidates,
    packageCandidates,
    worktreeCandidates,
  ] as const) {
    for (const candidate of enumerate(repositoryRoot)) {
      inspectCandidate(candidate, privateHashes, rawIndex, normalizedIndex, semanticIndex, locators, state);
    }
  }
  process.stdout.write('Private authority boundary verified.\n');
}

try {
  main();
} catch (error) {
  process.stderr.write(
    `${error instanceof BoundaryViolation ? error.message : 'Private authority boundary verification failed.'}\n`,
  );
  process.exitCode = 1;
}
