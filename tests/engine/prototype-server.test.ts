import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import test from 'node:test';

import { createPrototypeServer } from '../../src/prototype/server.ts';

type JsonObject = Record<string, unknown>;

const server = createPrototypeServer();
await new Promise<void>((resolve, reject) => {
  server.once('error', reject);
  server.listen(0, '127.0.0.1', resolve);
});
const address = server.address() as AddressInfo;
const origin = `http://127.0.0.1:${address.port}`;

test.after(async () => {
  await new Promise<void>((resolve, reject) => {
    server.close((error) => error ? reject(error) : resolve());
  });
});

async function json(path: string, init?: RequestInit): Promise<JsonObject> {
  const response = await fetch(`${origin}${path}`, init);
  const body = await response.json() as JsonObject;
  assert.equal(response.status, 200, JSON.stringify(body));
  return body;
}

async function post(path: string, body?: unknown): Promise<JsonObject> {
  return json(path, {
    method: 'POST',
    ...(body === undefined ? {} : {
      body: JSON.stringify(body),
      headers: { 'content-type': 'application/json' },
    }),
  });
}

function observation(response: JsonObject): JsonObject {
  return response.view as JsonObject;
}

function actions(response: JsonObject): JsonObject[] {
  return response.actions as JsonObject[];
}

function action(response: JsonObject, label: string): JsonObject {
  const found = actions(response).find((candidate) => candidate.label === label);
  assert.ok(found, `${label} must be legal`);
  return found;
}

async function reset(seed = 1): Promise<JsonObject> {
  return post('/api/reset', { seed });
}

async function submit(candidate: JsonObject): Promise<JsonObject> {
  return post('/api/action', {
    actionId: candidate.actionId,
    seat: candidate.seat,
    stateVersion: candidate.stateVersion,
  });
}

test('prototype page renders its synthetic scope and visual-only 5x4 realm', async () => {
  const response = await fetch(origin);
  const page = await response.text();

  assert.equal(response.status, 200);
  assert.match(response.headers.get('content-security-policy') ?? '', /img-src 'none'/);
  assert.match(page, /Sorcery Engine Lab/);
  assert.match(page, /Synthetic/);
  assert.match(page, /Unranked/);
  assert.match(page, /Rules pending/);
  assert.equal(page.match(/class="cell"/g)?.length, 20);
  assert.doesNotMatch(page, /<img\b/i);
});

test('seat-scoped views redact an opponent hidden marker identity', async () => {
  const start = await reset(7);
  const accepted = await submit(action(start, 'Draw hidden marker'));
  const northMarker = (observation(accepted).markers as JsonObject).north as JsonObject;
  assert.equal(northMarker.status, 'hidden');
  assert.match(northMarker.identity as string, /^synthetic-marker:/);

  const south = await json('/api/view?seat=south');
  const southView = observation(south);
  assert.deepEqual((southView.markers as JsonObject).north, { status: 'hidden' });
  assert.doesNotMatch(JSON.stringify(south), /synthetic-marker:/);
});

test('an engine-issued action is accepted with a versioned hash receipt', async () => {
  const start = await reset(11);
  const preStateHash = start.stateHash;
  const accepted = await submit(action(start, 'Draw hidden marker'));
  const receipt = accepted.receipt as JsonObject;

  assert.equal(accepted.accepted, true);
  assert.equal('session' in accepted, false);
  assert.equal(receipt.stateVersion, 0);
  assert.equal(receipt.nextStateVersion, 1);
  assert.equal(receipt.preStateHash, preStateHash);
  assert.equal(receipt.postStateHash, accepted.stateHash);
  assert.notEqual(accepted.stateHash, preStateHash);
});

test('resubmitting a stale command rejects without changing state hash', async () => {
  const start = await reset(13);
  const command = action(start, 'Draw hidden marker');
  const accepted = await submit(command);
  const stale = await submit(command);

  assert.equal(stale.accepted, false);
  assert.equal('session' in stale, false);
  assert.equal(stale.reason, 'stale_version');
  assert.equal(stale.stateHash, accepted.stateHash);
  assert.deepEqual(stale.view, accepted.view);
});

test('reset replaces the session with the requested deterministic seed', async () => {
  const initial = await reset(17);
  await submit(action(initial, 'Draw hidden marker'));
  const firstReset = await reset(23);
  const secondReset = await reset(23);

  assert.equal(observation(firstReset).stateVersion, 0);
  assert.equal(firstReset.stateHash, secondReset.stateHash);
  assert.deepEqual(firstReset.view, secondReset.view);
  assert.deepEqual(firstReset.actions, secondReset.actions);
});

test('replay verifies the accepted transcript and terminal state hash', async () => {
  const start = await reset(29);
  const drawn = await submit(action(start, 'Draw hidden marker'));
  const revealed = await submit(action(drawn, 'Reveal marker'));
  const replay = await post('/api/replay');

  assert.equal(replay.verified, true);
  assert.equal(replay.acceptedActionCount, 2);
  assert.equal(replay.finalStateHash, revealed.stateHash);
  assert.doesNotMatch(JSON.stringify(replay), /synthetic-marker:/);
});
