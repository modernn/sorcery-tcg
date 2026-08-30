import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { createSyntheticDemoManifest } from '../../src/commands/run-game-demo.ts';
import { runTwoDeckGauntlet } from '../../src/simulator/gauntlet.ts';

test('two decks swap seats and aggregate identically across workers', async () => {
  const base = createSyntheticDemoManifest(31);
  const input = {
    authority: base.authority,
    cards: base.cards,
    decks: [
      { deck: base.decks.north, id: 'deck-a' },
      { deck: base.decks.south, id: 'deck-b' },
    ],
    seeds: [31],
  } as const;
  const oneWorker = await runTwoDeckGauntlet(input, 1);
  const twoWorkers = await runTwoDeckGauntlet(input, 2);

  assert.equal(
    canonicalJson(oneWorker as unknown as JsonValue),
    canonicalJson(twoWorkers as unknown as JsonValue),
  );
  assert.deepEqual(oneWorker.games.map(({ northDeckId, seed, southDeckId }) =>
    ({ northDeckId, seed, southDeckId })), [
    { northDeckId: 'deck-a', seed: 31, southDeckId: 'deck-b' },
    { northDeckId: 'deck-b', seed: 31, southDeckId: 'deck-a' },
  ]);
  assert.equal(oneWorker.gameCount, 2);
  assert.deepEqual(oneWorker.bySeat, {
    north: { draws: 0, games: 2, losses: 2, wins: 0 },
    south: { draws: 0, games: 2, losses: 0, wins: 2 },
  });
  assert.deepEqual(oneWorker.byDeck, {
    'deck-a': {
      asNorth: { draws: 0, games: 1, losses: 1, wins: 0 },
      asSouth: { draws: 0, games: 1, losses: 0, wins: 1 },
      draws: 0,
      games: 2,
      losses: 1,
      wins: 1,
    },
    'deck-b': {
      asNorth: { draws: 0, games: 1, losses: 1, wins: 0 },
      asSouth: { draws: 0, games: 1, losses: 0, wins: 1 },
      draws: 0,
      games: 2,
      losses: 1,
      wins: 1,
    },
  });
  assert.equal(oneWorker.averageTurns,
    oneWorker.games.reduce((sum, game) => sum + game.report.turnCount, 0) / 2);
  assert.equal(oneWorker.classification, 'unranked_partial_rules');
  await assert.rejects(runTwoDeckGauntlet({
    ...input,
    decks: [{ ...input.decks[0], id: 'same' }, { ...input.decks[1], id: 'same' }],
  }, 1), /distinct nonempty/);
  await assert.rejects(runTwoDeckGauntlet({ ...input, seeds: [] }, 1), /1-128 seeds/);
});
