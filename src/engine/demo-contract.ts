import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import { createEngineState, drawUint32, type EngineState } from './determinism.ts';

export type DemoSeat = 'north' | 'south';
type DemoActionKind = 'draw' | 'finish' | 'pass' | 'reveal';
type DemoMarker =
  | Readonly<{ status: 'empty' }>
  | Readonly<{ identity: string; status: 'hidden' | 'revealed' }>;

export type DemoState = Readonly<{
  activeSeat: DemoSeat;
  engine: EngineState;
  markers: Readonly<Record<DemoSeat, DemoMarker>>;
  schemaVersion: 1;
  stateVersion: number;
  terminal: boolean;
}>;

type ObservedMarker =
  | Readonly<{ status: 'empty' | 'hidden' }>
  | Readonly<{ identity: string; status: 'hidden' | 'revealed' }>;

export type DemoObservation = Readonly<{
  activeSeat: DemoSeat;
  markers: Readonly<Record<DemoSeat, ObservedMarker>>;
  schemaVersion: 1;
  stateVersion: number;
  terminal: boolean;
  viewer: DemoSeat;
}>;

export type DemoLegalAction = Readonly<{
  actionId: string;
  label: string;
  seat: DemoSeat;
  stateVersion: number;
}>;

export type DemoActionRequest = Readonly<{
  actionId: string;
  seat: DemoSeat;
  stateVersion: number;
}>;

export type DemoPublicEvent =
  | Readonly<{ seat: DemoSeat; type: 'marker-drawn' }>
  | Readonly<{ identity: string; seat: DemoSeat; type: 'marker-revealed' }>
  | Readonly<{ nextSeat: DemoSeat; seat: DemoSeat; type: 'turn-passed' }>
  | Readonly<{ seat: DemoSeat; type: 'demo-finished' }>;

export type DemoReceipt = Readonly<{
  actionId: string;
  event: DemoPublicEvent;
  postStateHash: ReturnType<typeof identityHash>;
  preStateHash: ReturnType<typeof identityHash>;
  seat: DemoSeat;
  stateVersion: number;
  nextStateVersion: number;
}>;

export type DemoSession = Readonly<{
  state: DemoState;
  transcript: readonly DemoReceipt[];
}>;

export type DemoRejectionReason = 'stale' | 'terminal' | 'unknown_action' | 'wrong_seat';

export type DemoStepResult =
  | Readonly<{ accepted: true; receipt: DemoReceipt; session: DemoSession }>
  | Readonly<{ accepted: false; reason: DemoRejectionReason; session: DemoSession }>;

const ACTION_LABELS: Readonly<Record<DemoActionKind, string>> = Object.freeze({
  draw: 'Draw hidden marker',
  finish: 'Finish demo',
  pass: 'Pass turn',
  reveal: 'Reveal marker',
});

