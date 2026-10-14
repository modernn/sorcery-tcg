import assert from 'node:assert/strict';
import { closeSync, fstatSync, mkdirSync, mkdtempSync, openSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { setTimeout } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { runBounded } from '../helpers/bounded-process.ts';
import {
  createRustGameSession,
  rustLegalGameActions,
  rustObserveGame,
  rustStepGame,
  shutdownRustLegalitySync,
} from '../../src/engine/rust-legality-sync.ts';
import type { GameSession } from '../../src/engine/game.ts';

// Synthetic protocol worker only: these tests prove TS transport/cache behavior.
const WORKER = String.raw`
import { appendFileSync, existsSync, readFileSync, writeFileSync, writeSync } from 'node:fs';
import { createInterface } from 'node:readline';
import { setTimeout } from 'node:timers/promises';
const mode = process.env.SYNC_TRANSPORT_CASE;
const log = process.env.SYNC_TRANSPORT_LOG;
const launches = existsSync(log) ? readFileSync(log, 'utf8').trim().split('\n').length : 0;
appendFileSync(log, process.pid + '\n');
let version = 0, failExport = false, failCheckpoint = false, failResume = false;
const text = mode === 'utf8' ? 'é…😀'.repeat(40000) : '';
for await (const line of createInterface({ input: process.stdin })) {
  const request = JSON.parse(line), params = request.params, method = request.method;
  if (launches === 0 && (mode === 'death-first' || mode === 'partial-death')) {
    if (mode === 'partial-death') writeSync(1, Buffer.from('{"id":1,"result":"é').subarray(0, -1));
    process.exit(101);
  }
  let result = {}, error;
  if (method === 'new') {
    version = 0;
    if (JSON.parse(params.manifestJson).seed === 99) {
      version = 99; error = { message: 'synthetic new failure' };
    }
  } else if (method === 'resume') {
    version = params.checkpoint.version;
    if (failResume) {
      failResume = false; version = 99; error = { message: 'synthetic resume failure' };
    }
    if (params.checkpoint.text !== text) throw new Error('checkpoint UTF8 did not round trip');
  } else if (method === 'checkpoint') {
    if (failCheckpoint) { failCheckpoint = false; error = { message: 'synthetic checkpoint failure' }; }
    else result = { checkpoint: { version, text } };
  } else if (method === 'exportSession') {
    if (failExport) { failExport = false; result = {}; }
    else result = { attempts: [], initialRandomDraws: [], transcript: [], state: {
      stateVersion: version, decisionSeat: 'north', text,
    } };
  } else if (method === 'legalActions') {
    result = { actions: [{ actionId: 'accept', seat: 'north', stateVersion: version }] };
  } else if (method === 'publicView') {
    result = { view: { stateVersion: version, text } };
  } else if (method === 'step') {
    const action = params.actionId;
    if (action === 'die') process.exit(101);
    version += 1;
    if (action === 'fail-step' || action === 'fail-resume') {
      failResume = action === 'fail-resume'; error = { message: 'synthetic step failure' };
    } else if (action === 'fail-export') failExport = true;
    else if (action === 'fail-checkpoint') failCheckpoint = true;
    result = { accepted: true, receipt: {} };
    if (action === 'malformed-step') result = {};
    if (action === 'wrong-id') { writeSync(1, JSON.stringify({ id: request.id + 1, result }) + '\n'); continue; }
    if (action === 'invalid-json') { writeSync(1, '{bad}\n'); continue; }
    if (action === 'duplicate-id') { writeSync(1, '{"id":' + request.id + ',"id":' + request.id + '}\n'); continue; }
    if (action === 'malformed-error') { writeSync(1, JSON.stringify({ id: request.id, error: {} }) + '\n'); continue; }
    if (action === 'missing-result') { writeSync(1, JSON.stringify({ id: request.id }) + '\n'); continue; }
    if (action === 'wrong-schema') { writeSync(1, JSON.stringify({ id: request.id, schemaVersion: 2, result }) + '\n'); continue; }
  }
  const response = Buffer.from(JSON.stringify({ id: request.id, schemaVersion: 1, ...(error ? { error } : { result }) }) + '\n');
  const cuts = mode === 'utf8' ? ['é', '…', '😀'].flatMap(character => {
    const bytes = Buffer.from(character), start = response.indexOf(bytes);
    return start < 0 ? [] : Array.from({ length: bytes.length - 1 }, (_, index) => start + index + 1);
  }).sort((a, b) => a - b) : [];
  let offset = 0;
  for (const end of [...cuts, response.length]) {
    while (offset < end) offset += writeSync(1, response, offset, end - offset);
    if (end !== response.length) await setTimeout(5);
  }
  if (mode === 'death-idle' && launches === 0 && method === 'publicView') {
    writeFileSync(log + '.dead', 'exiting'); process.exit(101);
  }
}
`;

function step(session: GameSession, actionId: string) {
  return rustStepGame(session, { actionId, seat: 'north', stateVersion: session.state.stateVersion });
}

function assertOriginal(session: GameSession): void {
  assert.equal(rustLegalGameActions(session.state, 'north')[0]?.stateVersion, 0);
  assert.equal(rustObserveGame(session.state, 'north').stateVersion, 0);
  const sibling = step(session, 'accept');
  assert.equal(sibling.session.state.stateVersion, 1);
  assert.equal(session.state.stateVersion, 0);
  assert.equal(rustLegalGameActions(session.state, 'north')[0]?.stateVersion, 0);
}

async function exercise(mode: string): Promise<void> {
  const manifest = createSyntheticDemoManifest(31);
  const launches = () => readFileSync(process.env.SYNC_TRANSPORT_LOG!, 'utf8').trim().split('\n').length;
  const listeners = process.listenerCount('exit');
  if (mode === 'death-first' || mode === 'partial-death' || mode === 'spawn-error') {
    assert.throws(() => createRustGameSession(manifest), /closed|EPIPE/);
    if (mode === 'spawn-error') process.env.PATH = process.env.SYNC_TRANSPORT_VALID_PATH;
    const recovered = createRustGameSession(manifest);
    assertOriginal(recovered);
    if (mode === 'spawn-error') {
      assert.equal(launches(), 1, 'spawn failure must invalidate the cached worker before recovery');
    }
  } else {
    const snapshot = createRustGameSession(manifest);
    if (mode === 'cache') {
      for (const action of ['fail-step', 'fail-export', 'fail-checkpoint', 'malformed-step']) {
        assert.throws(() => step(snapshot, action));
        assertOriginal(snapshot);
      }
      assert.throws(() => createRustGameSession(createSyntheticDemoManifest(99)), /synthetic new failure/);
      assertOriginal(snapshot);
      assert.throws(() => step(snapshot, 'fail-resume'), /synthetic step failure/);
      assert.throws(() => rustLegalGameActions(snapshot.state, 'north'), /synthetic resume failure/);
      assertOriginal(snapshot);
      assert.equal(launches(), 1, 'ordinary RPC errors must keep the worker usable');
    } else if (mode === 'utf8') {
      const expected = 'é…😀'.repeat(40000);
      assert.equal((snapshot.state as unknown as { text: string }).text, expected);
      createRustGameSession(createSyntheticDemoManifest(32));
      const view = rustObserveGame(snapshot.state, 'north') as unknown as { text: string };
      assert.equal(view.text, expected);
      assert.equal((step(snapshot, 'accept').session.state as unknown as { text: string }).text, expected);
    } else if (mode === 'protocol') {
      for (const action of ['wrong-id', 'invalid-json', 'duplicate-id', 'malformed-error', 'missing-result', 'wrong-schema']) {
        const before = launches();
        assert.throws(() => step(snapshot, action));
        assertOriginal(snapshot);
        assert.equal(launches(), before + 1, 'unusable protocol stream must restart and resume');
      }
    } else if (mode === 'death-later' || mode === 'death-idle') {
      if (mode === 'death-idle') {
        rustObserveGame(snapshot.state, 'north');
        // Block the host event loop: cleanup must not depend on the child exit callback.
        Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 100);
        assert.throws(() => step(snapshot, 'x'.repeat(1_048_576)), /EPIPE|closed/);
      } else assert.throws(() => step(snapshot, 'die'), /closed/);
      assertOriginal(snapshot);
      assert.equal(launches(), 2);
    } else if (mode === 'native-recovery') {
      const originalActions = rustLegalGameActions(snapshot.state, snapshot.state.decisionSeat);
      const pid = Number(readFileSync(process.env.SYNC_TRANSPORT_LOG!, 'utf8').trim());
      process.kill(pid, 'SIGKILL');
      // Keep the host event loop blocked so the transport must detect worker death itself.
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 100);
      assert.throws(() => rustLegalGameActions(snapshot.state, snapshot.state.decisionSeat), /EPIPE|closed/);
      assert.deepEqual(rustLegalGameActions(snapshot.state, snapshot.state.decisionSeat), originalActions);
      assert.equal(launches(), 2);
    } else if (mode === 'cleanup') {
      shutdownRustLegalitySync();
      await setTimeout(30);
      const fds = readdirSync('/proc/self/fd').length;
      for (let index = 0; index < 12; index += 1) {
        createRustGameSession(manifest);
        shutdownRustLegalitySync();
        const reused = openSync(process.env.SYNC_TRANSPORT_LOG!, 'r');
        await setTimeout(30);
        assert.ok(fstatSync(reused).isFile(), 'late exit cleanup must not close a reused descriptor');
        closeSync(reused);
        assert.equal(process.listenerCount('exit'), listeners);
        assert.equal(readdirSync('/proc/self/fd').length, fds);
      }
    }
  }
  shutdownRustLegalitySync();
  shutdownRustLegalitySync();
  await setTimeout(30);
  assert.equal(process.listenerCount('exit'), listeners, 'host exit listener must be removed');
}

