import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { identityHash } from '../../src/authority/hash.ts';
import { runGameBatch } from '../../src/commands/run-game-batch.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import type { GameManifest } from '../../src/engine/game.ts';

test('one and many workers produce byte-identical ordered game reports', async () => {
  const manifests = [createSyntheticDemoManifest(31), createSyntheticDemoManifest(23)];
  const oneWorker = await runGameBatch(manifests, 1);
  const twoWorkers = await runGameBatch(manifests, 2);
  assert.equal(
    canonicalJson(oneWorker as unknown as JsonValue),
    canonicalJson(twoWorkers as unknown as JsonValue),
  );
  assert.deepEqual(oneWorker.map(({ jobIndex }) => jobIndex), [0, 1]);
  assert.deepEqual(oneWorker.map(({ manifestId }) => manifestId),
    manifests.map(({ manifestId }) => manifestId));
  assert.equal(oneWorker.every(({ report }) =>
    report.replayVerified && report.terminal.status === 'finished'), true);

  const source = manifests[1]!;
  const cardId = Object.keys(source.cards)[0]!;
  const forgedBody = {
    authority: source.authority,
    cards: {
      ...source.cards,
      [cardId]: { ...source.cards[cardId]!, futureUnsupportedMechanic: true },
    },
    decks: source.decks,
    engineVersion: source.engineVersion,
    firstSeat: source.firstSeat,
    schemaVersion: source.schemaVersion,
    seed: source.seed,
  };
  const invalid = {
    ...forgedBody,
    manifestId: identityHash(forgedBody as unknown as JsonValue),
  } as unknown as GameManifest;
  await assert.rejects(
    runGameBatch([manifests[0]!, invalid], 2),
    /game batch manifest 1 is invalid/,
  );
  await assert.rejects(runGameBatch(manifests, 9), /requestedWorkers/);
});
