import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { identityHash } from '../../src/authority/hash.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpoint,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import {
  createGameSession,
  legalGameActions,
  stepGame,
  verifyGameReplay,
  type GameLegalAction,
  type GameSession,
} from '../../src/engine/game.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';

function action(
  session: GameSession,
  predicate: (candidate: GameLegalAction) => boolean,
): GameLegalAction {
  const found = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  assert.ok(found, 'expected legal action');
  return found;
}

function accept(session: GameSession, candidate: GameLegalAction): GameSession {
  const result = stepGame(session, candidate);
  assert.equal(result.accepted, true);
  return result.session;
}

test('a canonical checkpoint restores and continues an exact game session', () => {
  let session = createGameSession(createSyntheticDemoManifest(0x1234_5678));
  const northKeep = action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0);
  session = accept(session, northKeep);
  const rejected = stepGame(session, northKeep);
  assert.equal(rejected.accepted, false);
  session = rejected.session;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));

  const checkpoint = createGameCheckpoint(session);
  const serialized = serializeGameCheckpoint(checkpoint);
  const restored = resumeGameCheckpoint(parseGameCheckpoint(serialized));
  assert.equal(canonicalJson(restored as unknown as JsonValue), canonicalJson(session as unknown as JsonValue));
  assert.equal(restored.attempts.length, 4);
  assert.equal(restored.attempts[1]?.outcome, 'rejected');
  assert.equal(legalGameActions(restored.state, restored.state.decisionSeat)
    .map(({ actionId }) => actionId).join(','), legalGameActions(session.state, session.state.decisionSeat)
    .map(({ actionId }) => actionId).join(','));
  assert.equal(verifyGameReplay(restored), true);

  const next = action(session, ({ descriptor }) => descriptor.kind === 'end-turn');
  const originalNext = stepGame(session, next);
  const restoredNext = stepGame(restored, next);
  assert.equal(originalNext.accepted, true);
  assert.equal(restoredNext.accepted, true);
  assert.equal(
    canonicalJson(originalNext.session as unknown as JsonValue),
    canonicalJson(restoredNext.session as unknown as JsonValue),
  );

  assert.throws(() => parseGameCheckpoint(serialized.replace(
    '"kind":"sorcery-game-checkpoint"',
    '"kind":"sorcery-game-checkpoint","kind":"changed"',
  )), /duplicate_key/);
  const changed = structuredClone(checkpoint) as unknown as Record<string, unknown>;
  const requests = changed.requests as Array<Record<string, unknown>>;
  requests[0]!.actionId = 'changed-action';
  delete changed.checkpointId;
  changed.checkpointId = identityHash(changed as JsonValue);
  const changedCheckpoint = parseGameCheckpoint(JSON.stringify(changed));
  assert.throws(() => resumeGameCheckpoint(changedCheckpoint), /session hash/);
});
