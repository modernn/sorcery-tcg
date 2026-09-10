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
import {
  withRustSession,
  type RustGameSessionHandle,
} from '../engine/rust-session-helpers.ts';

export const NOVELTY_ROLLOUT_ACTION_LIMIT = 500;
export const NOVELTY_ROLLOUT_WIDTH_LIMIT = 128;

type ActionKind = GameLegalAction['descriptor']['kind'];
type FinishedTerminal = Extract<GameTerminal, { status: 'finished' }>;
type AcceptedStep = Extract<GameStepResult, { accepted: true }>;

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
  result: AcceptedStep;
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

async function probeStep(
  probe: RustGameSessionHandle,
  session: GameSession,
  action: GameLegalAction,
): Promise<GameStepResult> {
  await probe.resume(createGameCheckpoint(session) as unknown as JsonValue);
  return probe.stepAction(action);
}

export async function runNoveltyRollout(
  root: GameSession,
  options: NoveltyRolloutOptions = {},
): Promise<NoveltyRolloutResult> {
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
  const rootCheckpoint = createGameCheckpoint(root) as unknown as JsonValue;

  return withRustSession(root.manifest, async (live) => {
    await live.resume(rootCheckpoint);
    return withRustSession(root.manifest, async (replay) => {
      await replay.resume(rootCheckpoint);
      return withRustSession(root.manifest, async (probe) => {
        let session = live.snapshot;

        const captureCheckpoint = async (
          candidate: GameSession,
        ): Promise<Readonly<{ checkpointId: StateHash; ok: true } | { ok: false }>> => {
          try {
            const checkpoint = parseGameCheckpoint(
              serializeGameCheckpoint(createGameCheckpoint(candidate)),
            );
            await probe.resume(checkpoint as unknown as JsonValue);
            if (!sameReplay(candidate, probe.snapshot)) return { ok: false };
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

        const fail = async (reason: NoveltyRolloutFailureReason): Promise<NoveltyRolloutResult> => {
          const captured = await captureCheckpoint(session);
          if (!captured.ok) return checkpointFailure(reason.kind !== 'replay-mismatch');
          return deepFreeze({
            ...summary(session, reason.kind !== 'replay-mismatch'),
            failure: { ...reason, checkpointId: captured.checkpointId } as NoveltyRolloutFailure,
            status: 'failed' as const,
          });
        };

        if (!(await live.verifyReplay()) || !(await replay.verifyReplay())) {
          return fail({ kind: 'replay-mismatch' });
        }

        while (true) {
          session = live.snapshot;
          if (session.state.terminal.status === 'finished') {
            return deepFreeze({
              ...summary(session, true),
              status: 'completed' as const,
              terminal: session.state.terminal,
            });
          }
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
            actions = await live.legalActions(session.state.decisionSeat);
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
              const suggested = selectDeterministicGameAction(session, actions);
              fallback = actions.find(({ actionId }) => actionId === suggested.actionId)!;
              if (!fallback) return fail({ kind: 'exception', phase: 'selector' });
            } catch {
              return fail({ kind: 'exception', phase: 'selector' });
            }
            let result: GameStepResult;
            try {
              result = await probeStep(probe, session, fallback);
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
              const suggested = selectDeterministicGameAction(session, actions);
              const fallback = actions.find(({ actionId }) => actionId === suggested.actionId);
              if (!fallback) return fail({ kind: 'exception', phase: 'selector' });
              fallbackActionId = fallback.actionId;
            } catch {
              return fail({ kind: 'exception', phase: 'selector' });
            }

            const probes: Probe[] = [];
            for (const [index, candidate] of actions.entries()) {
              let result: GameStepResult;
              try {
                result = await probeStep(probe, session, candidate);
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
            const candidates = unchosen.flatMap((probeResult) => [
              ...(!committedAfterAction.has(actionKind(probeResult.action))
                ? [{ kind: 'action-kind' as const, value: actionKind(probeResult.action) }]
                : []),
              ...probeResult.eventTypes
                .filter((type) => !committedAfterEvents.has(type))
                .map((value) => ({ kind: 'event-type' as const, value })),
            ].map((signal) => ({ probe: probeResult, signal })));
            if (candidates.length > 0) {
              const captured = await captureCheckpoint(session);
              if (!captured.ok) return checkpointFailure();
              for (const { probe: probeResult, signal } of candidates) {
                const ranked: RankedFrontier = {
                  candidate: deepFreeze({
                    actionId: probeResult.action.actionId,
                    actionKind: actionKind(probeResult.action),
                    checkpointId: captured.checkpointId,
                    predictedEventTypes: probeResult.eventTypes,
                    predictedStateHash: hashGameState(probeResult.result.session.state),
                    signal,
                  }),
                  decisionIndex: acceptedActionCount,
                  legalIndex: probeResult.index,
                  newActionKind: probeResult.newActionKind,
                  newEventCount: probeResult.newEventCount,
                  selectedByFallback: probeResult.selectedByFallback,
                };
                const key = signalKey(signal.kind, signal.value);
                const existing = frontier.get(key);
                frontier.set(key, existing ? preferredFrontier(existing, ranked) : ranked);
              }
            }
          }

          probedActionKinds.add(actionKind(selected.action));
          selected.eventTypes.forEach((type) => probedEventTypes.add(type));

          let replayResult: GameStepResult;
          try {
            const committed = await live.stepAction(selected.action);
            if (!committed.accepted || !sameReplay(selected.result.session, committed.session)) {
              return fail({ kind: 'replay-mismatch' });
            }
            replayResult = await replay.step({
              actionId: selected.action.actionId,
              seat: replay.snapshot.state.decisionSeat,
              stateVersion: replay.snapshot.state.stateVersion,
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
            committedActionKinds.set(selectedKind, {
              firstSeen: committedPosition,
              value: selectedKind,
            });
          }
          for (const type of selected.eventTypes) {
            if (!committedEventTypes.has(type)) {
              committedEventTypes.set(type, { firstSeen: committedPosition, value: type });
            }
            frontier.delete(signalKey('event-type', type));
          }
          frontier.delete(signalKey('action-kind', selectedKind));
          session = live.snapshot;
          acceptedActionCount += 1;
        }
      });
    });
  });
}
