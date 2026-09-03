import { selectDeterministicGameAction } from '../commands/run-game-demo.ts';
import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  serializeGameCheckpoint,
  type GameCheckpoint,
} from '../engine/checkpoint.ts';
import { deepFreeze, type EngineRejectionCode, type StateHash } from '../engine/contract.ts';
import {
  hashGameState,
  type GameLegalAction,
  type GameSession,
  type GameStepResult,
  type GameTerminal,
} from '../engine/game.ts';
import { withRustSession, type RustGameSessionHandle } from '../engine/rust-session-helpers.ts';

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
    phase: 'legal-actions' | 'selector';
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

type NoveltyRolloutFailureReason = NoveltyRolloutFailure extends infer Failure
  ? Failure extends Readonly<{ checkpointId: StateHash }>
    ? Omit<Failure, 'checkpointId'>
    : never
  : never;

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

type Probe = Readonly<{
  action: GameLegalAction;
  eventTypes: readonly string[];
  index: number;
  newActionKind: boolean;
  newEventCount: number;
  result: Extract<GameStepResult, { accepted: true }>;
  selectedByFallback: boolean;
}>;

type RankedFrontier = Readonly<{
  candidate: NoveltyFrontierCandidate;
  decisionIndex: number;
  legalIndex: number;
  newActionKind: boolean;
  newEventCount: number;
  selectedByFallback: boolean;
}>;

function position(session: GameSession, decisionIndex: number): NoveltyPosition {
  return deepFreeze({
    decisionIndex,
    stateHash: hashGameState(session.state),
    transcriptLength: session.transcript.length,
  });
}

function actionKind(action: GameLegalAction): ActionKind {
  return action.descriptor.kind;
}

function sameReplay(left: GameSession, right: GameSession): boolean {
  return canonicalJson({
    initialRandomDraws: left.initialRandomDraws,
    state: left.state,
    transcript: left.transcript,
  } as unknown as JsonValue) === canonicalJson({
    initialRandomDraws: right.initialRandomDraws,
    state: right.state,
    transcript: right.transcript,
  } as unknown as JsonValue);
}

function preferredProbe(left: Probe, right: Probe): Probe {
  if (left.newEventCount !== right.newEventCount) {
    return left.newEventCount > right.newEventCount ? left : right;
  }
  if (left.newActionKind !== right.newActionKind) {
    return left.newActionKind ? left : right;
  }
  if (left.selectedByFallback !== right.selectedByFallback) {
    return left.selectedByFallback ? left : right;
  }
  return left.index < right.index ? left : right;
}

function preferredFrontier(left: RankedFrontier, right: RankedFrontier): RankedFrontier {
  if (left.newEventCount !== right.newEventCount) {
    return left.newEventCount > right.newEventCount ? left : right;
  }
  if (left.newActionKind !== right.newActionKind) {
    return left.newActionKind ? left : right;
  }
  if (left.selectedByFallback !== right.selectedByFallback) {
    return left.selectedByFallback ? left : right;
  }
  if (left.legalIndex !== right.legalIndex) {
    return left.legalIndex < right.legalIndex ? left : right;
  }
  if (left.decisionIndex !== right.decisionIndex) {
    return left.decisionIndex < right.decisionIndex ? left : right;
  }
  return compareStrings(left.candidate.actionId, right.candidate.actionId) <= 0 ? left : right;
}

function signalKey(kind: 'action-kind' | 'event-type', value: string): string {
  return `${kind}:\0${value}`;
}

