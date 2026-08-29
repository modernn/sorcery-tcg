import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createSyntheticDemoManifest,
  selectDeterministicGameAction,
} from '../../src/commands/run-game-demo.ts';
import {
  createGameSession,
  hashGameState,
  legalGameActions,
  stepGame,
  type GameLegalAction,
  type GameSession,
} from '../../src/engine/game.ts';
import { runCounterfactualRollouts } from '../../src/simulator/counterfactual.ts';

function accept(session: GameSession, candidate: GameLegalAction): GameSession {
  const result = stepGame(session, candidate);
  assert.equal(result.accepted, true);
  return result.session;
}

function action(
  session: GameSession,
  predicate: (candidate: GameLegalAction) => boolean,
): GameLegalAction {
  const found = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  assert.ok(found, 'expected legal action');
  return found;
}

test('counterfactual rollouts cover every root choice reproducibly without scoring horizons', () => {
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
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  const beforeHash = hashGameState(session.state);
  const beforeActions = legalGameActions(session.state, 'south');
  assert.deepEqual(beforeActions.map(({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone), ['atlas', 'spellbook']);

  const first = runCounterfactualRollouts(session, 2);
  const second = runCounterfactualRollouts(session, 2);
  assert.equal(canonicalJson(first as unknown as JsonValue), canonicalJson(second as unknown as JsonValue));
  assert.equal(first.status, 'complete');
  assert.equal(first.rootActionCount, 2);
  assert.deepEqual(first.branches.map(({ rootActionId }) => rootActionId),
    beforeActions.map(({ actionId }) => actionId));
  assert.equal(first.branches.every((branch) =>
    branch.outcome === 'unknown' && branch.reason === 'horizon' && branch.decisionCount === 3), true);
  assert.equal(first.recommendation, null);
  assert.equal(hashGameState(session.state), beforeHash);
  assert.equal(session.transcript.length, 5);

  let terminalRoot = session;
  for (let count = 0; count < 500; count += 1) {
    const result = stepGame(terminalRoot, selectDeterministicGameAction(terminalRoot));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    if (result.session.state.terminal.status === 'finished') break;
    terminalRoot = result.session;
  }
  const terminal = runCounterfactualRollouts(terminalRoot, 0);
  assert.equal(terminal.branches.length > 0, true);
  assert.equal(terminal.branches.every(({ outcome }) => outcome === 'terminal'), true);
  assert.notEqual(terminal.recommendation, null);
});
