import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  type GameCheckpoint,
} from '../engine/checkpoint.ts';
import { deepFreeze, type EngineRejectionCode, type StateHash } from '../engine/contract.ts';
import {
  type GameLegalAction,
  type GameManifest,
  type GameSession,
  type GameTerminal,
} from '../engine/game.ts';
import { withRustSession } from '../engine/rust-session-helpers.ts';

export const NOVELTY_ROLLOUT_ACTION_LIMIT = 500;
export const NOVELTY_ROLLOUT_WIDTH_LIMIT = 128;

type ActionKind = GameLegalAction['descriptor']['kind'];
type FinishedTerminal = Extract<GameTerminal, { status: 'finished' }>;

export type NoveltyPosition = Readonly<{
  decisionIndex: number;
  stateHash: StateHash;
  transcriptLength: number;
}>;

type FirstSeen<Value> = Readonly<{
  firstSeen: NoveltyPosition;
  value: Value;
}>;

export type NoveltyFrontierCandidate = Readonly<{
  actionId: string;
  actionKind: ActionKind;
  checkpointId: StateHash;
  predictedEventTypes: readonly string[];
  predictedStateHash: StateHash;
  signal: Readonly<{
    kind: 'action-kind' | 'event-type';
    value: string;
  }>;
}>;

export type NoveltyRolloutCoverage = Readonly<{
  branchFactors: readonly FirstSeen<number>[];
  committedActionKinds: readonly FirstSeen<ActionKind>[];
  committedEventTypes: readonly FirstSeen<string>[];
  offeredActionKinds: readonly FirstSeen<ActionKind>[];
}>;

export type NoveltyRolloutFailure =
  | Readonly<{
    actionId: string;
    checkpointId: StateHash;
    code: EngineRejectionCode;
    phase: 'fallback' | 'probe';
    kind: 'engine-rejection';
  }>
  | Readonly<{
    checkpointId: StateHash;
    phase: 'legal-actions' | 'probe' | 'selector';
    kind: 'exception';
  }>
  | Readonly<{
    actionId: string;
    checkpointId: StateHash;
    phase: 'fallback' | 'probe';
    kind: 'exception';
  }>
  | Readonly<{ checkpointId: StateHash; kind: 'deadlock' | 'replay-mismatch' }>
  | Readonly<{ kind: 'checkpoint-failure' }>;

type NoveltyRolloutSummary = Readonly<{
  acceptedActionCount: number;
  classification: 'authority-private';
  coverage: NoveltyRolloutCoverage;
  finalStateHash: StateHash;
  frontier: readonly NoveltyFrontierCandidate[];
  initialStateHash: StateHash;
  manifestId: StateHash;
  maxActions: number;
  policyVersion: 'one-step-novelty-v1';
  probed: Readonly<{
    actionKinds: readonly ActionKind[];
    eventTypes: readonly string[];
  }>;
  replayVerified: boolean;
  rulesCoverage: 'unranked_partial_rules';
  schemaVersion: 1;
  seed: number;
  tooWide: readonly NoveltyPosition[];
  transcriptHash: StateHash;
}>;

export type NoveltyRolloutResult =
  | Readonly<NoveltyRolloutSummary & {
    status: 'completed';
    terminal: FinishedTerminal;
  }>
  | Readonly<NoveltyRolloutSummary & {
    checkpointId: StateHash;
    status: 'horizon';
  }>
  | Readonly<NoveltyRolloutSummary & {
    failure: NoveltyRolloutFailure;
    status: 'failed';
  }>;

export type NoveltyRolloutOptions = Readonly<{
  maxActions?: number;
  onCheckpoint?: (checkpoint: GameCheckpoint) => void;
}>;

export type ForcedNoveltyOptions = Readonly<NoveltyRolloutOptions & {
  actionId: string;
  actionKind: ActionKind;
  predictedEventTypes: readonly string[];
  predictedStateHash: StateHash;
}>;

