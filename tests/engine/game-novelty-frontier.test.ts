import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { type GameCheckpoint } from '../../src/engine/checkpoint.ts';
import {
  runNoveltyFromForcedCheckpoint,
  runNoveltyFrontierSearch,
  runNoveltyRollout,
} from '../../src/simulator/novelty-rollout.ts';
import { withSetup } from './rust-setup-session.ts';

test('frontier search forces the unchosen seed-31 signals from captured checkpoints', async () => {
  await withSetup(createSyntheticDemoManifest(31), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    const root = ctx.session;
    const checkpoints = new Map<string, GameCheckpoint>();
    const first = await runNoveltyFrontierSearch(root, {
      maxActions: 3,
      maxBranches: 1,
      onCheckpoint(checkpoint) {
        checkpoints.set(checkpoint.checkpointId, checkpoint);
      },
    });

    assert.equal(first.policyVersion, 'signal-guided-bounded-frontier-v2');
    assert.equal(first.schemaVersion, 2);
    assert.equal(first.root.status, 'horizon');
    assert.equal(first.root.acceptedActionCount, 3);
    assert.deepEqual(
      first.root.frontier.map(({ signal }) => signal),
      [
        { kind: 'action-kind', value: 'summon-minion' },
        { kind: 'event-type', value: 'minion-summoned' },
      ],
    );
    assert.equal(first.frontierBranches.length, 1);
    const branch = first.frontierBranches[0]!;
    assert.equal(branch.parentJobId, 'root');
    assert.equal(branch.parentBranchId, null);
    assert.equal(branch.depth, 1);
    assert.equal(branch.entryActionCount, 1);
    assert.equal(branch.actionKind, 'summon-minion');
    assert.deepEqual(
      [...branch.signals],
      [
        { kind: 'action-kind', value: 'summon-minion' },
        { kind: 'event-type', value: 'minion-summoned' },
      ],
    );
    assert.equal(branch.result.initialStateHash, first.root.frontier[0]!.predictedStateHash);
    assert.equal(first.totals.frontierBranches, 1);
    assert.equal(first.totals.branchLimit, 1);
    assert.equal(first.totals.rootStatus, 'horizon');
    assert.equal(checkpoints.has(branch.checkpointId), true);
    assert.equal(
      first.frontierBranches.every(({ checkpointId }) => checkpoints.has(checkpointId)),
      true,
    );
    assert.equal(
      first.root.frontier.every(({ checkpointId }) => checkpoints.has(checkpointId)),
      true,
    );

    const second = await runNoveltyFrontierSearch(root, {
      maxActions: 3,
      maxBranches: 1,
    });
    assert.equal(
      canonicalJson(first as unknown as JsonValue),
      canonicalJson(second as unknown as JsonValue),
    );

    const parent = await runNoveltyRollout(root, { maxActions: 3 });
    assert.equal(
      canonicalJson(first.root as unknown as JsonValue),
      canonicalJson(parent as unknown as JsonValue),
    );
    const candidate = parent.frontier[0]!;
    const checkpoint = checkpoints.get(candidate.checkpointId);
    assert.ok(checkpoint);
    const manual = await runNoveltyFromForcedCheckpoint(checkpoint, {
      actionId: candidate.actionId,
      actionKind: candidate.actionKind,
      maxActions: 3,
      predictedEventTypes: candidate.predictedEventTypes,
      predictedStateHash: candidate.predictedStateHash,
    });
    assert.equal(
      canonicalJson(branch.result as unknown as JsonValue),
      canonicalJson(manual.result as unknown as JsonValue),
    );
    assert.deepEqual([...branch.entryEventTypes], [...manual.entry.eventTypes]);
  });
});
