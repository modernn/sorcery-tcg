import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import { opaqueActionId } from '../../src/engine/contract.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  observeGame,
  stepGame,
  verifyGameReplay,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
} from '../../src/engine/game.ts';
import {
  accept,
  action,
  cardsFor,
  deck,
  keep,
  manifest,
  SYNTHETIC_AUTHORITY_HASH,
  toNorthSecondMain,
  type SpellFacts,
} from './game-setup-helpers.ts';
import { SetupCtx, withSetup } from './rust-setup-session.ts';

test('RULE-01 setup shuffles two decks, deals split hidden hands, and places Avatars', async () => {
  await withSetup(manifest(7), async (ctx) => {
    const session = ctx.session;
    const north = session.state.players.north;
    const southView = observeGame(session.state, 'south');

    assert.equal(session.state.phase, 'mulligan');
    assert.equal(session.state.activeSeat, 'north');
    assert.equal(session.state.turnNumber, 0);
    assert.equal(north.hand.atlas.length, 3);
    assert.equal(north.hand.spellbook.length, 3);
    assert.equal(north.atlas.length, 27);
    assert.equal(north.spellbook.length, 47);
    assert.equal(north.avatar.location, 'C4');
    assert.equal(north.avatar.region, 'surface');
    assert.equal(north.avatar.life, 20);
    assert.deepEqual(north.cemetery, []);
    assert.equal(session.state.players.south.avatar.location, 'C1');
    assert.equal(session.state.decisionSeat, 'north');
    assert.ok(session.initialRandomDraws.length >= 156);
    assert.ok(session.initialRandomDraws.every(({ purpose }) => purpose.startsWith('setup_')));
    assert.equal(southView.players.north.hand.atlas, 3);
    assert.equal(southView.players.north.hand.spellbook, 3);
    assert.doesNotMatch(JSON.stringify(southView), /north-(?:site|spell)-/);
    const northActions = await ctx.legalActions('north');
    assert.equal(northActions.length, 76);
    assert.deepEqual(
      (await ctx.legalActions('north')).map(({ actionId }) => actionId),
      northActions.map(({ actionId }) => actionId),
    );
    assert.deepEqual(await ctx.legalActions('south'), []);
  });
});

test('RULE-06 the manifest accepts only exact deck-scoped supported card facts', () => {
  const decks = { north: deck('north'), south: deck('south') };
  const cards = cardsFor(decks);
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-catalog-validation-v1',
    },
    decks,
    firstSeat: 'north' as const,
    seed: 3,
  };
  const validManifest = createGameManifest({ ...input, cards });
  assert.throws(() => createGameSession({
    ...validManifest,
    firstSeat: 'east',
  } as unknown as GameManifest), /firstSeat is unsupported/);
  const firstSpell = decks.north.spellbook[0];
  assert.ok(firstSpell);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        futureUnsupportedMechanic: true,
      } as unknown as GameCardDefinition,
    },
  }), /futureUnsupportedMechanic is unsupported/);
  const conditionalStealthManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        gainsStealthAtEndOfTurnIfNoEnemiesNearby: true,
      } as unknown as GameCardDefinition,
    },
  });
  assert.equal(
    conditionalStealthManifest.cards[firstSpell]?.cardType === 'minion'
      && (conditionalStealthManifest.cards[firstSpell] as unknown as Readonly<Record<string, unknown>>)
        .gainsStealthAtEndOfTurnIfNoEnemiesNearby,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        gainsStealthAtEndOfTurnIfNoEnemiesNearby: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /gainsStealthAtEndOfTurnIfNoEnemiesNearby must be boolean/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        gainsStealthAtEndOfTurn: true,
        gainsStealthAtEndOfTurnIfNoEnemiesNearby: true,
      } as GameCardDefinition,
    },
  }), /simultaneous unconditional and conditional end-turn Stealth/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        gainsStealthAtEndOfTurnIfNoEnemiesNearby: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  }), /Waterbound with Ward or end-turn Stealth/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        thresholds: { air: 0, earth: 1, fire: 0, futureElement: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /thresholds.futureElement is unsupported/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      unused: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    },
  }), /exactly the deck-referenced definitions/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { cardType: 'magic' } as unknown as GameCardDefinition,
    },
  }), /exactly one supported Magic effect/);
  assert.doesNotThrow(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }));
  const randomLocationManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageRandomUnitAtLocation: 3,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(randomLocationManifest.cards[firstSpell], {
    cardType: 'magic',
    damageRandomUnitAtLocation: 3,
    manaCost: 2,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageRandomUnitAtLocation: 0,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /damageRandomUnitAtLocation/);
  const areaLocationManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageEachUnitAtLocationWithinTwoSteps: 3,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(areaLocationManifest.cards[firstSpell], {
    cardType: 'magic',
    damageEachUnitAtLocationWithinTwoSteps: 3,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageEachUnitAtLocationWithinTwoSteps: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /damageEachUnitAtLocationWithinTwoSteps/);
  const chargeManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantChargeToAllyThisTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(chargeManifest.cards[firstSpell], {
    cardType: 'magic',
    grantChargeToAllyThisTurn: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantChargeToAllyThisTurn: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantChargeToAllyThisTurn/);
  const lureManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        lureEnemyMinionOneStepCloser: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      },
    },
  });
  assert.deepEqual(lureManifest.cards[firstSpell], {
    cardType: 'magic',
    lureEnemyMinionOneStepCloser: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        lureEnemyMinionOneStepCloser: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      } as unknown as GameCardDefinition,
    },
  }), /lureEnemyMinionOneStepCloser/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 1,
        lureEnemyMinionOneStepCloser: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      },
    },
  }), /exactly one supported Magic effect/);
  const teleportManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        teleportAllyToTargetSite: true,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(teleportManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 2,
    teleportAllyToTargetSite: true,
    thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        teleportAllyToTargetSite: false,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /teleportAllyToTargetSite/);
  const rescueManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnMinionFromOwnCemetery: true,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(rescueManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 3,
    returnMinionFromOwnCemetery: true,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnMinionFromOwnCemetery: false,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnMinionFromOwnCemetery/);
  const freezeManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        disableTargetNearbyMinionUntilNextTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      },
    },
  });
  assert.deepEqual(freezeManifest.cards[firstSpell], {
    cardType: 'magic',
    disableTargetNearbyMinionUntilNextTurn: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        disableTargetNearbyMinionUntilNextTurn: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      } as unknown as GameCardDefinition,
    },
  }), /disableTargetNearbyMinionUntilNextTurn/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 1,
        manaCost: 3,
        returnMinionFromOwnCemetery: true,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  assert.doesNotThrow(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 7,
        manaCost: 2,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  }));
  const buryManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        burrowTargetMinionOrArtifact: true,
        cardType: 'magic',
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(buryManifest.cards[firstSpell], {
    burrowTargetMinionOrArtifact: true,
    cardType: 'magic',
    manaCost: 3,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        burrowTargetMinionOrArtifact: 'yes',
        cardType: 'magic',
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /burrowTargetMinionOrArtifact/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        burrowTargetMinionOrArtifact: true,
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageTargetUnit: 1,
        healController: 7,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /healController/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 1,
        targetNearby: 'yes',
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /targetNearby/);
  const lashManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 1,
        targetNearby: true,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
        untapTargetMinionAfterDamage: true,
      },
    },
  });
  assert.equal(
    lashManifest.cards[firstSpell]?.cardType === 'magic'
      && lashManifest.cards[firstSpell].untapTargetMinionAfterDamage,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
        untapTargetMinionAfterDamage: false,
      } as unknown as GameCardDefinition,
    },
  }), /untapTargetMinionAfterDamage must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
        untapTargetMinionAfterDamage: true,
      } as GameCardDefinition,
    },
  }), /untapTargetMinionAfterDamage requires damageTargetUnit/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, movementBonus: 3 } as unknown as GameCardDefinition,
    },
  }), /movementBonus/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, movesOnlyForward: 'yes' } as unknown as GameCardDefinition,
    },
  }), /movesOnlyForward/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        movesOnlyForward: true,
        movesOnlySideways: true,
      } as GameCardDefinition,
    },
  }), /only forward and only sideways/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, submerge: 'yes' } as unknown as GameCardDefinition,
    },
  }), /submerge/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        takesLessDamage: 2,
      } as unknown as GameCardDefinition,
    },
  }), /takesLessDamage/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        takesLessDamage: 1,
        ward: true,
      } as GameCardDefinition,
    },
  }), /competing damage prevention/);
  const sourcePreventionManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        preventsDamageFromUnitsWithPowerAtLeast: 4,
      } as GameCardDefinition,
    },
  });
  assert.equal(sourcePreventionManifest.cards[firstSpell]?.cardType === 'minion'
    && sourcePreventionManifest.cards[firstSpell]
      .preventsDamageFromUnitsWithPowerAtLeast, 4);
  for (const invalid of [0, 1.5, 101]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          preventsDamageFromUnitsWithPowerAtLeast: invalid,
        } as GameCardDefinition,
      },
    }), /preventsDamageFromUnitsWithPowerAtLeast/);
  }
  for (const competing of [{ takesLessDamage: 1 }, { ward: true }]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          ...competing,
          preventsDamageFromUnitsWithPowerAtLeast: 4,
        } as GameCardDefinition,
      },
    }), /competing damage prevention/);
  }
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, burrowing: 'yes' } as unknown as GameCardDefinition,
    },
  }), /burrowing/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, voidwalk: 'yes' } as unknown as GameCardDefinition,
    },
  }), /voidwalk/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        mustBeCastToOuterColumn: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /mustBeCastToOuterColumn/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        mustBeCastToWaterSite: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /mustBeCastToWaterSite/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        mustBeCastBurrowed: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /mustBeCastBurrowed/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        burrowing: false,
        mustBeCastBurrowed: true,
      } as GameCardDefinition,
    },
  }), /requires Burrowing/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        mustBeCastSubmerged: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /mustBeCastSubmerged/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        mustBeCastSubmerged: true,
        submerge: false,
      } as GameCardDefinition,
    },
  }), /requires Submerge/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        burrowing: true,
        mustBeCastBurrowed: true,
        mustBeCastSubmerged: true,
        submerge: true,
      } as GameCardDefinition,
    },
  }), /both burrowed and submerged/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, genesisDrawSpells: 'yes' } as unknown as GameCardDefinition,
    },
  }), /genesisDrawSpells/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSpell: true,
      } as unknown as GameCardDefinition,
    },
  }), /genesisDrawSpell is obsolete; use genesisDrawSpells/);
  for (const genesisDrawSpells of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          genesisDrawSpells,
        } as unknown as GameCardDefinition,
      },
    }), /genesisDrawSpells must be a safe integer between 1 and 200/);
  }
  for (const discardSpellToDamageRandomOtherUnitHere of [0, 1.5, 101]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          discardSpellToDamageRandomOtherUnitHere,
        } as unknown as GameCardDefinition,
      },
    }), /discardSpellToDamageRandomOtherUnitHere must be a safe integer between 1 and 100/);
  }
  const discardDamageManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        discardSpellToDamageRandomOtherUnitHere: 3,
      } as unknown as GameCardDefinition,
    },
  });
  assert.equal(
    discardDamageManifest.cards[firstSpell]?.cardType === 'minion'
      && (discardDamageManifest.cards[firstSpell] as unknown as Readonly<Record<string, unknown>>)
        .discardSpellToDamageRandomOtherUnitHere,
    3,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        discardSpellToDamageRandomOtherUnitHere: 3,
        occupiesSquareArea: 2,
      } as unknown as GameCardDefinition,
    },
  }), /occupiesSquareArea has an unsupported ability combination/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSite: true,
        genesisDrawSpells: 1,
      } as GameCardDefinition,
    },
  }), /simultaneous Genesis/);
  const bloodDemonManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, genesisLoseControllerLife: 2 } as GameCardDefinition,
    },
  });
  assert.equal(
    bloodDemonManifest.cards[firstSpell]?.cardType === 'minion'
      && bloodDemonManifest.cards[firstSpell].genesisLoseControllerLife,
    2,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisLoseControllerLife: 1,
      } as unknown as GameCardDefinition,
    },
  }), /genesisLoseControllerLife must be 2/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSite: true,
        genesisLoseControllerLife: 2,
      } as GameCardDefinition,
    },
  }), /simultaneous Genesis life loss and draw/);
  const grainSparrowManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, genesisHealController: 2 } as GameCardDefinition,
    },
  });
  assert.equal(
    grainSparrowManifest.cards[firstSpell]?.cardType === 'minion'
      && grainSparrowManifest.cards[firstSpell].genesisHealController,
    2,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisHealController: 1,
      } as unknown as GameCardDefinition,
    },
  }), /genesisHealController must be 2/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSite: true,
        genesisHealController: 2,
      } as GameCardDefinition,
    },
  }), /simultaneous Genesis healing/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisHealController: 2,
        waterbound: true,
      } as GameCardDefinition,
    },
  }), /Waterbound with Genesis/);
  const waterboundManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        submerge: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundManifest.cards[firstSpell]?.cardType === 'minion'
      && waterboundManifest.cards[firstSpell].submerge
      && waterboundManifest.cards[firstSpell].waterbound,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        waterbound: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /waterbound must be boolean/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        waterbound: true,
        ward: true,
      } as GameCardDefinition,
    },
  }), /Waterbound with Ward or end-turn Stealth/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        stealth: true,
        token: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  }), /Waterbound Stealth tokens are unsupported/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSpells: 1,
        waterbound: true,
      } as GameCardDefinition,
    },
  }), /Waterbound with Genesis/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, immobile: 'yes' } as unknown as GameCardDefinition,
    },
  }), /immobile/);
  for (const lanceCount of [0, 1.5, 4]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: { ...cards[firstSpell]!, lanceCount } as unknown as GameCardDefinition,
      },
    }), /lanceCount must be a safe integer between 1 and 3/);
  }
  const lanceManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, lanceCount: 3 } as GameCardDefinition,
    },
  });
  assert.equal(
    lanceManifest.cards[firstSpell]?.cardType === 'minion'
      && lanceManifest.cards[firstSpell].lanceCount,
    3,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, connectsTopBottom: 'yes' } as unknown as GameCardDefinition,
    },
  }), /connectsTopBottom/);
  const dragManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, shootsDragProjectile: true } as GameCardDefinition,
    },
  });
  const dragDefinition = dragManifest.cards[firstSpell];
  assert.equal(dragDefinition?.cardType === 'minion' && dragDefinition.shootsDragProjectile, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        shootsDragProjectile: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /shootsDragProjectile/);
  const firstSite = decks.north.atlas[0];
  assert.ok(firstSite);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSite]: {
        ...cards[firstSite]!,
        genesisGainMana: 1,
        genesisGainManaIfOnlyControlledCopy: 1,
      } as GameCardDefinition,
    },
  }), /simultaneous unconditional and conditional Genesis mana/);
  const shallowGraveManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSite]: { ...cards[firstSite]!, genesisDiscardTopSpells: 2 } as GameCardDefinition,
    },
  });
  assert.deepEqual(shallowGraveManifest.cards[firstSite], {
    cardType: 'site',
    elements: ['earth'],
    genesisDiscardTopSpells: 2,
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSite]: {
        ...cards[firstSite]!,
        genesisDiscardTopSpells: 1,
      } as unknown as GameCardDefinition,
    },
  }), /genesisDiscardTopSpells must be 2/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSite]: {
        ...cards[firstSite]!,
        genesisDiscardTopSpells: 2,
        genesisDrawSpellPerAdjacentSameCard: true,
      } as GameCardDefinition,
    },
  }), /simultaneous Genesis spell discard and draw/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSite]: {
        ...cards[firstSite]!,
        genesisDrawSpellPerAdjacentSameCard: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /genesisDrawSpellPerAdjacentSameCard/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSite]: {
        ...cards[firstSite]!,
        connectsBurrowedAllies: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /connectsBurrowedAllies/);
});

