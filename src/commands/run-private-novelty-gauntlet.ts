import { lstat, mkdir, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { resolveWithinAuthorityRoot } from '../authority/validate-bundle.ts';
import {
  serializeGameCheckpoint,
  type GameCheckpoint,
} from '../engine/checkpoint.ts';
import { deepFreeze } from '../engine/contract.ts';
import {
  createGameManifest,
  createGameSession,
  type GameManifest,
} from '../engine/game.ts';
import {
  NOVELTY_ROLLOUT_ACTION_LIMIT,
  runNoveltyRollout,
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

async function outputPath(revisionId: string): Promise<string> {
  const authorityRoot = resolve(REPOSITORY_ROOT, '.local', 'authority');
  const outputDirectory = resolve(authorityRoot, 'reports', revisionId);
  await mkdir(outputDirectory, { recursive: true });
  const confinedDirectory = await resolveWithinAuthorityRoot(
    authorityRoot,
    `reports/${revisionId}`,
  );
  const path = resolve(confinedDirectory, 'novelty-gauntlet.json');
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
  checkpoints: ReadonlyMap<string, GameCheckpoint>,
): Promise<number> {
  const authorityRoot = resolve(REPOSITORY_ROOT, '.local', 'authority');
  const relativeDirectory = `checkpoints/${revisionId}/novelty-gauntlet`;
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
): Promise<Readonly<{ outputPath: string; report: PrivateNoveltyGauntletReport }>> {
  if (!Number.isSafeInteger(maxActions) || maxActions < 0
    || maxActions > NOVELTY_ROLLOUT_ACTION_LIMIT) {
    throw new RangeError(`maxActions must be 0-${NOVELTY_ROLLOUT_ACTION_LIMIT}`);
  }
  const selected = lessons(await loadPrivateStarterCatalog(scenarioPath));
  const revisionId = selected[0]!.manifest.authority.revisionId;
  if (!REVISION_PATTERN.test(revisionId)) {
    throw new TypeError('private novelty revisionId must be one confined path segment');
  }
  if (selected.some(({ manifest }) => manifest.authority.revisionId !== revisionId)) {
    throw new Error('private novelty lesson manifests must use one authority revision');
  }

  const jobs: PrivateNoveltyGauntletReport['jobs'][number][] = [];
  const checkpoints = new Map<string, GameCheckpoint>();
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
          onCheckpoint(checkpoint) {
            const existing = checkpoints.get(checkpoint.checkpointId);
            if (existing
              && serializeGameCheckpoint(existing) !== serializeGameCheckpoint(checkpoint)) {
              throw new Error('private novelty checkpoint identity collision');
            }
            checkpoints.set(checkpoint.checkpointId, checkpoint);
          },
        }),
      });
    }
  }

  for (const { result } of jobs) {
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

  const savedCheckpoints = await saveCheckpoints(revisionId, checkpoints);
  const report: PrivateNoveltyGauntletReport = deepFreeze({
    classification: 'authority-private' as const,
    jobs,
    policyVersion: 'private-lesson-novelty-gauntlet-v1' as const,
    schemaVersion: 1 as const,
    totals: {
      completed: jobs.filter(({ result }) => result.status === 'completed').length,
      failed: jobs.filter(({ result }) => result.status === 'failed').length,
      horizon: jobs.filter(({ result }) => result.status === 'horizon').length,
      jobs: 4 as const,
      savedCheckpoints,
    },
  });
  const destination = await outputPath(revisionId);
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
