import assert from 'node:assert/strict';
import test from 'node:test';

import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { canonicalJson } from '../../src/authority/canonical-json.ts';
import { RustSessionClient } from '../../src/engine/rust-engine.ts';

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
    const actions = await client.legalActions('north');
    assert.ok(actions.length > 0);
    const keep = actions.find(({ descriptor }) =>
      descriptor.kind === 'mulligan'
        && Array.isArray(descriptor.atlasOrder)
        && descriptor.atlasOrder.length === 0
        && Array.isArray(descriptor.spellbookOrder)
        && descriptor.spellbookOrder.length === 0);
    assert.ok(keep);
    const stepped = await client.step({
      actionId: keep.actionId,
      seat: keep.seat,
      stateVersion: keep.stateVersion,
    });
    assert.equal(stepped.accepted, true);
    assert.equal(await client.verifyReplay(), true);
  } finally {
    await client.close();
  }
});
