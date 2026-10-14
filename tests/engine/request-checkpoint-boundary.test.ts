import assert from 'node:assert/strict';
import test from 'node:test';
import { Readable } from 'node:stream';
import type { Server } from 'node:http';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { parseGameCheckpoint } from '../../src/engine/checkpoint.ts';
import { createGameManifest, createGameSession, legalGameActions, stepGame } from '../../src/engine/game.ts';
import { shutdownRustLegalitySync } from '../../src/engine/rust-legality-sync.ts';
import { RustSessionClient, RustSessionRpcError } from '../../src/engine/rust-engine.ts';
import { identityHash } from '../../src/authority/hash.ts';
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
      assert.equal((capped.body.recovery as JsonObject).sessionStatus, 'blocked');
      assert.equal((await http(server, 'POST', '/api/checkpoint')).status, 503);
      assert.equal((await http(server, 'POST', '/api/replay')).status, 503);
      assert.equal(canonicalJson(await handle.client.checkpoint()), serialized);
      assert.equal(await handle.client.verifyReplay(), true);
    });
  } finally {
    server.emit('close');
  }
});

function southFirstManifest() {
  const { manifestId: _id, ...fields } = createSyntheticDemoManifest(31);
  void _id;
  return createGameManifest({ ...fields, firstSeat: 'south' });
}

function command(action: JsonObject) {
  return { actionId: action.actionId, seat: action.seat, stateVersion: action.stateVersion };
}

function keep(data: JsonObject) {
  const action = (data.actions as JsonObject[]).find(({ descriptor }) => {
    const value = descriptor as JsonObject;
    return value.kind === 'mulligan' && (value.atlasOrder as unknown[]).length === 0
      && (value.spellbookOrder as unknown[]).length === 0;
  });
  assert.ok(action);
  return command(action);
}

function assertNoSnapshot(data: JsonObject, metadataUnknown = false) {
  for (const key of ['view', 'stateHash', 'actions', 'cardFacts', 'cardNames']) assert.equal(data[key], undefined, key);
  if (metadataUnknown) for (const key of ['opponent', 'presetId', 'seed', 'mode', 'presets']) assert.equal(data[key], undefined, key);
}

test('real native 998/999 fillers preserve exact accepted receipts, computer prefix, active state and replay at capacity', async () => {
  const manifest = southFirstManifest();
  for (const fillers of [998, 999]) {
    const server = createGamePrototypeServer(31, [{ id: 'south-first', label: 'South first', manifest }], 'south');
    const client = await RustSessionClient.start();
    try {
      const initial = await client.newSession(canonicalJson(manifest as unknown as JsonValue));
      for (let index = 0; index < fillers; index++) {
        assert.equal((await client.step({ actionId: 'synthetic-stale', seat: 'north', stateVersion: Number.MAX_SAFE_INTEGER })).accepted, false);
      }
      const checkpoint = await client.checkpoint();
      const before = checkpoint as JsonObject;
      assert.equal((before.requests as unknown[]).length, fillers);
      const exported = await client.exportSession() as JsonObject;
      assert.equal((exported.transcript as unknown[]).length, 0);
      assert.equal((exported.state as JsonObject).stateVersion, 0);
      assert.equal((exported.state as JsonObject).decisionSeat, 'north');
      assert.equal((await client.publicView('north')).stateHash, initial.stateHash);
      const resumed = await http(server, 'POST', '/api/resume?seat=north&opponent=south', canonicalJson(checkpoint));
      assert.equal(resumed.status, 200);
      const result = await http(server, 'POST', '/api/action', keep(resumed.body));
      assert.equal(result.status, 200);
      assert.equal(result.body.accepted, true);
      const count = 999 - fillers;
      assert.equal(result.body.opponentActionCount, count);
      assert.equal((result.body.opponentActions as unknown[]).length, count);
      const receipt = result.body.receipt as JsonObject;
      assert.equal(receipt.receiptSequence, 1);
      assert.equal(receipt.stateVersion, 0);
      assert.equal(receipt.nextStateVersion, 1);
      assert.equal(receipt.preStateHash, initial.stateHash);
      const { receiptId, ...receiptBody } = receipt;
      assert.equal(identityHash(receiptBody as JsonValue), receiptId);
      const view = result.body.view as JsonObject;
      assert.equal(view.stateVersion, 1 + count);
      assert.equal(view.decisionSeat, 'south');
      assert.equal(view.phase, count ? 'main' : 'mulligan');
      assert.equal((view.terminal as JsonObject).status, 'active');
      assert.equal((result.body.recovery as JsonObject).snapshotStatus, 'current');
      assert.equal((result.body.recovery as JsonObject).sessionStatus, 'blocked');
      assert.match((result.body.recovery as JsonObject).message as string, /1000/);
      // Capture from the real HTTP-owned native client without reopening blocked HTTP certification.
      // The method interception only records its receiver, and forwards all calls unchanged.
      const captured: { client?: RustSessionClient } = {};
      const originalView = RustSessionClient.prototype.publicView;
      const mocked = test.mock.method(RustSessionClient.prototype, 'publicView', async function(this: RustSessionClient, seat: 'north' | 'south') {
        captured.client = this;
        return originalView.call(this, seat);
      });
      await http(server, 'GET', '/api/view?seat=north');
      mocked.mock.restore();
      assert.ok(captured.client);
      const final = await captured.client.checkpoint() as JsonObject;
      assert.equal((final.requests as unknown[]).length, 1000);
      assert.equal((await captured.client.exportSession() as JsonObject).transcript instanceof Array, true);
      const finalExport = await captured.client.exportSession() as JsonObject;
      const receipts = finalExport.transcript as JsonObject[];
      assert.equal(receipts.length, 1 + count);
      assert.deepEqual(receipts[0], receipt);
      assert.equal(receipts.at(-1)!.postStateHash, result.body.stateHash);
      assert.equal(await captured.client.verifyReplay(), true);
      assert.equal((await captured.client.replaySteps()).stepCount, 1 + count);
      assert.equal((await http(server, 'POST', '/api/action', keep(resumed.body))).status, 400);
    } finally { server.emit('close'); await client.close(); }
  }
});

