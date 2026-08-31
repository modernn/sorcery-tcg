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
import {
  hashGameState,
  legalGameActions,
  stepGame,
} from '../../src/engine/game.ts';
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
    const first = await runPrivateNoveltyGauntlet(undefined, 10, TEST_OUTPUT_ID);
    const second = await runPrivateNoveltyGauntlet(undefined, 10, TEST_OUTPUT_ID);

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
      branchLimit: 32,
      completed: 0,
      failed: 0,
      frontierBranches: 8,
      frontierCompleted: 0,
      frontierFailed: 0,
      frontierHorizon: 8,
      frontierLimitReached: false,
      frontierMaxDepth: 2,
      frontierPending: 0,
      frontierPendingSignals: 0,
      frontierPrunedCovered: 28,
      horizon: 4,
      jobs: 4,
      savedCheckpoints: first.report.totals.savedCheckpoints,
    });
    assert.equal(first.report.jobs.every(({ result }) =>
      result.acceptedActionCount === 10 && result.replayVerified), true);
    assert.equal(
      first.report.jobs.reduce((total, { result }) => total + result.frontier.length, 0),
      20,
    );
    assert.equal(first.report.frontierBranches.length, 8);
    assert.equal(first.report.frontierBranches.every(({ result }) =>
      result.acceptedActionCount === 10 && result.replayVerified), true);
    assert.equal(first.report.frontierBranches.every((branch) =>
      branch.depth >= 1
        && branch.depth <= 2
        && branch.entryActionCount === 1
        && branch.novelSignalsAtDispatch.length > 0), true);
    assert.equal(first.report.frontierBranches.some((branch) => branch.depth === 2), true);
    assert.equal(first.report.frontierBranches.every((branch) =>
      branch.novelSignalsAtDispatch.length <= branch.signals.length), true);
    assert.deepEqual(first.report.frontierPending, []);
    assert.equal(first.report.policyVersion, 'signal-guided-bounded-frontier-v2');
    assert.equal(first.report.schemaVersion, 2);

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
    const reportedResults = [
      ...first.report.jobs.map(({ result }) => result),
      ...first.report.frontierBranches.map(({ result }) => result),
    ];
    const reportedCheckpointIds = new Set(reportedResults.flatMap((result) => [
      ...(result.status === 'horizon'
        ? [result.checkpointId]
        : result.status === 'failed' && 'checkpointId' in result.failure
          ? [result.failure.checkpointId]
          : []),
      ...result.frontier.map(({ checkpointId }) => checkpointId),
    ]).concat(first.report.frontierPending.map(({ checkpointId }) => checkpointId)));
    assert.equal(first.report.totals.savedCheckpoints >= reportedCheckpointIds.size, true);
    for (const checkpointId of reportedCheckpointIds) {
      const checkpointPath = resolve(
        checkpointDirectory,
        `${checkpointId.slice('sha256:'.length)}.json`,
      );
      const checkpointBytes = await readFile(checkpointPath, 'utf8');
      const checkpoint = parseGameCheckpoint(checkpointBytes);
      assert.equal(checkpoint.checkpointId, checkpointId);
      assert.equal(
        checkpointBytes,
        `${serializeGameCheckpoint(checkpoint)}\n`,
      );
      const relativeCheckpoint = relative(REPOSITORY_ROOT, checkpointPath).replaceAll('\\', '/');
      const checkpointIgnored = await runBounded(
        'git',
        ['check-ignore', '--quiet', relativeCheckpoint],
        REPOSITORY_ROOT,
      );
      assert.equal(checkpointIgnored.code, 0, checkpointIgnored.stderr);
    }
    for (const result of reportedResults) {
      if (result.status !== 'horizon') continue;
      const checkpointPath = resolve(
        checkpointDirectory,
        `${result.checkpointId.slice('sha256:'.length)}.json`,
      );
      assert.equal(
        hashGameState(resumeGameCheckpoint(parseGameCheckpoint(
          await readFile(checkpointPath, 'utf8'),
        )).state),
        result.finalStateHash,
      );
    }

    for (const branch of first.report.frontierBranches) {
      const parentResult = branch.parentBranchId === null
        ? first.report.jobs.find(({ jobId }) => jobId === branch.parentJobId)?.result
        : first.report.frontierBranches
          .find(({ branchId }) => branchId === branch.parentBranchId)?.result;
      assert.ok(parentResult);
      if (branch.parentBranchId === null) {
        assert.equal(branch.depth, 1);
      } else {
        const parentBranch = first.report.frontierBranches
          .find(({ branchId }) => branchId === branch.parentBranchId);
        assert.ok(parentBranch);
        assert.equal(branch.depth, parentBranch.depth + 1);
        assert.equal(branch.parentJobId, parentBranch.parentJobId);
      }
      const candidates = parentResult.frontier.filter(({ actionId, checkpointId }) =>
        actionId === branch.actionId && checkpointId === branch.checkpointId);
      assert.equal(candidates.length > 0, true);
      assert.deepEqual(candidates.map(({ signal }) => signal), branch.signals);
      assert.equal(candidates.every(({ actionKind }) => actionKind === branch.actionKind), true);
      assert.equal(branch.novelSignalsAtDispatch.every((signal) =>
        branch.signals.some((claimed) => canonicalJson(claimed as unknown as JsonValue)
          === canonicalJson(signal as unknown as JsonValue))), true);
      const candidate = candidates[0]!;
      const checkpointPath = resolve(
        checkpointDirectory,
        `${candidate.checkpointId.slice('sha256:'.length)}.json`,
      );
      const resumed = resumeGameCheckpoint(parseGameCheckpoint(
        await readFile(checkpointPath, 'utf8'),
      ));
      const issued = legalGameActions(resumed.state, resumed.state.decisionSeat)
        .filter(({ actionId }) => actionId === candidate.actionId);
      assert.equal(issued.length, 1);
      const applied = stepGame(resumed, issued[0]!);
      assert.equal(applied.accepted, true);
      if (!applied.accepted) continue;
      assert.equal(hashGameState(applied.session.state), candidate.predictedStateHash);
      assert.equal(branch.result.initialStateHash, candidate.predictedStateHash);
      assert.deepEqual(
        [...new Set(applied.receipt.events.map(({ type }) => type))].sort(),
        branch.entryEventTypes,
      );
    }
  } finally {
    globalThis.fetch = originalFetch;
  }
});
