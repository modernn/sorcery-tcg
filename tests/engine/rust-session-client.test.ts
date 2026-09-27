import assert from 'node:assert/strict';
import test from 'node:test';

import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { canonicalJson } from '../../src/authority/canonical-json.ts';
import { RustSessionClient } from '../../src/engine/rust-engine.ts';
import { withSetup } from './rust-setup-session.ts';

test('Rust session-json client creates, views, steps, and verifies a synthetic match', async () => {
  const client = await RustSessionClient.start();
  try {
    const manifest = createSyntheticDemoManifest(31);
    const created = await client.newSession(canonicalJson(manifest as never));
    assert.match(created.stateHash, /^sha256:[0-9a-f]{64}$/);
    const { view, stateHash } = await client.publicView('north');
    assert.equal(stateHash, created.stateHash);
    assert.equal((view as { viewer: string }).viewer, 'north');
    const southHand = (view as {
      players: { south: { hand: { atlas: number } } };
    }).players.south.hand.atlas;
    assert.equal(typeof southHand, 'number');
    const selected = await client.selectPolicyAction();
    assert.equal(selected.descriptor.kind, 'mulligan');
    assert.deepEqual(selected.descriptor.atlasOrder, []);
    assert.deepEqual(selected.descriptor.spellbookOrder, []);
    const actions = await client.legalActions('north');
    assert.ok(actions.some(({ actionId }) => actionId === selected.actionId));
    const novelty = await client.probeNovelty({
      committedActionKinds: [],
      committedEventTypes: [],
    });
    assert.equal(novelty.tooWide, false);
    assert.ok(novelty.selectedIndex < novelty.probes.length);
    assert.equal(
      novelty.probes.filter(({ selectedByFallback }) => selectedByFallback).length,
      1,
    );
    assert.ok(novelty.probes.some(({ actionId, selectedByFallback }) =>
      actionId === selected.actionId && selectedByFallback));
    assert.ok(novelty.probes.every(({ actionId }) =>
      actions.some((action) => action.actionId === actionId)));
    const stepped = await client.step({
      actionId: selected.actionId,
      seat: selected.seat,
      stateVersion: selected.stateVersion,
    });
    assert.equal(stepped.accepted, true);
    assert.equal(await client.verifyReplay(), true);
    const steps = await client.replaySteps();
    assert.equal(steps.stepCount, 1);
    assert.equal(steps.chained, true);
    assert.equal(steps.steps[0]?.index, 0);
    assert.equal(steps.steps[0]?.seat, 'north');
  } finally {
    await client.close();
  }
});

test('Rust session client defaults to v1 and explicitly opts into v2 policy behavior', async () => {
  const legacy = await RustSessionClient.start();
  const improved = await RustSessionClient.start();
  try {
    const manifestJson = canonicalJson(createSyntheticDemoManifest(31) as never);
    await legacy.newSession(manifestJson);
    await improved.newSession(manifestJson);

    for (let index = 0; index < 700; index += 1) {
      const legacyAction = await legacy.selectPolicyAction();
      const improvedAction = await improved.selectPolicyAction(2);
      if (legacyAction.actionId !== improvedAction.actionId) {
        assert.notEqual(legacyAction.descriptor.kind, 'draw-site');
        assert.equal(improvedAction.descriptor.kind, 'draw-site');
        return;
      }
      await legacy.step({
        actionId: legacyAction.actionId,
        seat: legacyAction.seat,
        stateVersion: legacyAction.stateVersion,
      });
      await improved.step({
        actionId: improvedAction.actionId,
        seat: improvedAction.seat,
        stateVersion: improvedAction.stateVersion,
      });
    }
    assert.fail('seed-31 did not reach a DrawSite policy divergence');
  } finally {
    await Promise.all([legacy.close(), improved.close()]);
  }
});

test('Rust session client forwards counterfactual workers without changing the default report', async () => {
  const client = await RustSessionClient.start();
  try {
    await client.newSession(canonicalJson(createSyntheticDemoManifest(31) as never));
    const serial = await client.runCounterfactual({ maxContinuationDecisions: 0 });
    const parallel = await client.runCounterfactual({
      maxContinuationDecisions: 0,
      workers: 2,
    });
    assert.deepEqual(parallel, serial);
    await assert.rejects(
      client.runCounterfactual({ maxContinuationDecisions: 0, workers: 0 }),
      /counterfactual workers must be 1-8/,
    );
  } finally {
    await client.close();
  }
});

test('SetupCtx forwards explicit v3 movement-progress policy behavior', async () => {
  await withSetup(createSyntheticDemoManifest(31), async (ctx) => {
    const selected = await ctx.selectPolicyAction(3);
    const legalActions = await ctx.legalActions(selected.seat);
    assert.ok(legalActions.some(({ actionId }) => actionId === selected.actionId));
  });
});
