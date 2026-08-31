import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  parseGameCheckpoint,
  resumeGameCheckpoint,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import { hashGameState } from '../../src/engine/game.ts';
import {
  runPrivateNoveltyGauntlet,
} from '../../src/commands/run-private-novelty-gauntlet.ts';
import { loadPrivateStarterCatalog } from '../../src/commands/run-private-game-check.ts';
import { runBounded } from '../helpers/bounded-process.ts';

const REPOSITORY_ROOT = resolve(import.meta.dirname, '..', '..');
const TEST_OUTPUT_ID = 'novelty-gauntlet-test';

test('private novelty gauntlet runs both actual lessons and seat swaps without leaking card data', async () => {
  let networkCalls = 0;
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (() => {
    networkCalls += 1;
    throw new Error('network access is forbidden in private novelty gauntlets');
  }) as typeof fetch;
  try {
    const lessons = (await loadPrivateStarterCatalog())
      .filter(({ id }) => id.endsWith('-lesson'));
    const first = await runPrivateNoveltyGauntlet(undefined, 1, TEST_OUTPUT_ID);
    const second = await runPrivateNoveltyGauntlet(undefined, 1, TEST_OUTPUT_ID);

    assert.equal(networkCalls, 0);
    assert.equal(
      canonicalJson(first.report as unknown as JsonValue),
      canonicalJson(second.report as unknown as JsonValue),
    );
    assert.deepEqual(first.report.jobs.map(({ lessonId, orientation, result }) => ({
      lessonId,
      orientation,
      seed: result.seed,
      status: result.status,
    })), [
      {
        lessonId: 'air-vs-earth-lesson',
        orientation: 'original',
        seed: lessons[0]!.manifest.seed,
        status: 'horizon',
      },
      {
        lessonId: 'air-vs-earth-lesson',
        orientation: 'swapped',
        seed: lessons[0]!.manifest.seed,
        status: 'horizon',
      },
      {
        lessonId: 'earth-vs-air-lesson',
        orientation: 'original',
        seed: 7_382,
        status: 'horizon',
      },
      {
        lessonId: 'earth-vs-air-lesson',
        orientation: 'swapped',
        seed: 7_382,
        status: 'horizon',
      },
    ]);
    assert.deepEqual(first.report.totals, {
      completed: 0,
      failed: 0,
      horizon: 4,
      jobs: 4,
      savedCheckpoints: 4,
    });
    assert.equal(first.report.jobs.every(({ result }) =>
      result.acceptedActionCount === 1 && result.replayVerified), true);

    const serialized = await readFile(first.outputPath, 'utf8');
    assert.equal(serialized, `${canonicalJson(first.report as unknown as JsonValue)}\n`);
    assert.doesNotMatch(
      serialized,
      /"(?:cards|decks|descriptor|expectedSessionHash|label|manifest|payload|requests|rulesText|session)":/u,
    );
    assert.doesNotMatch(serialized, /sorcery-game-checkpoint/u);
    for (const lesson of lessons) {
      assert.equal(serialized.includes(lesson.label), false);
      assert.equal(serialized.includes(lesson.manifest.authority.contentHash), false);
      assert.equal(serialized.includes(lesson.manifest.authority.revisionId), false);
      assert.equal(Object.keys(lesson.manifest.cards).some((cardId) => serialized.includes(cardId)), false);
      assert.equal(Object.values(lesson.cardNames).some((name) => serialized.includes(name)), false);
    }

    const relativeOutput = relative(REPOSITORY_ROOT, first.outputPath).replaceAll('\\', '/');
    assert.equal(
      relativeOutput,
      `.local/authority/reports/${lessons[0]!.manifest.authority.revisionId}/${TEST_OUTPUT_ID}.json`,
    );
    const ignored = await runBounded('git', ['check-ignore', '--quiet', relativeOutput], REPOSITORY_ROOT);
    assert.equal(ignored.code, 0, ignored.stderr);

    const checkpointDirectory = resolve(
      REPOSITORY_ROOT,
      '.local',
      'authority',
      'checkpoints',
      lessons[0]!.manifest.authority.revisionId,
      TEST_OUTPUT_ID,
    );
    const checkpointFiles = await readdir(checkpointDirectory);
    assert.equal(checkpointFiles.length >= first.report.totals.savedCheckpoints, true);
    for (const { result } of first.report.jobs) {
      assert.equal(result.status, 'horizon');
      if (result.status !== 'horizon') continue;
      const checkpointPath = resolve(
        checkpointDirectory,
        `${result.checkpointId.slice('sha256:'.length)}.json`,
      );
      const checkpointBytes = await readFile(checkpointPath, 'utf8');
      const checkpoint = parseGameCheckpoint(checkpointBytes);
      assert.equal(checkpoint.checkpointId, result.checkpointId);
      assert.equal(
        checkpointBytes,
        `${serializeGameCheckpoint(checkpoint)}\n`,
      );
      assert.equal(
        hashGameState(resumeGameCheckpoint(checkpoint).state),
        result.finalStateHash,
      );
      const relativeCheckpoint = relative(REPOSITORY_ROOT, checkpointPath).replaceAll('\\', '/');
      const checkpointIgnored = await runBounded(
        'git',
        ['check-ignore', '--quiet', relativeCheckpoint],
        REPOSITORY_ROOT,
      );
      assert.equal(checkpointIgnored.code, 0, checkpointIgnored.stderr);
    }
  } finally {
    globalThis.fetch = originalFetch;
  }
});
