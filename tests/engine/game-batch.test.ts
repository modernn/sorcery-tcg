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
  assert.equal(oneWorker.every(({ report }) =>
    report.classification === 'unranked_partial_rules_unverified_authority'), true);

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
  await assert.rejects(runGameBatch([manifests[0]!, invalid], 2), (error: unknown) => {
    assert.equal(error instanceof Error, true);
    if (!(error instanceof Error)) return false;
    assert.equal(error.message, 'game batch failed in Rust');
    assert.equal(error.message.includes(cardId), false);
    assert.equal(error.message.includes('futureUnsupportedMechanic'), false);
    return true;
  });
  const chainBody = {
    authority: source.authority,
    cards: Object.fromEntries(Object.entries(source.cards).map(([id, card]) => [
      id,
      id.startsWith('south-spell-')
        ? {
          cardType: 'magic',
          damageChainNearbyUnits: true,
          manaCost: 0,
          thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        }
        : card,
    ])),
    decks: source.decks,
    engineVersion: source.engineVersion,
    firstSeat: source.firstSeat,
    schemaVersion: source.schemaVersion,
    seed: source.seed,
  };
  const chain = {
    ...chainBody,
    manifestId: identityHash(chainBody as unknown as JsonValue),
  } as GameManifest;
  const chainOneWorker = await runGameBatch([chain], 1);
  const chainTwoWorkers = await runGameBatch([chain], 2);
  assert.equal(
    canonicalJson(chainOneWorker as unknown as JsonValue),
    canonicalJson(chainTwoWorkers as unknown as JsonValue),
  );
  assert.equal(chainOneWorker[0]?.report.replayVerified, true);
  await assert.rejects(runGameBatch(manifests, 9), /requestedWorkers/);
});