function compareStrings(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

/** Runs one coverage-guided rollout from `root` against the authoritative Rust engine. */
export async function runNoveltyRollout(
  root: GameSession,
  options: NoveltyRolloutOptions = {},
): Promise<NoveltyRolloutResult> {
  const maxActions = options.maxActions ?? NOVELTY_ROLLOUT_ACTION_LIMIT;
  if (!Number.isSafeInteger(maxActions) || maxActions < 0
    || maxActions > NOVELTY_ROLLOUT_ACTION_LIMIT) {
    throw new RangeError(`maxActions must be 0-${NOVELTY_ROLLOUT_ACTION_LIMIT}`);
  }
  // ponytail: one live session plus one probe session. The Rust boundary has no cheap undo,
  // so every speculative branch resumes the current node checkpoint instead.
  return withRustSession(root.manifest, async (main) =>
    withRustSession(root.manifest, async (probe) =>
      rollout(root, maxActions, options, main, probe)));
}

async function rollout(
  root: GameSession,
  maxActions: number,
  options: NoveltyRolloutOptions,
  main: RustGameSessionHandle,
  probe: RustGameSessionHandle,
): Promise<NoveltyRolloutResult> {
  const initialStateHash = hashGameState(root.state);
  const offeredActionKinds = new Map<ActionKind, FirstSeen<ActionKind>>();
  const branchFactors = new Map<number, FirstSeen<number>>();
  const committedActionKinds = new Map<ActionKind, FirstSeen<ActionKind>>();
  const committedEventTypes = new Map<string, FirstSeen<string>>();
  const probedActionKinds = new Set<ActionKind>();
  const probedEventTypes = new Set<string>();
  const frontier = new Map<string, RankedFrontier>();
  const tooWide: NoveltyPosition[] = [];
  const emittedCheckpoints = new Set<StateHash>();
  let acceptedActionCount = 0;
  let session = root;
  let prepared: Readonly<{ checkpoint: GameCheckpoint; session: GameSession }> | undefined;

  /** Serializes one node checkpoint and proves the Rust engine resumes it byte-for-byte. */
  const prepareCheckpoint = async (candidate: GameSession): Promise<GameCheckpoint | undefined> => {
    if (prepared?.session === candidate) return prepared.checkpoint;
    try {
      const checkpoint = parseGameCheckpoint(
        serializeGameCheckpoint(createGameCheckpoint(candidate)),
      );
      if (!sameReplay(candidate, await probe.resume(checkpoint as unknown as JsonValue))) {
        return undefined;
      }
      prepared = { checkpoint, session: candidate };
      return checkpoint;
    } catch {
      return undefined;
    }
  };

  const captureCheckpoint = async (candidate: GameSession): Promise<
    Readonly<{ checkpointId: StateHash; ok: true } | { ok: false }>
  > => {
    const checkpoint = await prepareCheckpoint(candidate);
    if (!checkpoint) return { ok: false };
    try {
      if (!emittedCheckpoints.has(checkpoint.checkpointId)) {
        options.onCheckpoint?.(checkpoint);
        emittedCheckpoints.add(checkpoint.checkpointId);
      }
    } catch {
      return { ok: false };
    }
    return { checkpointId: checkpoint.checkpointId, ok: true };
  };

  const summary = (candidate: GameSession, replayVerified: boolean): NoveltyRolloutSummary =>
    deepFreeze({
      acceptedActionCount,
      classification: 'authority-private' as const,
      coverage: {
        branchFactors: [...branchFactors.values()],
        committedActionKinds: [...committedActionKinds.values()],
        committedEventTypes: [...committedEventTypes.values()],
        offeredActionKinds: [...offeredActionKinds.values()],
      },
      finalStateHash: hashGameState(candidate.state),
      frontier: [...frontier.values()]
        .map(({ candidate: value }) => value)
        .sort((left, right) => compareStrings(
          signalKey(left.signal.kind, left.signal.value),
          signalKey(right.signal.kind, right.signal.value),
        )),
      initialStateHash,
      manifestId: root.manifest.manifestId,
      maxActions,
      policyVersion: 'one-step-novelty-v1' as const,
      probed: {
        actionKinds: [...probedActionKinds].sort(),
        eventTypes: [...probedEventTypes].sort(),
      },
      replayVerified,
      rulesCoverage: 'unranked_partial_rules' as const,
      schemaVersion: 1 as const,
      seed: root.manifest.seed,
      tooWide,
      transcriptHash: identityHash(candidate.transcript as unknown as JsonValue),
    });

  const checkpointFailure = (replayVerified = true): NoveltyRolloutResult => deepFreeze({
    ...summary(session, replayVerified),
    failure: { kind: 'checkpoint-failure' as const },
    status: 'failed' as const,
  });

  const fail = async (reason: NoveltyRolloutFailureReason): Promise<NoveltyRolloutResult> => {
    const captured = await captureCheckpoint(session);
    if (!captured.ok) return checkpointFailure(reason.kind !== 'replay-mismatch');
    return deepFreeze({
      ...summary(session, reason.kind !== 'replay-mismatch'),
      failure: { ...reason, checkpointId: captured.checkpointId } as NoveltyRolloutFailure,
      status: 'failed' as const,
    });
  };

  try {
    const rootCheckpoint = await prepareCheckpoint(root);
    if (!rootCheckpoint) return await fail({ kind: 'replay-mismatch' });
    await main.resume(rootCheckpoint as unknown as JsonValue);
    if (!sameReplay(root, main.snapshot) || !await main.verifyReplay()) {
      return await fail({ kind: 'replay-mismatch' });
    }
  } catch {
    return await fail({ kind: 'replay-mismatch' });
  }

  /** Applies one speculative action to a fresh copy of the current node. */
  const speculate = async (
    node: GameCheckpoint,
    candidate: GameLegalAction,
  ): Promise<GameStepResult> => {
    await probe.resume(node as unknown as JsonValue);
    return probe.stepAction(candidate);
  };

  while (true) {
    if (session.state.terminal.status === 'finished') {
      return deepFreeze({
        ...summary(session, true),
        status: 'completed' as const,
        terminal: session.state.terminal,
      });
    }

    const node = await prepareCheckpoint(session);
    if (!node) return checkpointFailure();

    if (acceptedActionCount >= maxActions) {
      const captured = await captureCheckpoint(session);
      if (!captured.ok) return checkpointFailure();
      return deepFreeze({
        ...summary(session, true),
        checkpointId: captured.checkpointId,
        status: 'horizon' as const,
      });
    }

    const currentPosition = position(session, acceptedActionCount);
    let actions: readonly GameLegalAction[];
    try {
      actions = await main.legalActions();
    } catch {
      return await fail({ kind: 'exception', phase: 'legal-actions' });
    }
    if (!branchFactors.has(actions.length)) {
      branchFactors.set(actions.length, { firstSeen: currentPosition, value: actions.length });
    }
    if (actions.length === 0) return await fail({ kind: 'deadlock' });

    for (const kind of new Set(actions.map(actionKind))) {
      if (!offeredActionKinds.has(kind)) {
        offeredActionKinds.set(kind, { firstSeen: currentPosition, value: kind });
      }
    }

    let fallback: GameLegalAction;
    try {
      const suggested = selectDeterministicGameAction(session, actions);
      const found = actions.find(({ actionId }) => actionId === suggested.actionId);
      if (!found) return await fail({ kind: 'exception', phase: 'selector' });
      fallback = found;
    } catch {
      return await fail({ kind: 'exception', phase: 'selector' });
    }

    let selected: Probe;
    if (actions.length > NOVELTY_ROLLOUT_WIDTH_LIMIT) {
      tooWide.push(currentPosition);
      let result: GameStepResult;
      try {
        result = await speculate(node, fallback);
      } catch {
        return await fail({ actionId: fallback.actionId, kind: 'exception', phase: 'fallback' });
      }
      if (!result.accepted) {
        return await fail({
          actionId: fallback.actionId,
          code: result.reason.code,
          kind: 'engine-rejection',
          phase: 'fallback',
        });
      }
      selected = {
        action: fallback,
        eventTypes: [...new Set(result.receipt.events.map(({ type }) => type))].sort(),
        index: actions.indexOf(fallback),
        newActionKind: !committedActionKinds.has(actionKind(fallback)),
        newEventCount: result.receipt.events.filter(({ type }, index, events) =>
          events.findIndex((event) => event.type === type) === index
            && !committedEventTypes.has(type)).length,
        result,
        selectedByFallback: true,
      };
    } else {
      const probes: Probe[] = [];
      for (const [index, candidate] of actions.entries()) {
        let result: GameStepResult;
        try {
          result = await speculate(node, candidate);
        } catch {
          return await fail({ actionId: candidate.actionId, kind: 'exception', phase: 'probe' });
        }
        if (!result.accepted) {
          return await fail({
            actionId: candidate.actionId,
            code: result.reason.code,
            kind: 'engine-rejection',
            phase: 'probe',
          });
        }
        const eventTypes = [...new Set(result.receipt.events.map(({ type }) => type))].sort();
        probedActionKinds.add(actionKind(candidate));
        eventTypes.forEach((type) => probedEventTypes.add(type));
        probes.push({
          action: candidate,
          eventTypes,
          index,
          newActionKind: !committedActionKinds.has(actionKind(candidate)),
          newEventCount: eventTypes.filter((type) => !committedEventTypes.has(type)).length,
          result,
          selectedByFallback: candidate.actionId === fallback.actionId,
        });
      }
      selected = probes.reduce(preferredProbe);

      const committedAfterAction = new Set(committedActionKinds.keys());
      committedAfterAction.add(actionKind(selected.action));
      const committedAfterEvents = new Set(committedEventTypes.keys());
      selected.eventTypes.forEach((type) => committedAfterEvents.add(type));
      const unchosen = probes.filter(({ action }) => action.actionId !== selected.action.actionId);
      const candidates = unchosen.flatMap((entry) => [
        ...(!committedAfterAction.has(actionKind(entry.action))
          ? [{ kind: 'action-kind' as const, value: actionKind(entry.action) }]
          : []),
        ...entry.eventTypes
          .filter((type) => !committedAfterEvents.has(type))
          .map((value) => ({ kind: 'event-type' as const, value })),
      ].map((signal) => ({ entry, signal })));
      if (candidates.length > 0) {
        const captured = await captureCheckpoint(session);
        if (!captured.ok) return checkpointFailure();
        for (const { entry, signal } of candidates) {
          const ranked: RankedFrontier = {
            candidate: deepFreeze({
              actionId: entry.action.actionId,
              actionKind: actionKind(entry.action),
              checkpointId: captured.checkpointId,
              predictedEventTypes: entry.eventTypes,
              predictedStateHash: hashGameState(entry.result.session.state),
              signal,
            }),
            decisionIndex: acceptedActionCount,
            legalIndex: entry.index,
            newActionKind: entry.newActionKind,
            newEventCount: entry.newEventCount,
            selectedByFallback: entry.selectedByFallback,
          };
          const key = signalKey(signal.kind, signal.value);
          const existing = frontier.get(key);
          frontier.set(key, existing ? preferredFrontier(existing, ranked) : ranked);
        }
      }
    }

    probedActionKinds.add(actionKind(selected.action));
    selected.eventTypes.forEach((type) => probedEventTypes.add(type));

    // The live session advances incrementally while the probe replayed the same action from the
    // manifest. Divergence between the two is a replay mismatch, not a rollout result.
    let applied: GameStepResult;
    try {
      applied = await main.stepAction(selected.action);
    } catch {
      return await fail({ kind: 'replay-mismatch' });
    }
    if (!applied.accepted || !sameReplay(selected.result.session, applied.session)) {
      return await fail({ kind: 'replay-mismatch' });
    }

    const committedPosition = currentPosition;
    const selectedKind = actionKind(selected.action);
    if (!committedActionKinds.has(selectedKind)) {
      committedActionKinds.set(selectedKind, { firstSeen: committedPosition, value: selectedKind });
    }
    for (const type of selected.eventTypes) {
      if (!committedEventTypes.has(type)) {
        committedEventTypes.set(type, { firstSeen: committedPosition, value: type });
      }
      frontier.delete(signalKey('event-type', type));
    }
    frontier.delete(signalKey('action-kind', selectedKind));
    session = applied.session;
    acceptedActionCount += 1;
  }
}
