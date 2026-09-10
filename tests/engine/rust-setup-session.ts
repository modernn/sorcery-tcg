import assert from 'node:assert/strict';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createGameCheckpoint,
  resumeGameCheckpointAsync,
  type GameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import type { StateHash } from '../../src/engine/contract.ts';
import type {
  GameActionRequest,
  GameLegalAction,
  GameManifest,
  GameObservation,
  GameSeat,
  GameSession,
  GameStepResult,
} from '../../src/engine/game.ts';
import {
  hashGameState,
  observeGame,
} from '../../src/engine/game.ts';
import { RustSessionClient } from '../../src/engine/rust-engine.ts';
import {
  parseExportedSession,
  RustGameSessionHandle,
} from '../../src/engine/rust-session-helpers.ts';

/** One Rust-backed setup session for engine rule proofs. */
export class SetupCtx {
  private readonly handle: RustGameSessionHandle;

  private constructor(handle: RustGameSessionHandle) {
    this.handle = handle;
  }

  /** Opens one fresh authoritative session. */
  static async open(manifest: GameManifest): Promise<SetupCtx> {
    return new SetupCtx(await RustGameSessionHandle.open(manifest));
  }

  /** Returns the latest exported session snapshot. */
  get session(): GameSession {
    return this.handle.snapshot;
  }

  /** Returns the latest authoritative state. */
  get state(): GameSession['state'] {
    return this.session.state;
  }

  /** Returns engine-issued legal actions for one seat. */
  async legalActions(seat?: GameSeat): Promise<readonly GameLegalAction[]> {
    return this.handle.legalActions(seat);
  }

  /** Selects one engine-issued action with the shared baseline deterministic policy. */
  async selectPolicyAction(): Promise<GameLegalAction> {
    return this.handle.selectPolicyAction();
  }

  /** Finds one legal action for the active or specified seat. */
  async action(
    predicate: (candidate: GameLegalAction) => boolean,
    seat?: GameSeat,
  ): Promise<GameLegalAction> {
    const found = (await this.legalActions(seat)).find(predicate);
    assert.ok(found, 'expected legal action');
    return found;
  }

  /** Applies one bound legal action and requires acceptance. */
  async accept(candidate: GameLegalAction): Promise<GameSession> {
    const result = await this.handle.stepAction(candidate);
    assert.equal(result.accepted, true);
    return result.session;
  }

  /** Finds and applies one matching legal action. */
  async take(predicate: (candidate: GameLegalAction) => boolean): Promise<GameSession> {
    return this.accept(await this.action(predicate));
  }

  /** Applies one bound action request and requires acceptance. */
  async acceptRequest(request: GameActionRequest): Promise<GameSession> {
    const result = await this.handle.step(request);
    assert.equal(result.accepted, true);
    return result.session;
  }

  /** Keeps the opening hand without reordering. */
  async keep(): Promise<GameSession> {
    return this.accept(await this.action(({ descriptor }) =>
      descriptor.kind === 'mulligan'
        && descriptor.atlasOrder.length === 0
        && descriptor.spellbookOrder.length === 0));
  }

  /** Applies one legal action without requiring acceptance. */
  async step(candidate: GameLegalAction): Promise<GameStepResult> {
    return this.handle.stepAction(candidate);
  }

  /** Applies one bound action request without requiring acceptance. */
  async stepRequest(request: GameActionRequest): Promise<GameStepResult> {
    return this.handle.step(request);
  }

  /** Returns a seat-scoped observation derived from the exported state. */
  observe(viewer: GameSeat): GameObservation {
    return observeGame(this.session.state, viewer);
  }

  /** Returns the authoritative state hash. */
  stateHash(): StateHash {
    return hashGameState(this.state);
  }

  /** Verifies replay integrity for the current journals. */
  async verifyReplay(): Promise<boolean> {
    return this.handle.verifyReplay();
  }

  /** Captures a resume-safe checkpoint of the current journals. */
  checkpoint(): GameCheckpoint {
    return createGameCheckpoint(this.session);
  }

  /** Opens an independent live session at this exact history. */
  async fork(): Promise<SetupCtx> {
    const forked = await SetupCtx.open(this.session.manifest);
    await forked.resume(createGameCheckpoint(this.session));
    return forked;
  }

  /** Replaces the live session with a fresh opening of one manifest. */
  async reset(manifest: GameManifest): Promise<GameSession> {
    return this.handle.reset(manifest);
  }

  /** Resumes one parsed checkpoint into this live session. */
  async resume(checkpoint: GameCheckpoint): Promise<GameSession> {
    return this.handle.resume(checkpoint as unknown as JsonValue);
  }

  /** Resumes one checkpoint in a fresh process and returns the exported session. */
  static async resumeCheckpoint(checkpoint: GameCheckpoint): Promise<GameSession> {
    return resumeGameCheckpointAsync(checkpoint);
  }

  async close(): Promise<void> {
    await this.handle.close();
  }
}

/** Runs one callback against a dedicated Rust-backed setup session. */
export async function withSetup<T>(
  manifest: GameManifest,
  run: (ctx: SetupCtx) => Promise<T>,
): Promise<T> {
  const ctx = await SetupCtx.open(manifest);
  try {
    return await run(ctx);
  } finally {
    await ctx.close();
  }
}

/** Opens one independent preview session for dry-run assertions. */
export async function withPreview<T>(
  manifest: GameManifest,
  run: (ctx: SetupCtx) => Promise<T>,
): Promise<T> {
  return withSetup(manifest, run);
}

/** Runs one callback against a fork of the current live history. */
export async function withFork<T>(
  ctx: SetupCtx,
  run: (forked: SetupCtx) => Promise<T>,
): Promise<T> {
  const forked = await ctx.fork();
  try {
    return await run(forked);
  } finally {
    await forked.close();
  }
}

/** Finds the first seed whose opening session matches a predicate. */
export async function findOpeningManifest(
  build: (seed: number) => GameManifest,
  matches: (session: GameSession) => boolean,
  range: Readonly<{ from?: number; to?: number }> = {},
): Promise<GameManifest> {
  const from = range.from ?? 1;
  const to = range.to ?? 4_096;
  const client = await RustSessionClient.start();
  try {
    for (let seed = from; seed <= to; seed += 1) {
      const candidate = build(seed);
      await client.newSession(canonicalJson(candidate as unknown as JsonValue));
      const session = parseExportedSession(await client.exportSession(), candidate);
      if (matches(session)) return candidate;
    }
  } finally {
    await client.close();
  }
  throw new Error(`no opening seed in ${from}..${to} matched`);
}