function deepFreeze<T>(value: T): T {
  if (value === null || typeof value !== 'object' || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

function asJson(value: unknown): JsonValue {
  return value as JsonValue;
}

function withStateVersion(engine: EngineState, stateVersion: number): EngineState {
  return deepFreeze({ ...engine, stateVersion });
}

function markerIdentity(value: number): string {
  return `synthetic-marker:${value.toString(16).padStart(8, '0')}`;
}

function otherSeat(seat: DemoSeat): DemoSeat {
  return seat === 'north' ? 'south' : 'north';
}

function actionId(kind: DemoActionKind, seat: DemoSeat, stateVersion: number): string {
  return identityHash({ contract: 'synthetic-demo-v1', kind, seat, stateVersion });
}

function actionKinds(state: DemoState, seat: DemoSeat): readonly DemoActionKind[] {
  if (state.terminal || seat !== state.activeSeat) return [];

  const kinds: DemoActionKind[] = ['pass'];
  const marker = state.markers[seat];
  if (marker.status === 'empty') kinds.push('draw');
  if (marker.status === 'hidden') kinds.push('reveal');
  if (state.markers.north.status === 'revealed' && state.markers.south.status === 'revealed') {
    kinds.push('finish');
  }
  return kinds;
}

function stateWith(
  state: DemoState,
  changes: Readonly<{
    activeSeat?: DemoSeat;
    engine?: EngineState;
    markers?: Readonly<Record<DemoSeat, DemoMarker>>;
    terminal?: boolean;
  }>,
): DemoState {
  const stateVersion = state.stateVersion + 1;
  if (!Number.isSafeInteger(stateVersion)) throw new RangeError('demo state version exhausted');
  return deepFreeze({
    ...state,
    ...changes,
    engine: withStateVersion(changes.engine ?? state.engine, stateVersion),
    stateVersion,
  });
}

export function hashDemoState(state: DemoState): ReturnType<typeof identityHash> {
  return identityHash(asJson(state));
}

export function createDemoState(seed: number): DemoState {
  return deepFreeze({
    activeSeat: 'north',
    engine: createEngineState(seed),
    markers: { north: { status: 'empty' }, south: { status: 'empty' } },
    schemaVersion: 1,
    stateVersion: 0,
    terminal: false,
  });
}

export function createDemoSession(seed: number): DemoSession {
  return deepFreeze({ state: createDemoState(seed), transcript: [] });
}

function observedMarker(marker: DemoMarker, owner: DemoSeat, viewer: DemoSeat): ObservedMarker {
  if (marker.status === 'empty') return { status: 'empty' };
  if (marker.status === 'hidden' && owner !== viewer) return { status: 'hidden' };
  return { identity: marker.identity, status: marker.status };
}

export function observeDemo(state: DemoState, seat: DemoSeat): DemoObservation {
  return deepFreeze({
    activeSeat: state.activeSeat,
    markers: {
      north: observedMarker(state.markers.north, 'north', seat),
      south: observedMarker(state.markers.south, 'south', seat),
    },
    schemaVersion: 1,
    stateVersion: state.stateVersion,
    terminal: state.terminal,
    viewer: seat,
  });
}

export function legalDemoActions(state: DemoState, seat: DemoSeat): readonly DemoLegalAction[] {
  return deepFreeze(
    actionKinds(state, seat)
      .map((kind) => ({
        actionId: actionId(kind, seat, state.stateVersion),
        label: ACTION_LABELS[kind],
        seat,
        stateVersion: state.stateVersion,
      }))
      .sort((left, right) => canonicalJson(left) .localeCompare(canonicalJson(right))),
  );
}

function applyAction(state: DemoState, seat: DemoSeat, kind: DemoActionKind): readonly [DemoState, DemoPublicEvent] {
  if (kind === 'draw') {
    const draw = drawUint32(state.engine);
    const markers = { ...state.markers, [seat]: { identity: markerIdentity(draw.value), status: 'hidden' } } as const;
    return [stateWith(state, { engine: draw.nextState, markers }), { seat, type: 'marker-drawn' }];
  }
  if (kind === 'reveal') {
    const marker = state.markers[seat];
    if (marker.status !== 'hidden') throw new Error('unreachable reveal action');
    const markers = { ...state.markers, [seat]: { identity: marker.identity, status: 'revealed' } } as const;
    return [
      stateWith(state, { markers }),
      { identity: marker.identity, seat, type: 'marker-revealed' },
    ];
  }
  if (kind === 'finish') {
    return [stateWith(state, { terminal: true }), { seat, type: 'demo-finished' }];
  }

  const nextSeat = otherSeat(seat);
  return [stateWith(state, { activeSeat: nextSeat }), { nextSeat, seat, type: 'turn-passed' }];
}

export function stepDemo(session: DemoSession, request: DemoActionRequest): DemoStepResult {
  const { state } = session;
  if (state.terminal) return deepFreeze({ accepted: false, reason: 'terminal', session });
  if (request.stateVersion !== state.stateVersion) {
    return deepFreeze({ accepted: false, reason: 'stale', session });
  }
  if (request.seat !== state.activeSeat) {
    return deepFreeze({ accepted: false, reason: 'wrong_seat', session });
  }

  const kind = actionKinds(state, request.seat).find(
    (candidate) => actionId(candidate, request.seat, state.stateVersion) === request.actionId,
  );
  if (kind === undefined) return deepFreeze({ accepted: false, reason: 'unknown_action', session });

  const preStateHash = hashDemoState(state);
  const [nextState, event] = applyAction(state, request.seat, kind);
  const receipt: DemoReceipt = deepFreeze({
    actionId: request.actionId,
    event,
    nextStateVersion: nextState.stateVersion,
    postStateHash: hashDemoState(nextState),
    preStateHash,
    seat: request.seat,
    stateVersion: state.stateVersion,
  });
  return deepFreeze({
    accepted: true,
    receipt,
    session: { state: nextState, transcript: [...session.transcript, receipt] },
  });
}

export function replayDemo(seed: number, actionIds: readonly string[]): DemoSession {
  let session = createDemoSession(seed);
  for (const replayActionId of actionIds) {
    const result = stepDemo(session, {
      actionId: replayActionId,
      seat: session.state.activeSeat,
      stateVersion: session.state.stateVersion,
    });
    if (!result.accepted) throw new Error(`demo replay rejected action: ${result.reason}`);
    session = result.session;
  }
  return session;
}

export function verifyDemoReplay(
  seed: number,
  actionIds: readonly string[],
  expectedTranscript: readonly DemoReceipt[],
): boolean {
  try {
    return canonicalJson(replayDemo(seed, actionIds).transcript) === canonicalJson(expectedTranscript);
  } catch {
    return false;
  }
}
