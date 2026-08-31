import { z } from 'zod';

import { parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { sha256 } from '../authority/hash.ts';

const API_URL = 'https://api.sorcerytcg.com/api/cards';
const DOCUMENTATION_URL = 'https://api.sorcerytcg.com/';
const MAX_RESPONSE_BYTES = 5_000_000;
const REQUEST_TIMEOUT_MS = 10_000;

export const SORCERY_CARD_API_POLICY = Object.freeze({
  requestsPerMinute: 30,
  responsibility: 'caller' as const,
});

const cardTypes = ['Avatar', 'Minion', 'Magic', 'Aura', 'Artifact', 'Site'] as const;
const categories = ['Avatar', 'Spell', 'Site', 'Token'] as const;
const rarities = ['Unique', 'Elite', 'Exceptional', 'Ordinary'] as const;
const elements = ['Earth', 'Fire', 'Water', 'Air', 'None'] as const;
const subtypes = [
  'Angel',
  'Beast',
  'Demon',
  'Dragon',
  'Dwarf',
  'Faerie',
  'Giant',
  'Gnome',
  'Goblin',
  'Merfolk',
  'Monster',
  'Mortal',
  'Ogre',
  'Spirit',
  'Sphinx',
  'Troll',
  'Undead',
  'Armor',
  'Automaton',
  'Device',
  'Document',
  'Instruments',
  'Monument',
  'Potion',
  'Relic',
  'Weapon',
  'Desert',
  'River',
  'Tower',
  'Village',
] as const;
const keywords = [
  'Airborne',
  'Burrowing',
  'Charge',
  'Deathrite',
  'Disabled',
  'Flood',
  'Genesis',
  'Immobile',
  'Lance',
  'Landbound',
  'Lethal',
  'Movement',
  'Ranged',
  'Spellcaster',
  'Stealth',
  'Submerge',
  'Voidwalk',
  'Ward',
  'Waterbound',
] as const;
const umbrellas = ['Knight', 'Royalty', 'Evil'] as const;
const finishes = ['Standard', 'Foil', 'Rainbow'] as const;
const products = [
  'Booster',
  'BoxTopper',
  'PreconstructedDeck',
  'Dust',
  'DraftKit',
  'WelcomeKit',
  'Kickstarter',
  'TeamCovenant',
  'AlphaInvestments',
  'StarCityGames',
  'OrganizedPlay',
] as const;

export type SorceryCardPrintingIdentity = Readonly<{
  finish: typeof finishes[number];
  officialPrintingId: string;
  officialPrintingSlug: string;
  printedAt: string;
  product: typeof products[number];
  set: Readonly<{
    code: string;
    name: string;
    releasedAt: string;
  }>;
}>;

export type SorceryCardIdentity = Readonly<{
  category: typeof categories[number];
  name: string;
  officialCardId: string;
  officialCardSlug: string;
  printings: readonly SorceryCardPrintingIdentity[];
  type: typeof cardTypes[number];
}>;

export type SorceryCardIdentityCatalog = Readonly<{
  cards: readonly SorceryCardIdentity[];
  schemaVersion: 1;
  source: Readonly<{
    apiVersion: 'v1';
    documentationUrl: typeof DOCUMENTATION_URL;
    endpoint: typeof API_URL;
    policy: typeof SORCERY_CARD_API_POLICY;
    responseByteHash: ReturnType<typeof sha256>;
    retrievedAt: string;
  }>;
}>;

export type IngestSorceryCardsOptions = Readonly<{
  fetchImpl?: typeof fetch;
  retrievedAt: string;
}>;

export type SorceryCardIngestionErrorCode =
  | 'http_error'
  | 'invalid_input'
  | 'invalid_json'
  | 'invalid_response'
  | 'response_too_large';

export class SorceryCardIngestionError extends Error {
  readonly code: SorceryCardIngestionErrorCode;
  readonly path: string;

  constructor(code: SorceryCardIngestionErrorCode, path: string, message: string) {
    super(message);
    this.name = 'SorceryCardIngestionError';
    this.code = code;
    this.path = path;
  }
}

const boundedText = (minimum = 1, maximum = 300) => z.string().min(minimum).max(maximum);
const documentedNumber = z.number().finite().min(-1_000_000).max(1_000_000).nullable();
const uniqueEnumArray = <T extends readonly [string, ...string[]]>(values: T) => z.array(z.enum(values)).max(100)
  .superRefine((items, context) => {
    for (const [index, item] of items.entries()) {
      if (items.indexOf(item) !== index) {
        context.addIssue({ code: 'custom', message: 'duplicate value', path: [index] });
      }
    }
  });
const engineFaceSchema = z.strictObject({
  air: documentedNumber,
  attack: documentedNumber,
  category: z.enum(categories),
  cost: documentedNumber,
  defense: documentedNumber,
  earth: documentedNumber,
  elements: uniqueEnumArray(elements),
  fire: documentedNumber,
  keywords: uniqueEnumArray(keywords),
  life: documentedNumber,
  rarity: z.enum(rarities).nullable(),
  rules: z.string().max(20_000).nullable(),
  slot: z.enum(rarities).nullable(),
  subtypes: uniqueEnumArray(subtypes),
  type: z.enum(cardTypes),
  umbrellas: uniqueEnumArray(umbrellas),
  water: documentedNumber,
});
const engineSchema = engineFaceSchema.extend({ back: engineFaceSchema.nullable() });
const artistSchema = z.strictObject({
  name: boundedText(1, 500),
  slug: boundedText(1, 300),
});
const printingMetaFaceSchema = z.strictObject({
  artist: artistSchema,
  finish: z.enum(finishes),
  flavor: z.string().max(20_000).nullable(),
  product: z.enum(products),
  typeline: z.string().max(2_000),
});
const printingMetaSchema = printingMetaFaceSchema.extend({ back: printingMetaFaceSchema.nullable() });
const printingSchema = z.strictObject({
  id: boundedText(1, 300),
  meta: printingMetaSchema,
  printedAt: z.iso.datetime({ offset: true }),
  set: z.strictObject({
    code: boundedText(1, 100),
    name: boundedText(1, 500),
    releasedAt: z.iso.datetime({ offset: true }),
  }),
  slug: boundedText(1, 300),
});
const cardSchema = z.strictObject({
  engine: engineSchema,
  id: boundedText(1, 300),
  name: boundedText(1, 500),
  printings: z.array(printingSchema).min(1).max(500),
  slug: boundedText(1, 300),
});
const responseSchema = z.array(cardSchema).min(1).max(3_000);
const optionsSchema = z.strictObject({ retrievedAt: z.iso.datetime({ offset: true }) });

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function issuePath(path: readonly PropertyKey[]): string {
  return path.length === 0
    ? ''
    : '/' + path.map((part) => String(part).replaceAll('~', '~0').replaceAll('/', '~1')).join('/');
}

function failFromZod(error: z.ZodError, code: 'invalid_input' | 'invalid_response'): never {
  const issue = error.issues[0];
  throw new SorceryCardIngestionError(
    code,
    issue === undefined ? '' : issuePath(issue.path),
    code === 'invalid_input' ? 'Invalid Sorcery card ingestion options' : 'Invalid Sorcery card API response',
  );
}

async function readBoundedResponse(response: Response): Promise<Uint8Array> {
  const lengthHeader = response.headers.get('content-length');
  if (lengthHeader !== null) {
    const length = Number(lengthHeader);
    if (!Number.isSafeInteger(length) || length < 0) {
      throw new SorceryCardIngestionError('invalid_response', '/headers/content-length', 'Invalid Content-Length');
    }
    if (length > MAX_RESPONSE_BYTES) {
      throw new SorceryCardIngestionError('response_too_large', '', 'Sorcery card response exceeds byte limit');
    }
  }

  if (response.body === null) return new Uint8Array();
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > MAX_RESPONSE_BYTES) {
      await reader.cancel();
      throw new SorceryCardIngestionError(
        'response_too_large',
        '',
        'Sorcery card response exceeds byte limit',
      );
    }
    chunks.push(value);
  }

  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return bytes;
}

