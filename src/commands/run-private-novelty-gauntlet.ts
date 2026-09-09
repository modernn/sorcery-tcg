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
const FRONTIER_BRANCH_LIMIT = 32;
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

type FrontierSeed = Readonly<{
  actionId: string;
  actionKind: NoveltyFrontierCandidate['actionKind'];
  branchId: string;
  checkpointId: string;
  depth: number;
  parentBranchId: string | null;
  parentJobId: string;
  predictedEventTypes: readonly string[];
  predictedStateHash: string;
  signals: NoveltyFrontierCandidate['signal'][];
}>;

function compareStrings(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function signalKey(signal: NoveltyFrontierCandidate['signal']): string {
  return `${signal.kind}:\0${signal.value}`;
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
        result: await runNoveltyRollout(createGameSession(manifest), {
          maxActions,
          onCheckpoint: captureCheckpoint,
        }),
      });
    }
  }

  const queue: FrontierSeed[] = [];
  const seenBranches = new Set<string>();
  let nextBranchOrdinal = 1;
  const enqueueFrontier = (
    frontier: NoveltyRolloutResult['frontier'],
    parentJobId: string,
    parentBranchId: string | null,
    depth: number,
  ): void => {
    const grouped = new Map<string, Omit<FrontierSeed, 'branchId'>>();
    for (const candidate of frontier) {
      const key = `${candidate.checkpointId}\0${candidate.actionId}`;
      const existing = grouped.get(key);
      if (existing) {
        if (existing.actionKind !== candidate.actionKind
          || existing.predictedStateHash !== candidate.predictedStateHash
          || canonicalJson(existing.predictedEventTypes as unknown as JsonValue)
            !== canonicalJson(candidate.predictedEventTypes as unknown as JsonValue)) {
          throw new Error('private novelty frontier action has inconsistent predictions');
        }
        existing.signals.push(candidate.signal);
      } else {
        grouped.set(key, {
          actionId: candidate.actionId,
          actionKind: candidate.actionKind,
          checkpointId: candidate.checkpointId,
          depth,
          parentBranchId,
          parentJobId,
          predictedEventTypes: candidate.predictedEventTypes,
          predictedStateHash: candidate.predictedStateHash,
          signals: [candidate.signal],
        });
      }
    }
    for (const seed of [...grouped.values()].sort((left, right) =>
      compareStrings(left.checkpointId, right.checkpointId)
        || compareStrings(left.actionId, right.actionId))) {
      const key = `${seed.checkpointId}\0${seed.actionId}`;
      if (seenBranches.has(key)) continue;
      seenBranches.add(key);
      seed.signals.sort((left, right) => compareStrings(signalKey(left), signalKey(right)));
      queue.push({
        ...seed,
        branchId: `${parentJobId}:branch-${nextBranchOrdinal}`,
      });
      nextBranchOrdinal += 1;
    }
  };

  const exercisedSignals = new Set<string>();
  for (const job of jobs) {
    enqueueFrontier(job.result.frontier, job.jobId, null, 1);
  }

  const frontierBranches: PrivateNoveltyGauntletReport['frontierBranches'][number][] = [];
  let cursor = 0;
  let frontierPrunedCovered = 0;
  let stopAfterFailure = false;
  while (cursor < queue.length
    && frontierBranches.length < FRONTIER_BRANCH_LIMIT
    && !stopAfterFailure) {
    const seed = queue[cursor++]!;
    const novelSignalsAtDispatch = seed.signals
      .filter((signal) => !exercisedSignals.has(signalKey(signal)));
    if (novelSignalsAtDispatch.length === 0) {
      frontierPrunedCovered += 1;
      continue;
    }
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
    exercisedSignals.add(signalKey({ kind: 'action-kind', value: seed.actionKind }));
    for (const value of entryEventTypes) {
      exercisedSignals.add(signalKey({ kind: 'event-type', value }));
    }
    const result = await runNoveltyRollout(entry.session, {
      maxActions,
      onCheckpoint: captureCheckpoint,
    });
    if (result.initialStateHash !== seed.predictedStateHash) {
      throw new Error('private novelty frontier rollout started from the wrong state');
    }
    frontierBranches.push({
      actionId: seed.actionId,
      actionKind: seed.actionKind,
      branchId: seed.branchId,
      checkpointId: seed.checkpointId,
      depth: seed.depth,
      entryActionCount: 1,
      entryEventTypes,
      novelSignalsAtDispatch,
      parentBranchId: seed.parentBranchId,
      parentJobId: seed.parentJobId,
      result,
      signals: seed.signals,
    });
    if (result.status === 'failed') {
      stopAfterFailure = true;
    } else {
      enqueueFrontier(result.frontier, seed.parentJobId, seed.branchId, seed.depth + 1);
    }
  }

  const frontierPending: PrivateNoveltyGauntletReport['frontierPending'][number][] = [];
  for (const seed of queue.slice(cursor)) {
    const signals = seed.signals.filter((signal) => !exercisedSignals.has(signalKey(signal)));
    if (signals.length === 0) {
      frontierPrunedCovered += 1;
      continue;
    }
    frontierPending.push({
      actionId: seed.actionId,
      actionKind: seed.actionKind,
      branchId: seed.branchId,
      checkpointId: seed.checkpointId,
      depth: seed.depth,
      parentBranchId: seed.parentBranchId,
      parentJobId: seed.parentJobId,
      predictedEventTypes: seed.predictedEventTypes,
      predictedStateHash: seed.predictedStateHash,
      signals,
    });
  }
  const pendingSignals = new Set(frontierPending.flatMap(({ signals }) =>
    signals.map(signalKey)));

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
  if (frontierPending.some(({ checkpointId }) => !checkpoints.has(checkpointId))) {
    throw new Error('private novelty gauntlet did not capture a pending checkpoint');
  }

  const savedCheckpoints = await saveCheckpoints(revisionId, outputId, checkpoints);
  const report: PrivateNoveltyGauntletReport = deepFreeze({
    classification: 'authority-private' as const,
    frontierBranches,
    frontierPending,
    jobs,
    policyVersion: 'signal-guided-bounded-frontier-v2' as const,
    schemaVersion: 2 as const,
    totals: {
      branchLimit: FRONTIER_BRANCH_LIMIT,
      completed: jobs.filter(({ result }) => result.status === 'completed').length,
      failed: jobs.filter(({ result }) => result.status === 'failed').length,
      frontierBranches: frontierBranches.length,
      frontierCompleted: frontierBranches.filter(({ result }) => result.status === 'completed').length,
      frontierFailed: frontierBranches.filter(({ result }) => result.status === 'failed').length,
      frontierHorizon: frontierBranches.filter(({ result }) => result.status === 'horizon').length,
      frontierLimitReached: frontierBranches.length === FRONTIER_BRANCH_LIMIT
        && frontierPending.length > 0,
      frontierMaxDepth: frontierBranches.reduce((maximum, { depth }) =>
        Math.max(maximum, depth), 0),
      frontierPending: frontierPending.length,
      frontierPendingSignals: pendingSignals.size,
      frontierPrunedCovered,
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
