import { deepFreeze, type StateHash } from '../engine/contract.ts';
import { createGameCheckpoint } from '../engine/checkpoint.ts';
import type { JsonValue } from '../authority/canonical-json.ts';
import {
  type GameSession,
  type GameTerminal,
} from '../engine/game.ts';
import { withRustSession } from '../engine/rust-session-helpers.ts';

const MAX_CONTINUATION_ACTIONS = 32;

type Terminal = Extract<GameTerminal, { status: 'finished' }>;

type BranchBase = Readonly<{
  /** Includes the root choice; maxContinuationDecisions does not. */
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

// Authority-private: continuations use the complete session and may reveal future hidden cards.
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
    return deepFreeze(
      await handle.runCounterfactual({ maxContinuationDecisions }),
    ) as CounterfactualReport;
  });
}
