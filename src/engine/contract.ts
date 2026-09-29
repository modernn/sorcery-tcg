import type { JsonValue } from '../authority/canonical-json.ts';
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
