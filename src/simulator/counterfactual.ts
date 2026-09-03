import { selectDeterministicGameAction } from '../commands/run-game-demo.ts';
import { deepFreeze, type StateHash } from '../engine/contract.ts';
import {
  createGameCheckpoint,
} from '../engine/checkpoint.ts';
import type { JsonValue } from '../authority/canonical-json.ts';
import {
  hashGameState,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
  type GameTerminal,
} from '../engine/game.ts';
import { withRustSession } from '../engine/rust-session-helpers.ts';

const MAX_ROOT_ACTIONS = 128;
const MAX_CONTINUATION_ACTIONS = 32;
const POLICY_VERSION = 'deterministic-demo-v1' as const;

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
  policyVersion: typeof POLICY_VERSION;
  recommendation: Readonly<{ rootActionId: string; score: -1 | 0 | 1 }> | null;
  rootActionCount: number;
  rootActionLimit: typeof MAX_ROOT_ACTIONS;
  rootStateHash: StateHash;
  status: 'complete' | 'too-wide';
}>;

function terminalScore(terminal: Terminal, perspective: GameSession['state']['decisionSeat']): -1 | 0 | 1 {
  if ('result' in terminal) return 0;
  return terminal.winner === perspective ? 1 : -1;
}

function preferredBranch(left: TerminalBranch, right: TerminalBranch): TerminalBranch {
  if (left.score !== right.score) return left.score > right.score ? left : right;
  if (left.score === 1 && left.decisionCount !== right.decisionCount) {
    return left.decisionCount < right.decisionCount ? left : right;
  }
  if (left.score === -1 && left.decisionCount !== right.decisionCount) {
    return left.decisionCount > right.decisionCount ? left : right;
  }
  return left;
}

async function rolloutBranch(
  manifest: GameManifest,
  checkpoint: JsonValue,
  rootAction: GameLegalAction,
  maxContinuationDecisions: number,
  perspective: GameSession['state']['decisionSeat'],
): Promise<CounterfactualBranch> {
  return withRustSession(manifest, async (handle) => {
    await handle.resume(checkpoint);
    const rootResult = await handle.stepAction(rootAction);
    if (!rootResult.accepted) {
      throw new Error(`engine rejected issued root action: ${rootResult.reason.code}`);
    }
    let decisionCount = 1;
    while (handle.snapshot.state.terminal.status === 'active'
      && decisionCount <= maxContinuationDecisions) {
      const issuedActions = await handle.legalActions();
      const result = await handle.stepAction(
        selectDeterministicGameAction(handle.snapshot, issuedActions),
      );
      if (!result.accepted) {
        throw new Error(`engine rejected issued continuation: ${result.reason.code}`);
      }
      decisionCount += 1;
    }
    const session = handle.snapshot;
    const common = {
      decisionCount,
      finalStateHash: hashGameState(session.state),
      rootActionId: rootAction.actionId,
    };
    return session.state.terminal.status === 'finished'
      ? deepFreeze({
        ...common,
        outcome: 'terminal' as const,
        score: terminalScore(session.state.terminal, perspective),
        terminal: session.state.terminal,
      })
      : deepFreeze({
        ...common,
        outcome: 'unknown' as const,
        reason: 'horizon' as const,
      });
  });
}

export async function runCounterfactualRollouts(
  root: GameSession,
  maxContinuationDecisions = MAX_CONTINUATION_ACTIONS,
): Promise<CounterfactualReport> {
  if (!Number.isSafeInteger(maxContinuationDecisions)
    || maxContinuationDecisions < 0
    || maxContinuationDecisions > MAX_CONTINUATION_ACTIONS) {
    throw new RangeError(`maxContinuationDecisions must be 0-${MAX_CONTINUATION_ACTIONS}`);
  }
  const rootStateHash = hashGameState(root.state);
  const checkpoint = createGameCheckpoint(root) as unknown as JsonValue;
  const actions = await withRustSession(root.manifest, async (handle) => {
    await handle.resume(checkpoint);
    return handle.legalActions(root.state.decisionSeat);
  });
  const base = {
    classification: 'authority-private-counterfactual' as const,
    maxContinuationDecisions,
    policyVersion: POLICY_VERSION,
    rootActionCount: actions.length,
    rootActionLimit: 128 as const,
    rootStateHash,
  };
  if (actions.length > MAX_ROOT_ACTIONS) {
    return deepFreeze({ ...base, branches: [], recommendation: null, status: 'too-wide' as const });
  }

  const perspective = root.state.decisionSeat;
  const branches = await Promise.all(actions.map((rootAction) =>
    rolloutBranch(root.manifest, checkpoint, rootAction, maxContinuationDecisions, perspective)));
  const terminalBranches = branches.filter((branch): branch is TerminalBranch =>
    branch.outcome === 'terminal');
  const allTerminal = branches.length > 0 && terminalBranches.length === branches.length;
  const preferred = allTerminal ? terminalBranches.reduce(preferredBranch) : undefined;
  return deepFreeze({
    ...base,
    branches,
    recommendation: preferred
      ? { rootActionId: preferred.rootActionId, score: preferred.score }
      : null,
    status: 'complete' as const,
  });
}
