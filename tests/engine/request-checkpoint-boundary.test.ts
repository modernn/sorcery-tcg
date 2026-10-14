import assert from 'node:assert/strict';
import test from 'node:test';
import { Readable } from 'node:stream';
import type { Server } from 'node:http';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { parseGameCheckpoint } from '../../src/engine/checkpoint.ts';
import { createGameSession, legalGameActions, stepGame } from '../../src/engine/game.ts';
import { shutdownRustLegalitySync } from '../../src/engine/rust-legality-sync.ts';
import { withRustSession } from '../../src/engine/rust-session-helpers.ts';
import { createGamePrototypeServer } from '../../src/prototype/game-server.ts';

type JsonObject = Record<string, unknown>;

// Invoke the production handler directly so these boundary checks need no listening socket.
async function http(server: Server, method: string, path: string, body?: unknown): Promise<{
  status: number;
  body: JsonObject;
}> {
  const request = Readable.from(body === undefined ? [] : [Buffer.from(
    typeof body === 'string' ? body : JSON.stringify(body),
  )]);
  Object.assign(request, { method, url: path, headers: { 'content-type': 'application/json' } });
  let status = 0;
  let result: JsonObject = {};
  const response = {
    writeHead(code: number) { status = code; },
    end(text: string) { result = JSON.parse(text) as JsonObject; },
  };
  await server.listeners('request')[0]!(request, response);
  return { status, body: result };
}

const malformed = ['', 'x'.repeat(257), '🦄'.repeat(129)];

test.after(() => shutdownRustLegalitySync());

test('malformed async and synchronous requests preserve snapshots and allow a later legal action', async () => {
  const manifest = createSyntheticDemoManifest(31);
  await withRustSession(manifest, async (handle) => {
    const before = handle.snapshot;
    const checkpoint = (await handle.checkpoint()).serialized;
    for (const actionId of malformed) {
      await assert.rejects(handle.step({ actionId, seat: 'north', stateVersion: 0 }), /invalid action request/);
      assert.strictEqual(handle.snapshot, before);
      assert.equal((await handle.checkpoint()).serialized, checkpoint);
    }
    await assert.rejects(handle.step({ actionId: 'x', seat: 'north', stateVersion: 2 ** 53 }), /unsafe_number/);
    assert.strictEqual(handle.snapshot, before);
    const recovered = await handle.stepAction(await handle.selectPolicyAction());
    assert.equal(recovered.accepted, true);
    assert.equal(handle.snapshot.state.stateVersion, 1);
    // A full LegalAction is accepted by the typed adapter, which projects only request fields.
    const action = await handle.selectPolicyAction();
    assert.equal((await handle.client.step(action)).accepted, true);
    const exported = await handle.client.exportSession() as unknown as { state: { stateVersion: number } };
    assert.equal(exported.state.stateVersion, 2);
    parseGameCheckpoint(canonicalJson(await handle.client.checkpoint()));
  });

  const session = createGameSession(manifest);
  for (const actionId of malformed) {
    assert.throws(() => stepGame(session, { actionId, seat: 'north', stateVersion: 0 }), /invalid action request/);
    assert.equal(session.state.stateVersion, 0);
    assert.equal(session.attempts.length, 0);
  }
  const action = legalGameActions(session.state, 'north')[0]!;
  const stepped = stepGame(session, action);
  assert.equal(stepped.accepted, true);
  assert.equal(stepped.session.state.stateVersion, 1);
});

