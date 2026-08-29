import { availableParallelism } from 'node:os';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isMainThread, parentPort, Worker, workerData } from 'node:worker_threads';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { deepFreeze } from '../engine/contract.ts';
import type { GameManifest } from '../engine/game.ts';
import {
  createSyntheticDemoManifest,
  runDeterministicGame,
  type DeterministicGameReport,
} from './run-game-demo.ts';

const PROTOCOL_VERSION = 1;
const MAX_JOBS = 256;
const MAX_WORKERS = 8;
const MAX_BATCH_BYTES = 64 * 1024 * 1024;

type BatchItem = Readonly<{ jobIndex: number; manifest: GameManifest }>;
type WorkerInput = Readonly<{ chunk: readonly BatchItem[]; protocolVersion: 1 }>;
type WorkerResponse =
  | Readonly<{ ok: true; protocolVersion: 1; results: readonly GameBatchResult[] }>
  | Readonly<{ failedJobIndex: number; ok: false; protocolVersion: 1 }>;

export type GameBatchResult = Readonly<{
  jobIndex: number;
  manifestId: GameManifest['manifestId'];
  report: DeterministicGameReport;
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function runWorker(): void {
  let failedJobIndex = -1;
  try {
    if (!parentPort || !isRecord(workerData)
      || workerData.protocolVersion !== PROTOCOL_VERSION
      || !Array.isArray(workerData.chunk)) throw new Error('invalid worker input');
    const input = workerData as WorkerInput;
    const results = input.chunk.map(({ jobIndex, manifest }): GameBatchResult => {
      failedJobIndex = jobIndex;
      return {
        jobIndex,
        manifestId: manifest.manifestId,
        report: runDeterministicGame(manifest),
      };
    });
    parentPort.postMessage({ ok: true, protocolVersion: PROTOCOL_VERSION, results } satisfies WorkerResponse);
  } catch {
    parentPort?.postMessage({
      failedJobIndex,
      ok: false,
      protocolVersion: PROTOCOL_VERSION,
    } satisfies WorkerResponse);
  }
}

if (!isMainThread) runWorker();

function startWorker(chunk: readonly BatchItem[]): Readonly<{
  promise: Promise<readonly GameBatchResult[]>;
  worker: Worker;
}> {
  const worker = new Worker(new URL(import.meta.url), {
    execArgv: [],
    workerData: { chunk, protocolVersion: PROTOCOL_VERSION } satisfies WorkerInput,
  });
  const promise = new Promise<readonly GameBatchResult[]>((resolvePromise, rejectPromise) => {
    let settled = false;
    const fail = (message: string): void => {
      if (settled) return;
      settled = true;
      rejectPromise(new Error(message));
    };
    worker.once('message', (message: unknown) => {
      if (settled) return;
      if (!isRecord(message) || message.protocolVersion !== PROTOCOL_VERSION) {
        return fail('game batch worker returned an invalid response');
      }
      if (message.ok !== true) {
        return fail(`game batch job ${String(message.failedJobIndex)} failed`);
      }
      if (!Array.isArray(message.results)) {
        return fail('game batch worker returned invalid results');
      }
      settled = true;
      resolvePromise(message.results as readonly GameBatchResult[]);
    });
    worker.once('error', () => fail('game batch worker failed'));
    worker.once('exit', (code) => {
      if (!settled) fail(code === 0
        ? 'game batch worker exited before returning results'
        : 'game batch worker exited unsuccessfully');
    });
  });
  return { promise, worker };
}

export async function runGameBatch(
  manifests: readonly GameManifest[],
  requestedWorkers = Math.min(availableParallelism(), MAX_WORKERS),
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
  const workerCount = Math.min(requestedWorkers, manifests.length);
  const chunks = Array.from({ length: workerCount }, () => [] as BatchItem[]);
  manifests.forEach((manifest, jobIndex) => {
    chunks[jobIndex % workerCount]!.push({ jobIndex, manifest });
  });
  const active = chunks.map(startWorker);
  try {
    const results = (await Promise.all(active.map(({ promise }) => promise)))
      .flat()
      .sort((left, right) => left.jobIndex - right.jobIndex);
    if (results.length !== manifests.length || results.some((result, jobIndex) =>
      result.jobIndex !== jobIndex
        || result.manifestId !== manifests[jobIndex]?.manifestId
        || result.report.replayVerified !== true)) {
      throw new Error('game batch results do not match the declared jobs');
    }
    return deepFreeze(results);
  } catch (error) {
    // ponytail: abort the whole batch; add per-job failure accounting when scheduled gauntlets need partial results.
    await Promise.allSettled(active.map(({ worker }) => worker.terminate()));
    throw error;
  }
}

if (isMainThread && process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [workerText = String(Math.min(availableParallelism(), MAX_WORKERS)), ...seedTexts] =
    process.argv.slice(2);
  const workers = Number(workerText);
  const seeds = (seedTexts.length > 0 ? seedTexts : ['1']).map(Number);
  const manifests = seeds.map((seed) => createSyntheticDemoManifest(seed));
  process.stdout.write(`${canonicalJson(await runGameBatch(manifests, workers) as unknown as JsonValue)}\n`);
}
