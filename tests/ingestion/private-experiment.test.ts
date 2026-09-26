import assert from 'node:assert/strict';
import test from 'node:test';

import { createPrivateExperiment } from '../../src/commands/create-private-experiment.ts';

test('private experiment rejects escaping output IDs before loading private inputs', async () => {
  for (const outputId of ['../outside', '/tmp/outside', 'nested/file', '.hidden', '']) {
    await assert.rejects(createPrivateExperiment(['--output-id', outputId]), /single path segment/);
  }
});
