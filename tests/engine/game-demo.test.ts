import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import test from 'node:test';

import { runGameDemo } from '../../src/commands/run-game-demo.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');

test('RULE-01 deterministic agents complete a skeletal match from setup to deck-out', () => {
  const result = runGameDemo(23);
  assert.deepEqual(result, {
    acceptedActionCount: 138,
    classification: 'unranked_partial_rules',
    finalStateHash: 'sha256:b70fa842b6bff068c27a6d6b53879d23800c2c7f060ead9e82697c4888a62d1a',
    loser: 'south',
    reason: 'deck_empty',
    replayVerified: true,
    turnCount: 56,
    winner: 'north',
  });
});

test('TEST-02 fresh processes emit byte-identical skeletal match results', () => {
  const command = resolve(REPOSITORY_ROOT, 'src', 'commands', 'run-game-demo.ts');
  const run = (): Buffer => {
    const result = spawnSync(process.execPath, [command, '23'], {
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
  assert.equal(JSON.parse(first.toString('utf8')).replayVerified, true);
});
