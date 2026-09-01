import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

type BenchmarkSchema = Readonly<{
  $defs: Readonly<{
    pairedRollout: Readonly<{
      properties: Readonly<{
        estimated60SecondCapacity: Readonly<{
          properties: Readonly<{
            method: Readonly<{ const: string }>;
            workload: Readonly<{ const: string }>;
          }>;
        }>;
        throughputPerSecond: Readonly<{ required: readonly string[] }>;
      }>;
      required: readonly string[];
    }>;
  }>;
  $id: string;
  properties: Readonly<{
    benchmarkVersion: Readonly<{ const: number }>;
    configuration: Readonly<{ required: readonly string[] }>;
    pairedRollout: Readonly<{ $ref: string }>;
  }>;
  required: readonly string[];
  title: string;
}>;

const schemaUrl = new URL('../../benchmarks/rust-engine-benchmark.schema.json', import.meta.url);

test('the public Rust benchmark schema tracks the v2 paired-rollout contract', () => {
  const schema = JSON.parse(readFileSync(schemaUrl, 'utf8')) as BenchmarkSchema;

  assert.equal(schema.$id, 'https://sorcery.local/schemas/rust-engine-benchmark-v2.json');
  assert.equal(schema.title, 'Sorcery Rust engine benchmark v2');
  assert.equal(schema.properties.benchmarkVersion.const, 2);
  assert.equal(schema.required.includes('pairedRollout'), true);
  assert.equal(
    schema.properties.configuration.required.includes('pairedRolloutRepetitionsPerSample'),
    true,
  );
  assert.equal(schema.properties.pairedRollout.$ref, '#/$defs/pairedRollout');
  assert.deepEqual(schema.$defs.pairedRollout.required, [
    'aggregate',
    'estimated60SecondCapacity',
    'pairLatencyMs',
    'replayPreflightOrientations',
    'samples',
    'seed',
    'throughputPerSecond',
    'uniqueSeeds',
  ]);
  assert.equal(
    schema.$defs.pairedRollout.properties.estimated60SecondCapacity.properties.method.const,
    'singleThreadShortSampleEstimate',
  );
  assert.equal(
    schema.$defs.pairedRollout.properties.estimated60SecondCapacity.properties.workload.const,
    'twoSeatSpeculativeRolloutOnly',
  );
  assert.deepEqual(schema.$defs.pairedRollout.properties.throughputPerSecond.required, [
    'gamesMedian',
    'gamesP95',
    'pairsMedian',
    'pairsP95',
    'transitionsMedian',
    'transitionsP95',
  ]);
});