test('HTTP exact request shape rejects malformed input and retains valid UTF-16 boundaries and opponent stepping', async () => {
  const server = createGamePrototypeServer(31);
  try {
    const before = await http(server, 'POST', '/api/checkpoint');
    assert.equal(before.status, 200);
    for (const request of [
      ...malformed.map((actionId) => ({ actionId, seat: 'north', stateVersion: 0 })),
      { actionId: 'x', seat: 'north', stateVersion: 2 ** 53 },
      { actionId: 'x', seat: 'north', stateVersion: 0, injected: true },
    ]) {
      assert.equal((await http(server, 'POST', '/api/action', request)).status, 400);
      assert.equal((await http(server, 'POST', '/api/checkpoint')).body.checkpoint, before.body.checkpoint);
    }
    for (const actionId of ['x'.repeat(256), '🦄'.repeat(128)]) {
      const rejected = await http(server, 'POST', '/api/action', { actionId, seat: 'north', stateVersion: 0 });
      assert.equal(rejected.status, 200);
      assert.equal(rejected.body.accepted, false);
    }
    const view = await http(server, 'GET', '/api/view?seat=north');
    const action = (view.body.actions as JsonObject[])[0]!;
    const accepted = await http(server, 'POST', '/api/action', {
      actionId: action.actionId, seat: action.seat, stateVersion: action.stateVersion,
    });
    assert.equal(accepted.status, 200);
    assert.equal(accepted.body.accepted, true);
    assert.equal((await http(server, 'POST', '/api/checkpoint')).status, 200);

    const reset = await http(server, 'POST', '/api/reset', { seed: 31, opponent: 'south' });
    assert.equal(reset.status, 200);
    const keep = (reset.body.actions as JsonObject[]).find((candidate) => {
      const descriptor = candidate.descriptor as JsonObject;
      return descriptor.kind === 'mulligan'
        && (descriptor.atlasOrder as unknown[]).length === 0
        && (descriptor.spellbookOrder as unknown[]).length === 0;
    })!;
    const opponent = await http(server, 'POST', '/api/action', {
      actionId: keep.actionId, seat: keep.seat, stateVersion: keep.stateVersion,
    });
    assert.equal(opponent.status, 200);
    assert.equal(opponent.body.accepted, true);
    assert.ok((opponent.body.opponentActionCount as number) > 0);
  } finally {
    server.emit('close');
  }
});

test('capacity errors through RPC, async handle, synchronous stepping and HTTP preserve capture and resume', async () => {
  const manifest = createSyntheticDemoManifest(31);
  const server = createGamePrototypeServer(31);
  try {
    await withRustSession(manifest, async (handle) => {
      const stale = { actionId: 'synthetic-unknown', seat: 'north' as const, stateVersion: Number.MAX_SAFE_INTEGER };
      for (let index = 0; index < 1_000; index += 1) {
        assert.equal((await handle.client.step(stale)).accepted, false);
      }
      const checkpoint = await handle.client.checkpoint();
      const serialized = canonicalJson(checkpoint);
      const parsed = parseGameCheckpoint(serialized);
      assert.equal(parsed.requests.length, 1_000);
      await handle.resume(checkpoint);
      const before = handle.snapshot;
      const legal = await handle.selectPolicyAction();
      for (const request of [stale, legal]) {
        await assert.rejects(handle.client.step(request), /session request limit of 1000 reached/);
        await assert.rejects(handle.step(request), /session request limit of 1000 reached/);
        assert.strictEqual(handle.snapshot, before);
        assert.throws(() => stepGame(before, request), /session request limit of 1000 reached/);
      }
      assert.equal(canonicalJson(await handle.client.checkpoint()), serialized);
      assert.equal(await handle.verifyReplay(), true);

      const injected = JSON.parse(serialized) as { requests: JsonObject[] };
      injected.requests[0]!.injected = true;
      assert.throws(() => parseGameCheckpoint(JSON.stringify(injected)), /checkpoint request 0 is invalid/);
      await assert.rejects(handle.client.resume(injected as unknown as JsonValue), /unknown field/);
      assert.equal(canonicalJson(await handle.client.checkpoint()), serialized);

      const resumed = await http(server, 'POST', '/api/resume?seat=north&opponent=manual', serialized);
      assert.equal(resumed.status, 200);
      const capped = await http(server, 'POST', '/api/action', {
        actionId: legal.actionId, seat: legal.seat, stateVersion: legal.stateVersion,
      });
      assert.equal(capped.status, 400);
      assert.match(capped.body.error as string, /session request limit of 1000 reached/);
      assert.equal((await http(server, 'POST', '/api/checkpoint')).body.checkpoint, serialized);
      assert.equal((await http(server, 'POST', '/api/replay')).body.verified, true);
    });
  } finally {
    server.emit('close');
  }
});