export type ForcedNoveltyDispatch = Readonly<{
  entry: Readonly<{
    actionId: string;
    actionKind: ActionKind;
    eventTypes: readonly string[];
    stateHash: StateHash;
  }>;
  result: NoveltyRolloutResult;
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function checkpointFailureFrom(result: NoveltyRolloutResult): NoveltyRolloutResult {
  const summary = { ...result } as Record<string, unknown>;
  delete summary.checkpointId;
  delete summary.failure;
  delete summary.status;
  delete summary.terminal;
  return deepFreeze({
    ...summary,
    failure: { kind: 'checkpoint-failure' as const },
    status: 'failed' as const,
  }) as NoveltyRolloutResult;
}

function applyEmittedCheckpoints(
  result: NoveltyRolloutResult,
  emitted: readonly JsonValue[],
  onCheckpoint?: (checkpoint: GameCheckpoint) => void,
): NoveltyRolloutResult {
  try {
    const seen = new Set<string>();
    for (const raw of emitted) {
      const checkpoint = parseGameCheckpoint(canonicalJson(raw));
      if (seen.has(checkpoint.checkpointId)) continue;
      seen.add(checkpoint.checkpointId);
      onCheckpoint?.(checkpoint);
    }
  } catch {
    return checkpointFailureFrom(result);
  }
  return result;
}

function boundedMaxActions(maxActions: number | undefined): number {
  const value = maxActions ?? NOVELTY_ROLLOUT_ACTION_LIMIT;
  if (!Number.isSafeInteger(value) || value < 0 || value > NOVELTY_ROLLOUT_ACTION_LIMIT) {
    throw new RangeError(`maxActions must be 0-${NOVELTY_ROLLOUT_ACTION_LIMIT}`);
  }
  return value;
}

export async function runNoveltyRollout(
  root: GameSession,
  options: NoveltyRolloutOptions = {},
): Promise<NoveltyRolloutResult> {
  const maxActions = boundedMaxActions(options.maxActions);
  const rootCheckpoint = createGameCheckpoint(root) as unknown as JsonValue;
  return withRustSession(root.manifest, async (handle) => {
    await handle.resume(rootCheckpoint);
    const payload = await handle.runNoveltyRollout({ maxActions });
    if (!isRecord(payload.result)) {
      throw new Error('Rust novelty rollout result was invalid');
    }
    return applyEmittedCheckpoints(
      deepFreeze(payload.result) as NoveltyRolloutResult,
      payload.emittedCheckpoints,
      options.onCheckpoint,
    );
  });
}

async function dispatchForcedNovelty(
  manifest: GameManifest,
  checkpoint: JsonValue,
  options: ForcedNoveltyOptions,
): Promise<ForcedNoveltyDispatch> {
  const maxActions = boundedMaxActions(options.maxActions);
  return withRustSession(manifest, async (handle) => {
    await handle.resume(checkpoint);
    const payload = await handle.runNoveltyFromForcedAction({
      actionId: options.actionId,
      actionKind: options.actionKind,
      maxActions,
      predictedEventTypes: options.predictedEventTypes,
      predictedStateHash: options.predictedStateHash,
    });
    if (!isRecord(payload.result)) {
      throw new Error('Rust forced novelty result was invalid');
    }
    return deepFreeze({
      entry: {
        actionId: payload.entry.actionId,
        actionKind: payload.entry.actionKind as ActionKind,
        eventTypes: payload.entry.eventTypes,
        stateHash: payload.entry.stateHash,
      },
      result: applyEmittedCheckpoints(
        deepFreeze(payload.result) as NoveltyRolloutResult,
        payload.emittedCheckpoints,
        options.onCheckpoint,
      ),
    });
  });
}

/** Forces one engine-issued action from `root`, then runs novelty from the prediction. */
export async function runNoveltyFromForcedAction(
  root: GameSession,
  options: ForcedNoveltyOptions,
): Promise<ForcedNoveltyDispatch> {
  return dispatchForcedNovelty(
    root.manifest,
    createGameCheckpoint(root) as unknown as JsonValue,
    options,
  );
}

/** Resumes a captured checkpoint, forces one engine-issued action, then runs novelty. */
export async function runNoveltyFromForcedCheckpoint(
  checkpoint: GameCheckpoint,
  options: ForcedNoveltyOptions,
): Promise<ForcedNoveltyDispatch> {
  return dispatchForcedNovelty(
    checkpoint.manifest,
    checkpoint as unknown as JsonValue,
    options,
  );
}