test('unknown computer effect retains player and received prefix, never trusts later advanced view with cached hash, explicit replacement clears latch', async () => {
  const server = createGamePrototypeServer(31, [{ id: 'south-first', label: 'South first', manifest: southFirstManifest() }], 'south');
  const original = RustSessionClient.prototype.step;
  let calls = 0;
  const captured: { client?: RustSessionClient } = {};
  const mocked = test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
    captured.client = this;
    calls++;
    if (calls === 3) throw new Error('HIDDEN-SOUTH-DESCRIPTOR');
    return original.call(this, request);
  });
  try {
    const initial = await http(server, 'GET', '/api/view?seat=north');
    const result = await http(server, 'POST', '/api/action', keep(initial.body));
    assert.equal(result.status, 200);
    assert.equal(result.body.accepted, true);
    assert.equal(result.body.opponentActionCount, 1);
    assert.equal((result.body.receipt as JsonObject).nextStateVersion, 1);
    assert.equal((result.body.recovery as JsonObject).snapshotStatus, 'untrusted');
    assertNoSnapshot(result.body);
    assert.doesNotMatch(JSON.stringify(result.body), /HIDDEN-SOUTH/);
    assert.ok(captured.client);
    const viewed = await captured.client.publicView('north');
    const read = test.mock.method(RustSessionClient.prototype, 'publicView', async () => ({
      stateHash: viewed.stateHash, view: { ...(viewed.view as JsonObject), stateVersion: 3 } as JsonValue,
    }));
    const actions = test.mock.method(RustSessionClient.prototype, 'legalActions', async () => []);
    for (const [method, path] of [['GET', '/api/view?seat=north'], ['POST', '/api/checkpoint'], ['POST', '/api/replay'], ['GET', '/api/replay/steps']] as const) {
      const response = await http(server, method, path);
      assert.equal(response.status, 503);
      assertNoSnapshot(response.body);
    }
    assert.equal(read.mock.callCount(), 0);
    assert.equal(actions.mock.callCount(), 0);
    assert.equal(calls, 3);
    read.mock.restore(); actions.mock.restore(); mocked.mock.restore();
    const reset = await http(server, 'POST', '/api/reset', { seed: 31, presetId: 'south-first', opponent: 'south' });
    assert.equal(reset.status, 200);
    assert.equal((reset.body.replacement as JsonObject).status, 'confirmed');
    assert.equal(reset.body.recovery, undefined);
  } finally { mocked.mock.restore(); server.emit('close'); }
});

