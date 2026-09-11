import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
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
  assert.deepEqual(summary.bySeat.north, { draws: 0, games: 2, losses: 2, wins: 0 });
  assert.deepEqual(summary.bySeat.south, { draws: 0, games: 2, losses: 0, wins: 2 });
  assert.equal(summary.byDeck.north.wins, 1);
  assert.equal(summary.byDeck.south.wins, 1);
  assert.equal(summary.seatEffect, -2);
  assert.deepEqual(summary.uncertainty, { firstPlayerScore: 0, scoreDenominator: 4 });
  assert.equal(summary.reliability.allReplayVerified, true);
  assert.equal(summary.reliability.replayVerifiedGames, 2);
  assert.equal(summary.reliability.replayFailedGames, 0);
  assert.deepEqual(summary.length, { totalActions: 436, totalFights: 11, totalTurns: 51 });
});

test('SIM-03 seed-31 schedule writes flat artifact directories without changing stdout', () => {
  const compact = runGameSchedule([31], 2);
  const dir = mkdtempSync(join(tmpdir(), 'sorcery-schedule-artifacts-'));
  try {
    const written = runGameSchedule([31], 2, dir);
    assert.deepEqual(written, compact);
    assert.deepEqual(readdirSync(dir).sort(), ['0', '1']);
    for (const jobIndex of ['0', '1']) {
      assert.deepEqual(readdirSync(join(dir, jobIndex)).sort(), [
        'coverage.json',
        'events.jsonl',
        'manifest.json',
        'outcome.json',
        'transcript.json',
      ]);
    }
    const firstOutcome = JSON.parse(readFileSync(join(dir, '0', 'outcome.json'), 'utf8')) as {
      replayVerified: boolean;
    };
    assert.equal(firstOutcome.replayVerified, true);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
