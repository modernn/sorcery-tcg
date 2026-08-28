import assert from 'node:assert/strict';
import test from 'node:test';

import { runPrivateGameCheck } from '../../src/commands/run-private-game-check.ts';

test('private actual-card decks complete a deterministic supported combat scenario', async () => {
  const result = await runPrivateGameCheck();
  assert.equal(result.classification, 'private-local_actual-cards_unranked-partial-rules');
  assert.equal(result.avatarSpellDrawn, true);
  assert.equal(result.charge.activatedOnSummon, true);
  assert.equal(result.genesis.siteDrawn, true);
  assert.equal(result.lethal.tougherMinionKilled, true);
  assert.equal(result.provider.affinityAdded, true);
  assert.equal(result.decks.north.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.decks.south.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.decks.north.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.decks.south.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.combat.northMinionDied, true);
  assert.equal(result.combat.southMinionDied, true);
  assert.equal(result.replayVerified, true);
});
