import assert from 'node:assert/strict';
import test, { after } from 'node:test';

import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import {
  createGameSession,
  legalGameActions,
  observeGame,
  stepGame,
  verifyGameReplay,
} from '../../src/engine/game.ts';
import { shutdownRustLegalitySync } from '../../src/engine/rust-legality-sync.ts';
import { withRustSession } from '../../src/engine/rust-session-helpers.ts';

after(() => {
  shutdownRustLegalitySync();
});

test('TypeScript legality exports open, step, and verify through Rust session-json', () => {
  const session = createGameSession(createSyntheticDemoManifest(0x1234_5678));
  assert.equal(session.state.phase, 'mulligan');
  assert.equal(session.state.decisionSeat, 'north');
  const keep = legalGameActions(session.state, 'north').find(({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0);
  assert.ok(keep);
  const northKept = stepGame(session, keep);
  assert.equal(northKept.accepted, true);
  if (!northKept.accepted) return;
  const southKeep = legalGameActions(northKept.session.state, 'south').find(({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0);
  assert.ok(southKeep);
  const opened = stepGame(northKept.session, southKeep);
  assert.equal(opened.accepted, true);
  if (!opened.accepted) return;
  assert.equal(opened.session.state.phase, 'main');
  const northView = observeGame(opened.session.state, 'north');
  const southView = observeGame(opened.session.state, 'south');
  assert.equal(northView.viewer, 'north');
  assert.equal(southView.viewer, 'south');
  assert.equal(northView.phase, 'main');
  assert.equal(typeof northView.players.north.affinity.earth, 'number');
  assert.ok(Array.isArray(northView.players.north.hand.spellbook));
  assert.equal(typeof southView.players.north.hand.spellbook, 'number');
  assert.equal(verifyGameReplay(opened.session), true);

  const forged = stepGame(opened.session, {
    actionId: 'sha256:9999999999999999999999999999999999999999999999999999999999999999',
    seat: 'north',
    stateVersion: opened.session.state.stateVersion,
  });
  assert.equal(forged.accepted, false);
  if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');

  const sibling = stepGame(session, keep);
  assert.equal(sibling.accepted, true);
  if (!sibling.accepted) return;
  assert.equal(sibling.session.state.stateVersion, northKept.session.state.stateVersion);
  assert.equal(sibling.session.state.decisionSeat, northKept.session.state.decisionSeat);
});

test('async-exported sessions bind so observeGame uses Rust publicView', async () => {
  await withRustSession(createSyntheticDemoManifest(0x1234_5678), async (handle) => {
    const view = observeGame(handle.snapshot.state, 'south');
    assert.equal(view.viewer, 'south');
    assert.equal(view.phase, 'mulligan');
    assert.ok(Array.isArray(view.players.south.hand.spellbook));
    assert.equal(typeof view.players.north.hand.spellbook, 'number');
  });
});
