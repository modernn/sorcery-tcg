import { z } from 'zod';

import { parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { sha256 } from '../authority/hash.ts';
import { jsonValueSchema } from '../authority/schemas.ts';

const API_URL = 'https://topdeck.gg/api/v2/tournaments';
const GAME = 'Sorcery: Contested Realm';
const MAX_RESPONSE_BYTES = 5_000_000;
const REQUEST_TIMEOUT_MS = 10_000;

export const TOPDECK_ATTRIBUTION = Object.freeze({
  text: 'Data provided by TopDeck.gg',
  url: 'https://topdeck.gg',
});

export type TopDeckDeckRow = Readonly<{
  lineNumber: number;
  mappingStatus: 'unresolved';
  parseStatus: 'parsed' | 'unrecognized';
  quantity: number | null;
  raw: string;
  section: string | null;
  sourceCardName: string | null;
}>;

export type TopDeckDeck = Readonly<{
  opaqueStructuredDeck: Readonly<Record<string, JsonValue>> | null;
  rows: readonly TopDeckDeckRow[];
  sourceText: string | null;
  sourceUrl: string | null;
}>;

export type TopDeckPlacement = Readonly<{
  deck: TopDeckDeck | null;
  placement: number;
  playerName: string;
  sourcePlayerId: string;
}>;

export type TopDeckTournament = Readonly<{
  format: '';
  game: typeof GAME;
  name: string;
  participantCount: number;
  placements: readonly TopDeckPlacement[];
  startedAt: string;
  status: 'completed';
  tournamentId: string;
}>;

export type TopDeckIngestion = Readonly<{
  schemaVersion: 1;
  source: Readonly<{
    apiVersion: 'v2';
    attribution: typeof TOPDECK_ATTRIBUTION;
    endpoint: typeof API_URL;
    retrievedAt: string;
    responseByteHash: ReturnType<typeof sha256>;
  }>;
  tournaments: readonly TopDeckTournament[];
}>;

export type IngestTopDeckOptions = Readonly<{
  apiKey: string;
  fetchImpl?: typeof fetch;
  lastDays: number;
  retrievedAt: string;
}>;

export type TopDeckIngestionErrorCode =
  | 'http_error'
  | 'invalid_input'
  | 'invalid_json'
  | 'invalid_response'
  | 'response_too_large'
  | 'unsupported_team_event';

export class TopDeckIngestionError extends Error {
  readonly code: TopDeckIngestionErrorCode;
  readonly path: string;

  constructor(code: TopDeckIngestionErrorCode, path: string, message: string) {
    super(message);
    this.name = 'TopDeckIngestionError';
    this.code = code;
    this.path = path;
  }
}

const boundedText = (minimum = 1, maximum = 300) => z.string().min(minimum).max(maximum);
const safeNonnegativeInteger = z.number().int().nonnegative().safe();
const timestampSeconds = safeNonnegativeInteger.max(253_402_300_799);
const deckObjectSchema = z.record(z.string().max(300), jsonValueSchema);
const standingSchema = z.strictObject({
  deckObj: deckObjectSchema.nullable().optional(),
  decklist: z.string().max(500_000).nullable().optional(),
  id: boundedText(1, 300),
  name: boundedText(1, 300),
  standing: z.number().int().positive().safe(),
});
const tournamentSchema = z.strictObject({
  TID: boundedText(1, 300),
  eventData: z.record(z.string().max(300), jsonValueSchema).nullable().optional(),
  format: z.literal(''),
  game: z.literal(GAME),
  isTeamEvent: z.boolean().optional(),
  sharedTeamGame: z.boolean().optional(),
  standings: z.array(standingSchema).max(2_048),
  startDate: timestampSeconds,
  swissNum: safeNonnegativeInteger,
  teamSize: z.number().int().positive().safe().optional(),
  teamTags: z.array(boundedText(1, 100)).max(100).optional(),
  topCut: safeNonnegativeInteger,
  tournamentName: boundedText(1, 500),
});
const responseSchema = z.array(tournamentSchema).max(256);
const optionsSchema = z.strictObject({
  apiKey: z.string().min(1).max(512).refine(
    (value) => value.trim() === value && !/[\r\n]/u.test(value),
    'API key must not contain surrounding whitespace or line breaks',
  ),
  lastDays: z.number().int().min(1).max(90),
  retrievedAt: z.iso.datetime({ offset: true }),
});

function issuePath(path: readonly PropertyKey[]): string {
  return path.length === 0
    ? ''
    : '/' + path.map((part) => String(part).replaceAll('~', '~0').replaceAll('/', '~1')).join('/');
}

function failFromZod(error: z.ZodError, code: 'invalid_input' | 'invalid_response'): never {
  const issue = error.issues[0];
  throw new TopDeckIngestionError(
    code,
    issue === undefined ? '' : issuePath(issue.path),
    code === 'invalid_input' ? 'Invalid TopDeck ingestion options' : 'Invalid TopDeck API response',
  );
}

async function readBoundedResponse(response: Response): Promise<Uint8Array> {
  const lengthHeader = response.headers.get('content-length');
  if (lengthHeader !== null) {
    const length = Number(lengthHeader);
    if (!Number.isSafeInteger(length) || length < 0) {
      throw new TopDeckIngestionError('invalid_response', '/headers/content-length', 'Invalid Content-Length');
    }
    if (length > MAX_RESPONSE_BYTES) {
      throw new TopDeckIngestionError('response_too_large', '', 'TopDeck response exceeds byte limit');
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
      throw new TopDeckIngestionError('response_too_large', '', 'TopDeck response exceeds byte limit');
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
    const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    json = parseJsonWithDuplicateKeyCheck(text);
  } catch {
    throw new TopDeckIngestionError('invalid_json', '', 'TopDeck response is not valid UTF-8 JSON');
  }
  const parsed = responseSchema.safeParse(json);
  if (!parsed.success) failFromZod(parsed.error, 'invalid_response');
  return parsed.data;
}

function deckUrl(value: string): string | null {
  try {
    const parsed = new URL(value);
    return parsed.protocol === 'https:' && parsed.username === '' && parsed.password === ''
      ? parsed.href
      : null;
  } catch {
    return null;
  }
}

function deckRows(text: string): readonly TopDeckDeckRow[] {
  let section: string | null = null;
  const rows: TopDeckDeckRow[] = [];
  for (const [index, raw] of text.replaceAll('\r\n', '\n').replaceAll('\r', '\n').split('\n').entries()) {
    if (raw.trim() === '') continue;
    const heading = /^~~(.{1,200})~~$/u.exec(raw.trim());
    if (heading !== null) {
      section = heading[1]!.trim();
      continue;
    }
    const card = /^(\d{1,3})\s+(.{1,300})$/u.exec(raw.trim());
    const quantity = card === null ? null : Number(card[1]);
    const parsed = quantity !== null && quantity > 0;
    rows.push({
      lineNumber: index + 1,
      mappingStatus: 'unresolved',
      parseStatus: parsed ? 'parsed' : 'unrecognized',
      quantity: parsed ? quantity : null,
      raw,
      section,
      sourceCardName: parsed ? card![2]!.trim() : null,
    });
  }
  return rows;
}

function normalizeDeck(
  decklist: string | null | undefined,
  opaqueStructuredDeck: Readonly<Record<string, JsonValue>> | null | undefined,
): TopDeckDeck | null {
  const trimmed = decklist?.trim() ?? '';
  if (trimmed === '' && opaqueStructuredDeck == null) return null;
  const sourceUrl = trimmed === '' ? null : deckUrl(trimmed);
  const sourceText = trimmed === '' || sourceUrl !== null ? null : decklist ?? null;
  return {
    opaqueStructuredDeck: opaqueStructuredDeck ?? null,
    rows: sourceText === null ? [] : deckRows(sourceText),
    sourceText,
    sourceUrl,
  };
}

function normalizeTournament(
  tournament: z.infer<typeof tournamentSchema>,
  tournamentIndex: number,
): TopDeckTournament {
  if (tournament.isTeamEvent === true || tournament.teamSize !== undefined ||
      tournament.teamTags !== undefined || tournament.sharedTeamGame === true) {
    throw new TopDeckIngestionError(
      'unsupported_team_event',
      `/${tournamentIndex}`,
      'TopDeck team tournaments are not supported by this ingestion boundary',
    );
  }
  const placements = tournament.standings
    .map((standing): TopDeckPlacement => ({
      deck: normalizeDeck(standing.decklist, standing.deckObj),
      placement: standing.standing,
      playerName: standing.name,
      sourcePlayerId: standing.id,
    }))
    .sort((left, right) =>
      left.placement - right.placement || left.sourcePlayerId.localeCompare(right.sourcePlayerId),
    );
  if (new Set(placements.map(({ placement }) => placement)).size !== placements.length) {
    throw new TopDeckIngestionError(
      'invalid_response',
      `/${tournamentIndex}/standings`,
      'TopDeck standings contain duplicate placements',
    );
  }
  if (new Set(placements.map(({ sourcePlayerId }) => sourcePlayerId)).size !== placements.length) {
    throw new TopDeckIngestionError(
      'invalid_response',
      `/${tournamentIndex}/standings`,
      'TopDeck standings contain duplicate player IDs',
    );
  }
  return {
    format: '',
    game: GAME,
    name: tournament.tournamentName,
    participantCount: placements.length,
    placements,
    startedAt: new Date(tournament.startDate * 1_000).toISOString(),
    status: 'completed',
    tournamentId: tournament.TID,
  };
}

export async function ingestTopDeckTournaments(
  options: IngestTopDeckOptions,
): Promise<TopDeckIngestion> {
  const parsedOptions = optionsSchema.safeParse({
    apiKey: options.apiKey,
    lastDays: options.lastDays,
    retrievedAt: options.retrievedAt,
  });
  if (!parsedOptions.success) failFromZod(parsedOptions.error, 'invalid_input');

  const response = await (options.fetchImpl ?? globalThis.fetch)(API_URL, {
    body: JSON.stringify({
      columns: ['name', 'id', 'decklist'],
      format: '',
      game: GAME,
      last: parsedOptions.data.lastDays,
    }),
    headers: {
      Accept: 'application/json',
      Authorization: parsedOptions.data.apiKey,
      'Content-Type': 'application/json',
    },
    method: 'POST',
    redirect: 'error',
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  if (!response.ok) {
    throw new TopDeckIngestionError('http_error', '', `TopDeck API returned HTTP ${response.status}`);
  }
  if (!response.headers.get('content-type')?.toLowerCase().includes('application/json')) {
    throw new TopDeckIngestionError('invalid_response', '/headers/content-type', 'Expected JSON response');
  }

  const bytes = await readBoundedResponse(response);
  const tournaments = parseResponse(bytes).map(normalizeTournament).sort((left, right) =>
    left.startedAt.localeCompare(right.startedAt) || left.tournamentId.localeCompare(right.tournamentId),
  );
  if (new Set(tournaments.map(({ tournamentId }) => tournamentId)).size !== tournaments.length) {
    throw new TopDeckIngestionError('invalid_response', '', 'TopDeck response contains duplicate tournaments');
  }
  return {
    schemaVersion: 1,
    source: {
      apiVersion: 'v2',
      attribution: TOPDECK_ATTRIBUTION,
      endpoint: API_URL,
      retrievedAt: new Date(parsedOptions.data.retrievedAt).toISOString(),
      responseByteHash: sha256(bytes),
    },
    tournaments,
  };
}