test('new/resume replacement metadata is paired only with confirmed acknowledgements, conclusive failures retain the old binding', async () => {
  const oldManifest = createSyntheticDemoManifest(31);
  const candidate = southFirstManifest();
  const presets = [{ id: 'old', label: 'Old', manifest: oldManifest }, { id: 'candidate', label: 'Candidate', manifest: candidate }];
  const donor = await RustSessionClient.start();
  await donor.newSession(canonicalJson(candidate as unknown as JsonValue));
  const checkpoint = canonicalJson(await donor.checkpoint());
  await donor.close();
  for (const method of ['newSession', 'resume'] as const) {
    const server = createGamePrototypeServer(31, presets, 'south');
    try {
      const oldView = await http(server, 'GET', '/api/view?seat=north');
      const oldCheckpoint = await http(server, 'POST', '/api/checkpoint');
      const replace = () => method === 'newSession'
        ? http(server, 'POST', '/api/reset', { seed: 31, presetId: 'candidate', opponent: 'manual' })
        : http(server, 'POST', '/api/resume?seat=north&opponent=manual', checkpoint);
      const nativeMethod = method === 'newSession' ? 'new' : 'resume';
      const original = RustSessionClient.prototype[method] as (this: RustSessionClient, input: never) => Promise<JsonObject>;
      const unchanged = test.mock.method(RustSessionClient.prototype, method, async () => {
        throw new RustSessionRpcError('HIDDEN-CANDIDATE', nativeMethod, 'unchanged');
      });
      const failed = await replace();
      assert.equal(failed.status, 400);
      assert.equal((failed.body.replacement as JsonObject).status, 'unchanged');
      assert.doesNotMatch(JSON.stringify(failed.body), /HIDDEN-CANDIDATE/);
      unchanged.mock.restore();
      const retained = await http(server, 'GET', '/api/view?seat=north');
      assert.equal(retained.body.presetId, 'old');
      assert.equal(retained.body.opponent, 'south');
      assert.equal(retained.body.stateHash, oldView.body.stateHash);
      assert.equal((await http(server, 'GET', '/api/view?seat=south')).status, 403);
      assert.equal((await http(server, 'POST', '/api/action', { actionId: 'x', seat: 'south', stateVersion: 0 })).status, 400);
      assert.equal((await http(server, 'POST', '/api/checkpoint')).body.checkpoint, oldCheckpoint.body.checkpoint);
      let calls = 0;
      const counted = test.mock.method(RustSessionClient.prototype, method, async function(this: RustSessionClient, input: never) {
        calls++;
        return original.call(this, input);
      });
      const originalView = RustSessionClient.prototype.publicView;
      let failRead = true;
      const read = test.mock.method(RustSessionClient.prototype, 'publicView', async function(this: RustSessionClient, seat: 'north' | 'south') {
        if (failRead) { failRead = false; throw new Error('HIDDEN-READ'); }
        return originalView.call(this, seat);
      });
      const confirmed = await replace();
      assert.equal(confirmed.status, 200);
      assert.equal((confirmed.body.replacement as JsonObject).status, 'confirmed');
      const ack = (confirmed.body.replacement as JsonObject).receipt as JsonObject;
      assert.match(ack.stateHash as string, /^sha256:[0-9a-f]{64}$/);
      assert.equal(ack.stateVersion, 0);
      assert.equal(ack[method === 'newSession' ? 'manifestId' : 'expectedSessionHash'], method === 'newSession'
        ? candidate.manifestId : (JSON.parse(checkpoint) as JsonObject).expectedSessionHash);
      assert.equal((confirmed.body.recovery as JsonObject).snapshotStatus, 'unavailable');
      assertNoSnapshot(confirmed.body);
      const recovered = await http(server, 'GET', '/api/view?seat=north');
      assert.equal(recovered.status, 200);
      assert.equal(recovered.body.presetId, 'candidate');
      assert.equal(recovered.body.opponent, 'manual');
      assert.equal(recovered.body.stateHash, ack.stateHash);
      assert.equal((recovered.body.view as JsonObject).stateVersion, ack.stateVersion);
      assert.equal((await http(server, 'GET', '/api/view?seat=south')).status, 200);
      assert.equal(calls, 1);
      counted.mock.restore(); read.mock.restore();
      const lost = test.mock.method(RustSessionClient.prototype, method, async function(this: RustSessionClient, input: never) {
        await original.call(this, input);
        throw new Error('HIDDEN-LOST-ACK');
      });
      const uncertain = await replace();
      assert.equal(uncertain.status, 503);
      assert.equal((uncertain.body.replacement as JsonObject).status, 'unknown');
      assertNoSnapshot(uncertain.body, true);
      lost.mock.restore();
      for (const [verb, route] of [['GET', '/api/view?seat=north'], ['POST', '/api/checkpoint'], ['POST', '/api/replay'], ['GET', '/api/replay/steps'], ['POST', '/api/action']] as const) {
        const blocked = await http(server, verb, route, verb === 'POST' && route === '/api/action' ? keep(oldView.body) : undefined);
        assert.equal(blocked.status, 503);
        assertNoSnapshot(blocked.body, true);
      }
      const failedAgain = test.mock.method(RustSessionClient.prototype, method, async () => { throw new RustSessionRpcError('unchanged', nativeMethod, 'unchanged'); });
      assert.equal((await replace()).status, 400);
      assert.equal((await http(server, 'GET', '/api/view?seat=north')).status, 503);
      failedAgain.mock.restore();
      assert.equal((await replace()).status, 200);
      const mismatch = test.mock.method(RustSessionClient.prototype, method, async function(this: RustSessionClient, input: never) {
        const result = await original.call(this, input);
        return { ...result, [method === 'newSession' ? 'manifestId' : 'expectedSessionHash']: identityHash({ wrong: true }) };
      });
      assert.equal((await replace()).status, 503);
      assert.equal((await http(server, 'GET', '/api/view?seat=north')).status, 503);
      mismatch.mock.restore();
    } finally { test.mock.restoreAll(); server.emit('close'); }
  }
});

