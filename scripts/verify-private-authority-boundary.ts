import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { isAbsolute, relative, resolve, sep } from 'node:path';

type PrivateEntry = Readonly<{ relativePath: string; byteHash: string }>;
type PrivateLock = Readonly<{
  primaryRoot: string;
  backupRoot: string;
  entries: readonly PrivateEntry[];
  rulebookAcquisitionEvidence?: Readonly<{ privateLocatorEvidence?: string }>;
}>;
type Candidate = Readonly<{ path: string; bytes: Buffer }>;
type PackResult = Readonly<{ files?: readonly Readonly<{ path?: string }>[] }>;

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
  return execFileSync(command, arguments_, { cwd, encoding: 'buffer', maxBuffer: 1_073_741_824 });
}

function gitCandidates(repositoryRoot: string): readonly Candidate[] {
  const candidates: Candidate[] = [];
  const head = commandBytes('git', ['ls-tree', '-r', '-z', '--full-tree', 'HEAD'], repositoryRoot).toString('utf8');
  for (const record of head.split('\0')) {
    if (record === '') continue;
    const match = /^\d+ blob ([0-9a-f]+)\t([\s\S]+)$/.exec(record);
    if (match === null) throw new BoundaryViolation('Could not parse a Git HEAD blob record.');
    candidates.push({ path: match[2]!, bytes: commandBytes('git', ['cat-file', 'blob', match[1]!], repositoryRoot) });
  }
  const index = commandBytes('git', ['ls-files', '--stage', '-z'], repositoryRoot).toString('utf8');
  for (const record of index.split('\0')) {
    if (record === '') continue;
    const match = /^\d+ ([0-9a-f]+) \d+\t([\s\S]+)$/.exec(record);
    if (match === null) throw new BoundaryViolation('Could not parse a Git index blob record.');
    candidates.push({ path: match[2]!, bytes: commandBytes('git', ['cat-file', 'blob', match[1]!], repositoryRoot) });
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
      maxBuffer: 1_073_741_824,
    }),
  ) as PackResult | readonly PackResult[];
  const results = Array.isArray(parsed) ? parsed : [parsed];
  const candidates: Candidate[] = [];
  for (const result of results) {
    if (!Array.isArray(result.files)) throw new BoundaryViolation('Could not parse pnpm dry-run package files.');
    for (const file of result.files) {
      if (typeof file.path !== 'string') throw new BoundaryViolation('Could not parse a pnpm dry-run package path.');
      const path = resolve(repositoryRoot, file.path);
      const outside = relative(repositoryRoot, path);
      if (outside === '..' || outside.startsWith(`..${sep}`) || isAbsolute(outside)) {
        throw new BoundaryViolation('pnpm dry-run returned a path outside the repository.');
      }
      candidates.push({ path: file.path, bytes: readFileSync(path) });
    }
  }
  return candidates;
}

function sourceMarkers(bytes: Buffer): readonly Buffer[] {
  if (bytes.length < 32) return [];
  const offsets = [0, Math.floor((bytes.length - 32) / 2), bytes.length - 32];
  return [...new Set(offsets)].map((offset) => bytes.subarray(offset, offset + 32));
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

function fail(category: string, path: string): never {
  throw new BoundaryViolation(`Private authority boundary violation [${category}]: ${path}`);
}

function inspectCandidate(
  candidate: Candidate,
  privateHashes: ReadonlySet<string>,
  markers: readonly Buffer[],
  locators: readonly Buffer[],
): void {
  const normalizedPath = candidate.path.replaceAll('\\', '/');
  if (normalizedPath === '.local/authority' || normalizedPath.startsWith('.local/authority/')) {
    fail('forbidden-private-path', candidate.path);
  }
  if (/\.(?:avif|bmp|gif|ico|jpe?g|png|svg|tiff?|webp)$/i.test(normalizedPath)) {
    fail('artwork-path', candidate.path);
  }
  if (hasArtworkSignature(candidate.bytes)) fail('artwork-signature', candidate.path);
  const hash = createHash('sha256').update(candidate.bytes).digest('hex');
  if (privateHashes.has(hash)) fail('exact-private-bytes', candidate.path);
  if (markers.some((marker) => candidate.bytes.includes(marker))) fail('source-derived-marker', candidate.path);
  if (locators.some((locator) => candidate.bytes.includes(locator))) fail('private-locator', candidate.path);
}

function main(): void {
  const { repositoryRoot, lockPath } = parseArguments(process.argv.slice(2));
  const lock = JSON.parse(readFileSync(lockPath, 'utf8')) as PrivateLock;
  if (typeof lock.primaryRoot !== 'string' || typeof lock.backupRoot !== 'string' || !Array.isArray(lock.entries)) {
    throw new BoundaryViolation('Private lock is missing required boundary metadata.');
  }

  const privateHashes = new Set<string>();
  const markers: Buffer[] = [];
  for (const entry of lock.entries) {
    if (typeof entry.relativePath !== 'string' || typeof entry.byteHash !== 'string') {
      throw new BoundaryViolation('Private lock contains an invalid source entry.');
    }
    const bytes = readFileSync(resolve(lock.primaryRoot, ...entry.relativePath.split('/')));
    const hash = createHash('sha256').update(bytes).digest('hex');
    if (entry.byteHash !== `sha256:${hash}`) throw new BoundaryViolation('Private lock source hash does not match its bytes.');
    privateHashes.add(hash);
    markers.push(...sourceMarkers(bytes));
  }
  const locatorTexts = [
    lock.primaryRoot,
    lock.backupRoot,
    lock.rulebookAcquisitionEvidence?.privateLocatorEvidence,
  ].filter((value): value is string => typeof value === 'string' && value.length > 0);
  const locators = [...new Set(locatorTexts.flatMap((value) => [value, value.replaceAll('\\', '/')]))].map((value) =>
    Buffer.from(value, 'utf8'),
  );

  for (const candidate of [...gitCandidates(repositoryRoot), ...packageCandidates(repositoryRoot)]) {
    inspectCandidate(candidate, privateHashes, markers, locators);
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
