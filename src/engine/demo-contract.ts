import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import { createEngineState, drawUint32, type EngineState } from './determinism.ts';
import {
  createAttempt,
  createEvents,
  createReceipt,
  createRejection,
  deepFreeze,
  opaqueActionId,
  orderLegalActions,
  type EngineActionRequest,
  type EngineAttempt,
  type EngineEvent,
  type EngineLegalAction,
  type EngineRandomDraw,
  type EngineReceipt,
  type EngineRejection,
} from './contract.ts';

export type DemoSeat = 'north' | 'south';
type DemoActionKind = 'draw' | 'finish' | 'pass' | 'reveal';
type DemoActionDescriptor = Readonly<{
  kind: DemoActionKind;
  zone: 'marker';
}>;
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

export type DemoLegalAction = EngineLegalAction<DemoActionDescriptor>;
export type DemoActionRequest = EngineActionRequest;
export type DemoPublicEvent = EngineEvent;
export type DemoReceipt = EngineReceipt;

export type DemoManifest = Readonly<{
  contractVersion: 'synthetic-demo-v2';
  mode: 'synthetic';
  schemaVersion: 1;
  seed: number;
}>;

export type DemoSession = Readonly<{
  attempts: readonly EngineAttempt[];
  manifest: DemoManifest;
  state: DemoState;
  transcript: readonly DemoReceipt[];
}>;

export type DemoStepResult =
  | Readonly<{ accepted: true; receipt: DemoReceipt; session: DemoSession }>
  | Readonly<{ accepted: false; reason: EngineRejection; session: DemoSession }>;

const ACTION_LABELS: Readonly<Record<DemoActionKind, string>> = Object.freeze({
  draw: 'Draw hidden marker',
  finish: 'Finish demo',
  pass: 'Pass turn',
  reveal: 'Reveal marker',
});

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

function actionDescriptor(kind: DemoActionKind): DemoActionDescriptor {
  return deepFreeze({ kind, zone: 'marker' });
}

