import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import {
  createGameManifest,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameManifest,
} from '../../src/engine/game.ts';
import { withSetup } from './rust-setup-session.ts';

const SYNTHETIC_AUTHORITY_HASH =
  'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const;

function deck(prefix: string, atlasCount = 30, spellbookCount = 50): GameDeckSpec {
  return {
    atlas: Array.from({ length: atlasCount }, (_, index) => `${prefix}-site-${index + 1}`),
    avatar: `${prefix}-avatar`,
    spellbook: Array.from({ length: spellbookCount }, (_, index) => `${prefix}-spell-${index + 1}`),
  };
}

function manifest(seed: number, options: Readonly<{ north?: GameDeckSpec; south?: GameDeckSpec }> = {}): GameManifest {
  const decks = { north: options.north ?? deck('north'), south: options.south ?? deck('south') };
  const cards: Record<string, GameCardDefinition> = {};
  for (const playerDeck of Object.values(decks)) {
    cards[playerDeck.avatar] = {
      attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20,
    };
    for (const cardId of playerDeck.atlas) {
      cards[cardId] = {
        cardType: 'site', connectsBurrowedAllies: false, elements: ['earth'],
        genesisDrawSpellPerAdjacentSameCard: false,
      };
    }
    for (const cardId of playerDeck.spellbook) {
      cards[cardId] = {
        airborne: false, attack: 1, burrowing: false, cardType: 'minion',
        cannotAttackSites: false, cannotDefend: false, cannotDefendOrIntercept: false,
        charge: false, connectsTopBottom: false, deathriteDrawSite: false, defense: 1,
        gainsStealthAtEndOfTurn: false, gainsStealthAtEndOfTurnIfNoEnemiesNearby: false,
        genesisDrawSite: false, immobile: false, lethal: false, manaCost: 1,
        movesOnlyForward: false, movesOnlySideways: false, mustBeCastBurrowed: false,
        mustBeCastSubmerged: false, mustBeCastToWaterSite: false, ranged: false,
        shootsDragProjectile: false, stealth: false, strikesFirstWhileAttacking: false,
        strikesFirstWhileDefending: false, submerge: false, summonToAnySite: false,
        mustBeCastToOuterColumn: false, thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
        voidwalk: false, landbound: false, waterbound: false, ward: false,
      };
    }
  }
  return createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-setup-fixture-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed,
  });
}

// Retained original whole test ordinal 2; Rust-backed boundary assertions remain unchanged.
test('TEST-03 different opponent hidden cards cannot change an observation or legal actions', async () => {
  await withSetup(manifest(9), async (first) => {
    await withSetup(manifest(9, {
      north: {
        ...deck('north'),
        atlas: deck('north').atlas.map((_, index) => `alternate-site-${index + 1}`),
        spellbook: deck('north').spellbook.map((_, index) => `alternate-spell-${index + 1}`),
      },
    }), async (second) => {
      assert.notEqual(
        first.state.players.north.hand.atlas[0]?.cardId,
        second.state.players.north.hand.atlas[0]?.cardId,
      );
      assert.equal(canonicalJson(first.observe('south')), canonicalJson(second.observe('south')));
      assert.equal(
        canonicalJson(await first.legalActions('south')),
        canonicalJson(await second.legalActions('south')),
      );
    });
  });
});

// Retained original whole test ordinal 123; Rust-backed boundary assertions remain unchanged.
test('shared stale rejection leaves game state, PRNG, and accepted transcript unchanged', async () => {
  await withSetup(manifest(17), async (ctx) => {
    const command = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'mulligan'
        && descriptor.atlasOrder.length === 0
        && descriptor.spellbookOrder.length === 0);
    const accepted = await ctx.step(command);
    assert.equal(accepted.accepted, true);
    const before = canonicalJson(ctx.state);
    const beforeHash = ctx.stateHash();
    const stale = await ctx.step(command);

    assert.equal(stale.accepted, false);
    if (stale.accepted) return;
    assert.equal(stale.reason.code, 'stale_version');
    assert.equal(stale.reason.currentStateHash, beforeHash);
    assert.equal(canonicalJson(stale.session.state), before);
    assert.equal(stale.session.transcript.length, 1);
    assert.equal(stale.session.attempts.length, 2);
  });
});
