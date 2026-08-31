import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeEngineParityFixture } from '../../scripts/capture-engine-parity.ts';
import { canonicalJson, parseJsonWithDuplicateKeyCheck } from '../../src/authority/canonical-json.ts';

const FIXTURE_URL = new URL('./fixtures/typescript-parity-v1.json', import.meta.url);

test('TypeScript parity fixture regenerates byte-identically', () => {
  const expected = readFileSync(FIXTURE_URL, 'utf8');
  const regenerated = serializeEngineParityFixture();
  assert.equal(regenerated, expected);

  const fixture = JSON.parse(regenerated) as {
    fixtureVersion: number;
    games: Array<{
      actionIds: string[];
      manifestJson?: string;
      replay: { verified: boolean };
      seed: number;
      steps: unknown[];
    }>;
    prng: unknown[];
  };
  assert.equal(fixture.fixtureVersion, 1);
  assert.equal(fixture.games.length, 3);
  assert.equal(fixture.prng.length, 3);
  assert.equal(fixture.games.every(({ actionIds }) => actionIds.length > 0), true);
  assert.equal(fixture.games.every(({ replay }) => replay.verified), true);
  assert.equal(fixture.games.every(({ steps }) => steps.length === 8), true);
  const manifestJson = fixture.games.find(({ seed }) => seed === 31)?.manifestJson;
  assert.ok(manifestJson);
  assert.equal(canonicalJson(parseJsonWithDuplicateKeyCheck(manifestJson)), manifestJson);
});
