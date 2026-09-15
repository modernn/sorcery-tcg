import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { lstat, mkdir, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { resolveWithinAuthorityRoot } from '../authority/validate-bundle.ts';
import {
  parseGameCheckpoint,
  serializeGameCheckpoint,
  type GameCheckpoint,
} from '../engine/checkpoint.ts';
import { deepFreeze } from '../engine/contract.ts';
import {
  createGameManifest,
  type GameManifest,
} from '../engine/game.ts';
import {
  NOVELTY_ROLLOUT_ACTION_LIMIT,
  type NoveltyFrontierCandidate,
  type NoveltyRolloutResult,
} from '../simulator/novelty-rollout.ts';
import {
  loadPrivateStarterCatalog,
  type PrivateStarterPreset,
} from './run-private-game-check.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const DEFAULT_TARGET_DIR = resolve(REPOSITORY_ROOT, 'target');
const DEFAULT_SCENARIO = resolve(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'scenarios',
  'vanilla-constructed.json',
);
const LESSON_IDS = ['air-vs-earth-lesson', 'earth-vs-air-lesson'] as const;
const CHECKPOINT_ID_PATTERN = /^sha256:([0-9a-f]{64})$/u;
const REVISION_PATTERN = /^[a-z0-9][a-z0-9._-]*$/u;
const MAX_OUTPUT_BYTES = 128 * 1024 * 1024;

type LessonId = typeof LESSON_IDS[number];
type Orientation = 'original' | 'swapped';

export type PrivateNoveltyGauntletReport = Readonly<{
  classification: 'authority-private';
  frontierBranches: readonly Readonly<{
    actionId: string;
    actionKind: NoveltyFrontierCandidate['actionKind'];
    branchId: string;
    checkpointId: string;
    depth: number;
    entryActionCount: 1;
    entryEventTypes: readonly string[];
    novelSignalsAtDispatch: readonly NoveltyFrontierCandidate['signal'][];
    parentBranchId: string | null;
    parentJobId: string;
    result: NoveltyRolloutResult;
    signals: readonly NoveltyFrontierCandidate['signal'][];
  }>[];
  frontierPending: readonly Readonly<{
    actionId: string;
    actionKind: NoveltyFrontierCandidate['actionKind'];
    branchId: string;
    checkpointId: string;
    depth: number;
    parentBranchId: string | null;
    parentJobId: string;
    predictedEventTypes: readonly string[];
    predictedStateHash: string;
    signals: readonly NoveltyFrontierCandidate['signal'][];
  }>[];
  jobs: readonly Readonly<{
    jobId: string;
    lessonId: LessonId;
    orientation: Orientation;
    result: NoveltyRolloutResult;
  }>[];
  policyVersion: 'signal-guided-bounded-frontier-v2';
  schemaVersion: 2;
  totals: Readonly<{
    branchLimit: 32;
    completed: number;
    failed: number;
    frontierBranches: number;
    frontierCompleted: number;
    frontierFailed: number;
    frontierHorizon: number;
    frontierLimitReached: boolean;
    frontierMaxDepth: number;
    frontierPending: number;
    frontierPendingSignals: number;
    frontierPrunedCovered: number;
    horizon: number;
    jobs: 4;
    savedCheckpoints: number;
  }>;
}>;

function swappedManifest(manifest: GameManifest): GameManifest {
  return createGameManifest({
    authority: manifest.authority,
    cards: manifest.cards,
    decks: {
      north: manifest.decks.south,
      south: manifest.decks.north,
    },
    firstSeat: manifest.firstSeat,
    seed: manifest.seed,
  });
}

function lessons(
  catalog: readonly PrivateStarterPreset[],
): readonly PrivateStarterPreset[] {
  const selected = LESSON_IDS.map((id) => catalog.find((preset) => preset.id === id));
  if (selected.some((preset) => !preset)) {
    throw new Error('private novelty gauntlet requires both pinned lesson manifests');
  }
  return selected as readonly PrivateStarterPreset[];
}

async function outputPath(revisionId: string, outputId: string): Promise<string> {
  const authorityRoot = resolve(REPOSITORY_ROOT, '.local', 'authority');
  const outputDirectory = resolve(authorityRoot, 'reports', revisionId);
  await mkdir(outputDirectory, { recursive: true });
  const confinedDirectory = await resolveWithinAuthorityRoot(
    authorityRoot,
    `reports/${revisionId}`,
  );
  const path = resolve(confinedDirectory, `${outputId}.json`);
  try {
    if ((await lstat(path)).isSymbolicLink()) {
      throw new Error('private novelty gauntlet output path may not be a symbolic link or junction');
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
  return path;
}

async function saveCheckpoints(
  revisionId: string,
  outputId: string,
  checkpoints: ReadonlyMap<string, GameCheckpoint>,
): Promise<number> {
  const authorityRoot = resolve(REPOSITORY_ROOT, '.local', 'authority');
  const relativeDirectory = `checkpoints/${revisionId}/${outputId}`;
  await mkdir(resolve(authorityRoot, relativeDirectory), { recursive: true });
  const directory = await resolveWithinAuthorityRoot(authorityRoot, relativeDirectory);
  for (const [checkpointId, checkpoint] of [...checkpoints.entries()]
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)) {
    const match = CHECKPOINT_ID_PATTERN.exec(checkpointId);
    if (!match) throw new Error('private novelty checkpoint ID is invalid');
    const path = resolve(directory, `${match[1]}.json`);
    try {
      if ((await lstat(path)).isSymbolicLink()) {
        throw new Error('private novelty checkpoint output may not be a symbolic link or junction');
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
    }
    await writeFile(path, `${serializeGameCheckpoint(checkpoint)}\n`, 'utf8');
  }
  return checkpoints.size;
}

function noveltyGauntletLaunch(): Readonly<{ args: readonly string[]; command: string }> {
  const targetDir = process.env.CARGO_TARGET_DIR ?? DEFAULT_TARGET_DIR;
  const binaryName = process.platform === 'win32' ? 'sorcery-engine.exe' : 'sorcery-engine';
  const binary = join(targetDir, 'release', binaryName);
  if (existsSync(binary)) return { args: ['novelty-gauntlet-json'], command: binary };
  return {
    args: [
      'run', '--release', '--locked', '--quiet', '-p', 'sorcery-engine',
      '--bin', 'sorcery-engine', '--', 'novelty-gauntlet-json',
    ],
    command: 'cargo',
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

async function runRustNoveltyGauntlet(request: string): Promise<Readonly<{
  checkpoints: ReadonlyMap<string, GameCheckpoint>;
  report: PrivateNoveltyGauntletReport;
}>> {
  const launch = noveltyGauntletLaunch();
  const child = spawn(launch.command, [...launch.args], {
    cwd: REPOSITORY_ROOT,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
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
    child.once('error', () => fail('private novelty gauntlet failed to start Rust'));
    child.stdin.once('error', () => fail('private novelty gauntlet failed while sending Rust input'));
    child.stdout.on('data', (chunk: Buffer) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > MAX_OUTPUT_BYTES) {
        return fail('private novelty gauntlet Rust output exceeded its limit');
      }
      stdout.push(chunk);
    });
    child.stderr.on('data', (chunk: Buffer) => {
      stderrBytes += chunk.length;
      if (stderrBytes > MAX_OUTPUT_BYTES) {
        return fail('private novelty gauntlet Rust error output exceeded its limit');
      }
      stderr.push(chunk);
    });
    child.once('close', (code) => {
      if (settled) return;
      if (code !== 0) {
        const message = Buffer.concat(stderr).toString('utf8').trim() || 'unknown error';
        return fail(`private novelty gauntlet failed in Rust: ${message}`);
      }
      try {
        const text = Buffer.concat(stdout).toString('utf8').trim();
        const parsed: unknown = JSON.parse(text);
        if (canonicalJson(parsed as JsonValue) !== text) throw new Error('noncanonical output');
        if (!isRecord(parsed)
          || !isRecord(parsed.report)
          || !Array.isArray(parsed.checkpoints)) {
          throw new Error('invalid response shape');
        }
        const checkpoints = new Map<string, GameCheckpoint>();
        for (const entry of parsed.checkpoints) {
          if (!isRecord(entry)
            || typeof entry.checkpointId !== 'string'
            || entry.checkpoint === undefined) {
            throw new Error('invalid checkpoint entry');
          }
          const checkpoint = parseGameCheckpoint(
            canonicalJson(entry.checkpoint as JsonValue),
          );
          if (checkpoint.checkpointId !== entry.checkpointId) {
            throw new Error('checkpoint identity mismatch');
          }
          checkpoints.set(entry.checkpointId, checkpoint);
        }
        settled = true;
        resolvePromise({
          checkpoints,
          report: parsed.report as PrivateNoveltyGauntletReport,
        });
      } catch (error) {
        fail(error instanceof Error
          ? `private novelty gauntlet Rust output was invalid: ${error.message}`
          : 'private novelty gauntlet Rust output was invalid');
      }
    });
    child.stdin.end(`${request}\n`);
  });
}

export async function runPrivateNoveltyGauntlet(
  scenarioPath = DEFAULT_SCENARIO,
  maxActions = NOVELTY_ROLLOUT_ACTION_LIMIT,
  outputId = 'novelty-gauntlet',
): Promise<Readonly<{ outputPath: string; report: PrivateNoveltyGauntletReport }>> {
  if (!Number.isSafeInteger(maxActions) || maxActions < 0
    || maxActions > NOVELTY_ROLLOUT_ACTION_LIMIT) {
    throw new RangeError(`maxActions must be 0-${NOVELTY_ROLLOUT_ACTION_LIMIT}`);
  }
  const selected = lessons(await loadPrivateStarterCatalog(scenarioPath));
  const revisionId = selected[0]!.manifest.authority.revisionId;
  if (!REVISION_PATTERN.test(revisionId) || !REVISION_PATTERN.test(outputId)) {
    throw new TypeError('private novelty revisionId and outputId must be confined path segments');
  }
  if (selected.some(({ manifest }) => manifest.authority.revisionId !== revisionId)) {
    throw new Error('private novelty lesson manifests must use one authority revision');
  }

  const jobs = selected.flatMap((preset) => (
    ['original', 'swapped'] as const
  ).map((orientation) => {
    const manifest = orientation === 'original'
      ? preset.manifest
      : swappedManifest(preset.manifest);
    return {
      jobId: `${preset.id}:${orientation}`,
      lessonId: preset.id as LessonId,
      manifestJson: canonicalJson(manifest as unknown as JsonValue),
      orientation,
    };
  }));

  const rust = await runRustNoveltyGauntlet(canonicalJson({
    jobs,
    maxActions,
    schemaVersion: 1,
  } as JsonValue));

  const savedCheckpoints = await saveCheckpoints(revisionId, outputId, rust.checkpoints);
  const report = deepFreeze({
    ...rust.report,
    totals: {
      ...rust.report.totals,
      savedCheckpoints,
    },
  }) as PrivateNoveltyGauntletReport;
  const destination = await outputPath(revisionId, outputId);
  await writeFile(destination, `${canonicalJson(report as unknown as JsonValue)}\n`, 'utf8');
  return { outputPath: destination, report };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const maxActions = process.argv[3] === undefined
    ? NOVELTY_ROLLOUT_ACTION_LIMIT
    : Number(process.argv[3]);
  const { outputPath: destination, report } = await runPrivateNoveltyGauntlet(
    process.argv[2],
    maxActions,
  );
  process.stdout.write(`${canonicalJson({
    output: relative(REPOSITORY_ROOT, destination).replaceAll('\\', '/'),
    totals: report.totals,
  })}\n`);
}
