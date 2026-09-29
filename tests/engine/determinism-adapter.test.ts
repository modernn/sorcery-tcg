import assert from 'node:assert/strict';
import test from 'node:test';

import { createEngineState } from '../../src/engine/determinism.ts';

test('JavaScript state adapter freezes state records and rejects seeds outside uint32', () => {
  const initial = createEngineState(0);
  assert.equal(Object.isFrozen(initial), true);
  assert.equal(Object.isFrozen(initial.prng), true);

  for (const seed of [-1, 0.5, 0x1_0000_0000]) {
    assert.throws(() => createEngineState(seed), RangeError);
  }
});
