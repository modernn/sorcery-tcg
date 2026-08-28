import { z } from 'zod';

import {
  raiseZodValidationError,
  validateRawCardSnapshot,
  type RawCard,
  type RawCardSnapshot,
} from './schemas.ts';

const MAX_OFFICIAL_CARDS = 2_000;
const MAX_SETS_PER_CARD = 50;
const MAX_VARIANTS_PER_SET = 100;
const PRINTING_ID_PATTERN = /^[a-z0-9]+(?:[-_][a-z0-9]+)*$/;

const elementNames = ['Air', 'Earth', 'Fire', 'Water'] as const;
const elementMap = {
  Air: 'air',
  Earth: 'earth',
  Fire: 'fire',
  Water: 'water',
} as const;
const cardTypeMap = {
  Avatar: 'avatar',
  Site: 'site',
  Minion: 'minion',
  Aura: 'aura',
  Artifact: 'artifact',
  Magic: 'magic',
} as const;
const rarityMap = {
  Ordinary: 'ordinary',
  Exceptional: 'exceptional',
  Elite: 'elite',
  Unique: 'unique',
} as const;

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function validElements(value: string): boolean {
  if (value === 'None') return true;
  const parts = value.split(', ');
  return (
    parts.join(', ') === value &&
    parts.length <= elementNames.length &&
    parts.every((part, index) =>
      (elementNames as readonly string[]).includes(part) && parts.indexOf(part) === index)
  );
}

const nonnegativeInteger = z.number().int().nonnegative().safe();
const nullableNonnegativeInteger = nonnegativeInteger.nullable();
const thresholdsSchema = z.strictObject({
  air: nonnegativeInteger,
  earth: nonnegativeInteger,
  fire: nonnegativeInteger,
  water: nonnegativeInteger,
});
const metadataSchema = z.strictObject({
  attack: nullableNonnegativeInteger,
  cost: nullableNonnegativeInteger,
  defence: nullableNonnegativeInteger,
  life: nullableNonnegativeInteger,
  rarity: z.enum(['Ordinary', 'Exceptional', 'Elite', 'Unique']).nullable(),
  rulesText: z.string().max(20_000),
  thresholds: thresholdsSchema,
  type: z.enum(['Avatar', 'Site', 'Minion', 'Aura', 'Artifact', 'Magic']),
});
const variantSchema = z.strictObject({
  artist: z.string().max(1_000),
  finish: z.string().max(1_000),
  flavorText: z.string().max(20_000),
  product: z.string().max(1_000),
  slug: z.string().min(1).max(200).regex(PRINTING_ID_PATTERN),
  typeText: z.string().max(1_000),
});
const setSchema = z.strictObject({
  metadata: metadataSchema,
  name: z.string().min(1).max(300),
  releasedAt: z.iso.datetime({ offset: true }),
  variants: z.array(variantSchema).min(1).max(MAX_VARIANTS_PER_SET),
});
const officialCardSchema = z.strictObject({
  elements: z.string().min(1).max(100).refine(validElements, 'elements must use the audited official grammar'),
  guardian: metadataSchema,
  name: z.string().min(1).max(300),
  sets: z.array(setSchema).min(1).max(MAX_SETS_PER_CARD),
  subTypes: z.string().max(1_000),
});
const officialCardApiSchema = z.array(officialCardSchema).min(1).max(MAX_OFFICIAL_CARDS);

type OfficialCard = z.infer<typeof officialCardSchema>;

function printingRows(card: OfficialCard): Array<Readonly<{ releasedAt: string; slug: string }>> {
  return card.sets
    .flatMap((set) => set.variants.map((variant) => ({ releasedAt: set.releasedAt, slug: variant.slug })))
    .sort((left, right) =>
      Date.parse(left.releasedAt) - Date.parse(right.releasedAt) || compareText(left.slug, right.slug),
    );
}

function adaptCard(card: OfficialCard): RawCard {
  const printings = printingRows(card);
  const first = printings[0]!;
  const elements = card.elements === 'None'
    ? []
    : card.elements.split(', ').map((element) => elementMap[element as keyof typeof elementMap]);
  return {
    sourceCardId: first.slug,
    name: card.name,
    cardType: cardTypeMap[card.guardian.type],
    elements,
    rarity: card.guardian.rarity === null ? null : rarityMap[card.guardian.rarity],
    manaCost: card.guardian.cost,
    attack: card.guardian.attack,
    defense: card.guardian.defence,
    life: card.guardian.life,
    thresholds: {
      air: card.guardian.thresholds.air,
      earth: card.guardian.thresholds.earth,
      fire: card.guardian.thresholds.fire,
      water: card.guardian.thresholds.water,
    },
    rulesText: card.guardian.rulesText,
    printingSlugs: printings.map(({ slug }) => slug).sort(compareText),
    releasedAt: new Date(first.releasedAt).toISOString().slice(0, 10),
  };
}

export function adaptOfficialCardApiSnapshot(input: unknown): RawCardSnapshot {
  const parsed = officialCardApiSchema.safeParse(input);
  if (!parsed.success) raiseZodValidationError(parsed.error);
  return validateRawCardSnapshot({ cards: parsed.data.map(adaptCard) });
}
