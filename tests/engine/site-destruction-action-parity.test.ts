import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  opaqueActionId,
  orderLegalActions,
  type EngineActionDescriptor,
} from '../../src/engine/contract.ts';

const fixture = JSON.parse(readFileSync(
  new URL('./fixtures/site-destruction-action-v1.json', import.meta.url),
  'utf8',
)) as Readonly<{
  actions: readonly Readonly<{ actionId: string; descriptor: EngineActionDescriptor }>[];
  canonicalActionIds: readonly string[];
  contract: string;
  schemaVersion: number;
  seat: 'north';
  source: string;
  stateVersion: number;
}>;

test('Site destruction descriptors and action IDs stay fixed for Rust parity', () => {
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'typescript-legality-engine');
  for (const action of fixture.actions) {
    assert.equal(
      opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, action.descriptor),
      action.actionId,
    );
  }
  const ordered = orderLegalActions(fixture.actions.map((action) => ({
    ...action,
    label: '',
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ actionId }) => actionId), fixture.canonicalActionIds);
});
