import { lstat, mkdir, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { resolveWithinAuthorityRoot } from '../authority/validate-bundle.ts';
import {
  resumeGameCheckpoint,
  serializeGameCheckpoint,
  type GameCheckpoint,
} from '../engine/checkpoint.ts';
import { deepFreeze } from '../engine/contract.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  stepGame,
  type GameManifest,
} from '../engine/game.ts';
import {
  NOVELTY_ROLLOUT_ACTION_LIMIT,
  runNoveltyRollout,
  type NoveltyFrontierCandidate,
  type NoveltyRolloutResult,
} from '../simulator/novelty-rollout.ts';
import {
  loadPrivateStarterCatalog,
  type PrivateStarterPreset,
} from './run-private-game-check.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
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

type LessonId = typeof LESSON_IDS[number];
type Orientation = 'original' | 'swapped';

export type PrivateNoveltyGauntletReport = Readonly<{
  classification: 'authority-private';
  frontierBranches: readonly Readonly<{
    actionId: string;
    actionKind: NoveltyFrontierCandidate['actionKind'];
    branchId: string;
    checkpointId: string;
    entryActionCount: 1;
    entryEventTypes: readonly string[];
    parentJobId: string;
    result: NoveltyRolloutResult;
    signals: readonly NoveltyFrontierCandidate['signal'][];
  }>[];
  jobs: readonly Readonly<{
    jobId: string;
    lessonId: LessonId;
    orientation: Orientation;
    result: NoveltyRolloutResult;
  }>[];
  policyVersion: 'private-lesson-novelty-gauntlet-v1';
  schemaVersion: 1;
  totals: Readonly<{
    completed: number;
    failed: number;
    frontierBranches: number;
    frontierCompleted: number;
    frontierFailed: number;
    frontierHorizon: number;
    horizon: number;
    jobs: 4;
    savedCheckpoints: number;
  }>;
}>;

type FrontierSeed = Readonly<{
  actionId: string;
  actionKind: NoveltyFrontierCandidate['actionKind'];
  checkpointId: string;
  parentIndex: number;
  parentJobId: string;
  predictedEventTypes: readonly string[];
  predictedStateHash: string;
  signals: NoveltyFrontierCandidate['signal'][];
}>;

