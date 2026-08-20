import { identityHash, sha256 } from './hash.ts';
import {
  AuthorityValidationError,
  DEFAULT_AUTHORITY_JSON_LIMITS,
  createCanonicalArtifact,
  parseAuthorityJson,
  validateNormalizedCardSnapshot,
  validateRawCardSnapshot,
  validateSourceRecord,
  type CanonicalArtifact,
  type NormalizedCard,
  type NormalizedCardSnapshot,
  type RawCard,
  type SourceMetadata,
} from './schemas.ts';

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function deepFreeze<T>(value: T): T {
  if (value === null || typeof value !== 'object') return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

function stableHash(prefix: 'card' | 'card-snapshot', fields: Readonly<Record<string, string>>): string {
  return prefix + ':' + identityHash(fields).slice('sha256:'.length);
}

function normalizeCard(card: RawCard, source: SourceMetadata): NormalizedCard {
  return {
    stableId: stableHash('card', {
      sourceId: source.sourceId,
      sourceCardId: card.sourceCardId,
    }),
    officialSourceId: source.authorityClass === 'official' ? card.sourceCardId : null,
    name: card.name,
    cardType: card.cardType,
    elements: [...card.elements],
    rarity: card.rarity,
    manaCost: card.manaCost,
    attack: card.attack,
    defense: card.defense,
    rulesText: card.rulesText,
    printingSlugs: [...card.printingSlugs],
  };
}

export function normalizeCards(
  rawBytes: Uint8Array,
  sourceMetadata: SourceMetadata,
): CanonicalArtifact<NormalizedCardSnapshot> {
  const source = validateSourceRecord(sourceMetadata);
  const byteHash = sha256(rawBytes);
  if (source.byteHash !== byteHash) {
    throw new AuthorityValidationError([
      {
        path: '/byteHash',
        code: 'byte_hash_mismatch',
        message: 'source byteHash does not match the supplied raw bytes',
      },
    ]);
  }

  const rawSnapshot = validateRawCardSnapshot(
    parseAuthorityJson(rawBytes, DEFAULT_AUTHORITY_JSON_LIMITS),
  );
  const cards = rawSnapshot.cards.map((card) => normalizeCard(card, source));
  cards.sort((left, right) => compareText(left.stableId, right.stableId));

  const snapshot = validateNormalizedCardSnapshot({ cards });
  if (snapshot.cards.length !== rawSnapshot.cards.length) {
    throw new AuthorityValidationError([
      {
        path: '/cards',
        code: 'cardinality_mismatch',
        message: 'normalized card count does not match raw card count',
      },
    ]);
  }

  return deepFreeze(createCanonicalArtifact({
    artifactKind: 'card-snapshot',
    stableId: stableHash('card-snapshot', { sourceId: source.sourceId }),
    schemaVersion: 1,
    parentRefs: [],
    sourceRefs: [{ sourceId: source.sourceId, byteHash }],
    payload: snapshot,
  }));
}

