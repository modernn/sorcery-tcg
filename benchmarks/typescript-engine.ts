import { cpus, totalmem } from 'node:os';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import {
  createSyntheticDemoManifest,
  selectDeterministicGameAction,
} from '../src/commands/run-game-demo.ts';
import {
  createGameSession,
  stepGame,
  verifyGameReplay,
  type GameSession,
} from '../src/engine/game.ts';
import { runCounterfactualRollouts } from '../src/simulator/counterfactual.ts';

const SEEDS = [0, 31, 0xffff_ffff] as const;
const MAX_ACTIONS = 500;

type Sample = Readonly<{
  durationMs: number;
  latenciesMs?: readonly number[];
  operations: number;
}>;

let peakRssBytes = process.memoryUsage().rss;

function positiveInteger(name: string, fallback: number): number {
  const value = process.env[name] === undefined ? fallback : Number(process.env[name]);
  if (!Number.isSafeInteger(value) || value <= 0) throw new RangeError(`${name} must be a positive integer`);
  return value;
}

function round(value: number): number {
  return Number(value.toFixed(3));
}

function percentile(values: readonly number[], fraction: number): number {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)]!;
}

function recordMemory(): void {
  peakRssBytes = Math.max(peakRssBytes, process.memoryUsage().rss);
}

function summarize(samples: readonly Sample[]): JsonValue {
  const latency = samples.flatMap(({ durationMs, latenciesMs, operations }) =>
    latenciesMs ?? Array.from({ length: operations }, () => durationMs / operations));
  const throughput = samples.map(({ durationMs, operations }) => operations * 1_000 / durationMs);
  const durationMs = samples.reduce((sum, sample) => sum + sample.durationMs, 0);
  const operations = samples.reduce((sum, sample) => sum + sample.operations, 0);
  return {
    aggregate: {
      durationMs: round(durationMs),
      operations,
      perSecond: round(operations * 1_000 / durationMs),
    },
    latencyMs: {
      median: round(percentile(latency, 0.5)),
      p95: round(percentile(latency, 0.95)),
    },
    throughputPerSecond: {
      median: round(percentile(throughput, 0.5)),
      p95: round(percentile(throughput, 0.95)),
    },
  };
}

function totals(samples: readonly Sample[]): Readonly<{
  durationMs: number;
  operations: number;
  perSecond: number;
}> {
  const durationMs = samples.reduce((sum, sample) => sum + sample.durationMs, 0);
  const operations = samples.reduce((sum, sample) => sum + sample.operations, 0);
  return { durationMs, operations, perSecond: operations * 1_000 / durationMs };
}

function runGame(seed: number, transitionLatencies?: number[]): GameSession {
  let session = createGameSession(createSyntheticDemoManifest(seed));
  while (session.state.terminal.status === 'active' && session.transcript.length < MAX_ACTIONS) {
    const action = selectDeterministicGameAction(session);
    const started = performance.now();
    const result = stepGame(session, action);
    const durationMs = performance.now() - started;
    if (!result.accepted) throw new Error(`issued action was rejected: ${result.reason.code}`);
    transitionLatencies?.push(durationMs);
    session = result.session;
  }
  if (session.state.terminal.status !== 'finished') throw new Error(`seed ${seed} exceeded ${MAX_ACTIONS} actions`);
  return session;
}

function transitionSample(seed: number, gamesPerSample: number): Sample {
  const latencies: number[] = [];
  for (let game = 0; game < gamesPerSample; game += 1) runGame((seed + game) >>> 0, latencies);
  recordMemory();
  return {
    durationMs: latencies.reduce((sum, duration) => sum + duration, 0),
    latenciesMs: latencies,
    operations: latencies.length,
  };
}

function replaySample(seed: number, gamesPerSample: number): Sample {
  const latenciesMs: number[] = [];
  for (let game = 0; game < gamesPerSample; game += 1) {
    const started = performance.now();
    const session = runGame((seed + game) >>> 0);
    if (!verifyGameReplay(session)) throw new Error('authoritative replay failed');
    latenciesMs.push(performance.now() - started);
  }
  recordMemory();
  return {
    durationMs: latenciesMs.reduce((sum, duration) => sum + duration, 0),
    latenciesMs,
    operations: gamesPerSample,
  };
}

function searchSample(seed: number, horizon: number): Sample {
  const root = createGameSession(createSyntheticDemoManifest(seed));
  const started = performance.now();
  const report = runCounterfactualRollouts(root, horizon);
  const durationMs = performance.now() - started;
  if (report.status !== 'complete') throw new Error('synthetic search root exceeded width limit');
  recordMemory();
  return {
    durationMs,
    operations: report.branches.reduce((sum, branch) => sum + branch.decisionCount, 0),
  };
}

export function benchmarkTypeScriptEngine(): JsonValue {
  const sampleCount = positiveInteger('BENCHMARK_SAMPLES', 5);
  const gamesPerSample = positiveInteger('BENCHMARK_GAMES_PER_SAMPLE', 1);
  const searchHorizon = positiveInteger('BENCHMARK_SEARCH_HORIZON', 2);
  if (searchHorizon > 32) throw new RangeError('BENCHMARK_SEARCH_HORIZON must be at most 32');

  runGame(SEEDS[0]);
  runCounterfactualRollouts(createGameSession(createSyntheticDemoManifest(SEEDS[0])), 1);
  recordMemory();

  const transitions: Sample[] = [];
  const replays: Sample[] = [];
  const searches: Sample[] = [];
  for (let sample = 0; sample < sampleCount; sample += 1) {
    const seed = SEEDS[sample % SEEDS.length]!;
    transitions.push(transitionSample(seed, gamesPerSample));
    replays.push(replaySample(seed, gamesPerSample));
    searches.push(searchSample(seed, searchHorizon));
  }

  const cpu = cpus()[0];
  const transitionTotals = totals(transitions);
  const replayTotals = totals(replays);
  const searchTotals = totals(searches);
  return {
    aggregate: {
      games: replayTotals.operations,
      gamesPerSecond: round(replayTotals.perSecond),
      peakRssBytes,
      searchNodes: searchTotals.operations,
      searchNodesPerSecond: round(searchTotals.perSecond),
      transitions: transitionTotals.operations,
      transitionsPerSecond: round(transitionTotals.perSecond),
    },
    benchmarkVersion: 1,
    configuration: { gamesPerSample, sampleCount, searchHorizon, seeds: SEEDS },
    environment: {
      arch: process.arch,
      cpu: cpu?.model.trim() ?? 'unknown',
      logicalCpuCount: cpus().length,
      mode: process.env.NODE_ENV === 'production' ? 'node-production' : 'node-default',
      node: process.version,
      platform: process.platform,
      totalMemoryBytes: totalmem(),
    },
    gameReplay: summarize(replays),
    measuredAt: '2026-08-31',
    searchNodes: summarize(searches),
    transitions: summarize(transitions),
  };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.stdout.write(`${canonicalJson(benchmarkTypeScriptEngine())}\n`);
}