function actionId(kind: DemoActionKind, seat: DemoSeat, stateVersion: number): string {
  return opaqueActionId('synthetic-demo-v2', seat, stateVersion, actionDescriptor(kind));
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

export function createDemoManifest(seed: number): DemoManifest {
  createEngineState(seed);
  return deepFreeze({ contractVersion: 'synthetic-demo-v2', mode: 'synthetic', schemaVersion: 1, seed });
}

export function createDemoSession(manifest: DemoManifest): DemoSession {
  return deepFreeze({ attempts: [], manifest, state: createDemoState(manifest.seed), transcript: [] });
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
  return orderLegalActions(
    actionKinds(state, seat).map((kind) => {
      const descriptor = actionDescriptor(kind);
      return {
        actionId: actionId(kind, seat, state.stateVersion),
        descriptor,
        label: ACTION_LABELS[kind],
        seat,
        stateVersion: state.stateVersion,
      };
    }),
  );
}

type DemoOutcome = Readonly<{
  payload: JsonValue;
  type: 'demo-finished' | 'marker-drawn' | 'marker-revealed' | 'turn-passed';
}>;

function applyAction(
  state: DemoState,
  seat: DemoSeat,
  kind: DemoActionKind,
): readonly [DemoState, DemoOutcome, readonly EngineRandomDraw[]] {
  if (kind === 'draw') {
    const prePrngStateHash = identityHash(asJson(state.engine.prng));
    const draw = drawUint32(state.engine);
    const markers = { ...state.markers, [seat]: { identity: markerIdentity(draw.value), status: 'hidden' } } as const;
    const randomDraw = deepFreeze({
      domain: { kind: 'uint32', maximum: 0xffff_ffff, minimum: 0 },
      drawSequence: draw.nextState.prng.draws,
      postPrngStateHash: identityHash(asJson(draw.nextState.prng)),
      prePrngStateHash,
      purpose: 'synthetic_marker_identity',
      result: draw.value,
    });
    return [
      stateWith(state, { engine: draw.nextState, markers }),
      { payload: { seat }, type: 'marker-drawn' },
      [randomDraw],
    ];
  }
  if (kind === 'reveal') {
    const marker = state.markers[seat];
    if (marker.status !== 'hidden') throw new Error('unreachable reveal action');
    const markers = { ...state.markers, [seat]: { identity: marker.identity, status: 'revealed' } } as const;
    return [
      stateWith(state, { markers }),
      { payload: { identity: marker.identity, seat }, type: 'marker-revealed' },
      [],
    ];
  }
  if (kind === 'finish') {
    return [stateWith(state, { terminal: true }), { payload: { seat }, type: 'demo-finished' }, []];
  }

  const nextSeat = otherSeat(seat);
  return [
    stateWith(state, { activeSeat: nextSeat }),
    { payload: { nextSeat, seat }, type: 'turn-passed' },
    [],
  ];
}

export function stepDemo(session: DemoSession, request: DemoActionRequest): DemoStepResult {
  const { state } = session;
  const command: DemoActionRequest = deepFreeze({
    actionId: request.actionId,
    seat: request.seat,
    stateVersion: request.stateVersion,
  });
  const stateHash = hashDemoState(state);
  const reject = (code: EngineRejection['code']): DemoStepResult => {
    const reason = createRejection(code, state.stateVersion, stateHash);
    const attempt = createAttempt(
      session.attempts.length + 1,
      command,
      state.stateVersion,
      stateHash,
      { reasonCode: code },
    );
    return deepFreeze({
      accepted: false,
      reason,
      session: { ...session, attempts: [...session.attempts, attempt] },
    });
  };

  if (state.terminal) return reject('terminal_state');
  if (command.stateVersion !== state.stateVersion) {
    return reject('stale_version');
  }
  if (command.seat !== state.activeSeat) {
    return reject('wrong_seat');
  }

  const kind = actionKinds(state, command.seat).find(
    (candidate) => actionId(candidate, command.seat, state.stateVersion) === command.actionId,
  );
  if (kind === undefined) return reject('unknown_action');

  const receiptSequence = session.transcript.length + 1;
  // ponytail: the demo transcript is tiny; real match state will own the global event sequence.
  const firstEventSequence = session.transcript.reduce((count, receipt) => count + receipt.events.length, 0) + 1;
  const [nextState, outcome, randomDraws] = applyAction(state, command.seat, kind);
  const events = createEvents(command.actionId, receiptSequence, firstEventSequence, [outcome]);
  const receipt: DemoReceipt = createReceipt({
    actionId: command.actionId,
    events,
    nextStateVersion: nextState.stateVersion,
    postStateHash: hashDemoState(nextState),
    preStateHash: stateHash,
    randomDraws,
    receiptSequence,
    seat: command.seat,
    stateVersion: state.stateVersion,
  });
  const attempt = createAttempt(
    session.attempts.length + 1,
    command,
    state.stateVersion,
    stateHash,
    { receiptId: receipt.receiptId },
  );
  return deepFreeze({
    accepted: true,
    receipt,
    session: {
      attempts: [...session.attempts, attempt],
      manifest: session.manifest,
      state: nextState,
      transcript: [...session.transcript, receipt],
    },
  });
}

export function replayDemo(manifest: DemoManifest, actionIds: readonly string[]): DemoSession {
  let session = createDemoSession(manifest);
  for (const replayActionId of actionIds) {
    const result = stepDemo(session, {
      actionId: replayActionId,
      seat: session.state.activeSeat,
      stateVersion: session.state.stateVersion,
    });
    if (!result.accepted) throw new Error(`demo replay rejected action: ${result.reason.code}`);
    session = result.session;
  }
  return session;
}

export function verifyDemoReplay(
  manifest: DemoManifest,
  actionIds: readonly string[],
  expectedTranscript: readonly DemoReceipt[],
): boolean {
  try {
    return canonicalJson(replayDemo(manifest, actionIds).transcript) === canonicalJson(expectedTranscript);
  } catch {
    return false;
  }
}
