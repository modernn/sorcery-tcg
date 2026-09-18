import { mkdir, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { loadPrivateCardSnapshot } from '../authority/private-cards.ts';
import {
  buildCandidateManifest,
  resolvedDeckToSpec,
} from '../ingestion/candidate-manifest.ts';
import {
  loadLatestTopDeckCandidateSnapshot,
  loadTopDeckCandidateSnapshot,
} from '../ingestion/load-topdeck-snapshot.ts';
import { selectTopDecksPerAvatar, type MetaDeckCandidate } from '../ingestion/select-meta-decks.ts';
import { runTwoDeckGauntlet, type GauntletDeck } from '../simulator/gauntlet.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const DEFAULT_SCENARIO = resolve(
  REPOSITORY_ROOT,
  '.local',
  'authority',
  'scenarios',
  'vanilla-constructed.json',
);

type CommandIo = Readonly<{
  stderr: (line: string) => void;
  stdout: (line: string) => void;
}>;

export type MetaDeckGauntletReport = Readonly<{
  classification: string;
  matchupCount: number;
  matchups: readonly Readonly<{
    deckA: string;
    deckB: string;
    gauntlet: Awaited<ReturnType<typeof runTwoDeckGauntlet>>;
  }>[];
  perAvatar: number;
  schemaVersion: 1;
  seeds: readonly number[];
  selection: ReturnType<typeof selectTopDecksPerAvatar>;
  snapshotPath: string;
  unsupportedManifests: readonly Readonly<{
    deckId: string;
    unsupportedCardIds: readonly string[];
  }>[];
}>;

function parseSeeds(raw: string | undefined): readonly number[] {
  if (raw === undefined) return [31, 47, 59];
  const seeds = raw.split(',').map((part) => {
    const value = Number(part.trim());
    if (!Number.isInteger(value) || value < 0 || value > 4_294_967_295) {
      throw new RangeError('seeds must be comma-separated unsigned 32-bit integers');
    }
    return value;
  });
  if (seeds.length === 0 || seeds.length > 128) {
    throw new RangeError('provide 1 through 128 seeds');
  }
  return seeds;
}

function deckLabel(candidate: MetaDeckCandidate): string {
  return `${candidate.avatarName}#${candidate.placement}@${candidate.tournamentId.slice(0, 8)}`;
}

function manifestForPair(
  authority: Awaited<ReturnType<typeof loadPrivateCardSnapshot>>,
  left: MetaDeckCandidate,
  right: MetaDeckCandidate,
  seed: number,
) {
  return buildCandidateManifest(authority, left.deck, seed, {
    north: resolvedDeckToSpec(left.deck),
    south: resolvedDeckToSpec(right.deck),
  });
}

