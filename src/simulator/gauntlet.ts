import { deepFreeze, type EngineSeat } from '../engine/contract.ts';
import {
  createGameManifest,
  type GameDeckSpec,
  type GameManifest,
} from '../engine/game.ts';
import {
  runGameBatch,
  type GameBatchResult,
} from '../commands/run-game-batch.ts';

type Outcome = 'draws' | 'losses' | 'wins';
type OutcomeCounts = Readonly<{ draws: number; games: number; losses: number; wins: number }>;
type MutableOutcomeCounts = { draws: number; games: number; losses: number; wins: number };

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

function emptyCounts(): MutableOutcomeCounts {
  return { draws: 0, games: 0, losses: 0, wins: 0 };
}

function record(counts: MutableOutcomeCounts, outcome: Outcome): void {
  counts.games += 1;
  counts[outcome] += 1;
}

export async function runTwoDeckGauntlet(
  input: TwoDeckGauntletInput,
  requestedWorkers?: number,
): Promise<TwoDeckGauntletReport> {
  const [deckA, deckB] = input.decks;
  if (!deckA.id.trim() || !deckB.id.trim() || deckA.id === deckB.id) {
    throw new RangeError('gauntlet deck IDs must be distinct nonempty strings');
  }
  if (input.seeds.length === 0 || input.seeds.length > 128) {
    throw new RangeError('gauntlet must contain 1-128 seeds');
  }
  const jobs = input.seeds.flatMap((seed) => [
    { north: deckA, seed, south: deckB },
    { north: deckB, seed, south: deckA },
  ]);
  const results = await runGameBatch(jobs.map(({ north, seed, south }) => createGameManifest({
    authority: input.authority,
    cards: input.cards,
    decks: { north: north.deck, south: south.deck },
    firstSeat: 'north',
    seed,
  })), requestedWorkers);
  const bySeat = { north: emptyCounts(), south: emptyCounts() };
  const byDeck = new Map(input.decks.map(({ id }) => [id, {
    ...emptyCounts(),
    asNorth: emptyCounts(),
    asSouth: emptyCounts(),
  }]));
  const games = results.map((result): GauntletGameResult => {
    const job = jobs[result.jobIndex]!;
    for (const seat of ['north', 'south'] as const) {
      const outcome: Outcome = 'result' in result.report.terminal
        ? 'draws'
        : result.report.terminal.winner === seat ? 'wins' : 'losses';
      const deck = byDeck.get(job[seat].id)!;
      record(bySeat[seat], outcome);
      record(deck, outcome);
      record(seat === 'north' ? deck.asNorth : deck.asSouth, outcome);
    }
    return {
      ...result,
      northDeckId: job.north.id,
      seed: job.seed,
      southDeckId: job.south.id,
    };
  });
  return deepFreeze({
    averageTurns: games.reduce((total, game) => total + game.report.turnCount, 0) / games.length,
    byDeck: Object.fromEntries(byDeck),
    bySeat,
    classification: 'unranked_partial_rules_unverified_authority' as const,
    gameCount: games.length,
    games,
    seeds: [...input.seeds],
  });
}