test('native failed new/resume candidates preserve prior view, journal and pairing', async () => {
  const client = await RustSessionClient.start();
  try {
    const created = await client.newSession(canonicalJson(createSyntheticDemoManifest(31) as unknown as JsonValue));
    await client.step(await client.selectPolicyAction());
    const before = await client.publicView('north');
    const checkpoint = canonicalJson(await client.checkpoint());
    for (const method of ['new', 'resume'] as const) {
      await assert.rejects(method === 'new' ? client.newSession('{}') : client.resume({}), (error: unknown) => {
        assert.ok(error instanceof RustSessionRpcError);
        assert.equal(error.method, method);
        assert.equal(error.sessionEffect, 'unchanged');
        return true;
      });
      assert.deepEqual(await client.publicView('north'), before);
      assert.equal(canonicalJson(await client.checkpoint()), checkpoint);
      assert.equal(await client.verifyReplay(), true);
    }
    const ack = await client.newSession(canonicalJson(southFirstManifest() as unknown as JsonValue));
    assert.notEqual(ack.manifestId, created.manifestId);
    assert.equal((await client.publicView('north')).stateHash, ack.stateHash);
  } finally { await client.close(); }
});

test('whole HTTP operation queue covers every session route, prevents mixed-match responses, and releases after faults', async () => {
  const server = createGamePrototypeServer(31);
  const initial = await http(server, 'GET', '/api/view?seat=north');
  const saved = await http(server, 'POST', '/api/checkpoint');
  const original = RustSessionClient.prototype.step;
  let release!: () => void;
  let entered!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  const started = new Promise<void>((resolve) => { entered = resolve; });
  const step = test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
    entered(); await held; return original.call(this, request);
  });
  const newOriginal = RustSessionClient.prototype.newSession;
  const newSession = test.mock.method(RustSessionClient.prototype, 'newSession', function(this: RustSessionClient, text: string) {
    return newOriginal.call(this, text);
  });
  const publicOriginal = RustSessionClient.prototype.publicView;
  const read = test.mock.method(RustSessionClient.prototype, 'publicView', function(this: RustSessionClient, seat: 'north' | 'south') {
    return publicOriginal.call(this, seat);
  });
  try {
    const action = http(server, 'POST', '/api/action', keep(initial.body));
    await started;
    const calls = read.mock.callCount();
    const followers = [];
    for (const [verb, route, body] of [
      ['POST', '/api/reset', { seed: 42, opponent: 'manual' }],
      ['GET', '/api/view?seat=north', undefined],
      ['POST', '/api/checkpoint', undefined],
      ['POST', '/api/replay', undefined],
      ['GET', '/api/replay/steps', undefined],
      ['POST', '/api/resume?seat=north&opponent=manual', saved.body.checkpoint],
    ] as const) {
      followers.push(http(server, verb, route, body));
      await new Promise((resolve) => setImmediate(resolve));
    }
    assert.equal(read.mock.callCount(), calls);
    assert.equal(newSession.mock.callCount(), 0);
    release();
    const accepted = await action;
    assert.equal(accepted.body.accepted, true);
    assert.equal(accepted.body.seed, 31);
    assert.equal((accepted.body.view as JsonObject).stateVersion, 1);
    const responses = await Promise.all(followers);
    assert.ok(responses.every(({ status }) => status === 200));
    const [reset, viewed, captured, replay, steps, resumed] = responses;
    assert.equal(reset!.body.seed, 42);
    assert.equal(viewed!.body.stateHash, reset!.body.stateHash);
    assert.equal(captured!.body.stateHash, reset!.body.stateHash);
    assert.equal(replay!.body.finalStateHash, reset!.body.stateHash);
    assert.equal(steps!.body.stepCount, 0);
    assert.deepEqual(steps!.body.steps, []);
    assert.equal(steps!.body.finalStateHash, undefined);
    assert.equal(resumed!.body.seed, 31);
    step.mock.restore();
    const failure = test.mock.method(RustSessionClient.prototype, 'step', async () => { throw new Error('unknown'); });
    const pending = http(server, 'POST', '/api/action', keep(resumed!.body));
    const replacement = http(server, 'POST', '/api/reset', { seed: 31, presetId: 'synthetic', opponent: 'manual' });
    assert.equal((await pending).status, 503);
    assert.equal((await replacement).status, 200);
    failure.mock.restore();
    assert.equal((await http(server, 'GET', '/api/view?seat=north')).status, 200);
  } finally { release(); test.mock.restoreAll(); server.emit('close'); }
});

