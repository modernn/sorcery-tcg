import assert from 'node:assert/strict';
import test from 'node:test';

import { runGameSchedule } from '../../src/commands/run-game-schedule.ts';

test('SIM-04 seed-31 schedule runs both seat orientations', () => {
  const report = runGameSchedule([31], 2);
  assert.equal(report.status, 'completed');
  assert.equal(report.failurePolicy, 'abort');
  assert.equal(report.classification, 'unranked_partial_rules_unverified_authority');
  assert.deepEqual(report.plannedSeeds, [31]);
  assert.deepEqual(report.completedSeeds, [31]);
  assert.equal(report.gameCount, 2);
});
