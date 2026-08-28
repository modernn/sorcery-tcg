import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';

export type EngineSeat = 'north' | 'south';
export type StateHash = ReturnType<typeof identityHash>;

export type EngineActionDescriptor = Readonly<{
  kind: string;
  [key: string]: JsonValue;
}>;

export type EngineActionRequest = Readonly<{
  actionId: string;
  seat: EngineSeat;
  stateVersion: number;
}>;

export type EngineLegalAction<Descriptor extends EngineActionDescriptor = EngineActionDescriptor> = Readonly<{
  actionId: string;
  descriptor: Descriptor;
  label: string;
  seat: EngineSeat;
  stateVersion: number;
}>;

export type EngineRejectionCode =
  | 'stale_version'
  | 'terminal_state'
  | 'unknown_action'
  | 'wrong_seat';

export type EngineRejection = Readonly<{
  code: EngineRejectionCode;
  currentStateHash: StateHash;
  currentStateVersion: number;
  message: string;
}>;

export type EngineAttempt = Readonly<{
  attemptSequence: number;
  authoritativeStateHash: StateHash;
  authoritativeStateVersion: number;
  outcome: 'accepted' | 'rejected';
  reasonCode?: EngineRejectionCode;
  receiptId?: StateHash;
  request: EngineActionRequest;
}>;

export type EngineEvent = Readonly<{
  cause: Readonly<{
    actionId: string;
    receiptSequence: number;
  }>;
  eventId: StateHash;
  eventSequence: number;
  payload: JsonValue;
  type: string;
}>;

export type EngineRandomDraw = Readonly<{
  domain: JsonValue;
  drawSequence: number;
  postPrngStateHash: StateHash;
  prePrngStateHash: StateHash;
  purpose: string;
  result: JsonValue;
}>;

export type EngineReceipt = Readonly<{
  actionId: string;
  events: readonly EngineEvent[];
  nextStateVersion: number;
  postStateHash: StateHash;
  preStateHash: StateHash;
  randomDraws: readonly EngineRandomDraw[];
  receiptId: StateHash;
  receiptSequence: number;
  seat: EngineSeat;
  stateVersion: number;
}>;

const REJECTION_MESSAGES: Readonly<Record<EngineRejectionCode, string>> = Object.freeze({
  stale_version: 'That action belongs to an earlier game state.',
  terminal_state: 'The game is already over.',
  unknown_action: 'That action is not available.',
  wrong_seat: 'That action belongs to the other seat.',
});

export function deepFreeze<T>(value: T): T {
  if (value === null || typeof value !== 'object' || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

export function opaqueActionId(
  contract: string,
  seat: EngineSeat,
  stateVersion: number,
  descriptor: EngineActionDescriptor,
): StateHash {
  return identityHash({ contract, descriptor, seat, stateVersion });
}

export function orderLegalActions<Action extends EngineLegalAction>(actions: readonly Action[]): readonly Action[] {
  return deepFreeze([...actions].sort((left, right) => {
    const descriptorOrder = canonicalJson(left.descriptor).localeCompare(canonicalJson(right.descriptor));
    return descriptorOrder || left.actionId.localeCompare(right.actionId);
  }));
}

export function createRejection(
  code: EngineRejectionCode,
  currentStateVersion: number,
  currentStateHash: StateHash,
): EngineRejection {
  return deepFreeze({ code, currentStateHash, currentStateVersion, message: REJECTION_MESSAGES[code] });
}

export function createEvents(
  actionId: string,
  receiptSequence: number,
  firstEventSequence: number,
  outcomes: readonly Readonly<{ payload: JsonValue; type: string }>[] ,
): readonly EngineEvent[] {
  return deepFreeze(outcomes.map(({ payload, type }, index) => {
    const eventSequence = firstEventSequence + index;
    const cause = { actionId, receiptSequence } as const;
    return {
      cause,
      eventId: identityHash({ cause, eventSequence, payload, type }),
      eventSequence,
      payload,
      type,
    };
  }));
}

export function createReceipt(input: Readonly<Omit<EngineReceipt, 'receiptId'>>): EngineReceipt {
  const body = deepFreeze({ ...input });
  return deepFreeze({ ...body, receiptId: identityHash(body as unknown as JsonValue) });
}

export function createAttempt(
  attemptSequence: number,
  request: EngineActionRequest,
  authoritativeStateVersion: number,
  authoritativeStateHash: StateHash,
  outcome: Readonly<{ receiptId: StateHash } | { reasonCode: EngineRejectionCode }>,
): EngineAttempt {
  return deepFreeze({
    attemptSequence,
    authoritativeStateHash,
    authoritativeStateVersion,
    outcome: 'receiptId' in outcome ? 'accepted' : 'rejected',
    ...outcome,
    request,
  });
}