test('complete valid receipts are mandatory and effects belong to the failing step RPC', async () => {
  for (const fault of ['missing', 'hash', 'version', 'event', 'other-method', 'old-engine', 'capacity', 'unsupported'] as const) {
    const server = createGamePrototypeServer(31);
    const original = RustSessionClient.prototype.step;
    try {
      const initial = await http(server, 'GET', '/api/view?seat=north');
      const injected = test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
        if (fault === 'other-method') throw new RustSessionRpcError('hidden', 'publicView', 'capacity');
        if (fault === 'old-engine') throw new Error('session request limit of 1000 reached');
        if (fault === 'capacity' || fault === 'unsupported') throw new RustSessionRpcError('HIDDEN-DETAIL', 'step', fault);
        const result = await original.call(this, request);
        const receipt = result.receipt as JsonObject;
        return { accepted: true, ...(fault === 'missing' ? {} : { receipt: { ...receipt,
          ...(fault === 'hash' ? { receiptId: identityHash({ fake: true }) } : {}),
          ...(fault === 'version' ? { nextStateVersion: 99 } : {}),
          ...(fault === 'event' ? { events: [{ type: 'incomplete' }] } : {}),
        } as JsonValue }) };
      });
      const response = await http(server, 'POST', '/api/action', keep(initial.body));
      assert.equal(response.status, fault === 'capacity' || fault === 'unsupported' ? 400 : 503);
      assert.equal(response.body.accepted, undefined);
      assert.equal(response.body.receipt, undefined);
      assert.doesNotMatch(JSON.stringify(response.body), /HIDDEN-DETAIL|hidden/);
      if (fault === 'capacity' || fault === 'unsupported') {
        assert.equal((response.body.recovery as JsonObject).snapshotStatus, 'current');
        assert.equal(response.body.stateHash, initial.body.stateHash);
      } else {
        assert.equal((response.body.recovery as JsonObject).snapshotStatus, 'untrusted');
        assertNoSnapshot(response.body);
      }
      injected.mock.restore();
    } finally { test.mock.restoreAll(); server.emit('close'); }
  }
});

test('BRIDGE counter evidence only: 500 received stub receipts retain prefix and distinguish still-south budget from normal exit', async () => {
  for (const after500 of ['south', 'north', 'terminal'] as const) {
    const server = createGamePrototypeServer(31, undefined, 'south');
    const originalStep = RustSessionClient.prototype.step;
    const originalView = RustSessionClient.prototype.publicView;
    try {
      const initial = await http(server, 'GET', '/api/view?seat=north');
      let viewed = initial.body.view as JsonObject;
      let stateHash = initial.body.stateHash as `sha256:${string}`;
      let count = 0;
      let playerReceipt: JsonValue | undefined;
      const reads = test.mock.method(RustSessionClient.prototype, 'publicView', async () => ({ stateHash, view: viewed as JsonValue }));
      const policy = test.mock.method(RustSessionClient.prototype, 'selectPolicyAction', async () => ({
        actionId: identityHash({ stub: count }), seat: 'south' as const, stateVersion: viewed.stateVersion as number,
        label: 'HIDDEN-SOUTH-LABEL', descriptor: { kind: 'end-turn', privateSentinel: 'HIDDEN-SOUTH-DESCRIPTOR' },
      }));
      const steps = test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
        if (request.seat === 'north') {
          const result = await originalStep.call(this, request);
          playerReceipt = result.receipt;
          const native = await originalView.call(this, 'north');
          viewed = native.view as JsonObject; stateHash = native.stateHash;
          return result;
        }
        const body = { ...command(request as unknown as JsonObject), events: [], randomDraws: [],
          nextStateVersion: request.stateVersion + 1, preStateHash: stateHash,
          postStateHash: identityHash({ bridgeStub: count + 1 }), receiptSequence: count + 2 };
        count++;
        stateHash = body.postStateHash;
        viewed = { ...viewed, stateVersion: body.nextStateVersion,
          decisionSeat: count === 500 && after500 === 'north' ? 'north' : 'south',
          terminal: count === 500 && after500 === 'terminal' ? { status: 'finished' } : { status: 'active' } };
        return { accepted: true, receipt: { ...body, receiptId: identityHash(body as JsonValue) } as JsonValue };
      });
      const result = await http(server, 'POST', '/api/action', keep(initial.body));
      assert.equal(result.status, 200);
      assert.equal(result.body.accepted, true);
      assert.deepEqual(result.body.receipt, playerReceipt);
      assert.equal(result.body.opponentActionCount, 500);
      assert.equal((result.body.opponentActions as unknown[]).length, 500);
      assert.equal(policy.mock.callCount(), 500);
      assert.equal(steps.mock.callCount(), 501);
      assert.equal((result.body.view as JsonObject).stateVersion, 501);
      assert.equal(result.body.stateHash, stateHash);
      assert.doesNotMatch(JSON.stringify(result.body.opponentActions), /HIDDEN|sha256:/);
      if (after500 === 'south') {
        assert.equal((result.body.recovery as JsonObject).stage, 'opponent-budget');
        assert.equal((result.body.recovery as JsonObject).sessionStatus, 'blocked');
      } else assert.equal(result.body.recovery, undefined);
      reads.mock.restore(); policy.mock.restore(); steps.mock.restore();
    } finally { test.mock.restoreAll(); server.emit('close'); }
  }
});

