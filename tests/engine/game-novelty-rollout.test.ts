import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createSyntheticDemoManifest,
  selectDeterministicGameAction,
} from '../../src/commands/run-game-demo.ts';
import {
  resumeGameCheckpoint,
  type GameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import {
  createGameSession,
  hashGameState,
  legalGameActions,
  stepGame,
  type GameLegalAction,
  type GameSession,
} from '../../src/engine/game.ts';
import { runNoveltyRollout } from '../../src/simulator/novelty-rollout.ts';

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

function southFirstDecision(): GameSession {
  let session = createGameSession(createSyntheticDemoManifest(31));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  return accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
}

test('coverage-guided rollout reports committed novelty and a resumable horizon checkpoint', () => {
  const root = southFirstDecision();
  const checkpoints = new Map<string, GameCheckpoint>();
  const result = runNoveltyRollout(root, {
    maxActions: 3,
    onCheckpoint(checkpoint) {
      checkpoints.set(checkpoint.checkpointId, checkpoint);
    },
  });

  assert.equal(result.status, 'horizon');
  if (result.status !== 'horizon') return;
  assert.equal(result.acceptedActionCount, 3);
  assert.equal(result.manifestId, root.manifest.manifestId);
  assert.equal(result.maxActions, 3);
  assert.equal(result.policyVersion, 'one-step-novelty-v1');
  assert.equal(result.replayVerified, true);
  assert.equal(result.schemaVersion, 1);
  assert.equal(result.seed, root.manifest.seed);
  assert.equal(result.coverage.offeredActionKinds.some(({ value }) => value === 'draw'), true);
  assert.equal(result.coverage.branchFactors.some(({ value }) => value === 2), true);
  assert.equal(result.coverage.committedActionKinds.some(({ value }) => value === 'draw'), true);
  assert.equal(result.coverage.committedEventTypes.length > 0, true);
  assert.equal(result.probed.actionKinds.length > 0, true);
  assert.equal(result.probed.eventTypes.length > 0, true);
  assert.deepEqual(
    result.frontier.map(({ signal }) => signal),
    [
      { kind: 'action-kind', value: 'summon-minion' },
      { kind: 'event-type', value: 'minion-summoned' },
    ],
  );
  assert.equal(result.frontier.every(({ checkpointId }) => checkpoints.has(checkpointId)), true);

  const checkpoint = checkpoints.get(result.checkpointId);
  assert.ok(checkpoint);
  assert.equal(hashGameState(resumeGameCheckpoint(checkpoint).state), result.finalStateHash);
  assert.equal('checkpoint' in result, false);
  assert.equal('manifest' in result, false);
  assert.equal('session' in result, false);
  assert.doesNotMatch(
    canonicalJson(result as unknown as JsonValue),
    /"(?:checkpoint|decks|descriptor|error|label|manifest|message|payload|session)":/u,
  );
  assert.equal(
    canonicalJson(runNoveltyRollout(root, { maxActions: 3 }) as unknown as JsonValue),
    canonicalJson(result as unknown as JsonValue),
  );

  let terminal = root;
  for (let count = 0; count < 500 && terminal.state.terminal.status === 'active'; count += 1) {
    terminal = accept(terminal, selectDeterministicGameAction(terminal));
  }
  assert.equal(terminal.state.terminal.status, 'finished');
  let terminalCheckpointCount = 0;
  const completed = runNoveltyRollout(terminal, {
    maxActions: 0,
    onCheckpoint() {
      terminalCheckpointCount += 1;
    },
  });
  assert.equal(completed.status, 'completed');
  assert.equal(completed.acceptedActionCount, 0);
  assert.equal(completed.replayVerified, true);
  assert.equal(terminalCheckpointCount, 0);

  const checkpointFailure = runNoveltyRollout(root, {
    maxActions: 0,
    onCheckpoint() {
      throw new Error('synthetic checkpoint sink failure');
    },
  });
  assert.equal(checkpointFailure.status, 'failed');
  if (checkpointFailure.status === 'failed') {
    assert.deepEqual(checkpointFailure.failure, { kind: 'checkpoint-failure' });
  }
});
