import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { canonicalJson } from '../authority/canonical-json.ts';
import {
  AuthorityValidationError,
  sortDiagnostics,
  type Diagnostic,
  type Hash,
} from '../authority/schemas.ts';
import { validateAuthorityBundle } from '../authority/validate-bundle.ts';

const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
const STABLE_ID_PATTERN = /^[a-z][a-z0-9-]*(?::[a-z0-9][a-z0-9._-]*)+$/;

type ValidateAuthorityArguments = Readonly<{
  root: string;
  bundlePath: string;
  stableId: string;
  contentHash: Hash;
}>;

type CommandIo = Readonly<{
  stdout: (line: string) => void;
  stderr: (line: string) => void;
}>;

function fail(path: string, code: string, message: string): never {
  throw new AuthorityValidationError([{ path, code, message }]);
}

const VALIDATE_FLAGS = new Map([
  ['--root', 'root'],
  ['--bundle', 'bundlePath'],
  ['--id', 'stableId'],
  ['--hash', 'contentHash'],
] as const);

type ValidateFlag = '--root' | '--bundle' | '--id' | '--hash';

function parseArguments(argv: readonly string[]): ValidateAuthorityArguments {
  if (argv.length !== VALIDATE_FLAGS.size * 2) {
    fail('/arguments', 'invalid_arguments', 'provide each documented validation argument exactly once');
  }
  const parsed = new Map<string, string>();
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index]!;
    const key = VALIDATE_FLAGS.get(flag as ValidateFlag);
    const value = argv[index + 1];
    if (key === undefined || value === undefined || value.length === 0 || parsed.has(key)) {
      fail('/arguments', 'invalid_arguments', 'provide each documented validation argument exactly once');
    }
    parsed.set(key, value);
  }
  const stableId = parsed.get('stableId')!;
  const contentHash = parsed.get('contentHash')!;
  if (!STABLE_ID_PATTERN.test(stableId)) {
    fail('/id', 'invalid_stable_id', 'id must be a documented stable artifact ID');
  }
  if (!HASH_PATTERN.test(contentHash)) {
    fail('/hash', 'invalid_format', 'hash must be a lowercase SHA-256 digest');
  }
  return {
    root: parsed.get('root')!,
    bundlePath: parsed.get('bundlePath')!,
    stableId,
    contentHash: contentHash as Hash,
  };
}

const defaultIo: CommandIo = {
  stdout: (line) => process.stdout.write(line + '\n'),
  stderr: (line) => process.stderr.write(line + '\n'),
};

export async function runValidateAuthorityCommand(
  argv: readonly string[],
  io: CommandIo = defaultIo,
): Promise<number> {
  try {
    const args = parseArguments(argv);
    const validated = await validateAuthorityBundle(
      args.root,
      args.bundlePath,
      { stableId: args.stableId, contentHash: args.contentHash },
    );
    const evidence = [
      ...validated.storedBytesRehashed.map((source) => ({
        ...source,
        verification: 'stored-bytes-rehashed',
      })),
      ...validated.manifestByteBindingsVerified.map((source) => ({
        ...source,
        verification: 'manifest-byte-binding-verified',
      })),
      ...validated.manifestDeclarationsBound.map((source) => ({
        ...source,
        verification: 'manifest-declaration-bound',
      })),
    ].sort((left, right) =>
      left.sourceId < right.sourceId
        ? -1
        : left.sourceId > right.sourceId
          ? 1
          : left.verification < right.verification
            ? -1
            : left.verification > right.verification
              ? 1
              : 0,
    );
    io.stdout(canonicalJson({
      status: 'valid',
      bundleId: validated.bundle.identity.stableId,
      bundleRootHash: validated.bundle.contentHash,
      evidence,
    }));
    return 0;
  } catch (error: unknown) {
    const diagnostics: readonly Diagnostic[] = error instanceof AuthorityValidationError
      ? error.diagnostics
      : [{ path: '', code: 'internal_error', message: 'authority validation failed' }];
    for (const diagnostic of sortDiagnostics(diagnostics)) io.stderr(canonicalJson(diagnostic));
    return 1;
  }
}

if (process.argv[1] !== undefined && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  process.exitCode = await runValidateAuthorityCommand(process.argv.slice(2));
}
