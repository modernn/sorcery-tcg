import assert from 'node:assert/strict';
import test from 'node:test';

import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';

test('game manifest ingestion accepts unsigned 32-bit seeds and rejects others', () => {
  assert.equal(createSyntheticDemoManifest(0).seed, 0);

  for (const seed of [-1, 0.5, 0x1_0000_0000]) {
    assert.throws(() => createSyntheticDemoManifest(seed), RangeError);
  }
});
