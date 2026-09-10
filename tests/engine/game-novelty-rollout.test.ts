import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createSyntheticDemoManifest,
  selectDeterministicGameAction,
} from '../../src/commands/run-game-demo.ts';
import { type GameCheckpoint } from '../../src/engine/checkpoint.ts';
import { hashGameState } from '../../src/engine/game.ts';
import { runNoveltyRollout } from '../../src/simulator/novelty-rollout.ts';
import { SetupCtx, withSetup } from './rust-setup-session.ts';

test('coverage-guided rollout reports committed novelty and a resumable horizon checkpoint', async () => {
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
    const result = await runNoveltyRollout(root, {
      maxActions: 3,
      onCheckpoint(checkpoint) {
        checkpoints.set(checkpoint.checkpointId, checkpoint);
      },
    });

    assert.equal(result.status, 'horizon');
    if (result.status !== 'horizon') return;
    assert.equal(result.acceptedActionCount, 3);
    assert.equal(result.manifestId, root.manifest.manifestId);
    assert.equal(result.maxActions, 3);
    assert.equal(result.policyVersion, 'one-step-novelty-v1');
    assert.equal(result.replayVerified, true);
    assert.equal(result.schemaVersion, 1);
    assert.equal(result.seed, root.manifest.seed);
    assert.equal(result.coverage.offeredActionKinds.some(({ value }) => value === 'draw'), true);
    assert.equal(result.coverage.branchFactors.some(({ value }) => value === 2), true);
    assert.equal(result.coverage.committedActionKinds.some(({ value }) => value === 'draw'), true);
    assert.equal(result.coverage.committedEventTypes.length > 0, true);
    assert.equal(result.probed.actionKinds.length > 0, true);
    assert.equal(result.probed.eventTypes.length > 0, true);
    assert.deepEqual(
      result.frontier.map(({ signal }) => signal),
      [
        { kind: 'action-kind', value: 'summon-minion' },
        { kind: 'event-type', value: 'minion-summoned' },
      ],
    );
    assert.equal(result.frontier.every(({ checkpointId }) => checkpoints.has(checkpointId)), true);

    const checkpoint = checkpoints.get(result.checkpointId);
    assert.ok(checkpoint);
    assert.equal(
      hashGameState((await SetupCtx.resumeCheckpoint(checkpoint)).state),
      result.finalStateHash,
    );
    assert.equal('checkpoint' in result, false);
    assert.equal('manifest' in result, false);
    assert.equal('session' in result, false);
    assert.doesNotMatch(
      canonicalJson(result as unknown as JsonValue),
      /"(?:checkpoint|decks|descriptor|error|label|manifest|message|payload|session)":/u,
    );
    assert.equal(
      canonicalJson(await runNoveltyRollout(root, { maxActions: 3 }) as unknown as JsonValue),
      canonicalJson(result as unknown as JsonValue),
    );

    for (let count = 0; count < 500 && ctx.state.terminal.status === 'active'; count += 1) {
      const issued = await ctx.legalActions();
      await ctx.accept(selectDeterministicGameAction(ctx.session, issued));
    }
    assert.equal(ctx.state.terminal.status, 'finished');
    let terminalCheckpointCount = 0;
    const completed = await runNoveltyRollout(ctx.session, {
      maxActions: 0,
      onCheckpoint() {
        terminalCheckpointCount += 1;
      },
    });
    assert.equal(completed.status, 'completed');
    assert.equal(completed.acceptedActionCount, 0);
    assert.equal(completed.replayVerified, true);
    assert.equal(terminalCheckpointCount, 0);

    const checkpointFailure = await runNoveltyRollout(root, {
      maxActions: 0,
      onCheckpoint() {
        throw new Error('synthetic checkpoint sink failure');
      },
    });
    assert.equal(checkpointFailure.status, 'failed');
    if (checkpointFailure.status === 'failed') {
      assert.deepEqual(checkpointFailure.failure, { kind: 'checkpoint-failure' });
    }
  });
});
