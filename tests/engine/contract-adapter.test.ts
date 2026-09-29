import assert from 'node:assert/strict';
import test from 'node:test';

import {
  createAttempt,
  createEvents,
  createReceipt,
  createRejection,
  opaqueActionId,
  orderLegalActions,
  type EngineLegalAction,
} from '../../src/engine/contract.ts';

const STATE_HASH = 'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' as const;
const NEXT_HASH = 'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb' as const;

test('TypeScript contract adapter freezes its exposed records', () => {
  const descriptor = { kind: 'draw', zone: 'atlas' } as const;
  const action: EngineLegalAction = {
    actionId: opaqueActionId('test-v1', 'north', 3, descriptor),
    descriptor,
    label: 'draw',
    seat: 'north',
    stateVersion: 3,
  };
  const ordered = orderLegalActions([action]);
  assert.equal(Object.isFrozen(ordered), true);
  assert.equal(Object.isFrozen(ordered[0]), true);
  assert.equal(Object.isFrozen(ordered[0]?.descriptor), true);

  const events = createEvents(action.actionId, 2, 4, [
    { payload: { seat: 'north', zone: 'atlas' }, type: 'card-drawn' },
  ]);
  const receipt = createReceipt({
    actionId: action.actionId,
    events,
    nextStateVersion: 4,
    postStateHash: NEXT_HASH,
    preStateHash: STATE_HASH,
    randomDraws: [],
    receiptSequence: 2,
    seat: 'north',
    stateVersion: 3,
  });
  const rejection = createRejection('stale_version', 4, NEXT_HASH);
  const acceptedAttempt = createAttempt(1, action, 3, STATE_HASH, {
    receiptId: receipt.receiptId,
  });

  assert.equal(Object.isFrozen(events), true);
  assert.equal(Object.isFrozen(events[0]), true);
  assert.equal(Object.isFrozen(events[0]?.cause), true);
  assert.equal(Object.isFrozen(receipt), true);
  assert.equal(Object.isFrozen(rejection), true);
  assert.equal(Object.isFrozen(acceptedAttempt), true);
});
