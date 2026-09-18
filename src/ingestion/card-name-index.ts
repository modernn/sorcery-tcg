import type { NormalizedCard } from '../authority/schemas.ts';

export type CardNameMappingStatus = 'resolved' | 'ambiguous' | 'unresolved';

export type CardNameIndex = Readonly<{
  lookup: (name: string) => Readonly<{ cards: readonly NormalizedCard[]; status: CardNameMappingStatus }>;
}>;

function normalizeName(name: string): string {
  return name.trim().replace(/\s+/gu, ' ').toLocaleLowerCase('en-US');
}

export function createCardNameIndex(cards: readonly NormalizedCard[]): CardNameIndex {
  const byName = new Map<string, NormalizedCard[]>();
  for (const card of cards) {
    const key = normalizeName(card.name);
    const bucket = byName.get(key);
    if (bucket === undefined) byName.set(key, [card]);
    else bucket.push(card);
  }
  for (const bucket of byName.values()) {
    bucket.sort((left, right) => left.stableId.localeCompare(right.stableId));
  }
  return {
    lookup(name: string) {
      const matches = byName.get(normalizeName(name)) ?? [];
      if (matches.length === 0) return { cards: [], status: 'unresolved' };
      if (matches.length === 1) return { cards: matches, status: 'resolved' };
      return { cards: matches, status: 'ambiguous' };
    },
  };
}
