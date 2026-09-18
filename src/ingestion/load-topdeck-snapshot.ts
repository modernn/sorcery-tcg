import { readFile, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import type { TopDeckCandidateSnapshot } from './topdeck-candidates.ts';

const SNAPSHOT_PREFIX = 'topdeck-candidates-';
const SNAPSHOT_SUFFIX = '.json';

function isJsonRecord(value: JsonValue): value is Record<string, JsonValue> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseSnapshot(value: JsonValue): TopDeckCandidateSnapshot {
  if (!isJsonRecord(value)) {
    throw new Error('TopDeck candidate snapshot must be an object');
  }
  if (value.schemaVersion !== 1 || typeof value.contentHash !== 'string') {
    throw new Error('TopDeck candidate snapshot has an unsupported schema');
  }
  return value as TopDeckCandidateSnapshot;
}

export async function loadTopDeckCandidateSnapshot(path: string): Promise<TopDeckCandidateSnapshot> {
  return parseSnapshot(parseJsonWithDuplicateKeyCheck(await readFile(path, 'utf8')));
}

export async function loadLatestTopDeckCandidateSnapshot(
  repositoryRoot = process.cwd(),
): Promise<Readonly<{ path: string; snapshot: TopDeckCandidateSnapshot }>> {
  const directory = resolve(repositoryRoot, '.local', 'authority', 'topdeck-candidates');
  const entries = (await readdir(directory, { withFileTypes: true }))
    .filter((entry) =>
      entry.isFile()
        && entry.name.startsWith(SNAPSHOT_PREFIX)
        && entry.name.endsWith(SNAPSHOT_SUFFIX),
    )
    .map((entry) => entry.name)
    .sort((left, right) => right.localeCompare(left));
  const latest = entries[0];
  if (latest === undefined) {
    throw new Error('No TopDeck candidate snapshots found; run pnpm decks:import-topdeck first.');
  }
  const path = join(directory, latest);
  return { path, snapshot: await loadTopDeckCandidateSnapshot(path) };
}
