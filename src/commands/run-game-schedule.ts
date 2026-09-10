import { runRustEngineCommand, type Sha256Hash } from '../engine/rust-engine.ts';

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export type SyntheticScheduleReport = Readonly<{
  classification: 'unranked_partial_rules_unverified_authority';
  completedSeeds: readonly number[];
  failurePolicy: 'abort';
  gameCount: number;
  plannedSeeds: readonly number[];
  schemaVersion: 1;
  scheduleId: Sha256Hash;
  status: 'completed';
}>;

function parseSchedule(value: unknown): SyntheticScheduleReport {
  if (!isRecord(value)
    || value.classification !== 'unranked_partial_rules_unverified_authority'
    || value.failurePolicy !== 'abort'
    || value.status !== 'completed'
    || value.schemaVersion !== 1
    || !Array.isArray(value.plannedSeeds)
    || !Array.isArray(value.completedSeeds)
    || !isRecord(value.gauntlet)
    || !Number.isSafeInteger(value.gauntlet.gameCount)
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
    gameCount: value.gauntlet.gameCount as number,
    plannedSeeds: Object.freeze(
      value.plannedSeeds.map((seed) => {
        if (!Number.isSafeInteger(seed)) throw new Error('schedule plannedSeeds was invalid');
        return seed as number;
      }),
    ),
    schemaVersion: 1,
    scheduleId: value.scheduleId as Sha256Hash,
    status: 'completed',
  });
}

/** Runs one synthetic seat-swapped seed-block schedule. */
export function runGameSchedule(seeds: readonly number[], workers = 1): SyntheticScheduleReport {
  if (seeds.length === 0 || seeds.length > 128) {
    throw new RangeError('schedule must contain 1-128 seeds');
  }
  if (!Number.isSafeInteger(workers) || workers < 1 || workers > 8) {
    throw new RangeError('workers must be 1-8');
  }
  return parseSchedule(runRustEngineCommand([
    'schedule',
    String(workers),
    ...seeds.map(String),
  ]));
}
