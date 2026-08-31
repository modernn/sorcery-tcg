import { identityHash } from '../authority/hash.ts';
import type { JsonValue } from '../authority/canonical-json.ts';
import type { NormalizedCard } from '../authority/schemas.ts';

export const MECHANIC_WORKLOAD_CLASSIFIER_VERSION = 'generic-cues-v1' as const;

export const EFFECT_FAMILY_IDS = [
  'artifact-token-carry',
  'combat-projectiles',
  'control-object-identity',
  'damage-life-death',
  'movement-regions',
  'resource-casting',
  'site-terrain-domain',
  'status-ability-modifiers',
  'unit-placement-summoning',
  'zones-card-flow',
] as const;

export const FACET_IDS = [
  'randomness',
  'spatial-scope',
  'target-choice',
  'trigger-timing',
] as const;

export type EffectFamilyId = typeof EFFECT_FAMILY_IDS[number];
export type MechanicFacetId = typeof FACET_IDS[number];
export type MechanicWorkloadCardInput = Pick<
  NormalizedCard,
  'cardType' | 'name' | 'rulesText' | 'stableId'
>;

export type PresetCardDemand = Readonly<{
  northCopies: number;
  presetId: string;
  southCopies: number;
  stableId: string;
}>;

type CueDefinition<Id extends string> = Readonly<{
  cues: readonly string[];
  id: Id;
}>;

const EFFECT_FAMILIES: readonly CueDefinition<EffectFamilyId>[] = [
  { id: 'artifact-token-carry', cues: ['artifact', 'artifacts', 'bearer', 'carry', 'carried', 'drop', 'token'] },
  { id: 'combat-projectiles', cues: ['attack', 'attacked', 'attacking', 'charge', 'defend', 'defended', 'fight', 'intercept', 'lance', 'projectile', 'projectiles', 'ranged', 'shoot', 'strike', 'strikes', 'striking'] },
  { id: 'control-object-identity', cues: ['control', 'controlled', 'controller', 'copy', 'named', 'owner', 'ownership', 'same name'] },
  { id: 'damage-life-death', cues: ['banish', 'banished', 'cemetery', 'damage', 'damaged', 'dead', 'deathrite', 'destroy', 'destroyed', 'destroys', 'die', 'dies', 'heal', 'healing', 'kill', 'killed', 'lethal', 'life'] },
  { id: 'movement-regions', cues: ['airborne', 'burrow', 'burrowing', 'fly', 'move', 'moved', 'movement', 'path', 'pull', 'region', 'step', 'steps', 'submerge', 'void', 'voids', 'voidwalk', 'waterbound'] },
  { id: 'resource-casting', cues: ['affinity', 'cast', 'cost', 'magic', 'mana', 'pay', 'play', 'provide', 'provides', 'spellcaster', 'spellcasters', 'threshold'] },
  { id: 'site-terrain-domain', cues: ['domain', 'elements', 'land', 'realm', 'rubble', 'site', 'sites', 'terrain', 'water'] },
  { id: 'status-ability-modifiers', cues: ['abilities', 'ability', 'disable', 'disabled', 'halved', 'immobile', 'immobilize', 'modified', 'power', 'silence', 'silenced', 'stealth', 'tap', 'untap', 'untaps', 'ward'] },
  { id: 'unit-placement-summoning', cues: ['enter', 'entered', 'enters', 'place', 'placed', 'summon', 'summoned', 'summons', 'token'] },
  { id: 'zones-card-flow', cues: ['atlas', 'bottom', 'cemetery', 'deck', 'discard', 'draw', 'hand', 'spellbook'] },
];

const FACETS: readonly CueDefinition<MechanicFacetId>[] = [
  { id: 'randomness', cues: ['random', 'randomly', 'roll'] },
  { id: 'spatial-scope', cues: ['adjacent', 'atop', 'beneath', 'cardinal', 'column', 'connected', 'diagonal', 'distance', 'edges', 'here', 'location', 'locations', 'nearby', 'occupies', 'occupying', 'opposite', 'path', 'realm', 'region', 'row', 'site', 'sites'] },
  { id: 'target-choice', cues: ['choose', 'chosen', 'target', 'targeted'] },
  { id: 'trigger-timing', cues: ['after', 'deathrite', 'end', 'genesis', 'next', 'once', 'start', 'turn', 'until', 'when', 'whenever'] },
];

function cuePattern(cues: readonly string[]): RegExp {
  return new RegExp(`\\b(?:${cues.join('|')})\\b`, 'iu');
}

const FAMILY_PATTERNS = EFFECT_FAMILIES.map(({ cues, id }) => ({ id, pattern: cuePattern(cues) }));
const FACET_PATTERNS = FACETS.map(({ cues, id }) => ({ id, pattern: cuePattern(cues) }));

function label(id: string): string {
  return id.split('-').map((part) => part[0]!.toUpperCase() + part.slice(1)).join(' ');
}

