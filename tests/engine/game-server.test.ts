import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import test from 'node:test';

import { createGamePrototypeServer } from '../../src/prototype/game-server.ts';

type JsonObject = Record<string, unknown>;

const server = createGamePrototypeServer(31);
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

function actions(response: JsonObject): JsonObject[] {
  return response.actions as JsonObject[];
}

function descriptor(action: JsonObject): JsonObject {
  return action.descriptor as JsonObject;
}

function findAction(response: JsonObject, predicate: (value: JsonObject) => boolean): JsonObject {
  const found = actions(response).find((candidate) => predicate(descriptor(candidate)));
  assert.ok(found, 'expected legal action');
  return found;
}

async function submit(candidate: JsonObject): Promise<JsonObject> {
  return post('/api/action', {
    actionId: candidate.actionId,
    seat: candidate.seat,
    stateVersion: candidate.stateVersion,
  });
}

function keep(response: JsonObject): JsonObject {
  return findAction(response, (value) =>
    value.kind === 'mulligan'
      && (value.atlasOrder as unknown[]).length === 0
      && (value.spellbookOrder as unknown[]).length === 0);
}

function deterministicAction(response: JsonObject): JsonObject {
  const candidates = actions(response);
  const selected = candidates.find((candidate) => {
    const value = descriptor(candidate);
    return value.kind === 'mulligan'
      && (value.atlasOrder as unknown[]).length === 0
      && (value.spellbookOrder as unknown[]).length === 0;
  })
    ?? candidates.find((candidate) => descriptor(candidate).kind === 'play-site')
    ?? candidates.find((candidate) => descriptor(candidate).kind === 'summon-minion')
    ?? candidates.find((candidate) => {
      const value = descriptor(candidate);
      return value.kind === 'draw' && value.zone === 'atlas';
    })
    ?? candidates.find((candidate) => descriptor(candidate).kind === 'end-turn');
  assert.ok(selected, 'expected deterministic legal action');
  return selected;
}

test('playable-core page renders the authoritative 5x4 checkpoint without artwork', async () => {
  const response = await fetch(origin);
  const page = await response.text();
  assert.equal(response.status, 200);
  assert.match(page, /Sorcery Playable Core/);
  assert.match(page, /Unranked · partial rules/);
  assert.equal(page.match(/class="cell"/g)?.length, 20);
  assert.doesNotMatch(page, /<img\b/i);
  assert.match(response.headers.get('content-security-policy') ?? '', /img-src 'none'/);
  assert.match(page, /role="status" aria-live="polite"><strong>Game over<\/strong>/);
  assert.match(page, /Winner:.*Loser:.*Reason:/);
});

test('browser API plays setup through the second-seat draw choice and verifies replay', async () => {
  const northStart = await post('/api/reset', { seed: 31 });
  assert.equal(actions(northStart).length, 76);
  assert.doesNotMatch(JSON.stringify(northStart), /south-(?:site|spell)-/);

  const northKept = await submit(keep(northStart));
  assert.equal(northKept.accepted, true);
  assert.equal(((northKept.view as JsonObject).activeSeat), 'south');
  assert.deepEqual(actions(northKept), []);

  const southStart = await json('/api/view?seat=south');
  const southKept = await submit(keep(southStart));
  assert.equal(southKept.accepted, true);

  const northMain = await json('/api/view?seat=north');
  const played = await submit(findAction(northMain, ({ kind }) => kind === 'play-site'));
  assert.equal(played.accepted, true);
  const summoned = await submit(findAction(played, ({ kind }) => kind === 'summon-minion'));
  assert.equal(summoned.accepted, true);
  assert.equal((((summoned.view as JsonObject).realm as JsonObject).units as unknown[]).length, 1);
  const ended = await submit(findAction(summoned, ({ kind }) => kind === 'end-turn'));
  assert.equal(ended.accepted, true);

  const southDraw = await json('/api/view?seat=south');
  assert.deepEqual(actions(southDraw).map((candidate) => descriptor(candidate).zone), ['atlas', 'spellbook']);
  const drawn = await submit(findAction(southDraw, ({ kind, zone }) => kind === 'draw' && zone === 'atlas'));
  assert.equal(drawn.accepted, true);
  assert.doesNotMatch(JSON.stringify(drawn.receipt), /south-site-/);

  const replay = await post('/api/replay');
  assert.equal(replay.verified, true);
  assert.equal(replay.acceptedActionCount, 6);
  assert.equal(replay.finalStateHash, drawn.stateHash);
});

test('browser API rejects a stale action without exposing or mutating authority', async () => {
  const start = await post('/api/reset', { seed: 37 });
  const command = keep(start);
  const accepted = await submit(command);
  const stale = await submit(command);
  assert.equal(accepted.accepted, true);
  assert.equal(stale.accepted, false);
  assert.equal((stale.reason as JsonObject).code, 'stale_version');
  assert.equal(stale.stateHash, accepted.stateHash);
  assert.equal('session' in stale, false);
});

test('browser API reaches the explicit terminal outcome contract', async () => {
  let current = await post('/api/reset', { seed: 31 });
  for (let count = 0; count < 500; count += 1) {
    const view = current.view as JsonObject;
    if ((view.terminal as JsonObject).status === 'finished') break;
    if (actions(current).length === 0) current = await json('/api/view?seat=' + String(view.decisionSeat));
    current = await submit(deterministicAction(current));
  }

  const terminal = (current.view as JsonObject).terminal as JsonObject;
  assert.equal(terminal.status, 'finished');
  assert.equal(terminal.reason, 'deck_empty');
  assert.notEqual(terminal.winner, terminal.loser);
  assert.deepEqual(actions(current), []);
});
