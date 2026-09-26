import assert from 'node:assert/strict';
import test from 'node:test';

import { createPrivateExperiment } from '../../src/commands/create-private-experiment.ts';

test('private experiment rejects escaping output IDs before loading private inputs', async () => {
  for (const outputId of ['../outside', '/tmp/outside', 'nested/file', '.hidden', '']) {
    await assert.rejects(createPrivateExperiment(['--output-id', outputId]), /single path segment/);
  }
});

test('private experiment modes are mutually exclusive before loading private inputs', async () => {
  for (const args of [
    ['--catalog', '--preset', 'air-starter'],
    ['--catalog', '--decks', 'missing.json'],
    ['--preset', 'air-starter', '--decks', 'missing.json'],
  ]) await assert.rejects(createPrivateExperiment(args), /choose only one/);
});
