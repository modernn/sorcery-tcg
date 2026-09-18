import type { TopDeckCandidateSnapshot } from './topdeck-candidates.ts';
import { resolveTopDeckDeck, type ResolvedTopDeckDeck } from './resolve-topdeck-deck.ts';
import type { NormalizedCard } from '../authority/schemas.ts';

export type MetaDeckCandidate = Readonly<{
  avatarName: string;
  avatarStableId: string;
  deck: ResolvedTopDeckDeck;
  deckId: NonNullable<ResolvedTopDeckDeck['deckId']>;
  placement: number;
  sourceUrl: string | null;
  tournamentId: string;
}>;

export type MetaDeckSelection = Readonly<{
  byAvatar: Readonly<Record<string, readonly MetaDeckCandidate[]>>;
  candidates: readonly MetaDeckCandidate[];
  skipped: Readonly<{
    missingDeck: number;
    unresolvedDeck: number;
  }>;
}>;

function comparePlacement(left: MetaDeckCandidate, right: MetaDeckCandidate): number {
  return left.placement - right.placement
    || left.tournamentId.localeCompare(right.tournamentId)
    || left.deckId.localeCompare(right.deckId);
}

export function selectTopDecksPerAvatar(
  snapshot: TopDeckCandidateSnapshot,
  cards: readonly NormalizedCard[],
  perAvatar = 3,
): MetaDeckSelection {
  if (!Number.isInteger(perAvatar) || perAvatar < 1 || perAvatar > 8) {
    throw new RangeError('perAvatar must be an integer from 1 through 8');
  }

  const skipped = { missingDeck: 0, unresolvedDeck: 0 };
  const resolved: MetaDeckCandidate[] = [];
  for (const tournament of snapshot.tournaments) {
    for (const candidate of tournament.candidates) {
      if (candidate.deck.rows.length === 0 && candidate.deck.opaqueStructuredDeck === null) {
        skipped.missingDeck += 1;
        continue;
      }
      const deck = resolveTopDeckDeck(candidate.deck, cards);
      if (!deck.deckResolvable || deck.deckId === null || deck.avatarStableId === null || deck.avatarName === null) {
        skipped.unresolvedDeck += 1;
        continue;
      }
      resolved.push({
        avatarName: deck.avatarName,
        avatarStableId: deck.avatarStableId,
        deck,
        deckId: deck.deckId,
        placement: candidate.resultEvidence.placement,
        sourceUrl: candidate.deck.sourceUrl,
        tournamentId: tournament.tournamentId,
      });
    }
  }

  resolved.sort(comparePlacement);

  const byAvatar = new Map<string, MetaDeckCandidate[]>();
  for (const candidate of resolved) {
    const bucket = byAvatar.get(candidate.avatarStableId) ?? [];
    if (bucket.some((existing) => existing.deckId === candidate.deckId)) continue;
    if (bucket.length >= perAvatar) continue;
    bucket.push(candidate);
    byAvatar.set(candidate.avatarStableId, bucket);
  }

  const grouped = Object.fromEntries([...byAvatar.entries()].sort(([left], [right]) =>
    left.localeCompare(right),
  ));
  return {
    byAvatar: grouped,
    candidates: Object.values(grouped).flat(),
    skipped,
  };
}