function unsupportedManifest() {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 };
  return createGameManifest({
    authority: { contentHash: identityHash({ fixture: 'http-south-unsupported' }), mode: 'synthetic', revisionId: 'synthetic-http-south-unsupported-v1' },
    cards: {
      'north-avatar': { attack: 1, defense: 1, cardType: 'avatar', drawSpell: false, life: 20 },
      'south-avatar': { attack: 1, defense: 1, cardType: 'avatar', drawSpell: false, life: 20 },
      'north-site': { cardType: 'site', elements: ['earth'] },
      'south-site': { cardType: 'site', elements: ['earth'] },
      'north-minion': { attack: 4, defense: 4, cardType: 'minion', manaCost: 0, takesLessDamage: 1, thresholds },
      'north-ward': { cardType: 'magic', manaCost: 0, grantWardToTargetMinion: true, thresholds },
      'south-rain': { cardType: 'magic', manaCost: 1, damageEachAbovegroundMinion: 1, thresholds },
    },
    decks: {
      north: { atlas: Array(5).fill('north-site') as string[], avatar: 'north-avatar', spellbook: ['north-minion', 'north-ward', 'north-ward'] },
      south: { atlas: Array(5).fill('south-site') as string[], avatar: 'south-avatar', spellbook: Array(5).fill('south-rain') as string[] },
    }, firstSeat: 'south', seed: 31,
  });
}

test('real engine-issued south unsupported damage preserves last valid pair and receipts, inspected HTTP state stays blocked', async () => {
  const manifest = unsupportedManifest();
  const client = await RustSessionClient.start();
  const server = createGamePrototypeServer(31, [{ id: 'warded', label: 'Warded reduction', manifest }], 'south');
  async function acceptWhere(predicate: (descriptor: JsonObject) => boolean) {
    const north = await client.publicView('north');
    const seat = (north.view as JsonObject).decisionSeat as 'north' | 'south';
    const issued = (await client.legalActions(seat)).find(({ descriptor }) => predicate(descriptor));
    assert.ok(issued, 'actual issued setup action: '+JSON.stringify((await client.legalActions(seat)).map(({ descriptor }) => descriptor)));
    const result = await client.step(issued);
    assert.equal(result.accepted, true);
  }
  try {
    await client.newSession(canonicalJson(manifest as unknown as JsonValue));
    for (let index = 0; index < 2; index++) await acceptWhere((d) => d.kind === 'mulligan' && (d.atlasOrder as unknown[]).length === 0 && (d.spellbookOrder as unknown[]).length === 0);
    await acceptWhere((d) => d.kind === 'play-site' && d.cell === 'C1');
    await acceptWhere((d) => d.kind === 'end-turn');
    await acceptWhere((d) => d.kind === 'draw' && d.zone === 'atlas');
    await acceptWhere((d) => d.kind === 'play-site' && d.cell === 'C4');
    await acceptWhere((d) => d.kind === 'summon-minion');
    await acceptWhere((d) => d.kind === 'cast-magic' && d.cardId === 'north-ward');
    const setup = await client.exportSession() as JsonObject;
    assert.equal((setup.transcript as unknown[]).length, 8);
    const checkpoint = await client.checkpoint();
    const resumed = await http(server, 'POST', '/api/resume?seat=north&opponent=south', canonicalJson(checkpoint));
    assert.equal(resumed.status, 200);
    const end = (resumed.body.actions as JsonObject[]).find(({ descriptor }) => (descriptor as JsonObject).kind === 'end-turn');
    assert.ok(end);
    let failedBefore: JsonObject | undefined;
    let failedView: unknown;
    let failedHash: string | undefined;
    const captured: { client?: RustSessionClient } = {};
    const policy = test.mock.method(RustSessionClient.prototype, 'selectPolicyAction', async function(this: RustSessionClient) {
      captured.client = this;
      const issued = await this.legalActions('south');
      const action = issued.find(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook')
        ?? issued.find(({ descriptor }) => descriptor.kind === 'cast-magic' && descriptor.cardId === 'south-rain');
      assert.ok(action, 'south policy interception selects an actual issued draw or damage action');
      if (action.descriptor.kind === 'cast-magic') {
        const view = await this.publicView('north');
        failedView = view.view; failedHash = view.stateHash;
        const position = view.view as JsonObject;
        assert.equal(position.decisionSeat, 'south');
        assert.equal(position.phase, 'main');
        const units = (position.realm as JsonObject).units as JsonObject[];
        assert.ok(units.some((unit) => unit.cardId === 'north-minion' && unit.warded === true));
        failedBefore = await this.exportSession() as JsonObject;
        assert.equal(action.descriptor.cardId, 'south-rain');
      }
      return action;
    });
    const originalStep = RustSessionClient.prototype.step;
    let nativeError: unknown;
    const step = test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
      try { return await originalStep.call(this, request); }
      catch (error) { nativeError = error; throw error; }
    });
    const result = await http(server, 'POST', '/api/action', command(end));
    assert.equal(result.status, 200);
    assert.equal(result.body.accepted, true);
    assert.equal(result.body.opponentActionCount, 1);
    const player = result.body.receipt as JsonObject;
    assert.equal(player.receiptSequence, 9);
    assert.equal(player.nextStateVersion, 9);
    assert.ok(nativeError instanceof RustSessionRpcError);
    assert.equal(nativeError.sessionEffect, 'unsupported');
    assert.equal(nativeError.method, 'step');
    assert.equal((result.body.recovery as JsonObject).sessionStatus, 'blocked');
    assert.equal((result.body.recovery as JsonObject).snapshotStatus, 'current');
    assert.equal((result.body.recovery as JsonObject).actionsAvailable, false);
    assert.deepEqual(result.body.actions, []);
    assert.equal(result.body.stateHash, failedHash);
    assert.deepEqual(result.body.view, failedView);
    assert.ok(failedBefore); assert.ok(captured.client);
    assert.equal((failedBefore.transcript as unknown[]).length, 10);
    assert.equal((failedBefore.attempts as unknown[]).length, 10);
    // The native integration proof checks inaccessible journals directly; HTTP checks the last valid pair and denies certification.
    assert.deepEqual(await captured.client.publicView('north'), { stateHash: failedHash, view: failedView });
    for (const operation of [() => captured.client!.checkpoint(), () => captured.client!.verifyReplay(), () => captured.client!.exportSession(), () => captured.client!.replaySteps()]) await assert.rejects(operation(), /ordering Ward/);
    for (const path of ['/api/checkpoint', '/api/replay', '/api/action']) assert.ok((await http(server, 'POST', path, path === '/api/action' ? command(end) : undefined)).status >= 400);
    assert.equal((await http(server, 'GET', '/api/view?seat=north')).status, 200);
    policy.mock.restore(); step.mock.restore();
  } finally { test.mock.restoreAll(); server.emit('close'); await client.close(); }
});

