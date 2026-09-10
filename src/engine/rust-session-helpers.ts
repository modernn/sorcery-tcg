import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import { deepFreeze, type EngineReceipt, type EngineRejection } from './contract.ts';
import type {
  GameActionRequest,
  GameLegalAction,
  GameManifest,
  GameSeat,
  GameSession,
  GameStepResult,
} from './game.ts';
import {
  RustSessionClient,
  type RustEmittedResult,
  type RustNoveltyStep,
  type Sha256Hash,
} from './rust-engine.ts';
import { bindRustExportedSession } from './rust-legality-sync.ts';
import { asGameLegalAction, asGameLegalActions, parseExportedSession } from './rust-session-parse.ts';

export { asGameLegalAction, asGameLegalActions, parseExportedSession } from './rust-session-parse.ts';

export const RUST_LEGALITY_SOURCE = 'rust-legality-engine';

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

async function boundSnapshot(
  client: RustSessionClient,
  manifest: GameManifest,
): Promise<GameSession> {
  const session = parseExportedSession(await client.exportSession(), manifest);
  bindRustExportedSession(session, await client.checkpoint());
  return session;
}

export type RustCheckpointSnapshot = Readonly<{
  checkpoint: JsonValue;
  checkpointId: Sha256Hash;
  expectedSessionHash: Sha256Hash;
  serialized: string;
  serializedCheckpointHash: ReturnType<typeof identityHash>;
}>;

/** One authoritative session backed by a dedicated `session-json` process. */
export class RustGameSessionHandle {
  readonly client: RustSessionClient;
  private manifest: GameManifest;
  private session: GameSession;

  private constructor(client: RustSessionClient, manifest: GameManifest, session: GameSession) {
    this.client = client;
    this.manifest = manifest;
    this.session = session;
  }

  private async loadSnapshot(manifest: GameManifest = this.manifest): Promise<GameSession> {
    this.manifest = manifest;
    this.session = await boundSnapshot(this.client, manifest);
    return this.session;
  }

  /** Opens one fresh Rust-backed session. */
  static async open(manifest: GameManifest): Promise<RustGameSessionHandle> {
    const client = await RustSessionClient.start();
    await client.newSession(canonicalJson(manifest as unknown as JsonValue));
    return new RustGameSessionHandle(client, manifest, await boundSnapshot(client, manifest));
  }

  /** Returns the latest exported session snapshot. */
  get snapshot(): GameSession {
    return this.session;
  }

  /** Returns engine-issued legal actions for one seat. */
  async legalActions(seat?: GameSeat): Promise<readonly GameLegalAction[]> {
    const target = seat ?? this.session.state.decisionSeat;
    return asGameLegalActions(await this.client.legalActions(target));
  }

  /** Selects one engine-issued action with the shared baseline deterministic policy. */
  async selectPolicyAction(): Promise<GameLegalAction> {
    return asGameLegalAction(await this.client.selectPolicyAction());
  }

  /** Scores one-step novelty over the current engine-issued legal actions. */
  async probeNovelty(input: Readonly<{
    committedActionKinds: readonly string[];
    committedEventTypes: readonly string[];
  }>): Promise<RustNoveltyStep> {
    return this.client.probeNovelty(input);
  }

  /** Expands every engine-issued root action, then follows the baseline policy. */
  async runCounterfactual(input: Readonly<{
    maxContinuationDecisions: number;
  }>): Promise<JsonValue> {
    return this.client.runCounterfactual(input);
  }

  /** Runs the coverage-guided one-step novelty rollout from this snapshot. */
  async runNoveltyRollout(input: Readonly<{ maxActions: number }>): Promise<RustEmittedResult> {
    return this.client.runNoveltyRollout(input);
  }

  /** Runs one-step novelty, then expands unchosen signals as forced branches. */
  async runNoveltyFrontierSearch(input: Readonly<{
    maxActions: number;
    maxBranches: number;
  }>): Promise<RustEmittedResult> {
    return this.client.runNoveltyFrontierSearch(input);
  }

  /** Forces one engine-issued action, checks its probe prediction, then rolls out novelty. */
  async runNoveltyFromForcedAction(input: Readonly<{
    actionId: string;
    actionKind: string;
    maxActions: number;
    predictedEventTypes: readonly string[];
    predictedStateHash: string;
  }>): Promise<Readonly<{
    emittedCheckpoints: readonly JsonValue[];
    entry: Readonly<{
      actionId: string;
      actionKind: string;
      eventTypes: readonly string[];
      stateHash: Sha256Hash;
    }>;
    result: JsonValue;
  }>> {
    return this.client.runNoveltyFromForcedAction(input);
  }