test('TEST-03 different opponent hidden cards cannot change an observation or legal actions', async () => {
  await withSetup(manifest(9), async (firstCtx) => {
    await withSetup(manifest(9, {
      north: {
        ...deck('north'),
        atlas: deck('north').atlas.map((_, index) => `alternate-site-${index + 1}`),
        spellbook: deck('north').spellbook.map((_, index) => `alternate-spell-${index + 1}`),
      },
    }), async (secondCtx) => {
      const first = firstCtx.session;
      const second = secondCtx.session;
      assert.notEqual(
        first.state.players.north.hand.atlas[0]?.cardId,
        second.state.players.north.hand.atlas[0]?.cardId,
      );
      assert.equal(
        canonicalJson(observeGame(first.state, 'south')),
        canonicalJson(observeGame(second.state, 'south')),
      );
      assert.equal(
        canonicalJson(await firstCtx.legalActions('south')),
        canonicalJson(await secondCtx.legalActions('south')),
      );
    });
  });
});

test('RULE-01 one mulligan returns at most three chosen cards to their deck bottoms and redraws', async () => {
  await withSetup(manifest(11), async (ctx) => {
    const initial = ctx.session;
    const returned = initial.state.players.north.hand.atlas[0];
    assert.ok(returned);
    const mulligan = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'mulligan'
        && descriptor.atlasOrder.length === 1
        && descriptor.atlasOrder[0] === returned.instanceId
        && descriptor.spellbookOrder.length === 0);
    const result = await ctx.step(mulligan);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const north = result.session.state.players.north;

    assert.equal(north.mulliganComplete, true);
    assert.equal(north.hand.atlas.length, 3);
    assert.equal(north.atlas.length, 27);
    assert.equal(north.atlas.at(-1)?.instanceId, returned.instanceId);
    assert.equal(north.hand.atlas.some(({ instanceId }) => instanceId === returned.instanceId), false);
    assert.deepEqual(result.receipt.events[0]?.payload, {
      atlasCount: 1,
      seat: 'north',
      spellbookCount: 0,
    });
    assert.equal(result.session.state.activeSeat, 'south');
  });
});

test('RULE-01 first player skips its draw, establishes a domain, then second player chooses a deck', async () => {
  await withSetup(manifest(13), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    let session = ctx.session;

    assert.equal(session.state.turnNumber, 1);
    assert.equal(session.state.activeSeat, 'north');
    assert.equal(session.state.phase, 'main');
    assert.equal(session.state.players.north.hand.atlas.length, 3);
    assert.equal(session.transcript.at(-1)?.events.at(-1)?.type, 'turn-started');
    assert.deepEqual(session.transcript.at(-1)?.events.at(-1)?.payload, {
      drawSkipped: true,
      seat: 'north',
      turnNumber: 1,
    });

    const site = await ctx.action(({ descriptor }) => descriptor.kind === 'play-site');
    const siteId = site.descriptor.kind === 'play-site' ? site.descriptor.cardInstanceId : '';
    session = await ctx.accept(site);
    assert.equal(session.state.realm.sites.C4?.instanceId, siteId);
    assert.equal(session.state.realm.sites.C4?.controller, 'north');
    assert.equal(session.state.players.north.avatar.tapped, true);
    assert.equal(session.state.players.north.domainEstablished, true);
    assert.equal(session.state.players.north.mana, 1);
    const afterSiteKinds = (await ctx.legalActions('north')).map(({ descriptor }) => descriptor.kind);
    assert.equal(afterSiteKinds.includes('play-site'), false);
    assert.equal(afterSiteKinds.includes('draw-site'), false);
    assert.equal(afterSiteKinds.includes('end-turn'), true);

    session = await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    assert.equal(session.state.turnNumber, 2);
    assert.equal(session.state.activeSeat, 'south');
    assert.equal(session.state.phase, 'draw');
    assert.deepEqual(
      (await ctx.legalActions('south')).map(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone),
      ['atlas', 'spellbook'],
    );

    session = await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
    assert.equal(session.state.phase, 'main');
    assert.equal(session.state.players.south.hand.atlas.length, 4);
    assert.deepEqual(session.transcript.at(-1)?.events[0]?.payload, { seat: 'south', zone: 'atlas' });
    assert.doesNotMatch(canonicalJson(session.transcript.at(-1)?.events[0]?.payload ?? null), /south-site-/);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02 sites expand through unoccupied orthogonal cells controlled by their player', async () => {
  await withSetup(manifest(23), async (ctx) => {
    await toNorthSecondMain(ctx);
    const northCard = ctx.state.players.north.hand.atlas[0];
    assert.ok(northCard);
    const northCells = (await ctx.legalActions('north'))
      .filter(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cardInstanceId === northCard.instanceId)
      .map(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell);
    assert.deepEqual(northCells, ['B4', 'C3', 'D4']);

    const playC3 = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === northCard.instanceId
        && descriptor.cell === 'C3');
    const beforeHand = ctx.state.players.north.hand.atlas.length;
    const result = await ctx.step(playC3);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.equal(ctx.state.realm.sites.C3?.instanceId, northCard.instanceId);
    assert.equal(ctx.state.realm.sites.C3?.controller, 'north');
    assert.equal(ctx.state.players.north.hand.atlas.length, beforeHand - 1);
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.players.north.mana, 2);
    assert.equal(result.receipt.events[0]?.type, 'site-played');
    const afterPlayKinds = (await ctx.legalActions('north')).map(({ descriptor }) => descriptor.kind);
    assert.equal(afterPlayKinds.includes('play-site'), false);
    assert.equal(afterPlayKinds.includes('draw-site'), false);
    assert.equal(afterPlayKinds.includes('end-turn'), true);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

    const expansion = [...new Set((await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'play-site')
      .map(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell))];
    assert.deepEqual(expansion, ['B3', 'B4', 'D3', 'D4']);
    assert.equal(ctx.state.players.north.avatar.tapped, false);
    assert.equal(ctx.state.players.north.mana, 2);
  });
});

