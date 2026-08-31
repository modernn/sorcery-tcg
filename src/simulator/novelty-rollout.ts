import { selectDeterministicGameAction } from '../commands/run-game-demo.ts';
import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpoint,
  serializeGameCheckpoint,
  type GameCheckpoint,
} from '../engine/checkpoint.ts';
import { deepFreeze, type EngineRejectionCode, type StateHash } from '../engine/contract.ts';
import {
  hashGameState,
  legalGameActions,
  replayGame,
  stepGame,
  verifyGameReplay,
  type GameLegalAction,
  type GameSession,
  type GameTerminal,
} from '../engine/game.ts';

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
  result: Extract<ReturnType<typeof stepGame>, { accepted: true }>;
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

export function runNoveltyRollout(
  root: GameSession,
  options: NoveltyRolloutOptions = {},
): NoveltyRolloutResult {
  const maxActions = options.maxActions ?? NOVELTY_ROLLOUT_ACTION_LIMIT;
  if (!Number.isSafeInteger(maxActions) || maxActions < 0
    || maxActions > NOVELTY_ROLLOUT_ACTION_LIMIT) {
    throw new RangeError(`maxActions must be 0-${NOVELTY_ROLLOUT_ACTION_LIMIT}`);
  }

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
  let replayed: GameSession | undefined;

  const captureCheckpoint = (
    candidate: GameSession,
  ): Readonly<{ checkpointId: StateHash; ok: true } | { ok: false }> => {
    try {
      const checkpoint = parseGameCheckpoint(
        serializeGameCheckpoint(createGameCheckpoint(candidate)),
      );
      const resumed = resumeGameCheckpoint(checkpoint);
      if (!sameReplay(candidate, resumed)) return { ok: false };
      if (!emittedCheckpoints.has(checkpoint.checkpointId)) {
        options.onCheckpoint?.(checkpoint);
        emittedCheckpoints.add(checkpoint.checkpointId);
      }
      return { checkpointId: checkpoint.checkpointId, ok: true };
    } catch {
      return { ok: false };
    }
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

  const fail = (reason: NoveltyRolloutFailureReason): NoveltyRolloutResult => {
    const captured = captureCheckpoint(session);
    if (!captured.ok) return checkpointFailure(reason.kind !== 'replay-mismatch');
    return deepFreeze({
      ...summary(session, reason.kind !== 'replay-mismatch'),
      failure: { ...reason, checkpointId: captured.checkpointId } as NoveltyRolloutFailure,
      status: 'failed' as const,
    });
  };

  if (!verifyGameReplay(root)) {
    return fail({ kind: 'replay-mismatch' });
  }
  try {
    replayed = replayGame(root.manifest, root.transcript.map(({ actionId }) => actionId));
  } catch {
    return fail({ kind: 'replay-mismatch' });
  }

  while (true) {
    if (session.state.terminal.status === 'finished') {
      return deepFreeze({
        ...summary(session, true),
        status: 'completed' as const,
        terminal: session.state.terminal,
      });
    }
    if (acceptedActionCount >= maxActions) {
      const captured = captureCheckpoint(session);
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
      actions = legalGameActions(session.state, session.state.decisionSeat);
    } catch {
      return fail({ kind: 'exception', phase: 'legal-actions' });
    }
    if (!branchFactors.has(actions.length)) {
      branchFactors.set(actions.length, { firstSeen: currentPosition, value: actions.length });
    }
    if (actions.length === 0) return fail({ kind: 'deadlock' });

    for (const kind of new Set(actions.map(actionKind))) {
      if (!offeredActionKinds.has(kind)) {
        offeredActionKinds.set(kind, { firstSeen: currentPosition, value: kind });
      }
    }

    let selected: Probe;
    if (actions.length > NOVELTY_ROLLOUT_WIDTH_LIMIT) {
      tooWide.push(currentPosition);
      let fallback: GameLegalAction;
      try {
        const suggested = selectDeterministicGameAction(session);
        fallback = actions.find(({ actionId }) => actionId === suggested.actionId)!;
        if (!fallback) return fail({ kind: 'exception', phase: 'selector' });
      } catch {
        return fail({ kind: 'exception', phase: 'selector' });
      }
      let result: ReturnType<typeof stepGame>;
      try {
        result = stepGame(session, fallback);
      } catch {
        return fail({ actionId: fallback.actionId, kind: 'exception', phase: 'fallback' });
      }
      if (!result.accepted) {
        return fail({
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
      let fallbackActionId: string;
      try {
        const suggested = selectDeterministicGameAction(session);
        const fallback = actions.find(({ actionId }) => actionId === suggested.actionId);
        if (!fallback) return fail({ kind: 'exception', phase: 'selector' });
        fallbackActionId = fallback.actionId;
      } catch {
        return fail({ kind: 'exception', phase: 'selector' });
      }

      const probes: Probe[] = [];
      for (const [index, candidate] of actions.entries()) {
        let result: ReturnType<typeof stepGame>;
        try {
          result = stepGame(session, candidate);
        } catch {
          return fail({ actionId: candidate.actionId, kind: 'exception', phase: 'probe' });
        }
        if (!result.accepted) {
          return fail({
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
          selectedByFallback: candidate.actionId === fallbackActionId,
        });
      }
      selected = probes.reduce(preferredProbe);

      const committedAfterAction = new Set(committedActionKinds.keys());
      committedAfterAction.add(actionKind(selected.action));
      const committedAfterEvents = new Set(committedEventTypes.keys());
      selected.eventTypes.forEach((type) => committedAfterEvents.add(type));
      const unchosen = probes.filter(({ action }) => action.actionId !== selected.action.actionId);
      const candidates = unchosen.flatMap((probe) => [
        ...(!committedAfterAction.has(actionKind(probe.action))
          ? [{ kind: 'action-kind' as const, value: actionKind(probe.action) }]
          : []),
        ...probe.eventTypes
          .filter((type) => !committedAfterEvents.has(type))
          .map((value) => ({ kind: 'event-type' as const, value })),
      ].map((signal) => ({ probe, signal })));
      if (candidates.length > 0) {
        const captured = captureCheckpoint(session);
        if (!captured.ok) return checkpointFailure();
        for (const { probe, signal } of candidates) {
          const ranked: RankedFrontier = {
            candidate: deepFreeze({
              actionId: probe.action.actionId,
              actionKind: actionKind(probe.action),
              checkpointId: captured.checkpointId,
              predictedEventTypes: probe.eventTypes,
              predictedStateHash: hashGameState(probe.result.session.state),
              signal,
            }),
            decisionIndex: acceptedActionCount,
            legalIndex: probe.index,
            newActionKind: probe.newActionKind,
            newEventCount: probe.newEventCount,
            selectedByFallback: probe.selectedByFallback,
          };
          const key = signalKey(signal.kind, signal.value);
          const existing = frontier.get(key);
          frontier.set(key, existing ? preferredFrontier(existing, ranked) : ranked);
        }
      }
    }

    probedActionKinds.add(actionKind(selected.action));
    selected.eventTypes.forEach((type) => probedEventTypes.add(type));

    let replayResult: ReturnType<typeof stepGame>;
    try {
      replayResult = stepGame(replayed, {
        actionId: selected.action.actionId,
        seat: replayed.state.decisionSeat,
        stateVersion: replayed.state.stateVersion,
      });
    } catch {
      return fail({ kind: 'replay-mismatch' });
    }
    if (!replayResult.accepted || !sameReplay(selected.result.session, replayResult.session)) {
      return fail({ kind: 'replay-mismatch' });
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
    session = selected.result.session;
    replayed = replayResult.session;
    acceptedActionCount += 1;
  }
}
