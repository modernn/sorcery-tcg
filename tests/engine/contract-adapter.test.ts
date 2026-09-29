import assert from 'node:assert/strict';
import test from 'node:test';

import { deepFreeze } from '../../src/engine/contract.ts';

test('Rust session adapter recursively freezes parsed records', () => {
  const value = deepFreeze({ nested: { values: [1, 2, 3] } });
  assert.equal(Object.isFrozen(value), true);
  assert.equal(Object.isFrozen(value.nested), true);
  assert.equal(Object.isFrozen(value.nested.values), true);
});
