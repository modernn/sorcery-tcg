import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import test from 'node:test';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');

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
  assert.equal(first.compare(second), 0);
});
