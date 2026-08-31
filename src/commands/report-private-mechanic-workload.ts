import { lstat, mkdir, readFile, realpath, writeFile } from 'node:fs/promises';
import { basename, dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  canonicalArtifactSchema,
  normalizedCardSnapshotSchema,
  type Hash,
} from '../authority/schemas.ts';
import {
  buildMechanicWorkload,
  type MechanicWorkloadReport,
  type PresetCardDemand,
} from '../mechanics/mechanic-workload.ts';
import { loadPrivateStarterCatalog } from './run-private-game-check.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const DEFAULT_SCENARIO = resolve(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'scenarios',
  'vanilla-constructed.json',
);
const REVISION_PATTERN = /^[a-z0-9][a-z0-9._-]*$/u;

export type PrivateMechanicWorkloadReport = MechanicWorkloadReport & Readonly<{
  authority: Readonly<{
    contentHash: Hash;
    revisionId: string;
  }>;
}>;

type PrivateScenarioReference = Readonly<{ revisionId: string }>;

function scenarioReference(value: JsonValue): PrivateScenarioReference {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new TypeError('private scenario must be an object');
  }
  const revisionId = (value as Readonly<Record<string, JsonValue>>).revisionId;
  if (typeof revisionId !== 'string' || !REVISION_PATTERN.test(revisionId)) {
    throw new TypeError('private scenario revisionId must be one confined path segment');
  }
  return { revisionId };
}

function presetDemand(
  presets: Awaited<ReturnType<typeof loadPrivateStarterCatalog>>,
): readonly PresetCardDemand[] {
  return presets.flatMap((preset) => {
    const copies = new Map<string, { northCopies: number; southCopies: number }>();
    for (const seat of ['north', 'south'] as const) {
      const deck = preset.manifest.decks[seat];
      const cardIds = [deck.avatar, ...deck.atlas, ...deck.spellbook];
      for (const stableId of cardIds) {
        const current = copies.get(stableId) ?? { northCopies: 0, southCopies: 0 };
        current[seat === 'north' ? 'northCopies' : 'southCopies'] += 1;
        copies.set(stableId, current);
      }
    }
    return [...copies.entries()].map(([stableId, copyCounts]) => ({
      ...copyCounts,
      presetId: preset.id,
      stableId,
    }));
  });
}

export async function resolveMechanicWorkloadOutputPath(
  authorityRoot: string,
  revisionId: string,
): Promise<string> {
  if (!REVISION_PATTERN.test(revisionId)) {
    throw new TypeError('private report revisionId must be one confined path segment');
  }
  const reportsRoot = resolve(authorityRoot, 'reports');
  const outputDirectory = resolve(reportsRoot, revisionId);
  if (dirname(outputDirectory) !== reportsRoot) {
    throw new Error('private mechanic workload output escaped its report root');
  }
  await mkdir(outputDirectory, { recursive: true });
  const [resolvedAuthorityRoot, resolvedReportsRoot, resolvedOutputDirectory] = await Promise.all([
    realpath(authorityRoot),
    realpath(reportsRoot),
    realpath(outputDirectory),
  ]);
  if (dirname(resolvedReportsRoot) !== resolvedAuthorityRoot
    || dirname(resolvedOutputDirectory) !== resolvedReportsRoot) {
    throw new Error('private mechanic workload resolved path escapes the configured authority root');
  }
  const outputPath = resolve(resolvedOutputDirectory, 'mechanic-workload.json');
  try {
    if ((await lstat(outputPath)).isSymbolicLink()) {
      throw new Error('private mechanic workload output path may not be a symbolic link or junction');
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error;
  }
  return outputPath;
}

export async function reportPrivateMechanicWorkload(
  scenarioPath = DEFAULT_SCENARIO,
): Promise<Readonly<{ outputPath: string; report: PrivateMechanicWorkloadReport }>> {
  const scenario = scenarioReference(parseJsonWithDuplicateKeyCheck(
    await readFile(scenarioPath, 'utf8'),
  ));
  const revisionRoot = resolve(
    REPOSITORY_ROOT,
    '.local',
    'authority',
    'revisions',
    scenario.revisionId,
  );
  if (basename(revisionRoot) !== scenario.revisionId) {
    throw new Error('private revision ID must be one path segment');
  }
  const artifact = canonicalArtifactSchema.parse(parseJsonWithDuplicateKeyCheck(
    await readFile(resolve(revisionRoot, 'cards.normalized.json'), 'utf8'),
  ));
  if (artifact.identity.artifactKind !== 'card-snapshot'
    || identityHash(artifact.identity as unknown as JsonValue) !== artifact.contentHash) {
    throw new Error('private normalized card artifact identity is invalid');
  }
  const snapshot = normalizedCardSnapshotSchema.parse(artifact.identity.payload);
  const presets = await loadPrivateStarterCatalog(scenarioPath);
  const workload = buildMechanicWorkload(snapshot.cards, presetDemand(presets));
  const report: PrivateMechanicWorkloadReport = {
    authority: {
      contentHash: artifact.contentHash,
      revisionId: scenario.revisionId,
    },
    ...workload,
  };

  const outputPath = await resolveMechanicWorkloadOutputPath(
    resolve(REPOSITORY_ROOT, '.local', 'authority'),
    scenario.revisionId,
  );
  await writeFile(outputPath, `${canonicalJson(report as unknown as JsonValue)}\n`, 'utf8');
  return { outputPath, report };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { outputPath, report } = await reportPrivateMechanicWorkload(process.argv[2]);
  process.stdout.write(`${canonicalJson({
    cards: report.totals.cards,
    output: relative(REPOSITORY_ROOT, outputPath).replaceAll('\\', '/'),
    presetDemandPresent: report.totals.presetDemandPresent,
    revisionId: report.authority.revisionId,
  })}\n`);
}