test('RULE-02 a player with no sites recovers at the closest available cell', async () => {
  const base = manifest(267);
  await withSetup(base, async (previewCtx) => {
    const sourceCardId = previewCtx.state.players.north.hand.atlas[0]?.cardId;
    assert.ok(sourceCardId);
    const gameManifest = createGameManifest({
      ...base,
      cards: {
        ...base.cards,
        [sourceCardId]: {
          ...base.cards[sourceCardId]!,
          sacrificeToDestroyNearbySite: true,
        } as GameCardDefinition,
      },
    });
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const sourceCard = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === sourceCardId);
      assert.ok(sourceCard);
      const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
        await ctx.accept(await ctx.action(predicate));
      };

      await take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === sourceCard.instanceId);
      await take(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
        && descriptor.sourceSiteInstanceId === sourceCard.instanceId
        && descriptor.targetSiteInstanceId === sourceCard.instanceId);
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await take(({ descriptor }) => descriptor.kind === 'play-site');
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

      const recoveryCard = ctx.state.players.north.hand.atlas[0];
      const rubble = ctx.state.realm.sites.C4;
      assert.ok(recoveryCard);
      assert.ok(rubble && 'rubble' in rubble);
      assert.equal(ctx.state.players.north.domainEstablished, true);
      assert.equal(ctx.state.players.north.avatar.location, 'C4');
      assert.equal(Object.values(ctx.state.realm.sites)
        .some(({ controller }) => controller === 'north'), false);
      assert.deepEqual((await ctx.legalActions('north'))
        .flatMap(({ descriptor }) => descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === recoveryCard.instanceId ? [descriptor.cell] : []), ['C4']);

      // Tied-distance nearest-cell recovery (Avatar at C3) is proven in Rust
      // `zero_site_recovery_should_issue_every_nearest_cell_in_canonical_order`.
      const beforeForge = canonicalJson(ctx.state);
      const forged = await ctx.stepRequest({
        actionId: opaqueActionId('sorcery-core-v1', 'north', ctx.state.stateVersion, {
          cardId: recoveryCard.cardId,
          cardInstanceId: recoveryCard.instanceId,
          cell: 'A1',
          kind: 'play-site',
        }),
        seat: 'north',
        stateVersion: ctx.state.stateVersion,
      });
      assert.equal(forged.accepted, false);
      assert.equal(forged.reason.code, 'unknown_action');
      assert.equal(canonicalJson(forged.session.state), beforeForge);

      const handBefore = ctx.state.players.north.hand.atlas.length;
      const recovered = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === recoveryCard.instanceId
          && descriptor.cell === 'C4'));
      assert.equal(recovered.accepted, true);
      if (!recovered.accepted) return;
      assert.deepEqual(recovered.receipt.events.map(({ type }) => type), ['rubble-replaced', 'site-played']);
      assert.deepEqual(recovered.receipt.randomDraws, []);
      assert.equal(ctx.state.realm.sites.C4?.controller, 'north');
      assert.equal('rubble' in ctx.state.realm.sites.C4!, false);
      assert.equal(observeGame(ctx.state, 'north').players.north.affinity.earth, 1);
      assert.equal(ctx.state.players.north.avatar.tapped, true);
      assert.equal(ctx.state.players.north.hand.atlas.length, handBefore - 1);
      assert.equal(ctx.state.players.north.mana, 1);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-02 the Avatar may draw a private site instead of playing one', async () => {
  await withSetup(manifest(29), async (ctx) => {
    await toNorthSecondMain(ctx);
    const before = ctx.state.players.north;
    const drawn = before.atlas[0];
    assert.ok(drawn);
    const beforeAtlas = before.atlas.length;
    const beforeHand = before.hand.atlas.length;
    const result = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'draw-site'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;

    assert.equal(ctx.state.players.north.atlas.length, beforeAtlas - 1);
    assert.equal(ctx.state.players.north.hand.atlas.length, beforeHand + 1);
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.deepEqual(result.receipt.events[0]?.payload, { seat: 'north' });
    assert.equal(result.receipt.events[0]?.type, 'site-drawn');
    assert.equal(canonicalJson(result.receipt.events[0]?.payload ?? null).includes(drawn.cardId), false);
    assert.equal(canonicalJson(observeGame(ctx.state, 'south')).includes(drawn.cardId), false);
    const afterDrawKinds = (await ctx.legalActions('north')).map(({ descriptor }) => descriptor.kind);
    assert.equal(afterDrawKinds.includes('play-site'), false);
    assert.equal(afterDrawKinds.includes('draw-site'), false);
    assert.equal(afterDrawKinds.includes('end-turn'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a draw-spell Avatar pays its tap cost and keeps the drawn identity private', async () => {
  await withSetup(manifest(30, {
    avatar: { attack: 1, defense: 1, drawSpell: true, life: 20 },
  }), async (ctx) => {
    await toNorthSecondMain(ctx);
    const before = ctx.state.players.north;
    const drawn = before.spellbook[0];
    assert.ok(drawn);
    const beforeSpellbook = before.spellbook.length;
    const beforeHand = before.hand.spellbook.length;
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'draw-spell'));
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.players.north.spellbook.length, beforeSpellbook - 1);
    assert.equal(ctx.state.players.north.hand.spellbook.length, beforeHand + 1);
    assert.equal(ctx.session.transcript.at(-1)?.events[0]?.type, 'spell-drawn');
    assert.doesNotMatch(canonicalJson(ctx.session.transcript.at(-1)?.events[0]?.payload ?? null), /north-spell-/);
    assert.doesNotMatch(canonicalJson(observeGame(ctx.state, 'south')), new RegExp(drawn.cardId));
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02 drawing a site from an empty Atlas pays the tap cost and loses', async () => {
  await withSetup(manifest(31, {
    north: deck('north', 3, 4),
    south: deck('south', 3, 4),
  }), async (ctx) => {
    await toNorthSecondMain(ctx);
    assert.equal(ctx.state.players.north.atlas.length, 0);
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'draw-site'));

    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.deepEqual(ctx.state.terminal, {
      loser: 'north',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'south',
    });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02 forged spatial actions cannot mutate the game', async () => {
  await withSetup(manifest(37), async (ctx) => {
    await toNorthSecondMain(ctx);
    const beforeState = canonicalJson(ctx.state);
    const transcriptLength = ctx.session.transcript.length;
    const result = await ctx.stepRequest({
      actionId: 'sha256:2222222222222222222222222222222222222222222222222222222222222222',
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });

    assert.equal(result.accepted, false);
    assert.equal(result.reason.code, 'unknown_action');
    assert.equal(canonicalJson(result.session.state), beforeState);
    assert.equal(result.session.transcript.length, transcriptLength);
  });
});

// TODO(rust-cutover): synthetic state, needs a Rust-side proof. Reproduced directly against
// the Rust engine (no synthetic state, no checkpoint branching): after Sinkhole destroys a
// site, an unequipped Artifact left lying at that cell keeps region 'underwater' instead of
// following its square's minions to 'underground' the way the legacy TS engine does. This is
// a genuine Rust/TS parity gap in region tracking for un-equipped Artifacts on rubble
// conversion, not a migration-mechanics issue -- left on the legacy engine pending a Rust fix.
test('RULE-03 Sinkhole sacrifices sites into neutral Rubble and preserves relative subsurface', () => {
  const base = manifest(244);
  const preview = createGameSession(base);
  const northSites = preview.state.players.north.hand.atlas;
  const southSites = preview.state.players.south.hand.atlas;
  const southMinions = preview.state.players.south.hand.spellbook;
  const sourceCardId = northSites[1]?.cardId;
  const protectedCardId = northSites[0]?.cardId;
  const targetCardId = southSites[1]?.cardId;
  const replacementCardId = southSites[2]?.cardId;
  const drownedCardId = southMinions[0]?.cardId;
  const secondDrownedCardId = preview.state.players.south.spellbook[0]?.cardId;
  const survivorCardId = southMinions[1]?.cardId;
  const artifactCardId = southMinions[2]?.cardId;
  assert.ok(sourceCardId);
  assert.ok(protectedCardId);
  assert.ok(targetCardId);
  assert.ok(replacementCardId);
  assert.ok(drownedCardId);
  assert.ok(secondDrownedCardId);
  assert.ok(survivorCardId);
  assert.ok(artifactCardId);
  const cards: Record<string, GameCardDefinition> = { ...base.cards };
  cards[sourceCardId] = {
    ...cards[sourceCardId]!,
    sacrificeToDestroyNearbySite: true,
  } as GameCardDefinition;
  cards[protectedCardId] = {
    ...cards[protectedCardId]!,
    cannotBeMovedDestroyedOrModified: true,
  } as GameCardDefinition;
  cards[targetCardId] = { cardType: 'site', elements: ['water'] };
  cards[replacementCardId] = { cardType: 'site', elements: ['water'] };
  for (const cardId of [drownedCardId, secondDrownedCardId]) {
    cards[cardId] = {
      attack: 1,
      cardType: 'minion',
      deathriteDrawSite: true,
      defense: 1,
      manaCost: 0,
      submerge: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    };
  }
  cards[survivorCardId] = {
    attack: 1,
    burrowing: true,
    cardType: 'minion',
    defense: 1,
    manaCost: 0,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  cards[artifactCardId] = {
    cardType: 'artifact',
    grantsBearerPower: 2,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  assert.throws(() => createGameManifest({
    ...base,
    cards: {
      ...cards,
      [sourceCardId]: {
        ...cards[sourceCardId]!,
        sacrificeToDestroyNearbySite: false,
      } as unknown as GameCardDefinition,
    },
  }), /sacrificeToDestroyNearbySite/);
  assert.throws(() => createGameManifest({
    ...base,
    cards: {
      ...cards,
      [protectedCardId]: {
        ...cards[protectedCardId]!,
        cannotBeMovedDestroyedOrModified: false,
      } as unknown as GameCardDefinition,
    },
  }), /cannotBeMovedDestroyedOrModified must be true when defined/);
  const gameManifest = createGameManifest({
    authority: base.authority,
    cards,
    decks: base.decks,
    firstSeat: base.firstSeat,
    seed: base.seed,
  });
  let session = keep(keep(createGameSession(gameManifest)));
  const sourceCard = session.state.players.north.hand.atlas.find(({ cardId }) => cardId === sourceCardId);
  const protectedCard = session.state.players.north.hand.atlas.find(({ cardId }) =>
    cardId === protectedCardId);
  const targetCard = session.state.players.south.hand.atlas.find(({ cardId }) => cardId === targetCardId);
  const replacementCard = session.state.players.south.hand.atlas.find(({ cardId }) =>
    cardId === replacementCardId);
  const drownedCard = session.state.players.south.hand.spellbook.find(({ cardId }) => cardId === drownedCardId);
  const survivorCard = session.state.players.south.hand.spellbook.find(({ cardId }) => cardId === survivorCardId);
  const artifactCard = session.state.players.south.hand.spellbook.find(({ cardId }) =>
    cardId === artifactCardId);
  assert.ok(sourceCard);
  assert.ok(protectedCard);
  assert.ok(targetCard);
  assert.ok(replacementCard);
  assert.ok(drownedCard);
  assert.ok(survivorCard);
  assert.ok(artifactCard);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === protectedCard.instanceId
      && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId !== targetCard.instanceId
      && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === sourceCard.instanceId
      && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === targetCard.instanceId
      && descriptor.cell === 'C2'));
  const secondDrownedCard = session.state.players.south.hand.spellbook.find(({ cardId }) =>
    cardId === secondDrownedCardId);
  assert.ok(secondDrownedCard);
  for (const card of [drownedCard, secondDrownedCard]) {
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === card.instanceId
        && descriptor.cell === 'C2'
        && descriptor.region === 'underwater'));
  }
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === survivorCard.instanceId
      && descriptor.cell === 'C2'
      && descriptor.region === 'underwater'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-artifact'
      && descriptor.cardInstanceId === artifactCard.instanceId
      && descriptor.bearer?.instanceId === survivorCard.instanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'drop-artifacts'
      && descriptor.unit.instanceId === survivorCard.instanceId
      && descriptor.artifactInstanceIds[0] === artifactCard.instanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));

  const checkpoint = session;
  const actions = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-site-destruction'
      && descriptor.sourceSiteInstanceId === sourceCard.instanceId);
  assert.deepEqual(actions.flatMap(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
    ? [descriptor.targetCell]
    : []), ['C2', 'C3', 'C4']);
  const protectedActivation = actions.find(({ descriptor }) =>
    descriptor.kind === 'activate-site-destruction' && descriptor.targetCell === 'C4');
  assert.ok(protectedActivation);
  const protectedResult = stepGame(checkpoint, protectedActivation);
  assert.equal(protectedResult.accepted, true);
  if (!protectedResult.accepted) return;
  assert.deepEqual(protectedResult.receipt.events.map(({ type }) => type), [
    'site-sacrificed',
    'site-destruction-prevented',
    'rubble-created',
  ]);
  assert.deepEqual(protectedResult.session.state.realm.sites.C4, checkpoint.state.realm.sites.C4);
  assert.equal(protectedResult.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === sourceCard.instanceId), true);
  assert.equal(protectedResult.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === protectedCard.instanceId), false);
  assert.equal(verifyGameReplay(protectedResult.session), true);
  const activation = actions.find(({ descriptor }) =>
    descriptor.kind === 'activate-site-destruction' && descriptor.targetCell === 'C2');
  assert.ok(activation);
  const result = stepGame(checkpoint, activation);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
  assert.equal(session.state.stateVersion, checkpoint.state.stateVersion + 1);
  const stale = stepGame(session, activation);
  assert.equal(stale.accepted, false);
  assert.equal(stale.reason.code, 'stale_version');
  assert.deepEqual(result.receipt.events.map(({ type }) => type), [
    'site-sacrificed',
    'site-destroyed',
    'rubble-created',
    'rubble-created',
  ]);
  const drownedInstanceIds = [drownedCard.instanceId, secondDrownedCard.instanceId].sort();
  assert.equal(session.state.phase, 'deathrite-order');
  assert.equal(session.state.decisionSeat, 'south');
  assert.equal(drownedInstanceIds.every((instanceId) => !session.state.realm.units
    .some((unit) => unit.instanceId === instanceId)), true);
  assert.equal(drownedInstanceIds.every((instanceId) => !session.state.players.south.cemetery
    .some((card) => card.instanceId === instanceId)), true);
  const orderActions = legalGameActions(session.state, 'south').filter(({ descriptor }) =>
    descriptor.kind === 'order-deathrites');
  assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
    descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), drownedInstanceIds);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === sourceCard.instanceId), true);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === targetCard.instanceId), true);
  session = accept(session, orderActions[0]!);
  assert.equal(drownedInstanceIds.every((instanceId) => session.state.players.south.cemetery
    .some((card) => card.instanceId === instanceId)), true);
  const survivor = session.state.realm.units.find(({ instanceId }) =>
    instanceId === survivorCard.instanceId);
  assert.equal(survivor?.region, 'underground');
  const buriedArtifact = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === artifactCard.instanceId);
  assert.deepEqual(buriedArtifact, {
    cardId: artifactCard.cardId,
    instanceId: artifactCard.instanceId,
    location: 'C2',
    owner: 'south',
    region: 'underground',
    source: artifactCard.source,
  });
  assert.deepEqual(observeGame(session.state, 'north').realm.sites.C2, {
    cardId: 'rubble',
    controller: null,
    elements: [],
    instanceId: session.state.realm.sites.C2?.instanceId,
    rubble: true,
  });
  assert.equal(observeGame(session.state, 'north').players.north.affinity.earth, 1);
  assert.equal(observeGame(session.state, 'north').players.south.affinity.water, 0);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  assert.deepEqual(session.transcript.at(-1)?.events.map(({ type }) => type), [
    'rubble-replaced',
    'site-played',
  ]);
  assert.equal(session.state.realm.sites.C3?.controller, 'north');
  assert.equal('rubble' in session.state.realm.sites.C3!, false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const replacement = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === replacementCard.instanceId
      && descriptor.cell === 'C2'));
  assert.equal(replacement.accepted, true);
  if (!replacement.accepted) return;
  session = replacement.session;
  assert.deepEqual(replacement.receipt.events.map(({ type }) => type), [
    'rubble-replaced',
    'site-played',
  ]);
  assert.deepEqual(session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === artifactCard.instanceId), { ...buriedArtifact, region: 'underwater' });
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 a Spellcaster pays mana and summons a minion atop a controlled site', async () => {
  await withSetup(manifest(41), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    const before = ctx.state.players.north;
    const summons = (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'summon-minion');
    assert.equal(observeGame(ctx.state, 'north').players.north.affinity.earth, 1);
    assert.equal(summons.length, 3);
    assert.ok(summons.every(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cell === 'C4'
        && descriptor.casterInstanceId === before.avatar.card.instanceId
        && descriptor.manaCost === 1));

    const summon = summons[0];
    assert.ok(summon);
    const beforeHand = before.hand.spellbook.length;
    const result = await ctx.step(summon);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const unit = ctx.state.realm.units[0];
    assert.ok(unit);
    assert.equal(ctx.state.players.north.hand.spellbook.length, beforeHand - 1);
    assert.equal(ctx.state.players.north.mana, 0);
    assert.deepEqual({
      controller: unit.controller,
      damage: unit.damage,
      location: unit.location,
      summoningSickness: unit.summoningSickness,
      tapped: unit.tapped,
    }, {
      controller: 'north',
      damage: 0,
      location: 'C4',
      summoningSickness: true,
      tapped: false,
    });
    assert.equal(result.receipt.events[0]?.type, 'minion-summoned');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion'), false);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === unit.instanceId), false);

    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    assert.equal(ctx.state.realm.units[0]?.summoningSickness, false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 token Magic summons deterministically and tokens banish instead of entering a cemetery', async () => {
  const decks = {
    north: deck('token-north', 6, 6),
    south: deck('token-south', 6, 6),
  };
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const tokenCardId = 'synthetic-foot-soldier-token';
  const tokenDefinition = {
    attack: 1,
    cardType: 'minion',
    defense: 1,
    manaCost: 0,
    thresholds,
    token: true,
  } as const satisfies GameCardDefinition;
  const cards = cardsFor(decks, { manaCost: 0, thresholds });
  decks.north.spellbook.forEach((cardId) => {
    cards[cardId] = {
      cardType: 'magic',
      manaCost: 0,
      summonTokenToEachControlledSiteBorderingEnemySite: tokenCardId,
      thresholds,
    };
  });
  cards[tokenCardId] = tokenDefinition;
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-token-magic-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
    seed: 218,
  };
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards[tokenCardId], tokenDefinition);
  const missingTokenCards = { ...cards };
  delete missingTokenCards[tokenCardId];
  assert.throws(() => createGameManifest({ ...input, cards: missingTokenCards }),
    /must reference a token minion/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: { ...cards, 'unreferenced-token': tokenDefinition },
  }), /exactly the deck-referenced definitions/);
  const replacedSpellId = decks.north.spellbook[0]!;
  const deckTokenCards = { ...cards };
  delete deckTokenCards[replacedSpellId];
  assert.throws(() => createGameManifest({
    ...input,
    cards: deckTokenCards,
    decks: {
      ...decks,
      north: {
        ...decks.north,
        spellbook: [tokenCardId, ...decks.north.spellbook.slice(1)],
      },
    },
  }), /unsupported spell/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await ctx.accept(await ctx.action(predicate));
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const emptyCast = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target === undefined));
    assert.equal(emptyCast.accepted, true);
    if (!emptyCast.accepted) return;
    assert.deepEqual(emptyCast.receipt.events.map(({ type }) => type), ['magic-cast', 'magic-resolved']);
    assert.equal(ctx.state.realm.units.some(({ source }) => source === 'token'), false);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B2');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'B2');
    const attacker = ctx.state.realm.units.find(({ controller, location }) =>
      controller === 'south' && location === 'B2');
    assert.ok(attacker);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const tokenCast = await ctx.action(({ descriptor }) => descriptor.kind === 'cast-magic');
    assert.equal(tokenCast.descriptor.kind === 'cast-magic' && tokenCast.descriptor.target, undefined);
    const sourceInstanceId = tokenCast.descriptor.kind === 'cast-magic'
      ? tokenCast.descriptor.cardInstanceId
      : '';
    const summoned = await ctx.step(tokenCast);
    assert.equal(summoned.accepted, true);
    if (!summoned.accepted) return;
    const summonEvents = summoned.receipt.events.filter(({ payload, type }) =>
      type === 'minion-summoned'
        && (payload as { sourceInstanceId?: string }).sourceInstanceId === sourceInstanceId);
    assert.deepEqual(summonEvents.map(({ payload }) =>
      (payload as { cell: string }).cell), ['B3', 'C3']);
    const tokens = ctx.state.realm.units
      .filter(({ source }) => source === 'token')
      .sort((left, right) => left.location.localeCompare(right.location));
    assert.deepEqual(tokens.map(({ controller, damage, location, owner, region, source,
      summoningSickness, tapped }) => ({
      controller, damage, location, owner, region, source, summoningSickness, tapped,
    })), [
      {
        controller: 'north', damage: 0, location: 'B3', owner: 'north', region: 'surface',
        source: 'token', summoningSickness: true, tapped: false,
      },
      {
        controller: 'north', damage: 0, location: 'C3', owner: 'north', region: 'surface',
        source: 'token', summoningSickness: true, tapped: false,
      },
    ]);
    assert.equal(new Set(tokens.map(({ instanceId }) => instanceId)).size, 2);
    assert.deepEqual(observeGame(ctx.state, 'south').realm.units
      .filter(({ token }) => token)
      .map(({ attack, defense, instanceId }) => ({ attack, defense, instanceId })),
    tokens.map(({ instanceId }) => ({ attack: 1, defense: 1, instanceId })));
    assert.deepEqual(summonEvents.map(({ payload }) =>
      (payload as { instanceId: string }).instanceId), tokens.map(({ instanceId }) => instanceId));
    assert.deepEqual(summoned.receipt.randomDraws, []);
    const killed = tokens[0]!;
    const survivor = tokens[1]!;

    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attacker.instanceId
      && descriptor.to.cell === 'B3');
    await take(({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === killed.instanceId);
    const fight = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fight.accepted, true);
    if (!fight.accepted) return;
    const tokenExitEvents = fight.receipt.events.filter(({ payload, type }) =>
      (type === 'minion-died' || type === 'minion-banished')
        && (payload as { instanceId?: string }).instanceId === killed.instanceId);
    assert.deepEqual(tokenExitEvents.map(({ type }) => type), ['minion-died', 'minion-banished']);
    const diedIndex = fight.receipt.events.indexOf(tokenExitEvents[0]!);
    assert.equal(fight.receipt.events[diedIndex + 1], tokenExitEvents[1]);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === killed.instanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === killed.instanceId), false);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === killed.instanceId), false);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === survivor.instanceId)?.instanceId, survivor.instanceId);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Aramos Mercenaries may discard a deterministic random hand card instead of paying mana', async () => {
  const decks = {
    north: deck('aramos-north', 6, 8),
    south: deck('aramos-south', 6, 8),
  };
  const northSpell: SpellFacts = {
    attack: 3,
    defense: 3,
    discardRandomCardInsteadOfMana: true,
    manaCost: 3,
    thresholds: { air: 0, earth: 0, fire: 2, water: 0 },
  };
  const cards = cardsFor(decks, undefined, undefined, { elements: ['fire'] }, {
    north: northSpell,
    south: {
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  });
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-random-card-summon-cost-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
    seed: 417,
  };
  const aramosId = decks.north.spellbook[0]!;
  const gameManifest = createGameManifest(input);
  assert.equal(
    gameManifest.cards[aramosId]?.cardType === 'minion'
      && gameManifest.cards[aramosId].discardRandomCardInsteadOfMana,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [aramosId]: {
        ...cards[aramosId],
        discardRandomCardInsteadOfMana: false,
      } as unknown as GameCardDefinition,
    },
  }), /discardRandomCardInsteadOfMana must be true/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
    assert.equal(ctx.state.players.north.mana, 1);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion'), false);
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
    assert.equal(ctx.state.players.north.mana, 2);

    const alternateActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.paymentMode === 'random-card-discard');
    assert.ok(alternateActions.length > 0);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.paymentMode === undefined), false);
    assert.ok(alternateActions.every(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.manaCost === 0));
    const cast = alternateActions.find(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C3');
    assert.ok(cast);
    const castInstanceId = cast.descriptor.kind === 'summon-minion'
      ? cast.descriptor.cardInstanceId
      : '';
    const eligible = [
      ...ctx.state.players.north.hand.atlas.map((candidate) => ({ ...candidate, zone: 'atlas' as const })),
      ...ctx.state.players.north.hand.spellbook
        .filter(({ instanceId }) => instanceId !== castInstanceId)
        .map((candidate) => ({ ...candidate, zone: 'spellbook' as const })),
    ];
    assert.ok(eligible.length > 0);
    assert.deepEqual(new Set(eligible.map(({ zone }) => zone)), new Set(['atlas', 'spellbook']));
    const handBefore = ctx.state.players.north.hand;
    const southBefore = canonicalJson(observeGame(ctx.state, 'south'));
    assert.ok(eligible.every(({ cardId, instanceId }) =>
      !southBefore.includes(cardId) && !southBefore.includes(instanceId)));
    const cp = createGameCheckpoint(ctx.session);
    const first = await ctx.step(cast);
    assert.equal(first.accepted, true);
    await ctx.resume(cp);
    const second = await ctx.step(cast);
    assert.equal(second.accepted, true);
    assert.deepEqual(second.receipt.randomDraws, first.receipt.randomDraws);
    assert.deepEqual(second.receipt.events, first.receipt.events);
    assert.equal(hashGameState(second.session.state), hashGameState(first.session.state));

    const discard = first.receipt.events.find(({ type }) => type === 'card-discarded');
    assert.ok(discard);
    assert.equal(typeof discard.payload, 'object');
    assert.ok(discard.payload && !Array.isArray(discard.payload));
    const discardPayload = discard.payload as Readonly<Record<string, unknown>>;
    const discardedInstanceId = discardPayload.instanceId;
    const discardedCardId = discardPayload.cardId;
    const discardedZone = discardPayload.zone;
    assert.equal(discardPayload.sourceInstanceId, castInstanceId);
    assert.ok(eligible.some(({ cardId, instanceId, zone }) =>
      cardId === discardedCardId && instanceId === discardedInstanceId && zone === discardedZone));
    assert.deepEqual(first.receipt.events.map(({ type }) => type), [
      'card-discarded',
      'minion-summoned',
    ]);
    const summoned = first.receipt.events[1];
    assert.ok(summoned && typeof summoned.payload === 'object' && !Array.isArray(summoned.payload));
    assert.equal((summoned.payload as Readonly<Record<string, unknown>>).manaPaid, 0);
    assert.equal(ctx.state.players.north.mana, 2);
    assert.equal(ctx.state.players.north.hand.atlas.length,
      handBefore.atlas.length - (discardedZone === 'atlas' ? 1 : 0));
    assert.equal(ctx.state.players.north.hand.spellbook.length,
      handBefore.spellbook.length - 1 - (discardedZone === 'spellbook' ? 1 : 0));
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === discardedInstanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === castInstanceId), false);
    assert.ok(first.receipt.randomDraws.length > 0);
    assert.ok(first.receipt.randomDraws.every(({ domain, purpose }) => {
      if (typeof domain !== 'object' || domain === null || Array.isArray(domain)) return false;
      const drawDomain = domain as Readonly<Record<string, unknown>>;
      return purpose === 'summon_random_card_discard_cost'
        && drawDomain.kind === 'card_index_candidate'
        && drawDomain.exclusiveMaximum === eligible.length;
    }));
    const southAfter = canonicalJson(observeGame(ctx.state, 'south'));
    assert.ok(southAfter.includes(String(discardedCardId)));
    assert.ok(southAfter.includes(String(discardedInstanceId)));
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Gnarled Wendigo sacrifices local minions before paying its discounted summon', async () => {
  const decks = {
    north: deck('wendigo-north', 6, 8),
    south: deck('wendigo-south', 6, 8),
  };
  const cards = cardsFor(
    decks,
    { manaCost: 0, thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
    undefined,
    { elements: ['water'], genesisGainMana: 2 },
  );
  const wendigoId = decks.north.spellbook[0]!;
  const localMinionId = decks.north.spellbook[1]!;
  const submergedMinionId = decks.north.spellbook[2]!;
  const secondLocalMinionId = decks.north.spellbook[3]!;
  const enemyMinionId = decks.south.spellbook[0]!;
  cards[wendigoId] = {
    ...cards[wendigoId]!,
    attack: 5,
    defense: 5,
    manaCost: 6,
    sacrificeMinionAtSummoningLocationForManaDiscount: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
  } as GameCardDefinition;
  cards[submergedMinionId] = {
    ...cards[submergedMinionId]!,
    mustBeCastSubmerged: true,
    submerge: true,
  } as GameCardDefinition;
  cards[enemyMinionId] = {
    ...cards[enemyMinionId]!,
    summonToAnySite: true,
  } as GameCardDefinition;
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-summoning-location-sacrifice-discount-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [wendigoId]: {
        ...cards[wendigoId]!,
        sacrificeMinionAtSummoningLocationForManaDiscount: 1,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /sacrificeMinionAtSummoningLocationForManaDiscount must be 2/);

  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    // Seed search peeks opening hands via TS createGameSession (cheap); play path uses SetupCtx.
    const opening = createGameSession(candidate).state.players;
    if ([wendigoId, localMinionId, submergedMinionId].every((cardId) =>
      opening.north.hand.spellbook.some((card) => card.cardId === cardId))
      && opening.north.spellbook[0]?.cardId === secondLocalMinionId
      && opening.south.hand.spellbook.some((card) => card.cardId === enemyMinionId)) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  assert.equal(
    gameManifest.cards[wendigoId]?.cardType === 'minion'
      && gameManifest.cards[wendigoId].sacrificeMinionAtSummoningLocationForManaDiscount,
    2,
  );

  const readyToSummon = async (ctx: SetupCtx): Promise<void> => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await ctx.accept(await ctx.action(predicate));
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === localMinionId
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === submergedMinionId
      && descriptor.cell === 'C4'
      && descriptor.region === 'underwater');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === enemyMinionId
      && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === secondLocalMinionId
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
  };

  await withSetup(gameManifest, async (ctx) => {
    await readyToSummon(ctx);
    assert.equal(ctx.state.players.north.mana, 4);

    const local = ctx.state.realm.units.find(({ cardId }) => cardId === localMinionId);
    const secondLocal = ctx.state.realm.units.find(({ cardId }) => cardId === secondLocalMinionId);
    const submerged = ctx.state.realm.units.find(({ cardId }) => cardId === submergedMinionId);
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === enemyMinionId);
    assert.ok(local && secondLocal && submerged && enemy);
    const wendigoActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === wendigoId);
    assert.equal(wendigoActions.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.manaCost === 6
        && descriptor.sacrificedMinionInstanceIds === undefined), false);
    const discounted = wendigoActions.filter(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cell === 'C4'
        && descriptor.region === undefined
        && descriptor.manaCost === 4);
    assert.deepEqual(discounted.map(({ descriptor }) => descriptor.kind === 'summon-minion'
      ? descriptor.sacrificedMinionInstanceIds
      : undefined), [local.instanceId, secondLocal.instanceId]
      .sort()
      .map((instanceId) => [instanceId]));
    const doubleDiscounted = wendigoActions.filter(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cell === 'C4'
        && descriptor.region === undefined
        && descriptor.manaCost === 2);
    assert.equal(doubleDiscounted.length, 1);
    assert.deepEqual(
      doubleDiscounted[0]?.descriptor.kind === 'summon-minion'
        ? doubleDiscounted[0].descriptor.sacrificedMinionInstanceIds
        : undefined,
      [local.instanceId, secondLocal.instanceId].sort(),
    );
    const cast = discounted.find(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.sacrificedMinionInstanceIds?.length === 1
      && descriptor.sacrificedMinionInstanceIds[0] === local.instanceId);
    assert.ok(cast);
    assert.deepEqual(
      cast.descriptor.kind === 'summon-minion'
        ? cast.descriptor.sacrificedMinionInstanceIds
        : undefined,
      [local.instanceId],
    );
    assert.equal(wendigoActions.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && (descriptor.sacrificedMinionInstanceIds ?? []).some((instanceId) =>
        instanceId === submerged.instanceId || instanceId === enemy.instanceId)), false);

    const checkpointHash = hashGameState(ctx.state);
    const transcriptLengthBeforeForge = ctx.session.transcript.length;
    const forged = await ctx.stepRequest({
      actionId: `${cast.actionId}:forged`,
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    assert.equal(forged.reason?.code, 'unknown_action');
    assert.equal(hashGameState(forged.session.state), checkpointHash);
    assert.equal(forged.session.transcript.length, transcriptLengthBeforeForge);

    const preCastVersion = ctx.state.stateVersion;
    const result = await ctx.step(cast);
    assert.equal(result.accepted, true);
    if (!result.accepted) throw new Error('expected Gnarled Wendigo summon to be accepted');
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'minion-sacrificed',
      'minion-died',
      'minion-summoned',
    ]);
    const sacrificed = result.receipt.events[0];
    assert.ok(sacrificed && typeof sacrificed.payload === 'object' && !Array.isArray(sacrificed.payload));
    assert.deepEqual(sacrificed.payload, {
      cardId: local.cardId,
      instanceId: local.instanceId,
      owner: local.owner,
      seat: local.controller,
      sourceInstanceId: cast.descriptor.kind === 'summon-minion'
        ? cast.descriptor.cardInstanceId
        : '',
    });
    const summoned = result.receipt.events[2];
    assert.ok(summoned && typeof summoned.payload === 'object' && !Array.isArray(summoned.payload));
    assert.equal((summoned.payload as Readonly<Record<string, unknown>>).manaPaid, 4);
    assert.equal(ctx.state.players.north.mana, 0);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === local.instanceId), true);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === local.instanceId), false);
    const wendigo = ctx.state.realm.units.find(({ cardId }) => cardId === wendigoId);
    assert.ok(wendigo);
    const wendigoDefinition = gameManifest.cards[wendigo.cardId];
    assert.ok(wendigoDefinition?.cardType === 'minion');
    assert.deepEqual({
      attack: wendigoDefinition.attack,
      defense: wendigoDefinition.defense,
      location: wendigo.location,
      region: wendigo.region,
    }, { attack: 5, defense: 5, location: 'C4', region: 'surface' });
    assert.deepEqual(result.receipt.randomDraws, []);
    assert.equal(ctx.state.stateVersion, preCastVersion + 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  const deathriteManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [localMinionId]: {
        ...cards[localMinionId]!,
        deathriteDrawSite: true,
      } as GameCardDefinition,
      [secondLocalMinionId]: {
        ...cards[secondLocalMinionId]!,
        deathriteDrawSite: true,
      } as GameCardDefinition,
    },
    seed: gameManifest.seed,
  });

  await withSetup(deathriteManifest, async (ctx) => {
    await readyToSummon(ctx);
    const orderedLocals = ctx.state.realm.units
      .filter(({ cardId }) => cardId === localMinionId || cardId === secondLocalMinionId)
      .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
    assert.equal(orderedLocals.length, 2);
    const orderedPayment = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardId === wendigoId
        && descriptor.cell === 'C4'
        && descriptor.region === undefined
        && descriptor.manaCost === 2
        && descriptor.sacrificedMinionInstanceIds?.length === 2);
    const atlasBefore = ctx.state.players.north.atlas.length;
    const atlasHandBefore = ctx.state.players.north.hand.atlas.length;
    const interrupted = await ctx.step(orderedPayment);
    assert.equal(interrupted.accepted, true);
    if (!interrupted.accepted) throw new Error('expected ordered Gnarled payment to be accepted');
    assert.deepEqual(interrupted.receipt.events.map(({ type }) => type), [
      'minion-sacrificed',
      'minion-sacrificed',
    ]);
    assert.equal(ctx.state.phase, 'deathrite-order');
    assert.equal(ctx.state.decisionSeat, 'north');
    assert.deepEqual(ctx.state.terminal, { status: 'active' });
    assert.equal(ctx.state.players.north.mana, 2);
    assert.equal(ctx.state.players.north.hand.spellbook.some(({ cardId }) =>
      cardId === wendigoId), false);
    assert.equal(ctx.state.realm.units.some(({ cardId }) =>
      cardId === wendigoId), false);
    assert.equal(orderedLocals.every(({ instanceId }) => !ctx.state.realm.units
      .some((unit) => unit.instanceId === instanceId)), true);
    assert.equal(orderedLocals.every(({ instanceId }) => !ctx.state.players.north.cemetery
      .some((card) => card.instanceId === instanceId)), true);
    const orderActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(),
    orderedLocals.map(({ instanceId }) => instanceId));
    assert.deepEqual(await ctx.legalActions('south'), []);

    const cp = createGameCheckpoint(ctx.session);
    const parsedCp = parseGameCheckpoint(serializeGameCheckpoint(cp));
    const restored = await SetupCtx.resumeCheckpoint(parsedCp);
    assert.equal(
      canonicalJson(restored as unknown as JsonValue),
      canonicalJson(ctx.session as unknown as JsonValue),
    );
    await ctx.resume(parsedCp);
    assert.deepEqual(
      (await ctx.legalActions('north')).map(({ actionId }) => actionId),
      orderActions.map(({ actionId }) => actionId),
    );

    const branchHashes: string[] = [];
    for (const orderAction of orderActions) {
      assert.equal(orderAction.descriptor.kind, 'order-deathrites');
      if (orderAction.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
      const chosenInstanceId = orderAction.descriptor.sourceInstanceId;
      const otherInstanceId = orderedLocals.find(({ instanceId }) =>
        instanceId !== chosenInstanceId)?.instanceId;
      assert.ok(otherInstanceId);
      await ctx.resume(parsedCp);
      const ordered = await ctx.step(orderAction);
      assert.equal(ordered.accepted, true);
      if (!ordered.accepted) throw new Error('expected Deathrite order to resolve paid summon');
      assert.deepEqual(ordered.receipt.events.map(({ type }) => type), [
        'deathrite-order-committed',
        'site-drawn',
        'site-drawn',
        'minion-died',
        'minion-died',
        'minion-summoned',
      ]);
      assert.deepEqual(ordered.receipt.events.filter(({ type }) => type === 'site-drawn')
        .map(({ payload }) => payload !== null && typeof payload === 'object'
          && 'sourceInstanceId' in payload ? payload.sourceInstanceId : undefined), [
        chosenInstanceId,
        otherInstanceId,
      ]);
      assert.equal(ctx.state.phase, 'main');
      assert.equal(ctx.state.decisionSeat, 'north');
      assert.deepEqual(ctx.state.terminal, { status: 'active' });
      assert.equal(ctx.state.pendingDeathrites, undefined);
      assert.equal(ctx.state.players.north.mana, 2);
      assert.equal(ctx.state.players.north.atlas.length, atlasBefore - 2);
      assert.equal(ctx.state.players.north.hand.atlas.length, atlasHandBefore + 2);
      assert.equal(orderedLocals.every(({ instanceId }) => ctx.state.players.north.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      assert.equal(ctx.state.players.north.hand.spellbook.some(({ cardId }) =>
        cardId === wendigoId), false);
      assert.equal(ctx.state.players.north.cemetery.some(({ cardId }) =>
        cardId === wendigoId), false);
      const summonedWendigos = ctx.state.realm.units.filter(({ cardId }) => cardId === wendigoId);
      assert.equal(summonedWendigos.length, 1);
      assert.deepEqual(summonedWendigos.map(({ location, region }) => ({ location, region })), [{
        location: 'C4',
        region: 'surface',
      }]);
      assert.equal(await ctx.verifyReplay(), true);
      branchHashes.push(hashGameState(ctx.state));
    }
    assert.equal(new Set(branchHashes).size, 1);
  });

  await withSetup(deathriteManifest, async (ctx) => {
    await readyToSummon(ctx);
    const takeTerminal = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await ctx.accept(await ctx.action(predicate));
    };
    await takeTerminal(({ descriptor }) => descriptor.kind === 'end-turn');
    await takeTerminal(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeTerminal(({ descriptor }) => descriptor.kind === 'end-turn');
    await takeTerminal(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeTerminal(({ descriptor }) => descriptor.kind === 'end-turn');
    await takeTerminal(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeTerminal(({ descriptor }) => descriptor.kind === 'end-turn');
    await takeTerminal(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    assert.equal(ctx.state.players.north.atlas.length, 1);
    const terminalLocals = ctx.state.realm.units.filter(({ cardId }) =>
      cardId === localMinionId || cardId === secondLocalMinionId);
    assert.equal(terminalLocals.length, 2);
    const terminalManaBefore = ctx.state.players.north.mana;
    const terminalPayment = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardId === wendigoId
        && descriptor.cell === 'C4'
        && descriptor.manaCost === 2
        && descriptor.sacrificedMinionInstanceIds?.length === 2);
    await ctx.accept(terminalPayment);
    assert.equal(ctx.state.phase, 'deathrite-order');
    const terminalOrder = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    const terminalResult = await ctx.step(terminalOrder);
    assert.equal(terminalResult.accepted, true);
    if (!terminalResult.accepted) throw new Error('expected terminal Deathrite order to resolve');
    assert.deepEqual(terminalResult.receipt.events.map(({ type }) => type), [
      'deathrite-order-committed',
      'site-drawn',
      'minion-died',
      'minion-died',
      'game-ended',
    ]);
    assert.deepEqual(terminalResult.session.state.terminal, {
      loser: 'north',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'south',
    });
    assert.equal(terminalResult.session.state.phase, 'terminal');
    assert.equal(terminalResult.session.state.pendingDeathrites, undefined);
    assert.equal(terminalResult.session.state.players.north.mana, terminalManaBefore - 2);
    assert.equal(terminalResult.session.state.realm.units.some(({ cardId }) =>
      cardId === wendigoId), false);
    assert.equal(terminalResult.receipt.events.some(({ type }) => type === 'minion-summoned'), false);
    assert.equal(terminalLocals.every(({ instanceId }) => terminalResult.session.state.players.north.cemetery
      .some((card) => card.instanceId === instanceId)), true);
  });
});

// TODO(rust-cutover): synthetic state, needs a Rust-side proof. `paymentCheckpoint()` below
// hand-builds a GameSession with fabricated mana/hand/units to enumerate legal-action payment
// combinations (Aramos discard-vs-mana ordering, Gnarled sacrifice-tier stacking, roaming's
// summon-to-any-site bypass, all under the Hamlet discount) that are not reachable through
// legal play from a single seed. The core discount fact -- an ordinary minion at Hamlet costs
// 0 while a non-ordinary minion still pays full price -- is proven in Rust
// `hamlet_should_discount_only_ordinary_minions_at_that_site`
// (crates/sorcery-engine/tests/summon_rules.rs), but that proof does not cover the fuller
// payment-mode combinatorics this test also asserts, so this test stays on the legacy engine.
test('RULE-03 Hamlet reduces only Ordinary minion mana payments at that site', () => {
  type MinionDefinition = Extract<GameCardDefinition, Readonly<{ cardType: 'minion' }>>;
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const minion = (manaCost: number, facts: Partial<MinionDefinition> = {}): MinionDefinition => ({
    attack: 1,
    cardType: 'minion',
    defense: 1,
    manaCost,
    thresholds,
    ...facts,
  });
  const north: GameDeckSpec = {
    atlas: ['hamlet', 'hamlet', 'ordinary-site', 'ordinary-site'],
    avatar: 'north-avatar',
    spellbook: [
      'ordinary-one', 'nonordinary-one', 'aramos', 'gnarled', 'roaming',
      'helper', 'helper', 'helper',
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(4).fill('hamlet'),
    avatar: 'south-avatar',
    spellbook: Array(4).fill('filler'),
  };
  const cards: Record<string, GameCardDefinition> = {
    aramos: minion(3, { discardRandomCardInsteadOfMana: true, ordinary: true }),
    filler: minion(0),
    gnarled: minion(6, { sacrificeMinionAtSummoningLocationForManaDiscount: 2 }),
    hamlet: { cardType: 'site', elements: ['earth'], ordinaryMinionManaDiscount: 1 },
    helper: minion(0),
    'nonordinary-one': minion(1),
    'north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'ordinary-one': minion(1, { ordinary: true }),
    'ordinary-site': { cardType: 'site', elements: ['earth'] },
    roaming: minion(1, { ordinary: true, summonToAnySite: true }),
    'south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-hamlet-cost-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  const invalid = (cardId: string, facts: Record<string, unknown>) => createGameManifest({
    ...input,
    cards: { ...cards, [cardId]: { ...cards[cardId], ...facts } as GameCardDefinition },
    seed: 1,
  });
  assert.throws(() => invalid('hamlet', { ordinaryMinionManaDiscount: 0 }),
    /ordinaryMinionManaDiscount must be 1/);
  assert.throws(() => invalid('ordinary-one', { ordinary: false }), /ordinary must be true/);

  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 512; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const opening = createGameSession(candidate).state.players.north;
    if (opening.hand.spellbook.some(({ cardId }) => cardId === 'ordinary-one')) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  assert.deepEqual(gameManifest.cards.hamlet,
    { cardType: 'site', elements: ['earth'], ordinaryMinionManaDiscount: 1 });
  assert.equal(gameManifest.cards['ordinary-one']?.cardType === 'minion'
    && gameManifest.cards['ordinary-one'].ordinary, true);

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === 'hamlet' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === 'hamlet' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === 'ordinary-site' && descriptor.cell === 'C3');

  const summons = (checkpoint: GameSession, cardId: string) =>
    legalGameActions(checkpoint.state, 'north').flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === cardId ? [descriptor] : []);
  const ordinary = summons(session, 'ordinary-one');
  assert.deepEqual(ordinary.map(({ cell, manaCost }) => ({ cell, manaCost })), [
    { cell: 'C3', manaCost: 1 },
    { cell: 'C4', manaCost: 0 },
  ]);

  const northSpells = [
    ...session.state.players.north.hand.spellbook,
    ...session.state.players.north.spellbook,
  ];
  const helperCards = northSpells.filter(({ cardId }) => cardId === 'helper');
  const paymentCheckpoint = (mana: number, sacrificeHelpers = false): GameSession => ({
    ...session,
    state: {
      ...session.state,
      players: {
        ...session.state.players,
        north: {
          ...session.state.players.north,
          hand: {
            ...session.state.players.north.hand,
            spellbook: northSpells.filter(({ cardId }) => !sacrificeHelpers || cardId !== 'helper'),
          },
          mana,
          spellbook: [],
        },
      },
      realm: {
        ...session.state.realm,
        units: sacrificeHelpers ? helperCards.map((card) => ({
          ...card,
          controller: 'north' as const,
          damage: 0,
          location: 'C4' as const,
          region: 'surface' as const,
          stealthed: false,
          summoningSickness: false,
          tapped: false,
          warded: false,
        })) : session.state.realm.units,
      },
    },
  });
  const twoMana = paymentCheckpoint(2);
  assert.deepEqual(summons(twoMana, 'nonordinary-one').map(({ cell, manaCost }) => ({ cell, manaCost })), [
    { cell: 'C3', manaCost: 1 },
    { cell: 'C4', manaCost: 1 },
  ]);
  assert.equal(summons(twoMana, 'ordinary-one').some(({ cell }) => cell === 'C1'), false);
  assert.equal(summons(twoMana, 'roaming').some(({ cell, manaCost }) =>
    cell === 'C1' && manaCost === 0), true);
  assert.deepEqual(summons(twoMana, 'aramos')
    .map(({ cell, manaCost, paymentMode }) => `${cell}:${manaCost}:${paymentMode ?? 'mana'}`), [
    'C3:0:random-card-discard',
    'C4:0:random-card-discard',
    'C4:2:mana',
  ]);
  assert.deepEqual([...new Set(summons(paymentCheckpoint(6, true), 'gnarled')
    .filter(({ cell }) => cell === 'C4')
    .map(({ manaCost, sacrificedMinionInstanceIds }) =>
      `${sacrificedMinionInstanceIds?.length ?? 0}:${manaCost}`))].sort(), [
    '0:6', '1:4', '2:2', '3:0',
  ]);

  const zeroCost = action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'ordinary-one' && descriptor.cell === 'C4');
  const manaBefore = session.state.players.north.mana;
  const cast = stepGame(session, zeroCost);
  assert.equal(cast.accepted, true);
  if (!cast.accepted) throw new Error('expected Hamlet-discounted summon to be accepted');
  const summoned = cast.receipt.events.find(({ type }) => type === 'minion-summoned');
  assert.ok(summoned && typeof summoned.payload === 'object' && !Array.isArray(summoned.payload));
  assert.equal((summoned.payload as Readonly<Record<string, unknown>>).manaPaid, 0);
  assert.equal(cast.session.state.players.north.mana, manaBefore);
  assert.equal(verifyGameReplay(cast.session), true);
});

