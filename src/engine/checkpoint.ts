import { canonicalJson, parseJsonWithDuplicateKeyCheck, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import { deepFreeze, type StateHash } from './contract.ts';
import {
  createGameManifest,
  createGameSession,
  stepGame,
  type GameActionRequest,
  type GameManifest,
  type GameSession,
} from './game.ts';
import { withRustSession } from './rust-session-helpers.ts';

export const GAME_CHECKPOINT_MAX_BYTES = 16 * 1024 * 1024;
const MAX_CHECKPOINT_REQUESTS = 1_000;
const HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;

// Authority-private: this reconstructs both players' hidden state. Hashes detect
// corruption; they do not authenticate checkpoint data supplied by an untrusted client.
export type GameCheckpoint = Readonly<{
  checkpointId: StateHash;
  expectedSessionHash: StateHash;
  kind: 'sorcery-game-checkpoint';
  manifest: GameManifest;
  requests: readonly GameActionRequest[];
  schemaVersion: 1;
}>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  return Object.keys(value).sort().join(',') === [...expected].sort().join(',');
}

function sessionHash(session: GameSession): StateHash {
  return identityHash({
    attempts: session.attempts,
    initialRandomDraws: session.initialRandomDraws,
    manifestId: session.manifest.manifestId,
    state: session.state,
    transcript: session.transcript,
  } as unknown as JsonValue);
}

function canonicalManifest(value: unknown): GameManifest {
  if (!isRecord(value)) throw new RangeError('checkpoint manifest must be an object');
  try {
    const manifest = createGameManifest({
      authority: value.authority as GameManifest['authority'],
      cards: value.cards as GameManifest['cards'],
      decks: value.decks as GameManifest['decks'],
      firstSeat: value.firstSeat as GameManifest['firstSeat'],
      seed: value.seed as number,
    });
    if (canonicalJson(manifest as unknown as JsonValue) !== canonicalJson(value as JsonValue)) {
      throw new RangeError('checkpoint manifest identity is invalid');
    }
    return manifest;
  } catch (error) {
    if (error instanceof RangeError && error.message === 'checkpoint manifest identity is invalid') {
      throw error;
    }
    throw new RangeError('checkpoint manifest is invalid');
  }
}

function validateCheckpoint(value: unknown): GameCheckpoint {
  if (!isRecord(value)
    || !exactKeys(value, [
      'checkpointId',
      'expectedSessionHash',
      'kind',
      'manifest',
      'requests',
      'schemaVersion',
    ])) {
    throw new RangeError('checkpoint must contain exactly the supported fields');
  }
  if (value.kind !== 'sorcery-game-checkpoint' || value.schemaVersion !== 1) {
    throw new RangeError('checkpoint kind or schema version is unsupported');
  }
  if (typeof value.checkpointId !== 'string' || !HASH_PATTERN.test(value.checkpointId)
    || typeof value.expectedSessionHash !== 'string' || !HASH_PATTERN.test(value.expectedSessionHash)) {
    throw new RangeError('checkpoint hashes are invalid');
  }
  if (!Array.isArray(value.requests) || value.requests.length > MAX_CHECKPOINT_REQUESTS) {
    throw new RangeError(`checkpoint requests must contain at most ${MAX_CHECKPOINT_REQUESTS} entries`);
  }
  const requests = value.requests.map((request, index): GameActionRequest => {
    if (!isRecord(request)
      || !exactKeys(request, ['actionId', 'seat', 'stateVersion'])
      || typeof request.actionId !== 'string'
      || request.actionId.length === 0
      || request.actionId.length > 256
      || request.seat !== 'north' && request.seat !== 'south'
      || !Number.isSafeInteger(request.stateVersion)
      || (request.stateVersion as number) < 0) {
      throw new RangeError(`checkpoint request ${index} is invalid`);
    }
    return deepFreeze({
      actionId: request.actionId,
      seat: request.seat,
      stateVersion: request.stateVersion as number,
    });
  });
  const manifest = canonicalManifest(value.manifest);
  const body = deepFreeze({
    expectedSessionHash: value.expectedSessionHash as StateHash,
    kind: 'sorcery-game-checkpoint' as const,
    manifest,
    requests,
    schemaVersion: 1 as const,
  });
  if (identityHash(body as unknown as JsonValue) !== value.checkpointId) {
    throw new RangeError('checkpoint identity is invalid');
  }
  return deepFreeze({ ...body, checkpointId: value.checkpointId as StateHash });
}

export function createGameCheckpoint(session: GameSession): GameCheckpoint {
  const body = deepFreeze({
    expectedSessionHash: sessionHash(session),
    kind: 'sorcery-game-checkpoint' as const,
    manifest: session.manifest,
    requests: session.attempts.map(({ request }) => request),
    schemaVersion: 1 as const,
  });
  return deepFreeze({
    ...body,
    checkpointId: identityHash(body as unknown as JsonValue),
  });
}

export function serializeGameCheckpoint(checkpoint: GameCheckpoint): string {
  return canonicalJson(validateCheckpoint(checkpoint) as unknown as JsonValue);
}

export function parseGameCheckpoint(text: string): GameCheckpoint {
  if (new TextEncoder().encode(text).length > GAME_CHECKPOINT_MAX_BYTES) {
    throw new RangeError(`checkpoint exceeds ${GAME_CHECKPOINT_MAX_BYTES} bytes`);
  }
  return validateCheckpoint(parseJsonWithDuplicateKeyCheck(text));
}

export async function resumeGameCheckpointAsync(checkpoint: GameCheckpoint): Promise<GameSession> {
  const validated = validateCheckpoint(checkpoint);
  return withRustSession(validated.manifest, async (handle) => {
    await handle.resume(validated as unknown as JsonValue);
    const session = handle.snapshot;
    if (sessionHash(session) !== validated.expectedSessionHash) {
      throw new RangeError('checkpoint session hash does not match reconstructed history');
    }
    return session;
  });
}

export function resumeGameCheckpoint(checkpoint: GameCheckpoint): GameSession {
  const validated = validateCheckpoint(checkpoint);
  let session = createGameSession(validated.manifest);
  for (const request of validated.requests) {
    session = stepGame(session, request).session;
  }
  if (sessionHash(session) !== validated.expectedSessionHash) {
    throw new RangeError('checkpoint session hash does not match reconstructed history');
  }
  return session;
}
