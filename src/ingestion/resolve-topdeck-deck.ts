import { identityHash } from '../authority/hash.ts';
import type { JsonValue } from '../authority/canonical-json.ts';
import { createCardNameIndex, type CardNameIndex, type CardNameMappingStatus } from './card-name-index.ts';
import type { NormalizedCard } from '../authority/schemas.ts';
import type { TopDeckDeck, TopDeckDeckRow } from './topdeck.ts';

export type DeckZone = 'avatar' | 'atlas' | 'spellbook';

export type ResolvedDeckRow = Readonly<{
  mappingStatus: CardNameMappingStatus;
  quantity: number;
  sourceCardName: string;
  stableId: string | null;
  zone: DeckZone;
}>;

export type ResolvedTopDeckDeck = Readonly<{
  avatarName: string | null;
  avatarStableId: string | null;
  atlas: readonly ResolvedDeckRow[];
  deckId: ReturnType<typeof identityHash> | null;
  deckResolvable: boolean;
  spellbook: readonly ResolvedDeckRow[];
  unresolvedRows: number;
}>;

const ZONE_BY_SECTION: Readonly<Record<string, DeckZone | undefined>> = {
  Atlas: 'atlas',
  Avatar: 'avatar',
  Spellbook: 'spellbook',
};

function zoneForSection(section: string | null): DeckZone | null {
  if (section === null) return null;
  return ZONE_BY_SECTION[section] ?? null;
}

function resolveRow(
  index: CardNameIndex,
  zone: DeckZone,
  sourceCardName: string,
  quantity: number,
): ResolvedDeckRow {
  const lookup = index.lookup(sourceCardName);
  return {
    mappingStatus: lookup.status,
    quantity,
    sourceCardName,
    stableId: lookup.status === 'resolved' ? lookup.cards[0]!.stableId : null,
    zone,
  };
}

function rowsFromOpaqueDeck(
  index: CardNameIndex,
  opaque: Readonly<Record<string, JsonValue>>,
): readonly ResolvedDeckRow[] {
  const rows: ResolvedDeckRow[] = [];
  for (const [section, value] of Object.entries(opaque)) {
    const zone = ZONE_BY_SECTION[section];
    if (zone === undefined || typeof value !== 'object' || value === null || Array.isArray(value)) {
      continue;
    }
    for (const [sourceCardName, quantityValue] of Object.entries(value)) {
      if (typeof quantityValue !== 'number' || !Number.isInteger(quantityValue) || quantityValue < 1) {
        continue;
      }
      rows.push(resolveRow(index, zone, sourceCardName, quantityValue));
    }
  }
  return rows;
}

function rowsFromTextDeck(index: CardNameIndex, deckRows: readonly TopDeckDeckRow[]): readonly ResolvedDeckRow[] {
  const rows: ResolvedDeckRow[] = [];
  for (const row of deckRows) {
    if (row.parseStatus !== 'parsed' || row.quantity === null || row.sourceCardName === null) continue;
    const zone = zoneForSection(row.section);
    if (zone === null) continue;
    rows.push(resolveRow(index, zone, row.sourceCardName, row.quantity));
  }
  return rows;
}

function mergeRows(rows: readonly ResolvedDeckRow[]): readonly ResolvedDeckRow[] {
  const merged = new Map<string, ResolvedDeckRow>();
  for (const row of rows) {
    const key = `${row.zone}\0${row.sourceCardName.toLocaleLowerCase('en-US')}`;
    const existing = merged.get(key);
    if (existing === undefined) {
      merged.set(key, row);
      continue;
    }
    merged.set(key, {
      ...existing,
      mappingStatus: existing.mappingStatus === 'resolved' && row.mappingStatus === 'resolved'
        ? 'resolved'
        : existing.mappingStatus === 'unresolved' || row.mappingStatus === 'unresolved'
          ? 'unresolved'
          : 'ambiguous',
      quantity: existing.quantity + row.quantity,
      stableId: existing.stableId !== null && existing.stableId === row.stableId
        ? existing.stableId
        : null,
    });
  }
  return [...merged.values()].sort((left, right) =>
    left.zone.localeCompare(right.zone)
      || left.sourceCardName.localeCompare(right.sourceCardName),
  );
}

function countedZone(rows: readonly ResolvedDeckRow[], zone: DeckZone): readonly ResolvedDeckRow[] {
  return rows.filter((row) => row.zone === zone);
}

function deckIdentity(rows: readonly ResolvedDeckRow[]): ReturnType<typeof identityHash> | null {
  const resolved = rows.filter((row) => row.stableId !== null);
  if (resolved.length !== rows.length) return null;
  const atlas = countedZone(resolved, 'atlas')
    .map(({ quantity, stableId }) => ({ cardId: stableId!, copies: quantity }))
    .sort((left, right) => left.cardId.localeCompare(right.cardId));
  const spellbook = countedZone(resolved, 'spellbook')
    .map(({ quantity, stableId }) => ({ cardId: stableId!, copies: quantity }))
    .sort((left, right) => left.cardId.localeCompare(right.cardId));
  const avatar = countedZone(resolved, 'avatar');
  if (avatar.length !== 1 || avatar[0]!.quantity !== 1 || avatar[0]!.stableId === null) return null;
  return identityHash({
    avatar: avatar[0]!.stableId,
    atlas,
    spellbook,
  } as JsonValue);
}

export function resolveTopDeckDeck(
  deck: TopDeckDeck,
  cards: readonly NormalizedCard[],
): ResolvedTopDeckDeck {
  const index = createCardNameIndex(cards);
  const merged = mergeRows([
    ...(deck.opaqueStructuredDeck === null ? [] : rowsFromOpaqueDeck(index, deck.opaqueStructuredDeck)),
    ...rowsFromTextDeck(index, deck.rows),
  ]);
  const avatarRows = countedZone(merged, 'avatar');
  const avatarRow = avatarRows.length === 1 ? avatarRows[0]! : null;
  const unresolvedRows = merged.filter((row) => row.mappingStatus !== 'resolved').length;
  const deckResolvable = avatarRow?.mappingStatus === 'resolved'
    && avatarRow.quantity === 1
    && unresolvedRows === 0;
  return {
    avatarName: avatarRow?.sourceCardName ?? null,
    avatarStableId: avatarRow?.stableId ?? null,
    atlas: countedZone(merged, 'atlas'),
    deckId: deckIdentity(merged),
    deckResolvable,
    spellbook: countedZone(merged, 'spellbook'),
    unresolvedRows,
  };
}
