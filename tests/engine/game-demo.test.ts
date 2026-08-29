import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import test from 'node:test';

import {
  createSyntheticDemoManifest,
  runGameDemo,
  selectDeterministicGameAction,
} from '../../src/commands/run-game-demo.ts';
import { createGameSession, legalGameActions, stepGame } from '../../src/engine/game.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');

test('deterministic agents attack the opposing Avatar without walking away', () => {
  let session = createGameSession(createSyntheticDemoManifest(31));
  for (let count = 0; count < 500; count += 1) {
    const actions = legalGameActions(session.state, session.state.decisionSeat);
    const enemyCell = session.state.players[
      session.state.decisionSeat === 'north' ? 'south' : 'north'
    ].avatar.location;
    const attack = actions.find(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.path.length === 1
        && descriptor.to.cell === enemyCell
        && descriptor.to.region === 'surface');
    const buildsFirst = actions.some(({ descriptor }) =>
      descriptor.kind === 'play-site'
        || descriptor.kind === 'summon-minion'
        || (descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
    if (attack && !buildsFirst) {
      const selected = selectDeterministicGameAction(session).descriptor;
      assert.equal(selected.kind, 'move-and-attack');
      if (selected.kind !== 'move-and-attack') return;
      assert.equal(selected.path.length, 1);
      assert.equal(selected.to.cell, enemyCell);
      assert.equal(selected.to.region, 'surface');
      return;
    }
    const result = stepGame(session, selectDeterministicGameAction(session));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    session = result.session;
  }
  assert.fail('deterministic match never reached an in-place Avatar attack');
});

test('RULE-01 deterministic agents move, fight, and complete a match', () => {
  const result = runGameDemo(31);
  assert.deepEqual(result, {
    acceptedActionCount: 230,
    classification: 'unranked_partial_rules',
    finalStateHash: 'sha256:be86c59b046db97838faec73c34ccc8dd8b9d56587c04a3cd335be6c588ccc65',
    fightCount: 6,
    replayVerified: true,
    terminal: {
      loser: 'north',
      reason: 'avatar_defeated',
      status: 'finished',
      winner: 'south',
    },
    transcriptHash: 'sha256:fbdad70e092de2166ee9d853bae9300d45e9921c33a88865cb94147f4cd2ad47',
    turnCount: 27,
  });
});

test('TEST-02 fresh processes emit byte-identical combat match results', () => {
  const command = resolve(REPOSITORY_ROOT, 'src', 'commands', 'run-game-demo.ts');
  const run = (): Buffer => {
    const result = spawnSync(process.execPath, [command, '31'], {
      cwd: REPOSITORY_ROOT,
      maxBuffer: 1_048_576,
    });
    assert.equal(result.status, 0, result.stderr.toString('utf8'));
    assert.equal(result.stderr.length, 0);
    return result.stdout;
  };

  const first = run();
  const second = run();
  assert.deepEqual(first, second);
  const parsed = JSON.parse(first.toString('utf8'));
  assert.equal(parsed.replayVerified, true);
  assert.equal(parsed.terminal.reason, 'avatar_defeated');
});
