import { z } from 'zod';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import type { PrivateCardSnapshot } from '../authority/private-cards.ts';
import { AuthorityValidationError } from '../authority/schemas.ts';
import { readBoundedWithinAuthorityRoot } from '../authority/validate-bundle.ts';
import { validateCardDefinition, type GameCardDefinition } from '../engine/game.ts';
import type { PresetCardBinding } from './preset-card-pool.ts';

const hash = z.string().regex(/^sha256:[a-f0-9]{64}$/u);
const characteristicSupplement = z.strictObject({
  elements: z.array(z.enum(['earth', 'fire', 'water', 'air'])).max(4).optional(),
  subtypes: z.array(z.string().min(1).max(64)).max(16).optional(),
  sources: z.array(z.strictObject({ contentHash: hash, locator: z.string().min(1).max(512) })).min(1).max(16),
}).refine((value) => value.elements !== undefined || value.subtypes !== undefined,
  'a characteristic supplement must supply elements or subtypes');
const bindingFile = z.strictObject({
  schemaVersion: z.literal(1),
  authorityHash: hash,
  revisionId: z.string().min(1).max(128),
  cards: z.array(z.strictObject({
    cardId: z.string().min(1).max(256),
    sourceCardHash: hash,
    replacesFactsHash: hash.optional(),
    characteristicSupplement: characteristicSupplement.optional(),
    facts: z.record(z.string(), z.unknown()),
    review: z.strictObject({
      entireRulesText: z.literal(true),
      proofs: z.array(z.string().min(1).max(512)).min(1).max(32),
    }),
  })).max(10_000),
});

/** Reviewed local data binds cards to existing engine facts, never to executable code.
 * A replacement must name the exact canonical hash of an existing preset fact row;
 * this makes moving a card from scenario facts to reviewed facts explicit and fail closed.
 * Review references are provenance, not a claim of verified authority or ranked eligibility.
 */
export function mergeReviewedCardBindings(
  input: unknown,
  authority: PrivateCardSnapshot,
  existing: ReadonlyMap<string, PresetCardBinding>,
): ReadonlyMap<string, PresetCardBinding> {
  const file = bindingFile.parse(input);
  if (file.authorityHash !== authority.authorityHash || file.revisionId !== authority.revisionId) {
    throw new Error('reviewed binding authority does not match');
  }
  const cards = new Map(authority.cards.map((card) => [card.stableId, card]));
  const pool = new Map(existing);
  const seen = new Set<string>();
  for (const row of file.cards) {
    const source = cards.get(row.cardId);
    if (!source || identityHash(source as unknown as JsonValue) !== row.sourceCardHash) {
      throw new Error(`reviewed binding source changed or is absent: ${row.cardId}`);
    }
    if (seen.has(row.cardId)) throw new Error(`duplicate reviewed card: ${row.cardId}`);
    seen.add(row.cardId);
    const definition = row.facts as GameCardDefinition;
    validateCardDefinition(definition, `reviewed.${row.cardId}`);
    const fields = ['cardType', ...(source.cardType === 'site' ? ['elements']
      : source.cardType === 'avatar' ? ['attack', 'defense', 'life']
        : source.cardType === 'minion' ? ['attack', 'defense', 'manaCost', 'thresholds']
          : ['manaCost', 'thresholds'])];
    for (const key of fields) {
      if (canonicalJson(row.facts[key] as JsonValue)
        !== canonicalJson(source[key as keyof typeof source] as JsonValue)) {
        throw new Error(`reviewed binding differs from source ${key}: ${row.cardId}`);
      }
    }
    if ((source.cardType === 'minion' || source.cardType === 'site')
      && (row.facts.ordinary === true) !== (source.rarity === 'ordinary')) {
      throw new Error(`reviewed binding differs from source ordinary: ${row.cardId}`);
    }
    if (row.characteristicSupplement && source.cardType !== 'minion') {
      throw new Error(`characteristic supplements currently require a minion: ${row.cardId}`);
    }
    if (source.cardType === 'minion' && definition.cardType === 'minion') {
      // Supplements can fill omissions, never remove retained source characteristics.
      // Their references record a human review; they do not independently certify authority.
      const supplement = row.characteristicSupplement;
      const subtypes = [...new Set([...source.subtypes, ...(supplement?.subtypes ?? [])])]
        .sort((a, b) => Buffer.compare(Buffer.from(a), Buffer.from(b)));
      const allElements = new Set([...source.elements, ...(supplement?.elements ?? [])]);
      const elements = (['earth', 'fire', 'water', 'air'] as const).filter((element) => allElements.has(element));
      for (const [key, supplied, expected, supplemented] of [
        ['subtypes', definition.subtypes, subtypes, supplement?.subtypes !== undefined],
        ['elements', definition.elements, elements, supplement?.elements !== undefined],
      ] as const) {
        if ((supplied !== undefined || supplemented)
          && (supplied === undefined || canonicalJson(supplied as JsonValue) !== canonicalJson(expected))) {
          throw new Error(`reviewed binding differs from source ${key}: ${row.cardId}`);
        }
      }
      for (const [key, name] of [['mortal', 'Mortal'], ['undead', 'Undead'], ['demon', 'Demon']] as const) {
        const actual = definition.subtypes?.includes(name) ?? row.facts[key] === true;
        // Preserve old Demon review behavior for legacy facts without a complete subtype list.
        if ((key !== 'demon' || definition.subtypes !== undefined)
          && actual !== subtypes.includes(name)) {
          throw new Error(`reviewed binding differs from source ${key}: ${row.cardId}`);
        }
      }
    }
    const previous = pool.get(row.cardId);
    if (row.replacesFactsHash !== undefined) {
      if (previous === undefined
        || identityHash(previous.definition as JsonValue) !== row.replacesFactsHash) {
        throw new Error(`reviewed replacement does not match existing preset facts: ${row.cardId}`);
      }
    } else if (previous && canonicalJson(previous.definition as JsonValue) !== canonicalJson(definition as JsonValue)) {
      throw new Error(`conflicting reviewed facts: ${row.cardId}`);
    }
    pool.set(row.cardId, { definition,
      presetIds: [...new Set([...(previous?.presetIds ?? []), 'reviewed-local'])].sort() });
  }
  return new Map([...pool].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0));
}

export async function loadReviewedCardBindings(
  authorityRoot: string,
  authority: PrivateCardSnapshot,
  existing: ReadonlyMap<string, PresetCardBinding>,
): Promise<ReadonlyMap<string, PresetCardBinding>> {
  let read;
  try {
    read = await readBoundedWithinAuthorityRoot(authorityRoot, 'bindings/reviewed.json', 4_194_304);
  } catch (error) {
    if (error instanceof AuthorityValidationError
      && error.diagnostics.every(({ code }) => code === 'path_not_found')) return existing;
    throw error;
  }
  if (read.status !== 'ok') throw new Error('reviewed bindings must be a private JSON file of at most 4 MiB');
  return mergeReviewedCardBindings(parseJsonWithDuplicateKeyCheck(Buffer.from(read.bytes).toString('utf8')),
    authority, existing);
}