test('RULE-03 printed Spellcasters cast while tapped or summoning sick from their own location', async () => {
  const decks = {
    north: deck('spellcaster-north', 6, 8),
    south: deck('spellcaster-south', 6, 8),
  };
  const cards = cardsFor(decks, {
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const activeCasterId = decks.north.spellbook[0]!;
  const disabledCasterId = decks.north.spellbook[1]!;
  const freezeId = decks.north.spellbook[2]!;
  const summonId = decks.north.spellbook[3]!;
  const tappedCasterId = decks.north.spellbook[4]!;
  const targetId = decks.south.spellbook[0]!;
  const stealthedTargetId = decks.south.spellbook[1]!;
  cards[activeCasterId] = {
    ...cards[activeCasterId]!,
    spellcaster: true,
    stealth: true,
  } as unknown as GameCardDefinition;
  cards[disabledCasterId] = {
    ...cards[disabledCasterId]!,
    spellcaster: true,
    waterbound: true,
  } as unknown as GameCardDefinition;
  cards[freezeId] = {
    cardType: 'magic',
    disableTargetNearbyMinionUntilNextTurn: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  cards[summonId] = {
    ...cards[summonId]!,
    spellcaster: false,
  } as unknown as GameCardDefinition;
  cards[tappedCasterId] = {
    ...cards[tappedCasterId]!,
    spellcaster: true,
    tapForMana: 1,
  } as unknown as GameCardDefinition;
  cards[stealthedTargetId] = {
    ...cards[stealthedTargetId]!,
    stealth: true,
  } as GameCardDefinition;
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-printed-spellcaster-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [activeCasterId]: {
        ...cards[activeCasterId]!,
        spellcaster: 'yes',
      } as unknown as GameCardDefinition,
    },
    seed: 980,
  }), /spellcaster must be boolean/);

  let gameManifest: GameManifest | undefined;
  for (let seed = 980; seed < 2_980; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    // Seed search peeks opening hands via TS createGameSession (cheap); play path uses SetupCtx.
    const preview = createGameSession(candidate).state.players;
    const northAvailable = [
      ...preview.north.hand.spellbook,
      ...preview.north.spellbook.slice(0, 2),
    ].map(({ cardId }) => cardId);
    const southOpening = preview.south.hand.spellbook.map(({ cardId }) => cardId);
    if ([activeCasterId, disabledCasterId, freezeId, summonId, tappedCasterId]
      .every((cardId) => northAvailable.includes(cardId))
      && preview.north.hand.spellbook.some(({ cardId }) => cardId === tappedCasterId)
      && southOpening.includes(targetId)
      && southOpening.includes(stealthedTargetId)) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  assert.equal(
    gameManifest.cards[activeCasterId]?.cardType === 'minion'
      && (gameManifest.cards[activeCasterId] as unknown as { spellcaster?: boolean }).spellcaster,
    true,
  );
  assert.equal('spellcaster' in gameManifest.cards[summonId]!, false);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await ctx.accept(await ctx.action(predicate));
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === tappedCasterId && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetId && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === stealthedTargetId && descriptor.cell === 'C1');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetId);
    const stealthedTarget = ctx.state.realm.units.find(({ cardId }) =>
      cardId === stealthedTargetId);
    assert.ok(target);
    assert.ok(stealthedTarget);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const tappedCaster = ctx.state.realm.units.find(({ cardId }) => cardId === tappedCasterId);
    assert.ok(tappedCaster);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === tappedCaster.instanceId
      && descriptor.from.cell === 'C4'
      && descriptor.to.cell === 'C3'
      && descriptor.path.length === 2);
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === activeCasterId
      && descriptor.casterInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.cell === 'C2');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === disabledCasterId
      && descriptor.casterInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.cell === 'C3');
    const activeCaster = ctx.state.realm.units.find(({ cardId }) => cardId === activeCasterId);
    const disabledCaster = ctx.state.realm.units.find(({ cardId }) => cardId === disabledCasterId);
    const freeze = ctx.state.players.north.hand.spellbook.find(({ cardId }) => cardId === freezeId);
    const summonedCard = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === summonId);
    assert.ok(activeCaster);
    assert.ok(disabledCaster);
    assert.ok(freeze);
    assert.ok(summonedCard);
    await take(({ descriptor }) => descriptor.kind === 'activate-mana'
      && descriptor.unitInstanceId === tappedCaster.instanceId);
    const cp = createGameCheckpoint(ctx.session);
    const preBranchState = ctx.state;
    assert.deepEqual({
      stealthed: activeCaster.stealthed,
      summoningSickness: activeCaster.summoningSickness,
      tapped: preBranchState.realm.units.find(({ instanceId }) =>
        instanceId === activeCaster.instanceId)?.tapped,
    }, { stealthed: true, summoningSickness: true, tapped: false });
    assert.equal(preBranchState.realm.units.find(({ instanceId }) =>
      instanceId === tappedCaster.instanceId)?.tapped, true);
    assert.equal(observeGame(preBranchState, 'north').realm.units.find(({ instanceId }) =>
      instanceId === disabledCaster.instanceId)?.disabled, true);

    const actions = await ctx.legalActions('north');
    const targetFreezes = actions.filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === freeze.instanceId
        && descriptor.target?.instanceId === target.instanceId);
    assert.deepEqual(targetFreezes.map(({ descriptor }) =>
      descriptor.kind === 'cast-magic' ? descriptor.casterInstanceId : '').sort(), [
      activeCaster.instanceId,
    ]);
    assert.equal(actions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === freeze.instanceId
      && descriptor.target?.instanceId === stealthedTarget.instanceId), false);
    assert.match(targetFreezes[0]?.label ?? '', new RegExp(activeCaster.instanceId.slice(0, 15)));
    const summonCasters = new Set(actions.flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === summonedCard.instanceId
        ? [descriptor.casterInstanceId]
        : []));
    assert.deepEqual([...summonCasters].sort(), [
      activeCaster.instanceId,
      preBranchState.players.north.avatar.card.instanceId,
      tappedCaster.instanceId,
    ].sort());
    assert.equal(summonCasters.has(disabledCaster.instanceId), false);

    const frozen = await ctx.accept(targetFreezes[0]!);
    assert.deepEqual(frozen.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'stealth-lost',
      'minion-disabled',
      'magic-resolved',
    ]);
    assert.equal((frozen.transcript.at(-1)?.events[0]?.payload as unknown as
      Readonly<Record<string, unknown>>).casterInstanceId,
      activeCaster.instanceId);
    assert.equal((frozen.transcript.at(-1)?.events[2]?.payload as unknown as
      Readonly<Record<string, unknown>>).sourceInstanceId, freeze.instanceId);
    const casterAfterMagic = frozen.state.realm.units.find(({ instanceId }) =>
      instanceId === activeCaster.instanceId);
    assert.deepEqual({
      stealthed: casterAfterMagic?.stealthed,
      summoningSickness: casterAfterMagic?.summoningSickness,
      tapped: casterAfterMagic?.tapped,
    }, { stealthed: false, summoningSickness: true, tapped: false });
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(cp);
    const summoned = await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === summonedCard.instanceId
        && descriptor.casterInstanceId === activeCaster.instanceId
        && descriptor.cell === 'C2'));
    assert.deepEqual(summoned.transcript.at(-1)?.events.map(({ type }) => type), [
      'stealth-lost',
      'minion-summoned',
    ]);
    assert.equal((summoned.transcript.at(-1)?.events[1]?.payload as unknown as
      Readonly<Record<string, unknown>>).casterInstanceId,
      activeCaster.instanceId);
    assert.equal(summoned.state.realm.units.find(({ instanceId }) =>
      instanceId === summonedCard.instanceId)?.location, 'C2');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/05 targeted Magic is a non-unit source and resolves damage, Deathrite, and cemetery entry', () => {
  const decks = { north: deck('magic-north', 4, 6), south: deck('magic-south', 4, 6) };
  const cards = cardsFor(decks, {
    deathriteDrawSite: true,
    defense: 1,
    manaCost: 1,
    preventsDamageFromUnitsWithPowerAtLeast: 4,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, { attack: 4, defense: 4, drawSpell: false, life: 1 }, { elements: ['air'] });
  for (const cardId of decks.north.spellbook) {
    cards[cardId] = {
      cardType: 'magic',
      damageTargetUnit: 1,
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    };
  }
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-targeted-magic-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 148,
  });
  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const target = session.state.realm.units.find(({ controller }) => controller === 'south');
  assert.ok(target);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const spell = session.state.players.north.hand.spellbook[0];
  assert.ok(spell);
  const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
  assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target
    ? [`${descriptor.target.kind}:${descriptor.target.seat}:${descriptor.target.instanceId}`]
    : []).sort(), [
    `avatar:north:${session.state.players.north.avatar.card.instanceId}`,
    `avatar:south:${session.state.players.south.avatar.card.instanceId}`,
    `minion:south:${target.instanceId}`,
  ].sort());
  const before = session.state.players;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === spell.instanceId
      && descriptor.target !== undefined
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === target.instanceId));

  assert.equal(session.state.players.north.mana, before.north.mana - 1);
  assert.equal(session.state.players.north.hand.spellbook.length, before.north.hand.spellbook.length - 1);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === spell.instanceId), true);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === target.instanceId), false);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === target.instanceId), true);
  assert.equal(session.state.players.south.atlas.length, before.south.atlas.length - 1);
  assert.equal(session.state.players.south.hand.atlas.length, before.south.hand.atlas.length + 1);
  assert.deepEqual(session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-damage-allocated',
    'damage-dealt',
    'site-drawn',
    'minion-died',
    'magic-resolved',
  ]);
  assert.equal(session.state.phase, 'main');
  assert.equal(verifyGameReplay(session), true);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.target !== undefined
      && descriptor.target.kind === 'avatar'
      && descriptor.target.seat === 'south'));
  assert.equal(session.state.players.south.avatar.life, 0);
  assert.deepEqual(session.state.terminal, { status: 'active' });

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const finalSpell = session.state.players.north.hand.spellbook[0];
  assert.ok(finalSpell);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === finalSpell.instanceId
      && descriptor.target !== undefined
      && descriptor.target.kind === 'avatar'
      && descriptor.target.seat === 'south'));
  assert.deepEqual(session.state.terminal, {
    loser: 'south',
    reason: 'avatar_defeated',
    status: 'finished',
    winner: 'north',
  });
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === finalSpell.instanceId), true);
  assert.equal(session.transcript.at(-1)?.events.at(-1)?.type, 'game-ended');
  assert.equal(session.transcript.at(-1)?.events.at(-2)?.type, 'magic-resolved');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Duel makes a chosen ally fight a same-square targeted enemy', () => {
  const decks = {
    north: deck('duel-north', 4, 4),
    south: deck('duel-south', 4, 4),
  };
  const cards = cardsFor(decks, {
    defense: 4,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const duelId = decks.north.spellbook[0]!;
  const allyId = decks.north.spellbook[1]!;
  const casterId = decks.north.spellbook[2]!;
  const normalTargetId = decks.south.spellbook[0]!;
  const wardedTargetId = decks.south.spellbook[1]!;
  const stealthedTargetId = decks.south.spellbook[2]!;
  const disabledTargetId = decks.south.spellbook[3]!;
  cards[duelId] = {
    cardType: 'magic',
    fightAllyWithAdjacentEnemy: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as GameCardDefinition;
  cards[allyId] = {
    ...cards[allyId]!,
    attack: 3,
    defense: 4,
  } as GameCardDefinition;
  cards[casterId] = {
    ...cards[casterId]!,
    burrowing: true,
    spellcaster: true,
  } as GameCardDefinition;
  for (const targetId of [normalTargetId, wardedTargetId, stealthedTargetId, disabledTargetId]) {
    cards[targetId] = {
      ...cards[targetId]!,
      attack: 2,
      defense: 3,
      summonToAnySite: true,
    } as GameCardDefinition;
  }
  cards[wardedTargetId] = { ...cards[wardedTargetId]!, ward: true } as GameCardDefinition;
  cards[stealthedTargetId] = { ...cards[stealthedTargetId]!, stealth: true } as GameCardDefinition;
  cards[disabledTargetId] = { ...cards[disabledTargetId]!, waterbound: true } as GameCardDefinition;
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-duel-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [duelId]: { ...cards[duelId]!, fightAllyWithAdjacentEnemy: false } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /fightAllyWithAdjacentEnemy must be true/);

  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed < 100; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const opening = createGameSession(candidate).state.players.north.hand.spellbook;
    if ([duelId, allyId, casterId].every((cardId) =>
      opening.some((card) => card.cardId === cardId))) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  assert.equal(gameManifest.cards[duelId]?.cardType === 'magic'
    && gameManifest.cards[duelId].fightAllyWithAdjacentEnemy, true);
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === allyId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === casterId && descriptor.cell === 'C4'
    && descriptor.region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  for (const targetId of [normalTargetId, wardedTargetId, stealthedTargetId, disabledTargetId]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetId && descriptor.cell === 'C4');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  const ally = session.state.realm.units.find(({ cardId }) => cardId === allyId);
  const caster = session.state.realm.units.find(({ cardId }) => cardId === casterId);
  const normalTarget = session.state.realm.units.find(({ cardId }) => cardId === normalTargetId);
  const wardedTarget = session.state.realm.units.find(({ cardId }) => cardId === wardedTargetId);
  const stealthedTarget = session.state.realm.units.find(({ cardId }) => cardId === stealthedTargetId);
  const disabledTarget = session.state.realm.units.find(({ cardId }) => cardId === disabledTargetId);
  assert.ok(ally);
  assert.ok(caster);
  assert.ok(normalTarget);
  assert.ok(wardedTarget);
  assert.ok(stealthedTarget);
  assert.ok(disabledTarget);
  const duelActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardId === duelId
      && descriptor.casterInstanceId === caster.instanceId
      && descriptor.ally?.instanceId === ally.instanceId);
  const targetIds = duelActions.flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target ? [descriptor.target.instanceId] : []);
  assert.ok(targetIds.includes(normalTarget.instanceId));
  assert.ok(targetIds.includes(wardedTarget.instanceId));
  assert.ok(targetIds.includes(disabledTarget.instanceId));
  assert.equal(targetIds.includes(stealthedTarget.instanceId), false);
  assert.equal(targetIds.includes(session.state.players.south.avatar.card.instanceId), false);
  const normalAction = duelActions.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === normalTarget.instanceId);
  const wardedAction = duelActions.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === wardedTarget.instanceId);
  const disabledAction = duelActions.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === disabledTarget.instanceId);
  assert.ok(normalAction);
  assert.ok(wardedAction);
  assert.ok(disabledAction);
  const checkpoint = session;

  const fought = stepGame(checkpoint, normalAction);
  assert.equal(fought.accepted, true);
  assert.deepEqual(fought.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'fight-started',
    'strike-damage-allocated',
    'damage-dealt',
    'damage-dealt',
    'minion-died',
    'magic-resolved',
  ]);
  const fightingAlly = fought.session.state.realm.units.find(({ instanceId }) =>
    instanceId === ally.instanceId);
  assert.ok(fightingAlly);
  assert.deepEqual({
    damage: fightingAlly.damage,
    location: fightingAlly.location,
    tapped: fightingAlly.tapped,
  }, { damage: 2, location: 'C4', tapped: false });
  assert.equal(fought.session.state.realm.units.some(({ instanceId }) =>
    instanceId === normalTarget.instanceId), false);
  assert.equal(fought.session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === normalTarget.instanceId), true);
  assert.equal(fought.session.state.players.north.mana, 0);
  assert.equal(fought.session.state.players.north.cemetery.some(({ cardId }) => cardId === duelId), true);
  assert.equal(verifyGameReplay(fought.session), true);

  const warded = stepGame(checkpoint, wardedAction);
  assert.equal(warded.accepted, true);
  assert.deepEqual(warded.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'ward-broken',
    'magic-resolved',
  ]);
  assert.equal(warded.session.state.realm.units.find(({ instanceId }) =>
    instanceId === ally.instanceId)?.damage, 0);
  assert.equal(warded.session.state.realm.units.find(({ instanceId }) =>
    instanceId === wardedTarget.instanceId)?.warded, false);
  assert.equal(verifyGameReplay(warded.session), true);

  const disabled = stepGame(checkpoint, disabledAction);
  assert.equal(disabled.accepted, true);
  assert.equal(disabled.session.state.realm.units.find(({ instanceId }) =>
    instanceId === ally.instanceId)?.damage, 0);
  assert.equal(disabled.session.state.realm.units.some(({ instanceId }) =>
    instanceId === disabledTarget.instanceId), false);
  assert.equal(verifyGameReplay(disabled.session), true);
});

