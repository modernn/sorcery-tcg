import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeSparkmageActionParityFixture } from '../../scripts/capture-sparkmage-action-parity.ts';
import {
  opaqueActionId,
  orderLegalActions,
  type EngineActionDescriptor,
} from '../../src/engine/contract.ts';

type Fixture = Readonly<{
  actions: readonly Readonly<{
    actionId: string;
    descriptor: EngineActionDescriptor;
    label: string;
  }>[];
  avatarInstanceId: string;
  canonicalActionIds: readonly string[];
  contract: string;
  schemaVersion: number;
  seat: 'north';
  source: string;
  stateVersion: number;
}>;

const fixtureUrl = new URL('./fixtures/sparkmage-action-v1.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

test('Sparkmage fixture regenerates byte-identically from Rust legality', async () => {
  assert.equal(await serializeSparkmageActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'rust-legality-engine');
  assert.equal(fixture.actions.length, 6);
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.kind),
    Array(6).fill('activate-sparkmage'),
  );
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.targetLocation),
    ['B3', 'B4', 'C3', 'C4', 'D3', 'D4'].map((cell) => ({ cell, region: 'surface' })),
  );
  assert.deepEqual(
    fixture.actions.map(({ label }) => label),
    ['B3', 'B4', 'C3', 'C4', 'D3', 'D4']
      .map((cell) => `Tap Sparkmage to deal 0 to a random other unit at ${cell}`),
  );

  for (const action of fixture.actions) {
    assert.deepEqual(Object.keys(action.descriptor).sort(), [
      'kind',
      'sourceInstanceId',
      'targetLocation',
    ]);
    assert.equal(action.descriptor.sourceInstanceId, fixture.avatarInstanceId);
    assert.equal(
      opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, action.descriptor),
      action.actionId,
    );
  }

  const ordered = orderLegalActions(fixture.actions.map((action) => ({
    ...action,
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ actionId }) => actionId), fixture.canonicalActionIds);
});
