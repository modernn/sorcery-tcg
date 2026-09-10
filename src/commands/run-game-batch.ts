import { spawn } from 'node:child_process';
import { availableParallelism } from 'node:os';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import { deepFreeze } from '../engine/contract.ts';
import type { GameDeckSpec, GameManifest } from '../engine/game.ts';
import type { DeterministicGameReport } from './run-game-demo.ts';

const MAX_JOBS = 256;
const MAX_WORKERS = 8;
const MAX_BATCH_BYTES = 64 * 1024 * 1024;
const MAX_OUTPUT_BYTES = 16 * 1024 * 1024;
const REPOSITORY_ROOT = fileURLToPath(new URL('../..', import.meta.url));

type RustGameReport = Readonly<Omit<DeterministicGameReport, 'classification'> & {
  classification: 'unranked_partial_rules_unverified_authority';
}>;

export type GameBatchResult = Readonly<{
  jobIndex: number;
  manifestId: GameManifest['manifestId'];
  report: RustGameReport;
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function countedRows(cardIds: readonly string[]): readonly Readonly<{
  cardId: string;
  copies: number;
}>[] {
  const counts = new Map<string, number>();
  cardIds.forEach((cardId) => counts.set(cardId, (counts.get(cardId) ?? 0) + 1));
  return [...counts]
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
    .map(([cardId, copies]) => ({ cardId, copies }));
}

function deckId(deck: GameDeckSpec): ReturnType<typeof identityHash> {
  return identityHash({
    deck: {
      atlas: countedRows(deck.atlas),
      avatar: deck.avatar,
      spellbook: countedRows(deck.spellbook),
    },
    formatId: 'format:modeled-constructed-v1',
  } as JsonValue);
}

function policy(manifest: GameManifest, boundDeckId: ReturnType<typeof identityHash>): JsonValue {
  const body = {
    authorityHash: manifest.authority.contentHash,
    deckId: boundDeckId,
    engineVersion: manifest.engineVersion,
    generation: 0,
    observationVersion: 'seat-observation-v1',
    schemaVersion: 1,
    selector: {
      atlasReserve: 3,
      featurePriority: [
        'keep-mulligan', 'play-site', 'summon-minion', 'preferred-draw',
        'powered-movement', 'beneficial-tactic', 'move-toward-enemy',
        'end-turn', 'canonical-fallback',
      ],
    },
    tieBreak: 'canonical-action-order-v1',
  } as const;
  return { ...body, policyId: identityHash(body as unknown as JsonValue) } as JsonValue;
}

function rustRequest(
  manifests: readonly GameManifest[],
  workers: number,
  artifactsDir?: string,
): string {
  return canonicalJson({
    ...(artifactsDir === undefined ? {} : { artifactsDir }),
    jobs: manifests.map((manifest) => {
      const northDeckId = deckId(manifest.decks.north);
      const southDeckId = deckId(manifest.decks.south);
      return {
        manifestJson: canonicalJson(manifest as unknown as JsonValue),
        northDeckId,
        northPolicy: policy(manifest, northDeckId),
        southDeckId,
        southPolicy: policy(manifest, southDeckId),
      };
    }),
    schemaVersion: 1,
    workers,
  } as JsonValue);
}

async function runRustBatch(request: string): Promise<unknown> {
  const child = spawn('cargo', [
    'run', '--release', '--locked', '--quiet', '-p', 'sorcery-engine',
    '--bin', 'sorcery-engine', '--', 'batch-json',
  ], { cwd: REPOSITORY_ROOT, stdio: ['pipe', 'pipe', 'pipe'] });
  const stdout: Buffer[] = [];
  const stderr: Buffer[] = [];
  let stdoutBytes = 0;
  let stderrBytes = 0;
  return new Promise((resolvePromise, rejectPromise) => {
    let settled = false;
    const fail = (message: string): void => {
      if (settled) return;
      settled = true;
      child.kill();
      rejectPromise(new Error(message));
    };
    child.once('error', () => fail('game batch failed to start Rust'));
    child.stdin.once('error', () => fail('game batch failed while sending Rust input'));
    child.stdout.on('data', (chunk: Buffer) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > MAX_OUTPUT_BYTES) return fail('game batch Rust output exceeded its limit');
      stdout.push(chunk);
    });
    child.stderr.on('data', (chunk: Buffer) => {
      stderrBytes += chunk.length;
      if (stderrBytes > MAX_OUTPUT_BYTES) return fail('game batch Rust error output exceeded its limit');
      stderr.push(chunk);
    });
    child.once('close', (code) => {
      if (settled) return;
      if (code !== 0) {
        const unsupported = /not yet supported by Rust: ([A-Za-z][A-Za-z0-9:]*)/u
          .exec(Buffer.concat(stderr).toString('utf8'))?.[1];
        if (unsupported) {
          return fail(`game batch failed in Rust: unsupported fact ${unsupported}`);
        }
        return fail('game batch failed in Rust');
      }
      try {
        const text = Buffer.concat(stdout).toString('utf8').trim();
        const parsed: unknown = JSON.parse(text);
        if (canonicalJson(parsed as JsonValue) !== text) throw new Error('noncanonical output');
        settled = true;
        resolvePromise(parsed);
      } catch {
        fail('game batch Rust output was invalid');
      }
    });
    child.stdin.end(`${request}\n`);
  });
}

export async function runGameBatch(
  manifests: readonly GameManifest[],
  requestedWorkers = Math.min(availableParallelism(), MAX_WORKERS),
  artifactsDir?: string,
): Promise<readonly GameBatchResult[]> {
  if (manifests.length === 0 || manifests.length > MAX_JOBS) {
    throw new RangeError(`game batch must contain 1-${MAX_JOBS} manifests`);
  }
  if (!Number.isSafeInteger(requestedWorkers)
    || requestedWorkers < 1
    || requestedWorkers > MAX_WORKERS) {
    throw new RangeError(`requestedWorkers must be 1-${MAX_WORKERS}`);
  }
  if (Buffer.byteLength(canonicalJson(manifests as unknown as JsonValue)) > MAX_BATCH_BYTES) {
    throw new RangeError(`game batch exceeds ${MAX_BATCH_BYTES} bytes`);
  }
  const results = await runRustBatch(rustRequest(manifests, requestedWorkers, artifactsDir));
  if (!Array.isArray(results) || results.length !== manifests.length
    || results.some((result, jobIndex) => !isRecord(result)
      || result.jobIndex !== jobIndex
      || result.manifestId !== manifests[jobIndex]?.manifestId
      || !isRecord(result.report)
      || result.report.classification !== 'unranked_partial_rules_unverified_authority'
      || result.report.replayVerified !== true)) {
    throw new Error('game batch results do not match the declared jobs');
  }
  return deepFreeze(results as unknown as readonly GameBatchResult[]);
}
