import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import test from 'node:test';

import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import type { GameManifest } from '../../src/engine/game.ts';
import { SetupCtx, withSetup } from './rust-setup-session.ts';

function sessionJsonPids(): readonly number[] {
  const output = execFileSync('tasklist', [
    '/FI', 'IMAGENAME eq session-json.exe',
    '/FO', 'CSV',
    '/NH',
  ], { encoding: 'utf8' });
  return output.split(/\r?\n/).flatMap((line) => {
    const match = /^"session-json.exe","(\d+)"/.exec(line);
    return match ? [Number(match[1])] : [];
  });
}

test('Rust setup adapter rejects without mutation, forks checkpoints, and replays', async () => {
  await withSetup(createSyntheticDemoManifest(31), async (ctx) => {
    await ctx.keep();
    if (ctx.state.phase === 'mulligan') await ctx.keep();
    const rootHash = await ctx.stateHash();
    const rootObservation = await ctx.observe('north');
    const southObservation = await ctx.observe('south');
    assert.equal(rootObservation.viewer, 'north');
    assert.equal(Array.isArray(rootObservation.players.north.hand.spellbook), true);
    assert.equal(typeof southObservation.players.north.hand.spellbook, 'number');
    assert.equal(typeof southObservation.players.north.hand.atlas, 'number');

    const root = await ctx.checkpoint();
    const actions = await ctx.legalActions(ctx.state.decisionSeat);
    const plays = actions.filter(({ descriptor }) => descriptor.kind === 'play-site');
    const first = plays[0];
    const second = plays.find((action) => action.actionId !== first?.actionId);
    assert.ok(first && second, 'expected two independent site plays from one checkpoint');

    const transcriptBeforeReject = ctx.session.transcript.length;
    const stale = await ctx.stepRequest({
      actionId: first.actionId,
      seat: first.seat,
      stateVersion: first.stateVersion + 1,
    });
    assert.equal(stale.accepted, false);
    if (stale.accepted) return;
    assert.equal(stale.reason.code, 'stale_version');
    assert.equal(await ctx.stateHash(), rootHash);
    assert.equal(ctx.session.transcript.length, transcriptBeforeReject);

    await ctx.accept(first);
    const firstHash = await ctx.stateHash();
    assert.notEqual(firstHash, rootHash);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(root.checkpoint);
    assert.equal(await ctx.stateHash(), rootHash);
    await ctx.accept(second);
    const secondHash = await ctx.stateHash();
    assert.notEqual(secondHash, firstHash);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(root.checkpoint);
    assert.equal(await ctx.stateHash(), rootHash);
    const replayed = await ctx.accept(first);
    assert.equal(await ctx.stateHash(), firstHash);
    assert.equal(replayed.state.phase, (await ctx.observe('north')).phase);
  });
});

test('failed setup closes the spawned session process', async () => {
  const before = new Set(sessionJsonPids());
  await assert.rejects(() => SetupCtx.open({} as GameManifest));
  const leaked = sessionJsonPids().filter((pid) => !before.has(pid));
  assert.deepEqual(leaked, []);
});
