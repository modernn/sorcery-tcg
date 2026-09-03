import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import {
  canonicalJson,
  parseJsonWithDuplicateKeyCheck,
  type JsonValue,
} from '../authority/canonical-json.ts';

const REPOSITORY_ROOT = fileURLToPath(new URL('../..', import.meta.url));
const MAX_OUTPUT_BYTES = 1_048_576;

export type RustDeterministicGameReport = Readonly<{
  acceptedActionCount: number;
  classification: 'unranked_partial_rules_unverified_authority';
  fightCount: number;
  finalStateHash: string;
  replayVerified: boolean;
  terminal: Readonly<{
    loser: 'north' | 'south';
    reason: string;
    status: 'finished';
    winner: 'north' | 'south';
  }>;
  transcriptHash: string;
  turnCount: number;
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function parseRustReport(value: unknown): RustDeterministicGameReport {
  if (!isRecord(value)
    || value.classification !== 'unranked_partial_rules_unverified_authority'
    || value.replayVerified !== true
    || !Number.isSafeInteger(value.acceptedActionCount)
    || !Number.isSafeInteger(value.fightCount)
    || !Number.isSafeInteger(value.turnCount)
    || typeof value.finalStateHash !== 'string'
    || typeof value.transcriptHash !== 'string'
    || !isRecord(value.terminal)
    || value.terminal.status !== 'finished'
    || (value.terminal.winner !== 'north' && value.terminal.winner !== 'south')
    || (value.terminal.loser !== 'north' && value.terminal.loser !== 'south')
    || typeof value.terminal.reason !== 'string') {
    throw new Error('Rust engine report did not match the expected contract');
  }
  return value as RustDeterministicGameReport;
}

/** Runs one locked-release `sorcery-engine` subcommand and returns canonical JSON output. */
export function runRustEngineCommand(args: readonly string[]): JsonValue {
  const result = spawnSync('cargo', [
    'run', '--release', '--locked', '--quiet', '-p', 'sorcery-engine',
    '--bin', 'sorcery-engine', '--', ...args,
  ], {
    cwd: REPOSITORY_ROOT,
    encoding: 'utf8',
    maxBuffer: MAX_OUTPUT_BYTES,
  });
  if (result.status !== 0) {
    throw new Error(result.stderr?.trim() || 'Rust engine command failed');
  }
  const text = result.stdout.trim();
  const parsed = parseJsonWithDuplicateKeyCheck(text);
  if (canonicalJson(parsed) !== text) {
    throw new Error('Rust engine output was not canonical JSON');
  }
  return parsed;
}

/** Runs the authoritative synthetic demo rollout for one seed. */
export function runRustSyntheticDemo(seed: number): RustDeterministicGameReport {
  if (!Number.isSafeInteger(seed) || seed < 0 || seed > 0xffff_ffff) {
    throw new RangeError('seed must be a safe integer between 0 and 4294967295');
  }
  return parseRustReport(runRustEngineCommand(['demo', String(seed)]));
}