export async function runMetaDeckGauntlet(
  argv: readonly string[] = [],
  repositoryRoot = REPOSITORY_ROOT,
  io: CommandIo = {
    stderr: (line) => process.stderr.write(line + '\n'),
    stdout: (line) => process.stdout.write(line + '\n'),
  },
): Promise<MetaDeckGauntletReport> {
  const { values } = parseArgs({
    args: [...argv],
    allowPositionals: false,
    options: {
      'output-id': { type: 'string' },
      'per-avatar': { type: 'string', default: '3' },
      scenario: { type: 'string' },
      seeds: { type: 'string' },
      snapshot: { type: 'string' },
      workers: { type: 'string', default: '2' },
    },
    strict: true,
  });

  const perAvatar = Number(values['per-avatar']);
  if (!Number.isInteger(perAvatar) || perAvatar < 1 || perAvatar > 8) {
    throw new RangeError('--per-avatar must be an integer from 1 through 8');
  }
  const workers = Number(values.workers);
  if (!Number.isInteger(workers) || workers < 1 || workers > 8) {
    throw new RangeError('--workers must be an integer from 1 through 8');
  }
  const seeds = parseSeeds(values.seeds);

  const authority = await loadPrivateCardSnapshot(
    values.scenario === undefined ? DEFAULT_SCENARIO : resolve(repositoryRoot, values.scenario),
    repositoryRoot,
  );
  const loaded = values.snapshot === undefined
    ? await loadLatestTopDeckCandidateSnapshot(repositoryRoot)
    : {
      path: resolve(repositoryRoot, values.snapshot),
      snapshot: await loadTopDeckCandidateSnapshot(resolve(repositoryRoot, values.snapshot)),
    };
  const selection = selectTopDecksPerAvatar(loaded.snapshot, authority.cards, perAvatar);
  if (selection.candidates.length < 2) {
    throw new Error(
      `Need at least two fully resolved TopDeck candidates; got ${selection.candidates.length}.`,
    );
  }

  const unsupportedManifests: Array<{ deckId: string; unsupportedCardIds: readonly string[] }> = [];
  for (const candidate of selection.candidates) {
    const built = manifestForPair(authority, candidate, candidate, seeds[0]!);
    if (built.unsupportedCardIds.length > 0 && candidate.deckId !== null) {
      unsupportedManifests.push({
        deckId: candidate.deckId,
        unsupportedCardIds: built.unsupportedCardIds,
      });
    }
  }

  const matchupPairs: Array<readonly [MetaDeckCandidate, MetaDeckCandidate]> = [];
  for (const variants of Object.values(selection.byAvatar)) {
    for (let leftIndex = 0; leftIndex < variants.length; leftIndex += 1) {
      for (let rightIndex = leftIndex + 1; rightIndex < variants.length; rightIndex += 1) {
        matchupPairs.push([variants[leftIndex]!, variants[rightIndex]!]);
      }
    }
  }
  const showcase = [...selection.candidates].sort((left, right) =>
    left.placement - right.placement
      || left.avatarStableId.localeCompare(right.avatarStableId),
  ).reduce<Map<string, MetaDeckCandidate>>((best, candidate) => {
    if (!best.has(candidate.avatarStableId)) best.set(candidate.avatarStableId, candidate);
    return best;
  }, new Map());
  const showcaseDecks = [...showcase.values()];
  for (let leftIndex = 0; leftIndex < showcaseDecks.length; leftIndex += 1) {
    for (let rightIndex = leftIndex + 1; rightIndex < showcaseDecks.length; rightIndex += 1) {
      matchupPairs.push([showcaseDecks[leftIndex]!, showcaseDecks[rightIndex]!]);
    }
  }

  const seenPairs = new Set<string>();
  const matchups: Array<MetaDeckGauntletReport['matchups'][number]> = [];
  for (const [left, right] of matchupPairs) {
    const pairKey = [left.deckId, right.deckId].sort().join('\0');
    if (seenPairs.has(pairKey)) continue;
    seenPairs.add(pairKey);
    const sample = manifestForPair(authority, left, right, seeds[0]!);
    if (sample.unsupportedCardIds.length > 0) continue;
    const leftId = deckLabel(left);
    const rightId = deckLabel(right);
    matchups.push({
      deckA: leftId,
      deckB: rightId,
      gauntlet: await runTwoDeckGauntlet({
        authority: sample.manifest.authority,
        cards: sample.manifest.cards,
        decks: [
          { deck: sample.manifest.decks.north, id: leftId },
          { deck: sample.manifest.decks.south, id: rightId },
        ],
        seeds,
      }, workers),
    });
  }

  const report: MetaDeckGauntletReport = {
    classification: matchups[0]?.gauntlet.classification ?? 'unranked_unverified_authority',
    matchupCount: matchups.length,
    matchups,
    perAvatar,
    schemaVersion: 1,
    seeds,
    selection,
    snapshotPath: relative(repositoryRoot, loaded.path).replaceAll('\\', '/'),
    unsupportedManifests,
  };

  if (values['output-id'] !== undefined) {
    const outputDir = resolve(
      repositoryRoot,
      '.local',
      'authority',
      'reports',
      'meta-deck-gauntlet',
    );
    await mkdir(outputDir, { recursive: true });
    const outputPath = resolve(outputDir, `${values['output-id']}.json`);
    await writeFile(outputPath, canonicalJson(report as unknown as JsonValue), { mode: 0o600 });
    io.stdout(canonicalJson({
      matchupCount: report.matchupCount,
      outputPath: relative(repositoryRoot, outputPath).replaceAll('\\', '/'),
      selectedDecks: report.selection.candidates.length,
      status: 'created',
      unsupportedManifests: report.unsupportedManifests.length,
    } as JsonValue));
  } else {
    io.stdout(canonicalJson({
      avatars: Object.keys(report.selection.byAvatar).length,
      classification: report.classification,
      matchupCount: report.matchupCount,
      selectedDecks: report.selection.candidates.length,
      skipped: report.selection.skipped,
      snapshotPath: report.snapshotPath,
      unsupportedManifests: report.unsupportedManifests.length,
    } as JsonValue));
  }

  return report;
}

if (process.argv[1] !== undefined && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  runMetaDeckGauntlet(process.argv.slice(2)).catch((error: unknown) => {
    process.stderr.write(`${error instanceof Error ? error.message : 'meta deck gauntlet failed'}\n`);
    process.exitCode = 1;
  });
}
