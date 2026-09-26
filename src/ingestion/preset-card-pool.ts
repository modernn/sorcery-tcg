import { z } from 'zod';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import type { PrivateCardSnapshot } from '../authority/private-cards.ts';
import { createGameManifest, type GameCardDefinition, type GameDeckSpec, type GameManifest } from '../engine/game.ts';

export type PresetCardBinding = Readonly<{
  definition: GameCardDefinition;
  presetIds: readonly string[];
}>;

/** Existing source-checked manifests supply facts; no printed-text inference or new rules. */
export function buildPresetCardPool(
  authority: PrivateCardSnapshot,
  presets: readonly Readonly<{ id: string; manifest: GameManifest }>[],
): ReadonlyMap<string, PresetCardBinding> {
  const pool = new Map<string, PresetCardBinding>();
  const known = new Set(authority.cards.map(({ stableId }) => stableId));
  for (const preset of presets) {
    const binding = preset.manifest.authority;
    if (binding.mode !== 'private-local' || binding.contentHash !== authority.authorityHash
      || binding.revisionId !== authority.revisionId) {
      throw new Error('preset card pool authority binding does not match');
    }
    for (const [cardId, definition] of Object.entries(preset.manifest.cards)) {
      if (!known.has(cardId)) throw new Error(`preset card is absent from authority: ${cardId}`);
      const existing = pool.get(cardId);
      if (existing && canonicalJson(existing.definition as JsonValue) !== canonicalJson(definition as JsonValue)) {
        throw new Error(`conflicting preset facts for card: ${cardId}`);
      }
      pool.set(cardId, {
        definition,
        presetIds: [...new Set([...(existing?.presetIds ?? []), preset.id])].sort(),
      });
    }
  }
  if (pool.size === 0) throw new Error('preset card pool is empty');
  return new Map([...pool].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0));
}

export function presetCardCatalog(authority: PrivateCardSnapshot, pool: ReadonlyMap<string, PresetCardBinding>) {
  return {
    schemaVersion: 1,
    authorityHash: authority.authorityHash,
    revisionId: authority.revisionId,
    rankedEligible: false,
    format: authority.format,
    boundCardCount: pool.size,
    cards: [...authority.cards].sort((a, b) => a.stableId < b.stableId ? -1 : a.stableId > b.stableId ? 1 : 0)
      .map((card) => {
        const binding = pool.get(card.stableId);
        return {
          cardId: card.stableId,
          name: card.name,
          cardType: card.cardType,
          rarity: card.rarity,
          rulesText: card.rulesText,
          engineSupported: binding !== undefined,
          reason: binding ? null : 'no-reviewed-binding',
          presetIds: binding?.presetIds ?? [],
          sourceCardHash: identityHash(card as unknown as JsonValue),
          factsHash: binding ? identityHash(binding.definition as JsonValue) : null,
          facts: binding?.definition ?? null,
        };
      }),
    limitations: ['Bindings reuse source-checked scenarios and reviewed local facts; unbound cards remain unsupported.',
      'Engine support is not proof of Constructed deck legality or independently verified authority.'],
  };
}

const cardIdSchema = z.string().min(1).max(256);
const zoneSchema = z.union([
  z.array(cardIdSchema).max(200),
  z.array(z.strictObject({ cardId: cardIdSchema, copies: z.number().int().min(1).max(200) }))
    .max(200).refine((rows) => rows.reduce((sum, row) => sum + row.copies, 0) <= 200,
      'a deck zone may contain at most 200 cards'),
]).transform((rows) => rows.flatMap((row) => typeof row === 'string'
  ? [row] : Array<string>(row.copies).fill(row.cardId)).sort());
const deckSchema = z.strictObject({ avatar: cardIdSchema, atlas: zoneSchema, spellbook: zoneSchema });
const requestSchema = z.strictObject({
  schemaVersion: z.literal(1),
  candidate: deckSchema,
  opponent: deckSchema,
  seeds: z.array(z.number().int().min(0).max(0xffff_ffff)).min(1).max(128).default([1]),
  workers: z.number().int().min(1).max(64).default(1),
});

/** Creates a manifest from deck IDs only. The Rust session admits it before publication. */
export function prepareBoundExperiment(
  input: unknown,
  authority: PrivateCardSnapshot,
  pool: ReadonlyMap<string, PresetCardBinding>,
) {
  const request = requestSchema.parse(input);
  const decks: Readonly<{ north: GameDeckSpec; south: GameDeckSpec }> = {
    north: request.candidate, south: request.opponent,
  };
  const referenced = new Set(Object.values(decks).flatMap((deck) => [deck.avatar, ...deck.atlas, ...deck.spellbook]));
  const unsupported = [...referenced].filter((id) => !pool.has(id)).sort();
  if (unsupported.length) throw new Error(`unsupported cards (no reviewed binding): ${unsupported.join(', ')}`);
  // Include engine token dependencies, including tokens outside the selected deck zones.
  for (const id of referenced) {
    const facts = pool.get(id)!.definition;
    const tokenId = facts.cardType === 'magic'
      ? facts.summonTokenToAlliedMinionThenDrawSpell ?? facts.summonTokenToEachControlledSiteBorderingEnemySite
      : facts.cardType === 'site' ? facts.genesisPayOneManaToSummonToken : undefined;
    if (tokenId) {
      if (!pool.has(tokenId)) throw new Error(`unsupported token dependency: ${tokenId}`);
      referenced.add(tokenId);
    }
  }
  const baseManifest = createGameManifest({
    authority: { mode: 'private-local', contentHash: authority.authorityHash, revisionId: authority.revisionId },
    cards: Object.fromEntries([...referenced].map((id) => [id, pool.get(id)!.definition])),
    decks, firstSeat: 'north', seed: request.seeds[0]!,
  });
  return { ...request, baseManifest };
}
