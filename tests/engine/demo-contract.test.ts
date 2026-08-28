import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import test from 'node:test';
import { pathToFileURL } from 'node:url';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import {
  createDemoManifest,
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

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');

function newSession(seed: number): DemoSession {
  return createDemoSession(createDemoManifest(seed));
}

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
  let session = newSession(seed);
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
  const replayed = replayDemo(first.manifest, actionIds);
  const second = play(0x1020_3040);

  assert.equal(canonicalJson(replayed), canonicalJson(first));
  assert.equal(canonicalJson(second), canonicalJson(first));
  assert.equal(hashDemoState(replayed.state), hashDemoState(first.state));
  assert.equal(verifyDemoReplay(first.manifest, actionIds, first.transcript), true);
  assert.deepEqual(first.transcript.flatMap(({ events }) => events.map(({ type }) => type)), [
    'marker-drawn',
    'marker-revealed',
    'turn-passed',
    'marker-drawn',
    'marker-revealed',
    'demo-finished',
  ]);
  assert.deepEqual(first.transcript.map(({ receiptSequence }) => receiptSequence), [1, 2, 3, 4, 5, 6]);
  assert.ok(first.transcript.every(({ receiptId }) => /^sha256:[0-9a-f]{64}$/.test(receiptId)));
  assert.equal(first.transcript[0]?.randomDraws.length, 1);
  assert.deepEqual(first.transcript[0]?.randomDraws[0]?.domain, {
    kind: 'uint32', maximum: 0xffff_ffff, minimum: 0,
  });
  for (let index = 1; index < first.transcript.length; index += 1) {
    assert.equal(first.transcript[index]?.preStateHash, first.transcript[index - 1]?.postStateHash);
  }
});

test('hidden marker identity does not change the opponent observation or legal actions', () => {
  const hideFromSouth = (seed: number): DemoSession => {
    let session = accept(newSession(seed), 'Draw hidden marker');
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
  const initial = newSession(7);
  const draw = requestByLabel(initial, 'Draw hidden marker');
  const accepted = stepDemo(initial, draw);
  assert.equal(accepted.accepted, true);
  const session = accepted.session;
  const before = canonicalJson(session);
  const beforeHash = hashDemoState(session.state);
  const beforePrng = canonicalJson(session.state.engine.prng);

  const stale = stepDemo(session, draw);
  assert.equal(stale.accepted, false);
  assert.deepEqual(stale.reason, {
    code: 'stale_version',
    currentStateHash: beforeHash,
    currentStateVersion: session.state.stateVersion,
    message: 'That action belongs to an earlier game state.',
  });
  assert.equal(stale.session.attempts.length, session.attempts.length + 1);
  const wrongSeat = stepDemo(stale.session, { ...requestByLabel(session, 'Reveal marker'), seat: 'south' });
  assert.equal(wrongSeat.accepted, false);
  assert.equal(wrongSeat.reason.code, 'wrong_seat');
  const unknown = stepDemo(wrongSeat.session, {
    actionId: 'not-engine-issued',
    seat: 'north',
    stateVersion: session.state.stateVersion,
  });
  assert.equal(unknown.accepted, false);
  assert.equal(unknown.reason.code, 'unknown_action');

  assert.equal(canonicalJson(session), before);
  assert.equal(hashDemoState(session.state), beforeHash);
  assert.equal(canonicalJson(session.state.engine.prng), beforePrng);
  assert.equal(unknown.session.transcript.length, 1);
  assert.deepEqual(unknown.session.attempts.slice(-3).map(({ outcome, reasonCode }) => ({ outcome, reasonCode })), [
    { outcome: 'rejected', reasonCode: 'stale_version' },
    { outcome: 'rejected', reasonCode: 'wrong_seat' },
    { outcome: 'rejected', reasonCode: 'unknown_action' },
  ]);

  const finished = play(7);
  const terminal = stepDemo(finished, {
    actionId: 'anything',
    seat: finished.state.activeSeat,
    stateVersion: finished.state.stateVersion,
  });
  assert.equal(terminal.accepted, false);
  assert.equal(terminal.reason.code, 'terminal_state');
  assert.equal(terminal.reason.currentStateHash, hashDemoState(finished.state));
  assert.equal(terminal.session.transcript, finished.transcript);
});

test('authoritative state, observations, sessions, receipts, and legal actions are deeply frozen', () => {
  const initial = newSession(9);
  const observation = observeDemo(initial.state, 'north');
  const actions = legalDemoActions(initial.state, 'north');
  const result = stepDemo(initial, requestByLabel(initial, 'Draw hidden marker'));
  assert.equal(result.accepted, true);

  assertDeeplyFrozen(initial);
  assertDeeplyFrozen(observation);
  assertDeeplyFrozen(actions);
  assertDeeplyFrozen(result);
  assert.deepEqual(
    actions.map(({ descriptor }) => descriptor.kind),
    ['draw', 'pass'],
  );
});

test('TEST-02 fresh processes emit byte-identical full transcripts', () => {
  const demoUrl = pathToFileURL(resolve(REPOSITORY_ROOT, 'src', 'engine', 'demo-contract.ts')).href;
  const canonicalUrl = pathToFileURL(resolve(REPOSITORY_ROOT, 'src', 'authority', 'canonical-json.ts')).href;
  const program = [
    `import { canonicalJson } from ${JSON.stringify(canonicalUrl)};`,
    `import { createDemoManifest, createDemoSession, legalDemoActions, stepDemo } from ${JSON.stringify(demoUrl)};`,
    'let session = createDemoSession(createDemoManifest(270544960));',
    `for (const label of ${JSON.stringify([
      'Draw hidden marker',
      'Reveal marker',
      'Pass turn',
      'Draw hidden marker',
      'Reveal marker',
      'Finish demo',
    ])}) {`,
    'const action = legalDemoActions(session.state, session.state.activeSeat).find((candidate) => candidate.label === label);',
    'if (!action) throw new Error(`missing ${label}`);',
    'const result = stepDemo(session, action);',
    'if (!result.accepted) throw new Error(result.reason.code);',
    'session = result.session;',
    '}',
    'process.stdout.write(canonicalJson({ manifest: session.manifest, state: session.state, transcript: session.transcript }));',
  ].join('');

  const run = (): Buffer => {
    const result = spawnSync(process.execPath, ['--input-type=module', '--eval', program], {
      cwd: REPOSITORY_ROOT,
      maxBuffer: 1_048_576,
    });
    assert.equal(result.status, 0, result.stderr.toString('utf8'));
    assert.equal(result.stderr.length, 0);
    return result.stdout;
  };

  const first = run();
  const second = run();
  assert.deepEqual(first, second);
  assert.match(first.toString('utf8'), /"receiptSequence":6/);
  assert.match(first.toString('utf8'), /"terminal":true/);
});
