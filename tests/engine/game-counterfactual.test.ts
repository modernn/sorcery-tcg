import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { hashGameState } from '../../src/engine/game.ts';
import { runCounterfactualRollouts } from '../../src/simulator/counterfactual.ts';
import { withSetup } from './rust-setup-session.ts';

test('counterfactual rollouts cover every root choice reproducibly without scoring horizons', async () => {
  await withSetup(createSyntheticDemoManifest(31), async (ctx) => {
    await ctx.seed31AfterNorthOpening();
    const session = ctx.session;
    const beforeHash = hashGameState(session.state);
    const beforeActions = await ctx.legalActions('south');
    assert.deepEqual(beforeActions.map(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone), ['atlas', 'spellbook']);

    const first = await runCounterfactualRollouts(session, 2);
    const second = await runCounterfactualRollouts(session, 2);
    assert.equal(canonicalJson(first as unknown as JsonValue), canonicalJson(second as unknown as JsonValue));
    assert.equal(first.status, 'complete');
    assert.equal(first.rootActionCount, 2);
    assert.deepEqual(first.branches.map(({ rootActionId }) => rootActionId),
      beforeActions.map(({ actionId }) => actionId));
    assert.equal(first.branches.every((branch) =>
      branch.outcome === 'unknown' && branch.reason === 'horizon' && branch.decisionCount === 3), true);
    assert.equal(first.recommendation, null);
    assert.equal(hashGameState(session.state), beforeHash);
    assert.equal(session.transcript.length, 5);

    let terminalRoot = session;
    for (let count = 0; count < 500; count += 1) {
      const result = await ctx.step(await ctx.selectPolicyAction());
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      if (result.session.state.terminal.status === 'finished') break;
      terminalRoot = result.session;
    }
    const terminal = await runCounterfactualRollouts(terminalRoot, 0);
    assert.equal(terminal.branches.length > 0, true);
    assert.equal(terminal.branches.every(({ outcome }) => outcome === 'terminal'), true);
    assert.notEqual(terminal.recommendation, null);
  });
});