test('RULE-03/04 Leap Attack optionally steps an ally before it strikes every enemy there', () => {
  const decks = {
    north: deck('leap-north', 4, 4),
    south: deck('leap-south', 4, 4),
  };
  const cards = cardsFor(decks, {
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['fire'] });
  const leapId = decks.north.spellbook[0]!;
  const allyId = decks.north.spellbook[1]!;
  const immobileId = decks.north.spellbook[2]!;
  const disabledId = decks.north.spellbook[3]!;
  const originEnemyId = decks.south.spellbook[0]!;
  const normalEnemyId = decks.south.spellbook[1]!;
  const wardedEnemyId = decks.south.spellbook[2]!;
  const stealthedEnemyId = decks.south.spellbook[3]!;
  cards[leapId] = {
    cardType: 'magic',
    leapAttackAlly: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  } as GameCardDefinition;
  cards[allyId] = {
    ...cards[allyId]!,
    attack: 3,
    movementBonus: 2,
  } as GameCardDefinition;
  cards[immobileId] = { ...cards[immobileId]!, immobile: true } as GameCardDefinition;
  cards[disabledId] = { ...cards[disabledId]!, waterbound: true } as GameCardDefinition;
  for (const enemyId of [originEnemyId, normalEnemyId, wardedEnemyId, stealthedEnemyId]) {
    cards[enemyId] = {
      ...cards[enemyId]!,
      attack: 2,
      defense: 3,
      summonToAnySite: true,
    } as GameCardDefinition;
  }
  cards[wardedEnemyId] = {
    ...cards[wardedEnemyId]!,
    airborne: true,
    ward: true,
  } as GameCardDefinition;
  cards[stealthedEnemyId] = { ...cards[stealthedEnemyId]!, stealth: true } as GameCardDefinition;
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-leap-attack-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [leapId]: { ...cards[leapId]!, leapAttackAlly: false } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /leapAttackAlly must be true/);
  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed < 100; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const opening = createGameSession(candidate).state.players.north.hand.spellbook;
    if ([leapId, allyId].every((cardId) => opening.some((card) => card.cardId === cardId))) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  assert.equal(gameManifest.cards[leapId]?.cardType === 'magic'
    && gameManifest.cards[leapId].leapAttackAlly, true);
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === allyId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === originEnemyId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === immobileId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === disabledId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  for (const enemyId of [normalEnemyId, wardedEnemyId, stealthedEnemyId]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === enemyId && descriptor.cell === 'C3');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  const ally = session.state.realm.units.find(({ cardId }) => cardId === allyId);
  const immobile = session.state.realm.units.find(({ cardId }) => cardId === immobileId);
  const disabled = session.state.realm.units.find(({ cardId }) => cardId === disabledId);
  const originEnemy = session.state.realm.units.find(({ cardId }) => cardId === originEnemyId);
  const normalEnemy = session.state.realm.units.find(({ cardId }) => cardId === normalEnemyId);
  const wardedEnemy = session.state.realm.units.find(({ cardId }) => cardId === wardedEnemyId);
  const stealthedEnemy = session.state.realm.units.find(({ cardId }) => cardId === stealthedEnemyId);
  assert.ok(ally);
  assert.ok(immobile);
  assert.ok(disabled);
  assert.ok(originEnemy);
  assert.ok(normalEnemy);
  assert.ok(wardedEnemy);
  assert.ok(stealthedEnemy);
  const actionsFor = (instanceId: string): readonly GameLegalAction[] =>
    legalGameActions(session.state, 'north').filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardId === leapId
        && descriptor.ally?.instanceId === instanceId);
  assert.deepEqual(actionsFor(ally.instanceId).flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.allyDestination
      ? [`${descriptor.allyDestination.cell}/${descriptor.allyDestination.region}`]
      : []), ['C3/surface', 'C4/surface']);
  assert.deepEqual(actionsFor(immobile.instanceId).flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.allyDestination
      ? [descriptor.allyDestination.cell]
      : []), ['C4']);
  assert.deepEqual(actionsFor(disabled.instanceId).flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.allyDestination
      ? [descriptor.allyDestination.cell]
      : []), ['C4']);
  const noStepAction = actionsFor(ally.instanceId).find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.allyDestination?.cell === 'C4');
  const stepAction = actionsFor(ally.instanceId).find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.allyDestination?.cell === 'C3');
  assert.ok(noStepAction);
  assert.ok(stepAction);
  if (stepAction.descriptor.kind !== 'cast-magic') throw new Error('expected Leap Attack cast');
  const leapInstanceId = stepAction.descriptor.cardInstanceId;
  const checkpoint = session;

  const stayed = stepGame(checkpoint, noStepAction);
  assert.equal(stayed.accepted, true);
  assert.equal(stayed.receipt.events.some(({ type }) => type === 'unit-stepped'), false);
  assert.equal(stayed.receipt.events.filter(({ type }) => type === 'strike-damage-allocated').length, 1);
  assert.equal(stayed.session.state.realm.units.some(({ instanceId }) =>
    instanceId === originEnemy.instanceId), false);
  assert.equal(stayed.session.state.realm.units.some(({ instanceId }) =>
    instanceId === normalEnemy.instanceId), true);
  assert.equal(verifyGameReplay(stayed.session), true);

  const leaped = stepGame(checkpoint, stepAction);
  assert.equal(leaped.accepted, true);
  const stepped = leaped.receipt.events.find(({ type }) => type === 'unit-stepped');
  assert.ok(stepped && typeof stepped.payload === 'object' && !Array.isArray(stepped.payload));
  assert.deepEqual(stepped.payload, {
    from: { cell: 'C4', region: 'surface' },
    instanceId: ally.instanceId,
    seat: 'north',
    sourceInstanceId: leapInstanceId,
    steps: 1,
    to: { cell: 'C3', region: 'surface' },
  });
  assert.equal(leaped.receipt.events.filter(({ type }) => type === 'strike-damage-allocated').length, 3);
  const leapedAlly = leaped.session.state.realm.units.find(({ instanceId }) =>
    instanceId === ally.instanceId);
  assert.deepEqual({
    damage: leapedAlly?.damage,
    location: leapedAlly?.location,
    tapped: leapedAlly?.tapped,
  }, { damage: 0, location: 'C3', tapped: false });
  assert.equal(leaped.session.state.realm.units.some(({ instanceId }) =>
    instanceId === normalEnemy.instanceId), false);
  assert.equal(leaped.session.state.realm.units.some(({ instanceId }) =>
    instanceId === stealthedEnemy.instanceId), false);
  assert.equal(leaped.session.state.realm.units.find(({ instanceId }) =>
    instanceId === wardedEnemy.instanceId)?.warded, false);
  assert.equal(leaped.session.state.realm.units.some(({ instanceId }) =>
    instanceId === originEnemy.instanceId), true);
  assert.equal(leaped.session.state.players.north.mana, 1);
  assert.equal(leaped.session.state.players.north.cemetery.some(({ cardId }) => cardId === leapId), true);
  assert.equal(leaped.receipt.events.at(-1)?.type, 'magic-resolved');
  assert.equal(verifyGameReplay(leaped.session), true);
});
