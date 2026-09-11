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

test('SIM-05 seed-31 schedule reports integer W/D/L, seat effect, and eligibility', () => {
  const report = runGameSchedule([31], 2);
  const { summary } = report;
  assert.equal(summary.eligibility, 'unranked_partial_rules_unverified_authority');
  assert.equal(summary.games, 2);
  assert.equal(summary.bySeat.north.games, 2);
  assert.equal(summary.bySeat.south.games, 2);
  assert.equal(summary.byDeck.north.games, 2);
  assert.equal(summary.byDeck.south.games, 2);
  assert.equal(
    summary.seatEffect,
    summary.bySeat.north.wins - summary.bySeat.south.wins,
  );
  assert.equal(
    summary.uncertainty.firstPlayerScore,
    summary.bySeat.north.wins * 2 + summary.bySeat.north.draws,
  );
  assert.equal(summary.uncertainty.scoreDenominator, 4);
  assert.equal(summary.reliability.allReplayVerified, true);
  assert.equal(summary.reliability.replayVerifiedGames, 2);
  assert.equal(summary.reliability.replayFailedGames, 0);
  assert.ok(summary.length.totalActions > 0);
  assert.ok(summary.length.totalFights > 0);
  assert.ok(summary.length.totalTurns > 0);
});
