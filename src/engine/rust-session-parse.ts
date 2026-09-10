import type { JsonValue } from '../authority/canonical-json.ts';
import { deepFreeze } from './contract.ts';
import type { GameLegalAction, GameManifest, GameSession } from './game.ts';
import type { RustLegalAction } from './rust-engine.ts';

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/** Parses one Rust `exportSession` payload into a typed session snapshot. */
export function parseExportedSession(exported: JsonValue, manifest: GameManifest): GameSession {
  if (!isRecord(exported)
    || !Array.isArray(exported.attempts)
    || !Array.isArray(exported.initialRandomDraws)
    || !Array.isArray(exported.transcript)
    || !isRecord(exported.state)) {
    throw new Error('Rust exportSession result did not match GameSession');
  }
  return deepFreeze({
    attempts: exported.attempts as GameSession['attempts'],
    initialRandomDraws: exported.initialRandomDraws as GameSession['initialRandomDraws'],
    manifest,
    state: exported.state as GameSession['state'],
    transcript: exported.transcript as GameSession['transcript'],
  });
}

/** Casts Rust boundary actions to the shared engine action type. */
export function asGameLegalActions(actions: readonly RustLegalAction[]): readonly GameLegalAction[] {
  return actions as readonly GameLegalAction[];
}
