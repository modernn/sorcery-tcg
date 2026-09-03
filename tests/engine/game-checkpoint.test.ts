import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { identityHash } from '../../src/authority/hash.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpointAsync,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import type { GameLegalAction } from '../../src/engine/game.ts';
import {
  RustGameSessionHandle,
} from '../../src/engine/rust-session-helpers.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';

async function action(
  handle: RustGameSessionHandle,
  predicate: (candidate: GameLegalAction) => boolean,
): Promise<GameLegalAction> {
  const found = (await handle.legalActions()).find(predicate);
  assert.ok(found, 'expected legal action');
  return found;
}

async function accept(
  handle: RustGameSessionHandle,
  candidate: GameLegalAction,
): Promise<void> {
  const result = await handle.stepAction(candidate);
  assert.equal(result.accepted, true);
}

test('a canonical checkpoint restores and continues an exact game session', async () => {
  const manifest = createSyntheticDemoManifest(0x1234_5678);
  const handle = await RustGameSessionHandle.open(manifest);
  try {
    const northKeep = await action(handle, ({ descriptor }) =>
      descriptor.kind === 'mulligan'
        && descriptor.atlasOrder.length === 0
        && descriptor.spellbookOrder.length === 0);
    await accept(handle, northKeep);
    const rejected = await handle.stepAction(northKeep);
    assert.equal(rejected.accepted, false);
    await accept(handle, await action(handle, ({ descriptor }) =>
      descriptor.kind === 'mulligan'
        && descriptor.atlasOrder.length === 0
        && descriptor.spellbookOrder.length === 0));
    await accept(handle, await action(handle, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4'));

    const session = handle.snapshot;
    const checkpoint = createGameCheckpoint(session);
    const serialized = serializeGameCheckpoint(checkpoint);
    const parsed = parseGameCheckpoint(serialized);
    const restored = await resumeGameCheckpointAsync(parsed);
    assert.equal(canonicalJson(restored as unknown as JsonValue), canonicalJson(session as unknown as JsonValue));
    assert.equal(restored.attempts.length, 4);
    assert.equal(restored.attempts[1]?.outcome, 'rejected');

    const restoredHandle = await RustGameSessionHandle.open(manifest);
    try {
      await restoredHandle.resume(parsed as unknown as JsonValue);
      assert.equal(
        (await restoredHandle.legalActions()).map(({ actionId }) => actionId).join(','),
        (await handle.legalActions()).map(({ actionId }) => actionId).join(','),
      );
      assert.equal(await restoredHandle.verifyReplay(), true);

      const next = await action(handle, ({ descriptor }) => descriptor.kind === 'end-turn');
      const originalNext = await handle.stepAction(next);
      const restoredNext = await restoredHandle.stepAction(next);
      assert.equal(originalNext.accepted, true);
      assert.equal(restoredNext.accepted, true);
      assert.equal(
        canonicalJson(originalNext.session as unknown as JsonValue),
        canonicalJson(restoredNext.session as unknown as JsonValue),
      );
    } finally {
      await restoredHandle.close();
    }

    assert.throws(() => parseGameCheckpoint(serialized.replace(
      '"kind":"sorcery-game-checkpoint"',
      '"kind":"sorcery-game-checkpoint","kind":"changed"',
    )), /duplicate_key/);
    const changed = structuredClone(checkpoint) as unknown as Record<string, unknown>;
    const requests = changed.requests as Array<Record<string, unknown>>;
    requests[0]!.actionId = 'changed-action';
    delete changed.checkpointId;
    changed.checkpointId = identityHash(changed as JsonValue);
    const changedCheckpoint = parseGameCheckpoint(JSON.stringify(changed));
    await assert.rejects(
      () => resumeGameCheckpointAsync(changedCheckpoint),
      /session hash/,
    );
  } finally {
    await handle.close();
  }
});
