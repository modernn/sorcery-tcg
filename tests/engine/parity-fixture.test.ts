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
      steps: Array<{
        events: Array<{ eventId: string; type: string }>;
        eventIds: string[];
        eventTypes: string[];
        selectedAction: { actionId: string; descriptor: { kind: string } };
        selectedActionId: string;
      }>;
    }>;
    prng: unknown[];
  };
  assert.equal(fixture.fixtureVersion, 1);
  assert.equal(fixture.games.length, 3);
  assert.equal(fixture.prng.length, 3);
  assert.equal(fixture.games.every(({ actionIds }) => actionIds.length > 0), true);
  assert.equal(fixture.games.every(({ replay }) => replay.verified), true);
  assert.equal(fixture.games.every(({ actionIds, steps }) => steps.length === actionIds.length), true);
  assert.equal(fixture.games.every(({ steps }) => steps.every((step) =>
    step.selectedAction.actionId === step.selectedActionId
    && step.selectedAction.descriptor.kind.length > 0
    && step.events.map(({ eventId }) => eventId).join(',') === step.eventIds.join(',')
    && step.events.map(({ type }) => type).join(',') === step.eventTypes.join(','))), true);
  const manifestJson = fixture.games.find(({ seed }) => seed === 31)?.manifestJson;
  assert.ok(manifestJson);
  assert.equal(canonicalJson(parseJsonWithDuplicateKeyCheck(manifestJson)), manifestJson);
});
