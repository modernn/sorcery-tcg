import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { type GameCheckpoint } from '../../src/engine/checkpoint.ts';
import { withRustSession } from '../../src/engine/rust-session-helpers.ts';
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
