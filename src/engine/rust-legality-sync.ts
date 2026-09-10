import { spawn, spawnSync, type ChildProcess } from 'node:child_process';
import { closeSync, mkdtempSync, openSync, readSync, rmSync, writeSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { deepFreeze, type EngineReceipt, type EngineRejection } from './contract.ts';
import type {
  GameActionRequest,
  GameLegalAction,
  GameManifest,
  GameObservation,
  GameSeat,
  GameSession,
  GameState,
  GameStepResult,
} from './game.ts';
import { sessionJsonLaunch } from './rust-engine.ts';
import { asGameLegalActions, parseExportedSession } from './rust-session-parse.ts';

const REPOSITORY_ROOT = fileURLToPath(new URL('../..', import.meta.url));
const MAX_LINE_BYTES = 16 * 1024 * 1024;

type SessionBinding = Readonly<{
  checkpoint: JsonValue;
  manifest: GameManifest;
}>;

const bindings = new WeakMap<object, SessionBinding>();
let client: SyncSessionJson | undefined;
let liveCheckpoint: JsonValue | undefined;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/** Blocking FIFO pair. Node's spawned pipes are O_NONBLOCK, so readSync returns EAGAIN. */
function openBlockingFifos(): Readonly<{ stdinFd: number; stdoutFd: number }> {
  const dir = mkdtempSync(join(tmpdir(), 'sorcery-session-json-'));
  const stdinPath = join(dir, 'in');
  const stdoutPath = join(dir, 'out');
  const made = spawnSync('mkfifo', ['-m', '600', stdinPath, stdoutPath], { encoding: 'utf8' });
  if (made.status !== 0) {
    rmSync(dir, { force: true, recursive: true });
    throw new Error(`mkfifo failed: ${made.stderr || made.stdout || made.status}`);
  }
  let stdinFd: number | undefined;
  try {
    stdinFd = openSync(stdinPath, 'r+');
    const stdoutFd = openSync(stdoutPath, 'r+');
    rmSync(dir, { force: true, recursive: true });
    return { stdinFd, stdoutFd };
  } catch (error) {
    if (stdinFd !== undefined) closeSync(stdinFd);
    rmSync(dir, { force: true, recursive: true });
    throw error;
  }
}

function writeAll(fd: number, text: string): void {
  const buffer = Buffer.from(text, 'utf8');
  let offset = 0;
  while (offset < buffer.length) {
    const written = writeSync(fd, buffer, offset, buffer.length - offset);
    if (written === 0) throw new Error('Rust session-json stdin closed');
    offset += written;
  }
}

/** One persistent session-json process with blocking JSON-RPC. */
class SyncSessionJson {
  private readonly child: ChildProcess;
  private readonly stdinFd: number;
  private readonly stdoutFd: number;
  private closed = false;
  private nextId = 1;
  private remainder = '';

  constructor() {
    const fifos = openBlockingFifos();
    this.stdinFd = fifos.stdinFd;
    this.stdoutFd = fifos.stdoutFd;
    const launch = sessionJsonLaunch();
    this.child = spawn(launch.command, [...launch.args], {
      cwd: REPOSITORY_ROOT,
      stdio: [this.stdinFd, this.stdoutFd, 'ignore'],
    });
    this.child.unref();
    this.child.once('exit', () => {
      this.closed = true;
      if (client === this) client = undefined;
    });
    process.once('exit', () => {
      this.close();
    });
  }

  close(): void {
    this.closed = true;
    try {
      this.child.kill();
    } catch {
      // The worker may already have exited with the host process.
    }
    try {
      closeSync(this.stdinFd);
    } catch {
      // Already closed after a failed request or host shutdown.
    }
    try {
      closeSync(this.stdoutFd);
    } catch {
      // Already closed after a failed request or host shutdown.
    }
    if (client === this) client = undefined;
  }

  call(method: string, params: JsonValue): unknown {
    if (this.closed) throw new Error('Rust session-json process is closed');
    const id = this.nextId;
    this.nextId += 1;
    writeAll(this.stdinFd, `${canonicalJson({
      id,
      method,
      params,
      schemaVersion: 1,
    } as JsonValue)}\n`);
    while (true) {
      const newline = this.remainder.indexOf('\n');
      if (newline >= 0) {
        const line = this.remainder.slice(0, newline);
        this.remainder = this.remainder.slice(newline + 1);
        if (line.length === 0) continue;
        if (line.length > MAX_LINE_BYTES) {
          throw new Error('Rust session-json response exceeded 16 MiB');
        }
        const parsed = parseJsonWithDuplicateKeyCheck(line);
        if (!isRecord(parsed) || parsed.id !== id) {
          throw new Error('Rust session-json response id did not match the request');
        }
        if (parsed.error !== undefined) {
          const message = isRecord(parsed.error) && typeof parsed.error.message === 'string'
            ? parsed.error.message
            : 'Rust session-json request failed';
          throw new Error(message);
        }
        return parsed.result;
      }
      const chunk = Buffer.alloc(65_536);
      const bytes = readSync(this.stdoutFd, chunk);
      if (bytes === 0) {
        throw new Error('Rust session-json process closed');
      }
      this.remainder += chunk.toString('utf8', 0, bytes);
      if (this.remainder.length > MAX_LINE_BYTES) {
        throw new Error('Rust session-json response exceeded 16 MiB');
      }
    }
  }
}

function processClient(): SyncSessionJson {
  client ??= new SyncSessionJson();
  return client;
}

function bindSession(session: GameSession, checkpoint: JsonValue): void {
  const binding = { checkpoint, manifest: session.manifest };
  bindings.set(session, binding);
  bindings.set(session.state, binding);
}

/** Binds one async-exported snapshot so observe/legal/step can resume it on the shared worker. */
export function bindRustExportedSession(session: GameSession, checkpoint: JsonValue): void {
  bindSession(session, checkpoint);
}

function bindingFor(state: GameState): SessionBinding {
  const binding = bindings.get(state);
  if (!binding) {
    throw new Error('Rust-exported session state is required');
  }
  return binding;
}

function readCheckpoint(): JsonValue {
  const result = processClient().call('checkpoint', {});
  if (!isRecord(result) || result.checkpoint === undefined) {
    throw new Error('Rust session checkpoint result was malformed');
  }
  return result.checkpoint as JsonValue;
}

function resumeBinding(binding: SessionBinding): void {
  if (liveCheckpoint === binding.checkpoint) return;
  processClient().call('resume', { checkpoint: binding.checkpoint });
  liveCheckpoint = binding.checkpoint;
}

function exportBoundSession(manifest: GameManifest): GameSession {
  const session = parseExportedSession(processClient().call('exportSession', {}) as JsonValue, manifest);
  const checkpoint = readCheckpoint();
  bindSession(session, checkpoint);
  liveCheckpoint = checkpoint;
  return session;
}

/** Opens one authoritative session through the Rust legality engine. */
export function createRustGameSession(manifest: GameManifest): GameSession {
  processClient().call('new', { manifestJson: canonicalJson(manifest as unknown as JsonValue) });
  liveCheckpoint = undefined;
  return exportBoundSession(manifest);
}

/** Returns engine-issued legal actions for one seat of a Rust-exported state. */
export function rustLegalGameActions(state: GameState, seat: GameSeat): readonly GameLegalAction[] {
  const binding = bindingFor(state);
  resumeBinding(binding);
  const result = processClient().call('legalActions', { seat });
  if (!isRecord(result) || !Array.isArray(result.actions)) {
    throw new Error('Rust session legalActions result was invalid');
  }
  return asGameLegalActions(result.actions as Parameters<typeof asGameLegalActions>[0]);
}

/** Returns the Rust public view for one seat of a Rust-exported state. */
export function rustObserveGame(state: GameState, viewer: GameSeat): GameObservation {
  const binding = bindingFor(state);
  resumeBinding(binding);
  const result = processClient().call('publicView', { seat: viewer });
  if (!isRecord(result) || result.view === undefined) {
    throw new Error('Rust session publicView result was invalid');
  }
  return deepFreeze(result.view) as GameObservation;
}

/** Applies one bound action through the Rust legality engine. */
export function rustStepGame(session: GameSession, request: GameActionRequest): GameStepResult {
  const binding = bindings.get(session.state) ?? bindings.get(session);
  if (!binding) {
    throw new Error('stepGame requires a Rust-exported session');
  }
  resumeBinding(binding);
  const stepped = processClient().call('step', {
    actionId: request.actionId,
    seat: request.seat,
    stateVersion: request.stateVersion,
  });
  if (!isRecord(stepped) || typeof stepped.accepted !== 'boolean') {
    throw new Error('Rust session step result was invalid');
  }
  const next = exportBoundSession(session.manifest);
  if (stepped.accepted) {
    return deepFreeze({
      accepted: true,
      receipt: stepped.receipt as EngineReceipt,
      session: next,
    });
  }
  return deepFreeze({
    accepted: false,
    reason: stepped.rejection as EngineRejection,
    session: next,
  });
}

/** Replays one action-id journal through the Rust legality engine. */
export function rustReplayGame(manifest: GameManifest, actionIds: readonly string[]): GameSession {
  let session = createRustGameSession(manifest);
  for (const actionId of actionIds) {
    const result = rustStepGame(session, {
      actionId,
      seat: session.state.decisionSeat,
      stateVersion: session.state.stateVersion,
    });
    if (!result.accepted) throw new Error(`game replay rejected action: ${result.reason.code}`);
    session = result.session;
  }
  return session;
}

/** Verifies one exported session against a Rust replay of its journal. */
export function rustVerifyGameReplay(expected: GameSession): boolean {
  const binding = bindings.get(expected.state) ?? bindings.get(expected);
  if (binding) {
    try {
      resumeBinding(binding);
      const result = processClient().call('verifyReplay', {});
      return isRecord(result) && result.verified === true;
    } catch {
      return false;
    }
  }
  try {
    const replayed = rustReplayGame(
      expected.manifest,
      expected.transcript.map(({ actionId }) => actionId),
    );
    return canonicalJson({
      initialRandomDraws: replayed.initialRandomDraws,
      state: replayed.state,
      transcript: replayed.transcript,
    }) === canonicalJson({
      initialRandomDraws: expected.initialRandomDraws,
      state: expected.state,
      transcript: expected.transcript,
    });
  } catch {
    return false;
  }
}

/** Stops the shared session-json worker. Tests call this so the process can exit. */
export function shutdownRustLegalitySync(): void {
  client?.close();
  client = undefined;
  liveCheckpoint = undefined;
}
