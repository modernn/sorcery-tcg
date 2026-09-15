import { spawn } from 'node:child_process';
import { availableParallelism } from 'node:os';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import { deepFreeze, type EngineSeat } from '../engine/contract.ts';
import {
  createGameManifest,
  type GameDeckSpec,
  type GameManifest,
} from '../engine/game.ts';
import type { GameBatchResult } from '../commands/run-game-batch.ts';

const MAX_SEEDS = 128;
const MAX_WORKERS = 8;
const MAX_BATCH_BYTES = 64 * 1024 * 1024;
const MAX_OUTPUT_BYTES = 16 * 1024 * 1024;
const REPOSITORY_ROOT = fileURLToPath(new URL('../..', import.meta.url));

type OutcomeCounts = Readonly<{ draws: number; games: number; losses: number; wins: number }>;

export type GauntletDeck = Readonly<{ deck: GameDeckSpec; id: string }>;

export type TwoDeckGauntletInput = Readonly<{
  authority: GameManifest['authority'];
  cards: GameManifest['cards'];
  decks: readonly [GauntletDeck, GauntletDeck];
  seeds: readonly number[];
}>;

export type GauntletGameResult = Readonly<GameBatchResult & {
  northDeckId: string;
  seed: number;
  southDeckId: string;
}>;

export type TwoDeckGauntletReport = Readonly<{
  averageTurns: number;
  byDeck: Readonly<Record<string, Readonly<OutcomeCounts & {
    asNorth: OutcomeCounts;
    asSouth: OutcomeCounts;
  }>>>;
  bySeat: Readonly<Record<EngineSeat, OutcomeCounts>>;
  classification: 'unranked_partial_rules_unverified_authority';
  gameCount: number;
  games: readonly GauntletGameResult[];
  seeds: readonly number[];
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function countedRows(cardIds: readonly string[]): readonly Readonly<{
  cardId: string;
  copies: number;
}>[] {
  const counts = new Map<string, number>();
  cardIds.forEach((cardId) => counts.set(cardId, (counts.get(cardId) ?? 0) + 1));
  return [...counts]
    .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
    .map(([cardId, copies]) => ({ cardId, copies }));
}

function deckCompositionId(deck: GameDeckSpec): ReturnType<typeof identityHash> {
  return identityHash({
    deck: {
      atlas: countedRows(deck.atlas),
      avatar: deck.avatar,
      spellbook: countedRows(deck.spellbook),
    },
    formatId: 'format:modeled-constructed-v1',
  } as JsonValue);
}

function policy(manifest: GameManifest, boundDeckId: ReturnType<typeof identityHash>): JsonValue {
  const body = {
    authorityHash: manifest.authority.contentHash,
    deckId: boundDeckId,
    engineVersion: manifest.engineVersion,
    generation: 0,
    observationVersion: 'seat-observation-v1',
    schemaVersion: 1,
    selector: {
      atlasReserve: 3,
      featurePriority: [
        'keep-mulligan', 'play-site', 'summon-minion', 'preferred-draw',
        'powered-movement', 'beneficial-tactic', 'move-toward-enemy',
        'end-turn', 'canonical-fallback',
      ],
    },
    tieBreak: 'canonical-action-order-v1',
  } as const;
  return { ...body, policyId: identityHash(body as unknown as JsonValue) } as JsonValue;
}

function jobPayload(manifest: GameManifest): JsonValue {
  const northDeckId = deckCompositionId(manifest.decks.north);
  const southDeckId = deckCompositionId(manifest.decks.south);
  return {
    manifestJson: canonicalJson(manifest as unknown as JsonValue),
    northDeckId,
    northPolicy: policy(manifest, northDeckId),
    southDeckId,
    southPolicy: policy(manifest, southDeckId),
  };
}

function rustRequest(input: TwoDeckGauntletInput, workers: number): string {
  const [deckA, deckB] = input.decks;
  return canonicalJson({
    deckAId: deckA.id,
    deckBId: deckB.id,
    pairs: input.seeds.map((seed) => ({
      aNorth: jobPayload(createGameManifest({
        authority: input.authority,
        cards: input.cards,
        decks: { north: deckA.deck, south: deckB.deck },
        firstSeat: 'north',
        seed,
      })),
      bNorth: jobPayload(createGameManifest({
        authority: input.authority,
        cards: input.cards,
        decks: { north: deckB.deck, south: deckA.deck },
        firstSeat: 'north',
        seed,
      })),
      seed,
    })),
    schemaVersion: 1,
    workers,
  } as JsonValue);
}

async function runRustGauntlet(request: string): Promise<unknown> {
  const child = spawn('cargo', [
    'run', '--release', '--locked', '--quiet', '-p', 'sorcery-engine',
    '--bin', 'sorcery-engine', '--', 'gauntlet-json',
  ], { cwd: REPOSITORY_ROOT, stdio: ['pipe', 'pipe', 'pipe'] });
  const stdout: Buffer[] = [];
  const stderr: Buffer[] = [];
  let stdoutBytes = 0;
  let stderrBytes = 0;
  return new Promise((resolvePromise, rejectPromise) => {
    let settled = false;
    const fail = (message: string): void => {
      if (settled) return;
      settled = true;
      child.kill();
      rejectPromise(new Error(message));
    };
    child.once('error', () => fail('gauntlet failed to start Rust'));
    child.stdin.once('error', () => fail('gauntlet failed while sending Rust input'));
    child.stdout.on('data', (chunk: Buffer) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > MAX_OUTPUT_BYTES) return fail('gauntlet Rust output exceeded its limit');
      stdout.push(chunk);
    });
    child.stderr.on('data', (chunk: Buffer) => {
      stderrBytes += chunk.length;
      if (stderrBytes > MAX_OUTPUT_BYTES) return fail('gauntlet Rust error output exceeded its limit');
      stderr.push(chunk);
    });
    child.once('close', (code) => {
      if (settled) return;
      if (code !== 0) {
        const unsupported = /not yet supported by Rust: ([A-Za-z][A-Za-z0-9:]*)/u
          .exec(Buffer.concat(stderr).toString('utf8'))?.[1];
        if (unsupported) {
          return fail(`gauntlet failed in Rust: unsupported fact ${unsupported}`);
        }
        return fail(`gauntlet failed in Rust: ${Buffer.concat(stderr).toString('utf8').trim() || 'unknown error'}`);
      }
      try {
        const text = Buffer.concat(stdout).toString('utf8').trim();
        const parsed: unknown = JSON.parse(text);
        if (canonicalJson(parsed as JsonValue) !== text) throw new Error('noncanonical output');
        settled = true;
        resolvePromise(parsed);
      } catch {
        fail('gauntlet Rust output was invalid');
      }
    });
    child.stdin.end(`${request}\n`);
  });
}

