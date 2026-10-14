import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { type GameCheckpoint } from '../../src/engine/checkpoint.ts';
import {
  parseExportedSession,
  withRustSession,
} from '../../src/engine/rust-session-helpers.ts';
import {
  runNoveltyFromForcedCheckpoint,
  runNoveltyRollout,
} from '../../src/simulator/novelty-rollout.ts';
import { withSetup } from './rust-setup-session.ts';

test('forced frontier action replays the probe prediction then starts novelty', async () => {
  await withSetup(createSyntheticDemoManifest(31), async (ctx) => {
    await ctx.seed31AfterNorthOpening();
    const root = ctx.session;
    const checkpoints = new Map<string, GameCheckpoint>();
    const parent = await runNoveltyRollout(root, {
      maxActions: 3,
      onCheckpoint(checkpoint) {
        checkpoints.set(checkpoint.checkpointId, checkpoint);
      },
    });

    assert.deepEqual(
      parent.frontier.map(({ signal }) => signal),
      [
        { kind: 'action-kind', value: 'summon-minion' },
        { kind: 'event-type', value: 'minion-summoned' },
      ],
    );
    const candidate = parent.frontier[0]!;
    const checkpoint = checkpoints.get(candidate.checkpointId);
    assert.ok(checkpoint);

    await withRustSession(checkpoint.manifest, async (handle) => {
      await handle.resume(checkpoint as unknown as JsonValue);
      const before = handle.snapshot;
      const beforeCheckpoint = await handle.checkpoint();
      const result = await handle.runNoveltyFromForcedAction({
        actionId: candidate.actionId,
        actionKind: candidate.actionKind,
        maxActions: 0,
        predictedEventTypes: candidate.predictedEventTypes,
        predictedStateHash: candidate.predictedStateHash,
      });
      assert.equal(result.entry.actionId, candidate.actionId);
      assert.equal(result.entry.stateHash, candidate.predictedStateHash);
      assert.ok(result.result && typeof result.result === 'object' && !Array.isArray(result.result));
      assert.equal(
        (result.result as { readonly [key: string]: JsonValue }).initialStateHash,
        candidate.predictedStateHash,
      );
      assert.notEqual(handle.snapshot, before);
      assert.ok(handle.snapshot.state.stateVersion > before.state.stateVersion);
      const exported = parseExportedSession(await handle.client.exportSession(), checkpoint.manifest);
      assert.equal(handle.snapshot.state.stateVersion, exported.state.stateVersion);
      assert.equal(handle.snapshot.state.decisionSeat, exported.state.decisionSeat);
      assert.notEqual((await handle.checkpoint()).checkpointId, beforeCheckpoint.checkpointId);
      assert.deepEqual(
        await handle.legalActions(),
        await handle.client.legalActions(handle.snapshot.state.decisionSeat),
      );
    });

    await withRustSession(checkpoint.manifest, async (handle) => {
      await handle.resume(checkpoint as unknown as JsonValue);
      const before = handle.snapshot;
      const beforeCheckpoint = await handle.checkpoint();
      await assert.rejects(
        handle.runNoveltyFromForcedAction({
          actionId: candidate.actionId,
          actionKind: candidate.actionKind,
          maxActions: 0,
          predictedEventTypes: candidate.predictedEventTypes,
          predictedStateHash: `sha256:${'0'.repeat(64)}`,
        }),
        /forced novelty prediction did not replay exactly/,
      );
      assert.equal(handle.snapshot, before);
      assert.equal((await handle.checkpoint()).checkpointId, beforeCheckpoint.checkpointId);
      assert.ok((await handle.legalActions()).length > 0);
    });

    const first = await runNoveltyFromForcedCheckpoint(checkpoint, {
      actionId: candidate.actionId,
      actionKind: candidate.actionKind,
      maxActions: 0,
      predictedEventTypes: candidate.predictedEventTypes,
      predictedStateHash: candidate.predictedStateHash,
    });
    assert.equal(first.entry.actionId, candidate.actionId);
    assert.equal(first.entry.actionKind, candidate.actionKind);
    assert.equal(first.entry.stateHash, candidate.predictedStateHash);
    assert.deepEqual([...first.entry.eventTypes], [...candidate.predictedEventTypes]);
    assert.equal(first.result.initialStateHash, candidate.predictedStateHash);
    assert.equal(first.result.status, 'horizon');
    assert.equal(first.result.acceptedActionCount, 0);
    assert.equal(first.result.replayVerified, true);

    const second = await runNoveltyFromForcedCheckpoint(checkpoint, {
      actionId: candidate.actionId,
      actionKind: candidate.actionKind,
      maxActions: 0,
      predictedEventTypes: candidate.predictedEventTypes,
      predictedStateHash: candidate.predictedStateHash,
    });
    assert.equal(
      canonicalJson(first as unknown as JsonValue),
      canonicalJson(second as unknown as JsonValue),
    );

    const manual = await withRustSession(checkpoint.manifest, async (handle) => {
      await handle.resume(checkpoint as unknown as JsonValue);
      const issued = (await handle.legalActions()).filter(({ actionId }) =>
        actionId === candidate.actionId);
      assert.equal(issued.length, 1);
      const stepped = await handle.stepAction(issued[0]!);
      assert.equal(stepped.accepted, true);
      if (!stepped.accepted) return;
      return runNoveltyRollout(stepped.session, { maxActions: 0 });
    });
    assert.equal(
      canonicalJson(first.result as unknown as JsonValue),
      canonicalJson(manual as unknown as JsonValue),
    );
  });
});
