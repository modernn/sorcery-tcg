import { type ChildProcessWithoutNullStreams, spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { createInterface, type Interface } from 'node:readline';
import { fileURLToPath } from 'node:url';

import {
  canonicalJson,
  parseJsonWithDuplicateKeyCheck,
  type JsonValue,
} from '../authority/canonical-json.ts';

const REPOSITORY_ROOT = fileURLToPath(new URL('../..', import.meta.url));
const MAX_OUTPUT_BYTES = 1_048_576;
const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
const DEFAULT_TARGET_DIR = fileURLToPath(new URL('../../target', import.meta.url));

export function sessionJsonLaunch(): Readonly<{ args: readonly string[]; command: string }> {
  const targetDir = process.env.CARGO_TARGET_DIR ?? DEFAULT_TARGET_DIR;
  const binaryName = process.platform === 'win32' ? 'session-json.exe' : 'session-json';
  const binary = join(targetDir, 'release', binaryName);
  if (existsSync(binary)) return { args: [], command: binary };
  return {
    args: [
      'run', '--release', '--locked', '--quiet', '-p', 'sorcery-engine',
      '--bin', 'session-json',
    ],
    command: 'cargo',
  };
}

export type Sha256Hash = `sha256:${string}`;

export type RustDeterministicGameReport = Readonly<{
  acceptedActionCount: number;
  classification: 'unranked_partial_rules_unverified_authority';
  fightCount: number;
  finalStateHash: Sha256Hash;
  replayVerified: boolean;
  terminal: Readonly<{
    loser: 'north' | 'south';
    reason: string;
    status: 'finished';
    winner: 'north' | 'south';
  }>;
  transcriptHash: Sha256Hash;
  turnCount: number;
}>;

export type RustLegalAction = Readonly<{
  actionId: string;
  descriptor: Readonly<Record<string, JsonValue>>;
  label: string;
  seat: 'north' | 'south';
  stateVersion: number;
}>;

export type RustActionRequest = Readonly<{
  actionId: string;
  seat: 'north' | 'south';
  stateVersion: number;
}>;

export type RustStepResult = Readonly<{
  accepted: boolean;
  receipt?: JsonValue;
  rejection?: JsonValue;
}>;

export type RustNoveltyProbe = Readonly<{
  actionId: string;
  actionKind: string;
  eventTypes: readonly string[];
  newActionKind: boolean;
  newEventCount: number;
  postStateHash: Sha256Hash;
  selectedByFallback: boolean;
}>;

export type RustNoveltyStep = Readonly<{
  probes: readonly RustNoveltyProbe[];
  selectedIndex: number;
  tooWide: boolean;
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function requireHash(value: unknown, label: string): Sha256Hash {
  if (typeof value !== 'string' || !HASH_PATTERN.test(value)) {
    throw new Error(`Rust engine ${label} was not a sha256 hash`);
  }
  return value as Sha256Hash;
}

function parseNoveltyProbe(value: unknown): RustNoveltyProbe {
  if (!isRecord(value)
    || typeof value.actionId !== 'string'
    || typeof value.actionKind !== 'string'
    || !Array.isArray(value.eventTypes)
    || !value.eventTypes.every((eventType) => typeof eventType === 'string')
    || typeof value.newActionKind !== 'boolean'
    || !Number.isSafeInteger(value.newEventCount)
    || typeof value.selectedByFallback !== 'boolean') {
    throw new Error('Rust session novelty probe was invalid');
  }
  return Object.freeze({
    actionId: value.actionId,
    actionKind: value.actionKind,
    eventTypes: Object.freeze(value.eventTypes.slice() as string[]),
    newActionKind: value.newActionKind,
    newEventCount: value.newEventCount as number,
    postStateHash: requireHash(value.postStateHash, 'postStateHash'),
    selectedByFallback: value.selectedByFallback,
  });
}

function parseNoveltyStep(value: unknown): RustNoveltyStep {
  if (!isRecord(value)
    || !Array.isArray(value.probes)
    || typeof value.selectedIndex !== 'number'
    || !Number.isSafeInteger(value.selectedIndex)
    || typeof value.tooWide !== 'boolean') {
    throw new Error('Rust session probeNovelty result was invalid');
  }
  const probes = value.probes.map((probe) => parseNoveltyProbe(probe));
  const selectedIndex = value.selectedIndex;
  if (selectedIndex < 0 || selectedIndex >= probes.length) {
    throw new Error('Rust session probeNovelty selectedIndex was out of range');
  }
  return Object.freeze({
    probes: Object.freeze(probes),
    selectedIndex,
    tooWide: value.tooWide,
  });
}

function parseLegalAction(action: unknown): RustLegalAction {
  if (!isRecord(action)
    || typeof action.actionId !== 'string'
    || typeof action.label !== 'string'
    || (action.seat !== 'north' && action.seat !== 'south')
    || !Number.isSafeInteger(action.stateVersion)
    || !isRecord(action.descriptor)) {
    throw new Error('Rust session legal action was invalid');
  }
  return Object.freeze({
    actionId: action.actionId,
    descriptor: action.descriptor as Readonly<Record<string, JsonValue>>,
    label: action.label,
    seat: action.seat,
    stateVersion: action.stateVersion as number,
  });
}

function parseRustReport(value: unknown): RustDeterministicGameReport {
  if (!isRecord(value)
    || value.classification !== 'unranked_partial_rules_unverified_authority'
    || value.replayVerified !== true
    || !Number.isSafeInteger(value.acceptedActionCount)
    || !Number.isSafeInteger(value.fightCount)
    || !Number.isSafeInteger(value.turnCount)
    || !isRecord(value.terminal)
    || value.terminal.status !== 'finished'
    || (value.terminal.winner !== 'north' && value.terminal.winner !== 'south')
    || (value.terminal.loser !== 'north' && value.terminal.loser !== 'south')
    || typeof value.terminal.reason !== 'string') {
    throw new Error('Rust engine report did not match the expected contract');
  }
  return Object.freeze({
    acceptedActionCount: value.acceptedActionCount as number,
    classification: 'unranked_partial_rules_unverified_authority',
    fightCount: value.fightCount as number,
    finalStateHash: requireHash(value.finalStateHash, 'finalStateHash'),
    replayVerified: true,
    terminal: Object.freeze({
      loser: value.terminal.loser,
      reason: value.terminal.reason,
      status: 'finished' as const,
      winner: value.terminal.winner,
    }),
    transcriptHash: requireHash(value.transcriptHash, 'transcriptHash'),
    turnCount: value.turnCount as number,
  });
}

/** Runs one locked-release `sorcery-engine` subcommand and returns canonical JSON output. */
export function runRustEngineCommand(args: readonly string[]): JsonValue {
  const result = spawnSync('cargo', [
    'run', '--release', '--locked', '--quiet', '-p', 'sorcery-engine',
    '--bin', 'sorcery-engine', '--', ...args,
  ], {
    cwd: REPOSITORY_ROOT,
    encoding: 'utf8',
    maxBuffer: MAX_OUTPUT_BYTES,
  });
  if (result.status !== 0) {
    throw new Error(result.stderr?.trim() || 'Rust engine command failed');
  }
  const text = result.stdout.trim();
  const parsed = parseJsonWithDuplicateKeyCheck(text);
  if (canonicalJson(parsed) !== text) {
    throw new Error('Rust engine output was not canonical JSON');
  }
  return parsed;
}

/** Runs the authoritative synthetic demo rollout for one seed. */
export function runRustSyntheticDemo(seed: number): RustDeterministicGameReport {
  if (!Number.isSafeInteger(seed) || seed < 0 || seed > 0xffff_ffff) {
    throw new RangeError('seed must be a safe integer between 0 and 4294967295');
  }
  return parseRustReport(runRustEngineCommand(['demo', String(seed)]));
}

type PendingRpc = {
  reject: (error: Error) => void;
  resolve: (value: unknown) => void;
};

/** Persistent line-delimited JSON-RPC client for `session-json`. */
export class RustSessionClient {
  private readonly child: ChildProcessWithoutNullStreams;
  private closed = false;
  private readonly pending = new Map<number, PendingRpc>();
  private readonly reader: Interface;
  private nextId = 1;
  private stdoutBytes = 0;

  private constructor(child: ChildProcessWithoutNullStreams) {
    this.child = child;
    this.reader = createInterface({ crlfDelay: Infinity, input: child.stdout });
    this.reader.on('line', (line) => this.onLine(line));
    child.stderr.on('data', (chunk: Buffer) => {
      this.stdoutBytes += chunk.length;
      if (this.stdoutBytes > MAX_OUTPUT_BYTES) {
        this.failAll(new Error('Rust session-json stderr exceeded its limit'));
      }
    });
    child.once('exit', (code) => {
      this.closed = true;
      this.failAll(new Error(`Rust session-json exited with code ${code ?? 'null'}`));
    });
  }

  static async start(): Promise<RustSessionClient> {
    const launch = sessionJsonLaunch();
    const child = spawn(launch.command, [...launch.args], {
      cwd: REPOSITORY_ROOT,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    const client = new RustSessionClient(child);
    await new Promise<void>((resolve, reject) => {
      child.once('error', reject);
      setImmediate(() => resolve());
    });
    return client;
  }

  async close(): Promise<void> {
    if (this.closed) return;
    this.closed = true;
    this.reader.close();
    this.child.stdin.end();
    await new Promise<void>((resolve) => {
      this.child.once('exit', () => resolve());
      this.child.kill();
    });
  }

  async newSession(manifestJson: string): Promise<Readonly<{
    manifestId: string;
    stateHash: Sha256Hash;
    stateVersion: number;
  }>> {
    const result = await this.call('new', { manifestJson });
    if (!isRecord(result)
      || typeof result.manifestId !== 'string'
      || !Number.isSafeInteger(result.stateVersion)) {
      throw new Error('Rust session new result was invalid');
    }
    return Object.freeze({
      manifestId: result.manifestId,
      stateHash: requireHash(result.stateHash, 'stateHash'),
      stateVersion: result.stateVersion as number,
    });
  }

  async legalActions(seat: 'north' | 'south'): Promise<readonly RustLegalAction[]> {
    const result = await this.call('legalActions', { seat });
    if (!isRecord(result) || !Array.isArray(result.actions)) {
      throw new Error('Rust session legalActions result was invalid');
    }
    return Object.freeze(result.actions.map((action) => parseLegalAction(action)));
  }

  async selectPolicyAction(): Promise<RustLegalAction> {
    const result = await this.call('selectPolicyAction', {});
    if (!isRecord(result)) {
      throw new Error('Rust session selectPolicyAction result was invalid');
    }
    return parseLegalAction(result.action);
  }

  async probeNovelty(input: Readonly<{
    committedActionKinds: readonly string[];
    committedEventTypes: readonly string[];
  }>): Promise<RustNoveltyStep> {
    return parseNoveltyStep(await this.call('probeNovelty', {
      committedActionKinds: [...input.committedActionKinds],
      committedEventTypes: [...input.committedEventTypes],
    }));
  }

  async step(request: RustActionRequest): Promise<RustStepResult> {
    const result = await this.call('step', request as unknown as JsonValue);
    if (!isRecord(result) || typeof result.accepted !== 'boolean') {
      throw new Error('Rust session step result was invalid');
    }
    return Object.freeze({
      accepted: result.accepted,
      ...(result.receipt === undefined ? {} : { receipt: result.receipt as JsonValue }),
      ...(result.rejection === undefined ? {} : { rejection: result.rejection as JsonValue }),
    });
  }

  async publicView(seat: 'north' | 'south'): Promise<Readonly<{
    stateHash: Sha256Hash;
    view: JsonValue;
  }>> {
    const result = await this.call('publicView', { seat });
    if (!isRecord(result) || result.view === undefined) {
      throw new Error('Rust session publicView result was invalid');
    }
    return Object.freeze({
      stateHash: requireHash(result.stateHash, 'stateHash'),
      view: result.view as JsonValue,
    });
  }

  async observe(seat: 'north' | 'south'): Promise<JsonValue> {
    const result = await this.call('observe', { seat });
    if (!isRecord(result) || result.observation === undefined) {
      throw new Error('Rust session observe result was invalid');
    }
    return result.observation as JsonValue;
  }

  async verifyReplay(): Promise<boolean> {
    const result = await this.call('verifyReplay', {});
    if (!isRecord(result) || typeof result.verified !== 'boolean') {
      throw new Error('Rust session verifyReplay result was invalid');
    }
    return result.verified;
  }

  async exportSession(): Promise<JsonValue> {
    const result = await this.call('exportSession', {});
    return result as JsonValue;
  }

  async checkpoint(): Promise<JsonValue> {
    const result = await this.call('checkpoint', {});
    if (!isRecord(result) || result.checkpoint === undefined) {
      throw new Error('Rust session checkpoint result was invalid');
    }
    return result.checkpoint as JsonValue;
  }

  async resume(checkpoint: JsonValue): Promise<Readonly<{
    expectedSessionHash: Sha256Hash;
    stateHash: Sha256Hash;
    stateVersion: number;
  }>> {
    const result = await this.call('resume', { checkpoint });
    if (!isRecord(result) || !Number.isSafeInteger(result.stateVersion)) {
      throw new Error('Rust session resume result was invalid');
    }
    return Object.freeze({
      expectedSessionHash: requireHash(result.expectedSessionHash, 'expectedSessionHash'),
      stateHash: requireHash(result.stateHash, 'stateHash'),
      stateVersion: result.stateVersion as number,
    });
  }

  private async call(method: string, params: JsonValue): Promise<unknown> {
    if (this.closed) throw new Error('Rust session-json process is closed');
    const id = this.nextId;
    this.nextId += 1;
    const request = canonicalJson({
      id,
      method,
      params,
      schemaVersion: 1,
    } as JsonValue);
    return await new Promise((resolve, reject) => {
      this.pending.set(id, { reject, resolve });
      this.child.stdin.write(`${request}\n`, (error) => {
        if (error) {
          this.pending.delete(id);
          reject(error);
        }
      });
    });
  }

  private onLine(line: string): void {
    if (!line) return;
    let parsed: unknown;
    try {
      parsed = parseJsonWithDuplicateKeyCheck(line);
    } catch (error) {
      this.failAll(error instanceof Error ? error : new Error('invalid session-json response'));
      return;
    }
    if (!isRecord(parsed) || !Number.isSafeInteger(parsed.id)) {
      this.failAll(new Error('session-json response lacked an id'));
      return;
    }
    const waiter = this.pending.get(parsed.id as number);
    if (!waiter) return;
    this.pending.delete(parsed.id as number);
    if (parsed.error !== undefined) {
      const message = isRecord(parsed.error) && typeof parsed.error.message === 'string'
        ? parsed.error.message
        : 'Rust session-json request failed';
      waiter.reject(new Error(message));
      return;
    }
    waiter.resolve(parsed.result);
  }

  private failAll(error: Error): void {
    for (const waiter of this.pending.values()) waiter.reject(error);
    this.pending.clear();
  }
}
