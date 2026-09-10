import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

import { identityHash } from '../../src/authority/hash.ts';
import { runGameDemo, runGameRecord } from '../../src/commands/run-game-demo.ts';
import type { JsonValue } from '../../src/authority/canonical-json.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');

test('SIM-03 seed-31 record writes manifest, transcript, events, coverage, and outcome', () => {
  const report = runGameDemo(31);
  const record = runGameRecord(31);
  const events = record.eventJsonl.trimEnd() === ''
    ? []
    : record.eventJsonl.trimEnd().split('\n').map((line) => JSON.parse(line) as JsonValue);
  const transcriptEvents = record.transcript.flatMap((receipt) => {
    assert.equal(typeof receipt === 'object' && receipt !== null && !Array.isArray(receipt), true);
    const eventsField = (receipt as { events?: unknown }).events;
    assert.equal(Array.isArray(eventsField), true);
    return eventsField as JsonValue[];
  });

  assert.equal(record.schemaVersion, 1);
  assert.equal(record.classification, 'unranked_partial_rules_unverified_authority');
  assert.equal(record.replayVerified, true);
  assert.equal(record.acceptedActionCount, report.acceptedActionCount);
  assert.equal(record.fightCount, report.fightCount);
  assert.equal(record.turnCount, report.turnCount);
  assert.equal(record.finalStateHash, report.finalStateHash);
  assert.equal(record.transcriptHash, report.transcriptHash);
  assert.deepEqual(record.terminal, report.terminal);
  assert.equal(record.transcript.length, report.acceptedActionCount);
  assert.equal(
    (record.manifest as { manifestId?: string }).manifestId,
    record.manifestId,
  );
  assert.equal(events.length, transcriptEvents.length);
  assert.equal(identityHash(transcriptEvents as unknown as JsonValue), record.eventsHash);
  assert.equal(record.coverage.committedActionKinds.includes('summon-minion'), true);
  assert.equal(record.coverage.committedEventTypes.includes('fight-started'), true);
  assert.equal(record.coverage.offeredActionKinds.includes('end-turn'), true);
});

test('demo writes SIM-03 artifact files without changing compact stdout', () => {
  const dir = mkdtempSync(join(tmpdir(), 'sorcery-demo-artifacts-'));
  try {
    const compact = runGameDemo(31);
    const written = runGameDemo(31, dir);
    assert.deepEqual(written, compact);
    assert.deepEqual(readdirSync(dir).sort(), [
      'coverage.json',
      'events.jsonl',
      'manifest.json',
      'outcome.json',
      'transcript.json',
    ]);
    const outcome = JSON.parse(readFileSync(join(dir, 'outcome.json'), 'utf8')) as {
      finalStateHash: string;
      transcriptHash: string;
    };
    assert.equal(outcome.finalStateHash, compact.finalStateHash);
    assert.equal(outcome.transcriptHash, compact.transcriptHash);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('TEST-02 fresh processes emit byte-identical game records', () => {
  const run = (): Buffer => {
    const result = spawnSync('cargo', [
      'run', '--release', '--locked', '--quiet', '-p', 'sorcery-engine',
      '--bin', 'sorcery-engine', '--', 'record', '31',
    ], {
      cwd: REPOSITORY_ROOT,
      maxBuffer: 16 * 1_048_576,
    });
    assert.equal(result.status, 0, result.stderr.toString('utf8'));
    assert.equal(result.stderr.length, 0);
    return result.stdout;
  };

  const first = run();
  const second = run();
  assert.equal(first.compare(second), 0);
});
