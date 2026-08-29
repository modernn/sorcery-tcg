import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import test from 'node:test';

import { runGameDemo } from '../../src/commands/run-game-demo.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');

test('RULE-01 deterministic agents move, fight, and complete a match', () => {
  const result = runGameDemo(31);
  assert.deepEqual(result, {
    acceptedActionCount: 230,
    classification: 'unranked_partial_rules',
    finalStateHash: 'sha256:aafadc589f200dbd0e3a7d6a40f8913dc0bcadd075af03a28598ca86c6aa0a2a',
    fightCount: 6,
    replayVerified: true,
    terminal: {
      loser: 'north',
      reason: 'avatar_defeated',
      status: 'finished',
      winner: 'south',
    },
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
