import { runRustEngineCommand, type Sha256Hash } from '../engine/rust-engine.ts';

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function parseCount(value: unknown, label: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new Error(`schedule ${label} was invalid`);
  }
  return value as number;
}

export type OutcomeCounts = Readonly<{
  draws: number;
  games: number;
  losses: number;
  wins: number;
}>;

export type DeckOutcomeCounts = Readonly<{
  asNorth: OutcomeCounts;
  asSouth: OutcomeCounts;
  draws: number;
  games: number;
  losses: number;
  wins: number;
}>;

export type ScheduleSummary = Readonly<{
  byDeck: Readonly<{
    north: DeckOutcomeCounts;
    south: DeckOutcomeCounts;
  }>;
  bySeat: Readonly<{
    north: OutcomeCounts;
    south: OutcomeCounts;
  }>;
  eligibility: 'unranked_partial_rules_unverified_authority';
  games: number;
  length: Readonly<{
    totalActions: number;
    totalFights: number;
    totalTurns: number;
  }>;
  reliability: Readonly<{
    allReplayVerified: boolean;
    replayFailedGames: number;
    replayVerifiedGames: number;
  }>;
  seatEffect: number;
  uncertainty: Readonly<{
    firstPlayerScore: number;
    scoreDenominator: number;
  }>;
}>;

export type SyntheticScheduleReport = Readonly<{
  classification: 'unranked_partial_rules_unverified_authority';
  completedSeeds: readonly number[];
  failurePolicy: 'abort';
  gameCount: number;
  plannedSeeds: readonly number[];
  schemaVersion: 1;
  scheduleId: Sha256Hash;
  status: 'completed';
  summary: ScheduleSummary;
}>;

function parseOutcomeCounts(value: unknown, label: string): OutcomeCounts {
  if (!isRecord(value)) {
    throw new Error(`schedule ${label} was invalid`);
  }
  const draws = parseCount(value.draws, `${label}.draws`);
  const games = parseCount(value.games, `${label}.games`);
  const losses = parseCount(value.losses, `${label}.losses`);
  const wins = parseCount(value.wins, `${label}.wins`);
  if (draws + losses + wins !== games) {
    throw new Error(`schedule ${label} W/D/L did not sum to games`);
  }
  return Object.freeze({ draws, games, losses, wins });
}

function parseDeckOutcomeCounts(value: unknown, label: string): DeckOutcomeCounts {
  if (!isRecord(value)) {
    throw new Error(`schedule ${label} was invalid`);
  }
  const asNorth = parseOutcomeCounts(value.asNorth, `${label}.asNorth`);
  const asSouth = parseOutcomeCounts(value.asSouth, `${label}.asSouth`);
  const draws = parseCount(value.draws, `${label}.draws`);
  const games = parseCount(value.games, `${label}.games`);
  const losses = parseCount(value.losses, `${label}.losses`);
  const wins = parseCount(value.wins, `${label}.wins`);
  if (draws + losses + wins !== games || asNorth.games + asSouth.games !== games) {
    throw new Error(`schedule ${label} seat splits did not match totals`);
  }
  return Object.freeze({ asNorth, asSouth, draws, games, losses, wins });
}