test('read-only policy/view/actions failures and genuine computer rejection retain known receipts without creating mutation uncertainty', async () => {
  for (const fault of ['policy', 'view', 'actions', 'rejection'] as const) {
    const server = createGamePrototypeServer(31, [{ id: 'south-first', label: 'South first', manifest: southFirstManifest() }], fault === 'view' || fault === 'actions' ? 'manual' : 'south');
    const originalStep = RustSessionClient.prototype.step;
    const originalPolicy = RustSessionClient.prototype.selectPolicyAction;
    const originalView = RustSessionClient.prototype.publicView;
    const originalActions = RustSessionClient.prototype.legalActions;
    let accepted = 0;
    let receivedPlayer: unknown;
    try {
      const initial = await http(server, 'GET', '/api/view?seat=north');
      const steps = test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
        const result = await originalStep.call(this, fault === 'rejection' && accepted === 2
          ? { ...request, actionId: 'synthetic-genuine-rejection' } : request);
        if (result.accepted) {
          accepted++;
          if (accepted === 1) receivedPlayer = result.receipt;
        }
        return result;
      });
      const policy = test.mock.method(RustSessionClient.prototype, 'selectPolicyAction', async function(this: RustSessionClient) {
        if (fault === 'policy' && accepted === 2) throw new RustSessionRpcError('HIDDEN-POLICY', 'selectPolicyAction', 'capacity');
        return originalPolicy.call(this);
      });
      const read = test.mock.method(RustSessionClient.prototype, 'publicView', async function(this: RustSessionClient, seat: 'north' | 'south') {
        if (fault === 'view' && accepted > 0) throw new Error('HIDDEN-READ');
        return originalView.call(this, seat);
      });
      const actions = test.mock.method(RustSessionClient.prototype, 'legalActions', async function(this: RustSessionClient, seat: 'north' | 'south') {
        if (fault === 'actions' && accepted > 0) throw new Error('HIDDEN-ACTIONS');
        return originalActions.call(this, seat);
      });
      const result = await http(server, 'POST', '/api/action', keep(initial.body));
      assert.equal(result.status, 200);
      assert.equal(result.body.accepted, true);
      assert.deepEqual(result.body.receipt, receivedPlayer);
      assert.equal(result.body.opponentActionCount, fault === 'policy' || fault === 'rejection' ? 1 : 0);
      assert.equal((result.body.recovery as JsonObject).sessionStatus, 'ready');
      assert.equal((result.body.recovery as JsonObject).snapshotStatus, fault === 'view' ? 'unavailable' : 'current');
      if (fault === 'view') assertNoSnapshot(result.body);
      if (fault === 'actions') {
        assert.equal((result.body.recovery as JsonObject).actionsAvailable, false);
        assert.deepEqual(result.body.actions, []);
      }
      assert.doesNotMatch(JSON.stringify(result.body), /HIDDEN-/);
      steps.mock.restore(); policy.mock.restore(); read.mock.restore(); actions.mock.restore();
      const later = await http(server, 'GET', '/api/view?seat=north');
      assert.equal(later.status, 200);
      assert.equal(later.body.recovery, undefined);
      assert.equal((await http(server, 'POST', '/api/checkpoint')).status, 200);
      assert.equal((await http(server, 'POST', '/api/replay')).body.verified, true);
    } finally { test.mock.restoreAll(); server.emit('close'); }
  }
});

