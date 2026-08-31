import { link, mkdir, unlink, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { randomUUID } from 'node:crypto';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import type { TopDeckDeck, TopDeckIngestion } from './topdeck.ts';

export type TopDeckCandidateSnapshot = Readonly<{
  contentHash: ReturnType<typeof identityHash>;
  schemaVersion: 1;
  source: TopDeckIngestion['source'];
  tournaments: readonly Readonly<{
    candidates: readonly Readonly<{
      deck: Readonly<Pick<TopDeckDeck, 'opaqueStructuredDeck' | 'rows' | 'sourceUrl'>>;
      resultEvidence: Readonly<{ placement: number }>;
    }>[];
    participantCount: number;
    startedAt: string;
    tournamentId: string;
  }>[];
}>;

export function createTopDeckCandidateSnapshot(
  ingestion: TopDeckIngestion,
): TopDeckCandidateSnapshot {
  const body = {
    schemaVersion: 1,
    source: ingestion.source,
    tournaments: ingestion.tournaments.map((tournament) => ({
      candidates: tournament.placements.flatMap(({ deck, placement }) => deck === null ? [] : [{
        deck: {
          opaqueStructuredDeck: deck.opaqueStructuredDeck,
          rows: deck.rows,
          sourceUrl: deck.sourceUrl,
        },
        resultEvidence: { placement },
      }]),
      participantCount: tournament.participantCount,
      startedAt: tournament.startedAt,
      tournamentId: tournament.tournamentId,
    })),
  } as const;
  return { ...body, contentHash: identityHash(body as unknown as JsonValue) };
}

export async function publishTopDeckCandidateSnapshot(
  snapshot: TopDeckCandidateSnapshot,
  repositoryRoot = process.cwd(),
): Promise<string> {
  const outputRoot = resolve(repositoryRoot, '.local', 'authority', 'topdeck-candidates');
  await mkdir(outputRoot, { recursive: true });
  const digest = snapshot.contentHash.slice('sha256:'.length);
  const destination = join(outputRoot, `topdeck-candidates-${digest}.json`);
  const temporary = join(outputRoot, `.${digest}.${randomUUID()}.tmp`);
  await writeFile(temporary, canonicalJson(snapshot as unknown as JsonValue), { flag: 'wx', mode: 0o600 });
  try {
    await link(temporary, destination);
  } finally {
    await unlink(temporary).catch(() => undefined);
  }
  return destination;
}