function compareStrings(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

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

  const jobs: PrivateNoveltyGauntletReport['jobs'][number][] = [];
  const checkpoints = new Map<string, GameCheckpoint>();
  const captureCheckpoint = (checkpoint: GameCheckpoint): void => {
    const existing = checkpoints.get(checkpoint.checkpointId);
    if (existing && serializeGameCheckpoint(existing) !== serializeGameCheckpoint(checkpoint)) {
      throw new Error('private novelty checkpoint identity collision');
    }
    checkpoints.set(checkpoint.checkpointId, checkpoint);
  };
  for (const preset of selected) {
    for (const orientation of ['original', 'swapped'] as const) {
      const manifest = orientation === 'original'
        ? preset.manifest
        : swappedManifest(preset.manifest);
      jobs.push({
        jobId: `${preset.id}:${orientation}`,
        lessonId: preset.id as LessonId,
        orientation,
        result: runNoveltyRollout(createGameSession(manifest), {
          maxActions,
          onCheckpoint: captureCheckpoint,
        }),
      });
    }
  }

  const seeds = new Map<string, FrontierSeed>();
  for (const [parentIndex, job] of jobs.entries()) {
    for (const candidate of job.result.frontier) {
      const key = `${parentIndex}\0${candidate.checkpointId}\0${candidate.actionId}`;
      const existing = seeds.get(key);
      if (existing) {
        if (existing.actionKind !== candidate.actionKind
          || existing.predictedStateHash !== candidate.predictedStateHash
          || canonicalJson(existing.predictedEventTypes as unknown as JsonValue)
            !== canonicalJson(candidate.predictedEventTypes as unknown as JsonValue)) {
          throw new Error('private novelty frontier action has inconsistent predictions');
        }
        existing.signals.push(candidate.signal);
      } else {
        seeds.set(key, {
          actionId: candidate.actionId,
          actionKind: candidate.actionKind,
          checkpointId: candidate.checkpointId,
          parentIndex,
          parentJobId: job.jobId,
          predictedEventTypes: candidate.predictedEventTypes,
          predictedStateHash: candidate.predictedStateHash,
          signals: [candidate.signal],
        });
      }
    }
  }

  const frontierBranches: PrivateNoveltyGauntletReport['frontierBranches'][number][] = [];
  const orderedSeeds = [...seeds.values()].sort((left, right) =>
    left.parentIndex - right.parentIndex
      || compareStrings(left.checkpointId, right.checkpointId)
      || compareStrings(left.actionId, right.actionId));
  for (const [index, seed] of orderedSeeds.entries()) {
    const checkpoint = checkpoints.get(seed.checkpointId);
    if (!checkpoint) throw new Error('private novelty frontier checkpoint was not captured');
    const resumed = resumeGameCheckpoint(checkpoint);
    const issued = legalGameActions(resumed.state, resumed.state.decisionSeat)
      .filter(({ actionId }) => actionId === seed.actionId);
    if (issued.length !== 1) throw new Error('private novelty frontier action is stale');
    if (issued[0]!.descriptor.kind !== seed.actionKind) {
      throw new Error('private novelty frontier action kind changed');
    }
    const entry = stepGame(resumed, issued[0]!);
    if (!entry.accepted) {
      throw new Error(`private novelty frontier action rejected: ${entry.reason.code}`);
    }
    const entryEventTypes = [...new Set(entry.receipt.events.map(({ type }) => type))].sort();
    if (hashGameState(entry.session.state) !== seed.predictedStateHash
      || canonicalJson(entryEventTypes as unknown as JsonValue)
        !== canonicalJson(seed.predictedEventTypes as unknown as JsonValue)
      || seed.signals.some((signal) => signal.kind === 'action-kind'
        ? signal.value !== seed.actionKind
        : !entryEventTypes.includes(signal.value))) {
      throw new Error('private novelty frontier prediction did not replay exactly');
    }
    const result = runNoveltyRollout(entry.session, {
      maxActions,
      onCheckpoint: captureCheckpoint,
    });
    if (result.initialStateHash !== seed.predictedStateHash) {
      throw new Error('private novelty frontier rollout started from the wrong state');
    }
    frontierBranches.push({
      actionId: seed.actionId,
      actionKind: seed.actionKind,
      branchId: `${seed.parentJobId}:frontier-${index + 1}`,
      checkpointId: seed.checkpointId,
      entryActionCount: 1,
      entryEventTypes,
      parentJobId: seed.parentJobId,
      result,
      signals: seed.signals,
    });
  }

  for (const result of [
    ...jobs.map((job) => job.result),
    ...frontierBranches.map((branch) => branch.result),
  ]) {
    const required = [
      ...result.frontier.map(({ checkpointId }) => checkpointId),
      ...(result.status === 'horizon' ? [result.checkpointId] : []),
      ...(result.status === 'failed' && 'checkpointId' in result.failure
        ? [result.failure.checkpointId]
        : []),
    ];
    if (required.some((checkpointId) => !checkpoints.has(checkpointId))) {
      throw new Error('private novelty gauntlet did not capture a reported checkpoint');
    }
  }

  const savedCheckpoints = await saveCheckpoints(revisionId, outputId, checkpoints);
  const report: PrivateNoveltyGauntletReport = deepFreeze({
    classification: 'authority-private' as const,
    frontierBranches,
    jobs,
    policyVersion: 'private-lesson-novelty-gauntlet-v1' as const,
    schemaVersion: 1 as const,
    totals: {
      completed: jobs.filter(({ result }) => result.status === 'completed').length,
      failed: jobs.filter(({ result }) => result.status === 'failed').length,
      frontierBranches: frontierBranches.length,
      frontierCompleted: frontierBranches.filter(({ result }) => result.status === 'completed').length,
      frontierFailed: frontierBranches.filter(({ result }) => result.status === 'failed').length,
      frontierHorizon: frontierBranches.filter(({ result }) => result.status === 'horizon').length,
      horizon: jobs.filter(({ result }) => result.status === 'horizon').length,
      jobs: 4 as const,
      savedCheckpoints,
    },
  });
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