/** Runs a seat-swapped two-deck gauntlet inside the authoritative Rust batch path. */
export async function runTwoDeckGauntlet(
  input: TwoDeckGauntletInput,
  requestedWorkers = Math.min(availableParallelism(), MAX_WORKERS),
): Promise<TwoDeckGauntletReport> {
  const [deckA, deckB] = input.decks;
  if (!deckA.id.trim() || !deckB.id.trim() || deckA.id === deckB.id) {
    throw new RangeError('gauntlet deck IDs must be distinct nonempty strings');
  }
  if (input.seeds.length === 0 || input.seeds.length > MAX_SEEDS) {
    throw new RangeError('gauntlet must contain 1-128 seeds');
  }
  if (!Number.isSafeInteger(requestedWorkers)
    || requestedWorkers < 1
    || requestedWorkers > MAX_WORKERS) {
    throw new RangeError(`requestedWorkers must be 1-${MAX_WORKERS}`);
  }
  const request = rustRequest(input, requestedWorkers);
  if (Buffer.byteLength(request) > MAX_BATCH_BYTES) {
    throw new RangeError(`gauntlet exceeds ${MAX_BATCH_BYTES} bytes`);
  }
  const report = await runRustGauntlet(request);
  if (!isRecord(report)
    || report.classification !== 'unranked_partial_rules_unverified_authority'
    || report.gameCount !== input.seeds.length * 2
    || !Array.isArray(report.seeds)
    || report.seeds.length !== input.seeds.length
    || !Array.isArray(report.games)
    || report.games.length !== input.seeds.length * 2) {
    throw new Error('gauntlet results do not match the declared pairs');
  }
  return deepFreeze(report as unknown as TwoDeckGauntletReport);
}