test('both view version and hash must match the confirmed receipt anchor; valid-looking later reads cannot clear a mismatch', async () => {
  for (const mismatch of ['version', 'hash'] as const) {
    const server = createGamePrototypeServer(31);
    const originalStep = RustSessionClient.prototype.step;
    const originalView = RustSessionClient.prototype.publicView;
    let changed = false;
    try {
      const initial = await http(server, 'GET', '/api/view?seat=north');
      test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
        const result = await originalStep.call(this, request); changed = true; return result;
      });
      const read = test.mock.method(RustSessionClient.prototype, 'publicView', async function(this: RustSessionClient, seat: 'north' | 'south') {
        const value = await originalView.call(this, seat);
        return !changed ? value : { stateHash: mismatch === 'hash' ? identityHash({ syntheticMismatch: true }) : value.stateHash,
          view: { ...(value.view as JsonObject), ...(mismatch === 'version' ? { stateVersion: 2 } : {}) } as JsonValue };
      });
      const result = await http(server, 'POST', '/api/action', keep(initial.body));
      assert.equal(result.status, 200);
      assert.equal(result.body.accepted, true);
      assert.equal((result.body.receipt as JsonObject).nextStateVersion, 1);
      assert.equal((result.body.recovery as JsonObject).snapshotStatus, 'untrusted');
      assertNoSnapshot(result.body);
      read.mock.restore();
      const later = await http(server, 'GET', '/api/view?seat=north');
      assert.equal(later.status, 503);
      assertNoSnapshot(later.body);
      assert.equal((await http(server, 'POST', '/api/checkpoint')).status, 503);
      assert.equal((await http(server, 'POST', '/api/reset', { seed: 31, presetId: 'synthetic', opponent: 'manual' })).status, 200);
    } finally { test.mock.restoreAll(); server.emit('close'); }
  }
});

test('malformed successful post-player and post-prefix views preserve exact acceptance and known pair as unavailable presentation', async () => {
  for (const afterReceipts of [1, 2]) {
    for (const malformed of ['missing-terminal', 'null-terminal', 'invalid-terminal-status', 'invalid-decision-seat', 'missing-version', 'negative-version', 'wrong-viewer'] as const) {
      const server = createGamePrototypeServer(31, [{ id: 'south-first', label: 'South first', manifest: southFirstManifest() }], 'south');
      const originalStep = RustSessionClient.prototype.step;
      const originalView = RustSessionClient.prototype.publicView;
      const received: JsonObject[] = [];
      let broken = true;
      const steps = test.mock.method(RustSessionClient.prototype, 'step', async function(this: RustSessionClient, request: Parameters<RustSessionClient['step']>[0]) {
        const result = await originalStep.call(this, request);
        if (result.accepted) received.push(result.receipt as JsonObject);
        return result;
      });
      const reads = test.mock.method(RustSessionClient.prototype, 'publicView', async function(this: RustSessionClient, seat: 'north' | 'south') {
        const value = await originalView.call(this, seat);
        if (!broken || received.length < afterReceipts) return value;
        const view = { ...(value.view as JsonObject) };
        if (malformed === 'missing-terminal') delete view.terminal;
        if (malformed === 'null-terminal') view.terminal = null;
        if (malformed === 'invalid-terminal-status') view.terminal = { status: ['active'] };
        if (malformed === 'invalid-decision-seat') view.decisionSeat = null;
        if (malformed === 'missing-version') delete view.stateVersion;
        if (malformed === 'negative-version') view.stateVersion = -1;
        if (malformed === 'wrong-viewer') view.viewer = 'south';
        return { stateHash: value.stateHash, view: view as JsonValue };
      });
      try {
        const initial = await http(server, 'GET', '/api/view?seat=north');
        assert.equal(initial.status, 200);
        const result = await http(server, 'POST', '/api/action', keep(initial.body));
        assert.equal(result.status, 200, malformed);
        assert.equal(result.body.accepted, true);
        assert.deepEqual(result.body.receipt, received[0]);
        assert.equal(received.length, afterReceipts);
        assert.equal(steps.mock.callCount(), afterReceipts);
        assert.equal(result.body.opponentActionCount, afterReceipts - 1);
        assert.equal((result.body.opponentActions as unknown[]).length, afterReceipts - 1);
        assert.equal((result.body.receipt as JsonObject).postStateHash, received[0]!.postStateHash);
        const recovery = result.body.recovery as JsonObject;
        assert.equal(recovery.snapshotStatus, 'unavailable');
        assert.equal(recovery.sessionStatus, 'ready', 'Malformed read is not evidence of unknown mutation');
        assert.equal(recovery.metadataStatus, 'known');
        assert.equal(recovery.actionsAvailable, false);
        assertNoSnapshot(result.body);
        broken = false;
        const later = await http(server, 'GET', '/api/view?seat=north');
        assert.equal(later.status, 200);
        assert.equal(later.body.recovery, undefined);
        assert.equal((later.body.view as JsonObject).stateVersion, received.at(-1)!.nextStateVersion);
        assert.equal(later.body.stateHash, received.at(-1)!.postStateHash);
        assert.equal(steps.mock.callCount(), afterReceipts, 'A recovery read never resumes the computer');
        assert.equal((await http(server, 'POST', '/api/replay')).body.verified, true);
      } finally { reads.mock.restore(); steps.mock.restore(); server.emit('close'); }
    }
  }
});