export type MechanicWorkloadReport = Readonly<{
  cards: readonly Readonly<{
    cardType: NormalizedCard['cardType'];
    classification: 'blank' | 'classified' | 'unclassified';
    facetIds: readonly MechanicFacetId[];
    familyIds: readonly EffectFamilyId[];
    name: string;
    presetDemand: readonly Readonly<{
      northCopies: number;
      presetId: string;
      southCopies: number;
      totalCopies: number;
    }>[];
    presetDemandStatus: 'absent' | 'present';
    ruleDigest: ReturnType<typeof identityHash>;
    stableId: string;
  }>[];
  classifierVersion: typeof MECHANIC_WORKLOAD_CLASSIFIER_VERSION;
  facets: readonly WorkloadLabelSummary<MechanicFacetId>[];
  families: readonly WorkloadLabelSummary<EffectFamilyId>[];
  schemaVersion: 1;
  scopeDisclaimer: 'Lexical workload labels and preset demand only; this report proves neither engine implementation nor scenario verification.';
  totals: Readonly<{
    blank: number;
    cards: number;
    classified: number;
    presetDemandAbsent: number;
    presetDemandPresent: number;
    unclassified: number;
  }>;
}>;

type WorkloadLabelSummary<Id extends string> = Readonly<{
  cardCount: number;
  deckCopies: number;
  id: Id;
  label: string;
  presetDemandCardCount: number;
  presetIds: readonly string[];
}>;

function nonnegativeCopies(value: number, path: string): void {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new RangeError(`${path} must be a nonnegative safe integer`);
  }
}

export function buildMechanicWorkload(
  inputCards: readonly MechanicWorkloadCardInput[],
  demand: readonly PresetCardDemand[] = [],
): MechanicWorkloadReport {
  const cards = [...inputCards].sort((left, right) => left.stableId.localeCompare(right.stableId));
  if (new Set(cards.map(({ stableId }) => stableId)).size !== cards.length) {
    throw new RangeError('mechanic workload cards must have unique stable IDs');
  }
  const cardIds = new Set(cards.map(({ stableId }) => stableId));
  const demandByCard = new Map<string, Map<string, { northCopies: number; southCopies: number }>>();
  for (const [index, row] of demand.entries()) {
    if (!cardIds.has(row.stableId)) {
      throw new RangeError(`preset demand ${index} references an unknown stable ID`);
    }
    nonnegativeCopies(row.northCopies, `preset demand ${index}.northCopies`);
    nonnegativeCopies(row.southCopies, `preset demand ${index}.southCopies`);
    const byPreset = demandByCard.get(row.stableId) ?? new Map();
    const existing = byPreset.get(row.presetId) ?? { northCopies: 0, southCopies: 0 };
    byPreset.set(row.presetId, {
      northCopies: existing.northCopies + row.northCopies,
      southCopies: existing.southCopies + row.southCopies,
    });
    demandByCard.set(row.stableId, byPreset);
  }

  const reportCards = cards.map((card) => {
    const blank = card.rulesText.trim().length === 0;
    const familyIds = blank
      ? []
      : FAMILY_PATTERNS.filter(({ pattern }) => pattern.test(card.rulesText)).map(({ id }) => id);
    const facetIds = blank
      ? []
      : FACET_PATTERNS.filter(({ pattern }) => pattern.test(card.rulesText)).map(({ id }) => id);
    const presetDemand = [...(demandByCard.get(card.stableId)?.entries() ?? [])]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([presetId, copies]) => ({
        ...copies,
        presetId,
        totalCopies: copies.northCopies + copies.southCopies,
      }));
    return {
      cardType: card.cardType,
      classification: blank
        ? 'blank' as const
        : familyIds.length > 0
          ? 'classified' as const
          : 'unclassified' as const,
      facetIds,
      familyIds,
      name: card.name,
      presetDemand,
      presetDemandStatus: presetDemand.length > 0 ? 'present' as const : 'absent' as const,
      ruleDigest: identityHash(card.rulesText as JsonValue),
      stableId: card.stableId,
    };
  });

  const summaries = <Id extends EffectFamilyId | MechanicFacetId>(
    definitions: readonly CueDefinition<Id>[],
    idsFor: (card: typeof reportCards[number]) => readonly Id[],
  ): readonly WorkloadLabelSummary<Id>[] => definitions.map(({ id }) => {
    const matching = reportCards.filter((card) => idsFor(card).includes(id));
    const impacts = matching.flatMap(({ presetDemand }) => presetDemand);
    return {
      cardCount: matching.length,
      deckCopies: impacts.reduce((total, impact) => total + impact.totalCopies, 0),
      id,
      label: label(id),
      presetDemandCardCount: matching.filter(({ presetDemandStatus }) =>
        presetDemandStatus === 'present').length,
      presetIds: [...new Set(impacts.map(({ presetId }) => presetId))].sort(),
    };
  });

  const presetDemandPresent = reportCards.filter(({ presetDemandStatus }) =>
    presetDemandStatus === 'present').length;
  return {
    cards: reportCards,
    classifierVersion: MECHANIC_WORKLOAD_CLASSIFIER_VERSION,
    facets: summaries(FACETS, ({ facetIds }) => facetIds),
    families: summaries(EFFECT_FAMILIES, ({ familyIds }) => familyIds),
    schemaVersion: 1,
    scopeDisclaimer: 'Lexical workload labels and preset demand only; this report proves neither engine implementation nor scenario verification.',
    totals: {
      blank: reportCards.filter(({ classification }) => classification === 'blank').length,
      cards: reportCards.length,
      classified: reportCards.filter(({ classification }) => classification === 'classified').length,
      presetDemandAbsent: reportCards.length - presetDemandPresent,
      presetDemandPresent,
      unclassified: reportCards.filter(({ classification }) =>
        classification === 'unclassified').length,
    },
  };
}
