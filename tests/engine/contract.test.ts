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

test('shared contract binds opaque actions to descriptor, seat, and state version in canonical order', () => {
  const descriptors = [{ kind: 'pass' }, { kind: 'draw', zone: 'atlas' }] as const;
  const actions = descriptors.map((descriptor): EngineLegalAction => ({
    actionId: opaqueActionId('test-v1', 'north', 3, descriptor),
    descriptor,
    label: descriptor.kind,
    seat: 'north',
    stateVersion: 3,
  }));

  const ordered = orderLegalActions(actions);
  assert.deepEqual(ordered.map(({ descriptor }) => descriptor.kind), ['draw', 'pass']);
  assert.notEqual(actions[0]?.actionId, opaqueActionId('test-v1', 'south', 3, descriptors[0]));
  assert.notEqual(actions[0]?.actionId, opaqueActionId('test-v1', 'north', 4, descriptors[0]));
  assert.equal(Object.isFrozen(ordered), true);
  assert.equal(Object.isFrozen(ordered[0]?.descriptor), true);
});

test('shared contract builds causal events, identified receipts, safe rejections, and separate attempts', () => {
  const actionId = opaqueActionId('test-v1', 'north', 3, { kind: 'draw', zone: 'atlas' });
  const events = createEvents(actionId, 2, 4, [
    { payload: { seat: 'north', zone: 'atlas' }, type: 'card-drawn' },
    { payload: { seat: 'north' }, type: 'priority-retained' },
  ]);
  const receipt = createReceipt({
    actionId,
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
  const acceptedAttempt = createAttempt(1, { actionId, seat: 'north', stateVersion: 3 }, 3, STATE_HASH, {
    receiptId: receipt.receiptId,
  });
  const rejectedAttempt = createAttempt(2, { actionId, seat: 'north', stateVersion: 3 }, 4, NEXT_HASH, {
    reasonCode: rejection.code,
  });

  assert.deepEqual(events.map(({ eventSequence }) => eventSequence), [4, 5]);
  assert.ok(events.every(({ cause }) => cause.actionId === actionId && cause.receiptSequence === 2));
  assert.match(receipt.receiptId, /^sha256:[0-9a-f]{64}$/);
  assert.deepEqual(rejection, {
    code: 'stale_version',
    currentStateHash: NEXT_HASH,
    currentStateVersion: 4,
    message: 'That action belongs to an earlier game state.',
  });
  assert.equal(acceptedAttempt.outcome, 'accepted');
  assert.equal(rejectedAttempt.outcome, 'rejected');
  assert.equal('reasonCode' in receipt, false);
  assert.equal(Object.isFrozen(receipt), true);
  assert.equal(Object.isFrozen(events[0]?.cause), true);
});
