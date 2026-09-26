import { baselineGameDefinition } from '../commands/run-private-game-check.ts';
import {
  createGameManifest,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameManifest,
  type GameSeat,
} from '../engine/game.ts';
import type { PrivateCardSnapshot } from '../authority/private-cards.ts';
import type { ResolvedTopDeckDeck } from './resolve-topdeck-deck.ts';

export type CandidateManifestDiagnostic = Readonly<{
  cardId: string;
  reason: 'missing-card' | 'unbound-rules-text' | 'unsupported-card-facts';
}>;

export type CandidateManifestBuild = Readonly<{
  cardNames: Readonly<Record<string, string>>;
  diagnostics: readonly CandidateManifestDiagnostic[];
  unsupportedCardIds: readonly string[];
}> & (
  | Readonly<{ supported: true; manifest: GameManifest }>
  | Readonly<{ supported: false; manifest: null }>
);

function expandZone(rows: readonly ResolvedTopDeckDeck['atlas'][number][]): readonly string[] {
  return rows.flatMap((row) => {
    if (row.stableId === null) throw new Error('resolved deck row is missing stableId');
    return Array.from({ length: row.quantity }, () => row.stableId!);
  });
}

export function resolvedDeckToSpec(deck: ResolvedTopDeckDeck): GameDeckSpec {
  if (!deck.deckResolvable || deck.avatarStableId === null) {
    throw new Error('cannot build a deck spec from an unresolved TopDeck candidate');
  }
  return {
    atlas: expandZone(deck.atlas),
    avatar: deck.avatarStableId,
    spellbook: expandZone(deck.spellbook),
  };
}

export function buildCandidateManifest(
  authority: PrivateCardSnapshot,
  deck: ResolvedTopDeckDeck,
  seed: number,
  seats: Readonly<{ north: GameDeckSpec; south: GameDeckSpec }>,
  firstSeat: GameSeat = 'north',
): CandidateManifestBuild {
  const cardsById = new Map(authority.cards.map((card) => [card.stableId, card]));
  const referenced = new Set<string>([
    seats.north.avatar,
    seats.south.avatar,
    ...seats.north.atlas,
    ...seats.north.spellbook,
    ...seats.south.atlas,
    ...seats.south.spellbook,
  ]);
  const diagnostics: CandidateManifestDiagnostic[] = [];
  const definitions: Record<string, GameCardDefinition> = {};
  const cardNames: Record<string, string> = {};
  for (const stableId of referenced) {
    const card = cardsById.get(stableId);
    if (card === undefined) {
      diagnostics.push({ cardId: stableId, reason: 'missing-card' });
      continue;
    }
    cardNames[stableId] = card.name;
    // Baseline facts do not bind printed abilities. Never simulate them as blank cards.
    if (card.rulesText.trim() !== '') {
      diagnostics.push({ cardId: stableId, reason: 'unbound-rules-text' });
      continue;
    }
    try {
      definitions[stableId] = baselineGameDefinition(card, false);
    } catch {
      diagnostics.push({ cardId: stableId, reason: 'unsupported-card-facts' });
    }
  }
  const unsupportedCardIds = diagnostics.map(({ cardId }) => cardId);
  if (diagnostics.length > 0) {
    return { cardNames, diagnostics, manifest: null, supported: false, unsupportedCardIds };
  }
  return {
    cardNames,
    diagnostics,
    manifest: createGameManifest({
      authority: {
        contentHash: authority.authorityHash,
        mode: 'private-local',
        revisionId: authority.revisionId,
      },
      cards: definitions,
      decks: seats,
      firstSeat,
      seed,
    }),
    supported: true,
    unsupportedCardIds,
  };
}

export function buildSingleCandidateManifest(
  authority: PrivateCardSnapshot,
  deck: ResolvedTopDeckDeck,
  seed: number,
  opponent: GameDeckSpec,
  candidateSeat: GameSeat = 'north',
): CandidateManifestBuild {
  const candidate = resolvedDeckToSpec(deck);
  const seats = candidateSeat === 'north'
    ? { north: candidate, south: opponent }
    : { north: opponent, south: candidate };
  return buildCandidateManifest(authority, deck, seed, seats, 'north');
}