function parseResponse(bytes: Uint8Array): z.infer<typeof responseSchema> {
  let json: JsonValue;
  try {
    json = parseJsonWithDuplicateKeyCheck(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    throw new SorceryCardIngestionError(
      'invalid_json',
      '',
      'Sorcery card response is not valid UTF-8 JSON',
    );
  }
  const parsed = responseSchema.safeParse(json);
  if (!parsed.success) failFromZod(parsed.error, 'invalid_response');
  return parsed.data;
}

function normalizeCard(card: z.infer<typeof cardSchema>): SorceryCardIdentity {
  return {
    category: card.engine.category,
    name: card.name,
    officialCardId: card.id,
    officialCardSlug: card.slug,
    printings: card.printings.map((printing): SorceryCardPrintingIdentity => ({
      finish: printing.meta.finish,
      officialPrintingId: printing.id,
      officialPrintingSlug: printing.slug,
      printedAt: new Date(printing.printedAt).toISOString(),
      product: printing.meta.product,
      set: {
        code: printing.set.code,
        name: printing.set.name,
        releasedAt: new Date(printing.set.releasedAt).toISOString(),
      },
    })).sort((left, right) =>
      left.printedAt.localeCompare(right.printedAt) ||
      compareText(left.officialPrintingId, right.officialPrintingId),
    ),
    type: card.engine.type,
  };
}

function rejectDuplicateIdentities(cards: readonly SorceryCardIdentity[]): void {
  const cardIds = new Set<string>();
  const cardSlugs = new Set<string>();
  const printingIds = new Set<string>();
  const printingSlugs = new Set<string>();
  for (const [cardIndex, card] of cards.entries()) {
    if (cardIds.has(card.officialCardId)) {
      throw new SorceryCardIngestionError('invalid_response', `/${cardIndex}/id`, 'Duplicate official card ID');
    }
    if (cardSlugs.has(card.officialCardSlug)) {
      throw new SorceryCardIngestionError('invalid_response', `/${cardIndex}/slug`, 'Duplicate official card slug');
    }
    cardIds.add(card.officialCardId);
    cardSlugs.add(card.officialCardSlug);
    for (const [printingIndex, printing] of card.printings.entries()) {
      if (printingIds.has(printing.officialPrintingId)) {
        throw new SorceryCardIngestionError(
          'invalid_response',
          `/${cardIndex}/printings/${printingIndex}/id`,
          'Duplicate official printing ID',
        );
      }
      if (printingSlugs.has(printing.officialPrintingSlug)) {
        throw new SorceryCardIngestionError(
          'invalid_response',
          `/${cardIndex}/printings/${printingIndex}/slug`,
          'Duplicate official printing slug',
        );
      }
      printingIds.add(printing.officialPrintingId);
      printingSlugs.add(printing.officialPrintingSlug);
    }
  }
}

export async function ingestSorceryCards(
  options: IngestSorceryCardsOptions,
): Promise<SorceryCardIdentityCatalog> {
  const parsedOptions = optionsSchema.safeParse({ retrievedAt: options.retrievedAt });
  if (!parsedOptions.success) failFromZod(parsedOptions.error, 'invalid_input');

  const response = await (options.fetchImpl ?? globalThis.fetch)(API_URL, {
    headers: { Accept: 'application/json' },
    method: 'GET',
    redirect: 'error',
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  if (!response.ok) {
    throw new SorceryCardIngestionError(
      'http_error',
      '',
      `Sorcery card API returned HTTP ${response.status}`,
    );
  }
  if (!response.headers.get('content-type')?.toLowerCase().includes('application/json')) {
    throw new SorceryCardIngestionError(
      'invalid_response',
      '/headers/content-type',
      'Expected JSON response',
    );
  }

  const bytes = await readBoundedResponse(response);
  const cards = parseResponse(bytes).map(normalizeCard).sort((left, right) =>
    compareText(left.name, right.name) || compareText(left.officialCardId, right.officialCardId),
  );
  rejectDuplicateIdentities(cards);
  return {
    cards,
    schemaVersion: 1,
    source: {
      apiVersion: 'v1',
      documentationUrl: DOCUMENTATION_URL,
      endpoint: API_URL,
      policy: SORCERY_CARD_API_POLICY,
      responseByteHash: sha256(bytes),
      retrievedAt: new Date(parsedOptions.data.retrievedAt).toISOString(),
    },
  };
}
