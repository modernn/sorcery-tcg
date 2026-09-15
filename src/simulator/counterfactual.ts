import { deepFreeze, type StateHash } from '../engine/contract.ts';
import { createGameCheckpoint } from '../engine/checkpoint.ts';
import type { JsonValue } from '../authority/canonical-json.ts';
import type {
  GameSession,
  GameTerminal,
} from '../engine/game.ts';
import { withRustSession } from '../engine/rust-session-helpers.ts';

const MAX_CONTINUATION_ACTIONS = 32;

type Terminal = Extract<GameTerminal, { status: 'finished' }>;

type BranchBase = Readonly<{
  decisionCount: number;
  finalStateHash: StateHash;
  rootActionId: string;
}>;

type TerminalBranch = Readonly<BranchBase & {
  outcome: 'terminal';
  score: -1 | 0 | 1;
  terminal: Terminal;
}>;

export type CounterfactualBranch = TerminalBranch | Readonly<BranchBase & {
  outcome: 'unknown';
  reason: 'horizon';
}>;

export type CounterfactualReport = Readonly<{
  branches: readonly CounterfactualBranch[];
  classification: 'authority-private-counterfactual';
  maxContinuationDecisions: number;
  policyVersion: 'deterministic-demo-v1';
  recommendation: Readonly<{ rootActionId: string; score: -1 | 0 | 1 }> | null;
  rootActionCount: number;
  rootActionLimit: 128;
  rootStateHash: StateHash;
  status: 'complete' | 'too-wide';
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/** Runs one counterfactual search inside the authoritative Rust session. */
export async function runCounterfactualRollouts(
  root: GameSession,
  maxContinuationDecisions = MAX_CONTINUATION_ACTIONS,
): Promise<CounterfactualReport> {
  if (!Number.isSafeInteger(maxContinuationDecisions)
    || maxContinuationDecisions < 0
    || maxContinuationDecisions > MAX_CONTINUATION_ACTIONS) {
    throw new RangeError(`maxContinuationDecisions must be 0-${MAX_CONTINUATION_ACTIONS}`);
  }
  const checkpoint = createGameCheckpoint(root) as unknown as JsonValue;
  return withRustSession(root.manifest, async (handle) => {
    await handle.resume(checkpoint);
    const rolled = await handle.client.counterfactualRollout(maxContinuationDecisions);
    if (!isRecord(rolled) || !isRecord(rolled.result)) {
      throw new Error('Rust counterfactual result was invalid');
    }
    return deepFreeze(rolled.result as unknown as CounterfactualReport);
  });
}