  /** Applies one bound action request. */
  async step(request: GameActionRequest): Promise<GameStepResult> {
    const stepped = await this.client.step(request);
    if (!isRecord(stepped) || typeof stepped.accepted !== 'boolean') {
      throw new Error('Rust session step result was malformed');
    }
    await this.loadSnapshot();
    if (stepped.accepted) {
      return deepFreeze({
        accepted: true,
        receipt: stepped.receipt as EngineReceipt,
        session: this.session,
      });
    }
    return deepFreeze({
      accepted: false,
      reason: stepped.rejection as EngineRejection,
      session: this.session,
    });
  }

  /** Applies one fully bound legal action. */
  async stepAction(action: GameLegalAction): Promise<GameStepResult> {
    return this.step({
      actionId: action.actionId,
      seat: action.seat,
      stateVersion: action.stateVersion,
    });
  }

  /** Finds and applies one setup action. */
  async take(predicate: (action: GameLegalAction) => boolean): Promise<Readonly<{
    action: GameLegalAction;
    session: GameSession;
  }>> {
    const action = (await this.legalActions()).find(predicate);
    if (!action) throw new Error('expected deterministic setup action');
    const result = await this.stepAction(action);
    if (!result.accepted) {
      throw new Error(`issued setup action was rejected: ${result.reason.code}`);
    }
    return { action, session: result.session };
  }

  /** Verifies replay integrity for the current journals. */
  async verifyReplay(): Promise<boolean> {
    return this.client.verifyReplay();
  }

  /** Captures one canonical checkpoint for the current session. */
  async checkpoint(): Promise<RustCheckpointSnapshot> {
    const checkpoint = await this.client.checkpoint();
    if (!isRecord(checkpoint)
      || typeof checkpoint.checkpointId !== 'string'
      || typeof checkpoint.expectedSessionHash !== 'string') {
      throw new Error('Rust checkpoint result was malformed');
    }
    const serialized = canonicalJson(checkpoint as JsonValue);
    return Object.freeze({
      checkpoint,
      checkpointId: checkpoint.checkpointId as Sha256Hash,
      expectedSessionHash: checkpoint.expectedSessionHash as Sha256Hash,
      serialized,
      serializedCheckpointHash: identityHash(checkpoint as JsonValue),
    });
  }

  /** Replaces the live session with a fresh opening of one manifest. */
  async reset(manifest: GameManifest): Promise<GameSession> {
    await this.client.newSession(canonicalJson(manifest as unknown as JsonValue));
    return this.loadSnapshot(manifest);
  }

  /** Resumes from one validated checkpoint object. */
  async resume(checkpoint: JsonValue): Promise<GameSession> {
    await this.client.resume(checkpoint);
    return this.loadSnapshot();
  }

  /** Returns the authoritative state hash for one seat observation. */
  async stateHash(seat: GameSeat = this.session.state.decisionSeat): Promise<Sha256Hash> {
    return (await this.client.publicView(seat)).stateHash;
  }

  async close(): Promise<void> {
    await this.client.close();
  }
}

/** Runs one callback against a dedicated Rust session process. */
export async function withRustSession<T>(
  manifest: GameManifest,
  run: (handle: RustGameSessionHandle) => Promise<T>,
): Promise<T> {
  const handle = await RustGameSessionHandle.open(manifest);
  try {
    return await run(handle);
  } finally {
    await handle.close();
  }
}

/** Resumes a checkpoint on a dedicated process, then runs one callback. */
export async function withResumedRustSession<T>(
  manifest: GameManifest,
  checkpoint: JsonValue,
  run: (handle: RustGameSessionHandle) => Promise<T>,
): Promise<T> {
  return withRustSession(manifest, async (handle) => {
    await handle.resume(checkpoint);
    return run(handle);
  });
}

/** Applies one action from a saved checkpoint without mutating the caller's live session. */
export async function transitionFromCheckpoint(
  manifest: GameManifest,
  checkpoint: JsonValue,
  action: GameLegalAction,
  summary: JsonValue,
): Promise<JsonValue> {
  return withResumedRustSession(manifest, checkpoint, async (handle) => {
    const result = await handle.stepAction(action);
    if (!result.accepted) {
      throw new Error(`issued action was rejected: ${result.reason.code}`);
    }
    if (!(await handle.verifyReplay())) throw new Error('replay failed verification');
    const saved = await handle.checkpoint();
    return {
      checkpointId: saved.checkpointId,
      expectedSessionHash: saved.expectedSessionHash,
      receipt: result.receipt,
      replayVerified: true,
      selectedActionId: action.actionId,
      serializedCheckpointHash: saved.serializedCheckpointHash,
      stateHash: await handle.stateHash(action.seat),
      summary,
    };
  });
}