function parseSummary(value: unknown, gameCount: number): ScheduleSummary {
  if (!isRecord(value)
    || !isRecord(value.byDeck)
    || !isRecord(value.bySeat)
    || !isRecord(value.length)
    || !isRecord(value.reliability)
    || !isRecord(value.uncertainty)
    || value.eligibility !== 'unranked_partial_rules_unverified_authority'
    || !Number.isSafeInteger(value.seatEffect)) {
    throw new Error('Rust schedule summary did not match the expected contract');
  }
  const games = parseCount(value.games, 'summary.games');
  if (games !== gameCount) {
    throw new Error('schedule summary games did not match gameCount');
  }
  const bySeat = Object.freeze({
    north: parseOutcomeCounts(value.bySeat.north, 'bySeat.north'),
    south: parseOutcomeCounts(value.bySeat.south, 'bySeat.south'),
  });
  const byDeck = Object.freeze({
    north: parseDeckOutcomeCounts(value.byDeck.north, 'byDeck.north'),
    south: parseDeckOutcomeCounts(value.byDeck.south, 'byDeck.south'),
  });
  if (bySeat.north.games !== games || bySeat.south.games !== games) {
    throw new Error('schedule seat counts did not match games');
  }
  if (value.reliability.allReplayVerified !== true
    && value.reliability.allReplayVerified !== false) {
    throw new Error('schedule reliability.allReplayVerified was invalid');
  }
  const reliability = Object.freeze({
    allReplayVerified: value.reliability.allReplayVerified,
    replayFailedGames: parseCount(value.reliability.replayFailedGames, 'replayFailedGames'),
    replayVerifiedGames: parseCount(value.reliability.replayVerifiedGames, 'replayVerifiedGames'),
  });
  if (reliability.replayVerifiedGames + reliability.replayFailedGames !== games
    || reliability.allReplayVerified !== (reliability.replayFailedGames === 0)) {
    throw new Error('schedule reliability did not match games');
  }
  const uncertainty = Object.freeze({
    firstPlayerScore: parseCount(value.uncertainty.firstPlayerScore, 'firstPlayerScore'),
    scoreDenominator: parseCount(value.uncertainty.scoreDenominator, 'scoreDenominator'),
  });
  if (uncertainty.firstPlayerScore !== bySeat.north.wins * 2 + bySeat.north.draws
    || uncertainty.scoreDenominator !== games * 2
    || value.seatEffect !== bySeat.north.wins - bySeat.south.wins) {
    throw new Error('schedule uncertainty or seat effect did not match W/D/L');
  }
  return Object.freeze({
    byDeck,
    bySeat,
    eligibility: 'unranked_partial_rules_unverified_authority',
    games,
    length: Object.freeze({
      totalActions: parseCount(value.length.totalActions, 'totalActions'),
      totalFights: parseCount(value.length.totalFights, 'totalFights'),
      totalTurns: parseCount(value.length.totalTurns, 'totalTurns'),
    }),
    reliability,
    seatEffect: value.seatEffect as number,
    uncertainty,
  });
}

function parseSchedule(value: unknown): SyntheticScheduleReport {
  if (!isRecord(value)
    || value.classification !== 'unranked_partial_rules_unverified_authority'
    || value.failurePolicy !== 'abort'
    || value.status !== 'completed'
    || value.schemaVersion !== 1
    || !Array.isArray(value.plannedSeeds)
    || !Array.isArray(value.completedSeeds)
    || !Number.isSafeInteger(value.gameCount)
    || typeof value.scheduleId !== 'string'
    || !/^sha256:[0-9a-f]{64}$/u.test(value.scheduleId)) {
    throw new Error('Rust schedule report did not match the expected contract');
  }
  return Object.freeze({
    classification: 'unranked_partial_rules_unverified_authority',
    completedSeeds: Object.freeze(
      value.completedSeeds.map((seed) => {
        if (!Number.isSafeInteger(seed)) throw new Error('schedule completedSeeds was invalid');
        return seed as number;
      }),
    ),
    failurePolicy: 'abort',
    gameCount: value.gameCount as number,
    plannedSeeds: Object.freeze(
      value.plannedSeeds.map((seed) => {
        if (!Number.isSafeInteger(seed)) throw new Error('schedule plannedSeeds was invalid');
        return seed as number;
      }),
    ),
    schemaVersion: 1,
    scheduleId: value.scheduleId as Sha256Hash,
    status: 'completed',
    summary: parseSummary(value.summary, value.gameCount as number),
  });
}

/** Runs one synthetic seat-swapped seed-block schedule. */
export function runGameSchedule(
  seeds: readonly number[],
  workers = 1,
  artifactsDir?: string,
): SyntheticScheduleReport {
  if (seeds.length === 0 || seeds.length > 128) {
    throw new RangeError('schedule must contain 1-128 seeds');
  }
  if (!Number.isSafeInteger(workers) || workers < 1 || workers > 8) {
    throw new RangeError('workers must be 1-8');
  }
  return parseSchedule(runRustEngineCommand([
    'schedule',
    ...(artifactsDir === undefined ? [] : ['--out', artifactsDir]),
    String(workers),
    ...seeds.map(String),
  ]));
}
