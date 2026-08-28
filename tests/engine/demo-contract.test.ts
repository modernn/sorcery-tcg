import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import {
  createDemoSession,
  hashDemoState,
  legalDemoActions,
  observeDemo,
  replayDemo,
  stepDemo,
  verifyDemoReplay,
  type DemoActionRequest,
  type DemoSession,
} from '../../src/engine/demo-contract.ts';

function requestByLabel(session: DemoSession, label: string): DemoActionRequest {
  const action = legalDemoActions(session.state, session.state.activeSeat).find(
    (candidate) => candidate.label === label,
  );
  assert.ok(action, `missing action: ${label}`);
  return action;
}

function accept(session: DemoSession, label: string): DemoSession {
  const result = stepDemo(session, requestByLabel(session, label));
  assert.equal(result.accepted, true);
  return result.session;
}

function play(seed: number): DemoSession {
  let session = createDemoSession(seed);
  for (const label of [
    'Draw hidden marker',
    'Reveal marker',
    'Pass turn',
    'Draw hidden marker',
    'Reveal marker',
    'Finish demo',
  ]) {
    session = accept(session, label);
  }
  return session;
}

function assertDeeplyFrozen(value: unknown): void {
  if (value === null || typeof value !== 'object') return;
  assert.equal(Object.isFrozen(value), true);
  for (const child of Object.values(value)) assertDeeplyFrozen(child);
}

test('same seed and action IDs produce byte-identical receipts, events, and hashes', () => {
  const first = play(0x1020_3040);
  const actionIds = first.transcript.map(({ actionId }) => actionId);
  const replayed = replayDemo(0x1020_3040, actionIds);
  const second = play(0x1020_3040);

  assert.equal(canonicalJson(replayed), canonicalJson(first));
  assert.equal(canonicalJson(second), canonicalJson(first));
  assert.equal(hashDemoState(replayed.state), hashDemoState(first.state));
  assert.equal(verifyDemoReplay(0x1020_3040, actionIds, first.transcript), true);
  assert.deepEqual(first.transcript.map(({ event }) => event.type), [
    'marker-drawn',
    'marker-revealed',
    'turn-passed',
    'marker-drawn',
    'marker-revealed',
    'demo-finished',
  ]);
  for (let index = 1; index < first.transcript.length; index += 1) {
    assert.equal(first.transcript[index]?.preStateHash, first.transcript[index - 1]?.postStateHash);
  }
});

test('hidden marker identity does not change the opponent observation or legal actions', () => {
  const hideFromSouth = (seed: number): DemoSession => {
    let session = accept(createDemoSession(seed), 'Draw hidden marker');
    session = accept(session, 'Pass turn');
    return session;
  };
  const first = hideFromSouth(1);
  const second = hideFromSouth(2);

  assert.notEqual(first.state.markers.north.status === 'hidden' && first.state.markers.north.identity,
    second.state.markers.north.status === 'hidden' && second.state.markers.north.identity);
  assert.equal(canonicalJson(observeDemo(first.state, 'south')), canonicalJson(observeDemo(second.state, 'south')));
  assert.equal(
    canonicalJson(legalDemoActions(first.state, 'south')),
    canonicalJson(legalDemoActions(second.state, 'south')),
  );
  assert.deepEqual(observeDemo(first.state, 'south').markers.north, { status: 'hidden' });
});

test('rejections are typed and leave state, hash, PRNG, events, and transcript unchanged', () => {
  const initial = createDemoSession(7);
  const draw = requestByLabel(initial, 'Draw hidden marker');
  const accepted = stepDemo(initial, draw);
  assert.equal(accepted.accepted, true);
  const session = accepted.session;
  const before = canonicalJson(session);
  const beforeHash = hashDemoState(session.state);
  const beforePrng = canonicalJson(session.state.engine.prng);

  const stale = stepDemo(session, draw);
  assert.deepEqual(stale, { accepted: false, reason: 'stale_version', session });
  const wrongSeat = stepDemo(session, { ...requestByLabel(session, 'Reveal marker'), seat: 'south' });
  assert.deepEqual(wrongSeat, { accepted: false, reason: 'wrong_seat', session });
  const unknown = stepDemo(session, {
    actionId: 'not-engine-issued',
    seat: 'north',
    stateVersion: session.state.stateVersion,
  });
  assert.deepEqual(unknown, { accepted: false, reason: 'unknown_action', session });

  assert.equal(canonicalJson(session), before);
  assert.equal(hashDemoState(session.state), beforeHash);
  assert.equal(canonicalJson(session.state.engine.prng), beforePrng);
  assert.equal(session.transcript.length, 1);

  const finished = play(7);
  const terminal = stepDemo(finished, {
    actionId: 'anything',
    seat: finished.state.activeSeat,
    stateVersion: finished.state.stateVersion,
  });
  assert.deepEqual(terminal, { accepted: false, reason: 'terminal_state', session: finished });
});

test('authoritative state, observations, sessions, receipts, and legal actions are deeply frozen', () => {
  const initial = createDemoSession(9);
  const observation = observeDemo(initial.state, 'north');
  const actions = legalDemoActions(initial.state, 'north');
  const result = stepDemo(initial, requestByLabel(initial, 'Draw hidden marker'));
  assert.equal(result.accepted, true);

  assertDeeplyFrozen(initial);
  assertDeeplyFrozen(observation);
  assertDeeplyFrozen(actions);
  assertDeeplyFrozen(result);
  assert.deepEqual(
    [...actions].map(({ actionId }) => actionId),
    [...actions].map(({ actionId }) => actionId).sort(),
  );
});