if (process.env.SYNC_TRANSPORT_CASE) {
  try {
    await exercise(process.env.SYNC_TRANSPORT_CASE);
  } finally {
    shutdownRustLegalitySync();
  }
} else {
  const modes = ['cache', 'utf8', 'death-first', 'partial-death', 'death-later', 'death-idle', 'protocol', 'cleanup', 'spawn-error'];
  if (process.env.SYNC_TRANSPORT_NATIVE_BIN) modes.push('native-recovery');
  for (const mode of modes) {
    test(`sync transport: ${mode}`, { timeout: 12_000 }, async () => {
      const directory = mkdtempSync(join(tmpdir(), 'sorcery-sync-test-'));
      try {
        const worker = join(directory, 'worker.mjs');
        writeFileSync(worker, WORKER);
        // Production launches cargo; this shim runs only our synthetic worker.
        const shim = mode === 'native-recovery'
          ? `#!/bin/sh\necho $$ >> '${join(directory, 'launches')}'\nexec '${process.env.SYNC_TRANSPORT_NATIVE_BIN}'\n`
          : `#!/bin/sh\nexec '${process.execPath}' '${worker}'\n`;
        writeFileSync(join(directory, 'cargo'), shim, { mode: 0o700 });
        const validPath = `${directory}:${process.env.PATH}`;
        const environment: NodeJS.ProcessEnv = {
          ...process.env,
          PATH: mode === 'spawn-error' ? join(directory, 'missing-cargo') : validPath,
          SYNC_TRANSPORT_CASE: mode,
          SYNC_TRANSPORT_LOG: join(directory, 'launches'),
          SYNC_TRANSPORT_VALID_PATH: validPath,
        };
        // The subprocess is a scenario runner, not a child of Node's test reporter.
        delete environment.NODE_TEST_CONTEXT;
        if (mode === 'spawn-error') {
          // Keep mkfifo available, but leave cargo's executable missing until recovery.
          mkdirSync(join(directory, 'missing-cargo'));
          writeFileSync(join(directory, 'missing-cargo', 'mkfifo'), '#!/bin/sh\nexec /usr/bin/mkfifo "$@"\n', { mode: 0o700 });
        }
        const run = await runBounded(process.execPath, [fileURLToPath(import.meta.url)], fileURLToPath(new URL('../..', import.meta.url)), {
          environment, timeout: 8_000, maxBuffer: 1_048_576,
        });
        assert.equal(run.code, 0, `bounded ${mode} child failed:\n${run.stdout}\n${run.stderr}`);
      } finally {
        rmSync(directory, { recursive: true, force: true });
      }
    });
  }
}
