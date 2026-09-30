import assert from 'node:assert/strict';
import test from 'node:test';
import {
  assertCanonicalGameManifest,
  createGameManifest,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameManifest,
} from '../../src/engine/game.ts';

const SYNTHETIC_AUTHORITY_HASH =
  'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const;


function deck(prefix: string, atlasCount = 30, spellbookCount = 50): GameDeckSpec {
  return {
    atlas: Array.from({ length: atlasCount }, (_, index) => `${prefix}-site-${index + 1}`),
    avatar: `${prefix}-avatar`,
    spellbook: Array.from({ length: spellbookCount }, (_, index) => `${prefix}-spell-${index + 1}`),
  };
}

type SpellFacts = Readonly<{
  airborne?: boolean;
  atStartOfControllerTurnLureNearbyEnemyMinion?: true;
  attack?: number;
  burrowing?: boolean;
  cannotAttackSites?: boolean;
  cannotDefend?: boolean;
  cannotDefendOrIntercept?: boolean;
  charge?: boolean;
  connectsTopBottom?: boolean;
  deathriteDamageEachUnitHere?: number;
  deathriteDrawSite?: boolean;
  deathriteDrawSpells?: boolean;
  deathriteHeal?: number;
  deathriteMillSites?: boolean;
  deathriteMillSpells?: boolean;
  deathriteLoseLifePerNearbySiteControlled?: 1;
  defense?: number;
  discardSpellToDamageRandomOtherUnitHere?: number;
  discardRandomCardInsteadOfMana?: true;
  diesAtEndOfControllerTurn?: true;
  gainsStealthAtEndOfTurn?: boolean;
  gainsStealthAtEndOfTurnIfNoEnemiesNearby?: boolean;
  genesisDrawSpells?: number;
  genesisDrawSite?: boolean;
  genesisHealController?: 2;
  genesisLoseControllerLife?: 2;
  genesisMayDamageTargetAdjacentUnit?: 2;
  genesisDisableSelfUntilDamaged?: true;
  genesisEachPlayerControlledByPreviousPlayerNextTurn?: true;
  genesisGainControlOfTappedMinionsHereUntilThisLeaves?: true;
  nearbyAvatarsMayDiscardCardToGainControlOfThis?: true;
  genesisStrikeEachEnemyHere?: true;
  genesisUntapAdjacentAllies?: true;
  immobile?: boolean;
  lanceCount?: 1 | 2 | 3;
  lethal?: boolean;
  manaCost: number;
  mayRangedStrikeOnceDuringBasicMovement?: true;
  mayStepAfterRangedStrike?: true;
  movementBonus?: 1 | 2;
  nearbyEnemiesPermanentlyLoseStealth?: true;
  otherNearbyAlliesPowerBonus?: 1;
  occupiesSquareArea?: 2;
  movesOnlyForward?: boolean;
  movesOnlySideways?: boolean;
  mustBeCastBurrowed?: boolean;
  mustBeCastSubmerged?: boolean;
  mustBeCastToWaterSite?: boolean;
  provides?: 'air' | 'earth' | 'fire' | 'water';
  preventsDamageFromUnitsWithPowerAtLeast?: number;
  ranged?: boolean;
  sacrificeMinionAtSummoningLocationForManaDiscount?: 2;
  shootsDragProjectile?: boolean;
  tapToShootProjectileDamage?: number;
  stealth?: boolean;
  strikesFirstWhileAttacking?: boolean;
  strikesFirstWhileDefending?: boolean;
  submerge?: boolean;
  summonToAnySite?: boolean;
  mustBeCastToOuterColumn?: boolean;
  tapForMana?: number;
  takesLessDamage?: 1;
  thresholds: Readonly<{ air: number; earth: number; fire: number; water: number }>;
  doesNotUntapDuringControllersStartPhase?: true;
  untapsAtEndOfControllerTurn?: true;
  voidwalk?: boolean;
  landbound?: boolean;
  waterbound?: boolean;
  ward?: boolean;
}>;

type AvatarFacts = Readonly<{
  attack: number;
  defense: number;
  drawSpell: boolean;
  earthSitePlayCreatesAdjacentRubble?: true;
  life: number;
  replaceAdjacentRubbleWithTopAtlasSite?: true;
  tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn?: true;
}>;

type SiteFacts = Readonly<{
  airborneMinionsAtopMoveFreelyAway?: true;
  blocksGroundMinionEntryWhileMinionAtop?: true;
  cannotBeMovedDestroyedOrModified?: true;
  connectsBurrowedAllies?: boolean;
  elements?: readonly ('air' | 'earth' | 'fire' | 'water')[];
  flyToNearbyVoidOncePerTurnAtAirThreshold?: 3;
  genesisDiscardTopSpells?: 2;
  genesisDrawSpellPerAdjacentSameCard?: boolean;
  genesisGainMana?: number;
  genesisGainManaIfOnlyControlledCopy?: 1;
  genesisHealNearbyAvatars?: 3;
  genesisImmobilizeNearbyUntilNextTurn?: true;
  genesisMayBottomNextSpell?: true;
  genesisReorderNextSpells?: 3;
  minionsHereGainVoidwalkUntilLeavingVoid?: true;
  ordinary?: true;
  rangedUnitsHereRangeBonus?: 1;
  sacrificeToDestroyNearbySite?: true;
}>;

function cardsFor(
  decks: Readonly<Record<'north' | 'south', GameDeckSpec>>,
  spell: SpellFacts = {
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  },
  avatar: AvatarFacts = { attack: 1, defense: 1, drawSpell: false, life: 20 },
  site: SiteFacts = {},
  seatSpells: Readonly<Partial<Record<'north' | 'south', SpellFacts>>> = {},
): Record<string, GameCardDefinition> {
  const cards: Record<string, GameCardDefinition> = {};
  for (const [seat, playerDeck] of Object.entries(decks) as ['north' | 'south', GameDeckSpec][]) {
    const facts = seatSpells[seat] ?? spell;
    cards[playerDeck.avatar] = {
      attack: avatar.attack,
      cardType: 'avatar',
      defense: avatar.defense,
      drawSpell: avatar.drawSpell,
      ...(avatar.earthSitePlayCreatesAdjacentRubble === true
        ? { earthSitePlayCreatesAdjacentRubble: true as const }
        : {}),
      life: avatar.life,
      ...(avatar.replaceAdjacentRubbleWithTopAtlasSite === true
        ? { replaceAdjacentRubbleWithTopAtlasSite: true as const }
        : {}),
      ...(avatar.tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn === true
        ? { tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true as const }
        : {}),
    };
    playerDeck.atlas.forEach((cardId) => {
      cards[cardId] = {
        ...(site.airborneMinionsAtopMoveFreelyAway === true
          ? { airborneMinionsAtopMoveFreelyAway: true as const }
          : {}),
        ...(site.blocksGroundMinionEntryWhileMinionAtop === true
          ? { blocksGroundMinionEntryWhileMinionAtop: true as const }
          : {}),
        ...(site.cannotBeMovedDestroyedOrModified === true
          ? { cannotBeMovedDestroyedOrModified: true as const }
          : {}),
        cardType: 'site',
        connectsBurrowedAllies: site.connectsBurrowedAllies ?? false,
        elements: site.elements ?? ['earth'],
        ...(site.flyToNearbyVoidOncePerTurnAtAirThreshold === 3
          ? { flyToNearbyVoidOncePerTurnAtAirThreshold: 3 as const }
          : {}),
        ...(site.genesisDiscardTopSpells === 2 ? { genesisDiscardTopSpells: 2 as const } : {}),
        genesisDrawSpellPerAdjacentSameCard:
          site.genesisDrawSpellPerAdjacentSameCard ?? false,
        ...(site.genesisGainMana ? { genesisGainMana: site.genesisGainMana } : {}),
        ...(site.genesisGainManaIfOnlyControlledCopy
          ? { genesisGainManaIfOnlyControlledCopy: site.genesisGainManaIfOnlyControlledCopy }
          : {}),
        ...(site.genesisHealNearbyAvatars === 3
          ? { genesisHealNearbyAvatars: 3 as const }
          : {}),
        ...(site.genesisImmobilizeNearbyUntilNextTurn === true
          ? { genesisImmobilizeNearbyUntilNextTurn: true as const }
          : {}),
        ...(site.genesisMayBottomNextSpell === true
          ? { genesisMayBottomNextSpell: true as const }
          : {}),
        ...(site.genesisReorderNextSpells === 3
          ? { genesisReorderNextSpells: 3 as const }
          : {}),
        ...(site.minionsHereGainVoidwalkUntilLeavingVoid === true
          ? { minionsHereGainVoidwalkUntilLeavingVoid: true as const }
          : {}),
        ...(site.ordinary === true ? { ordinary: true as const } : {}),
        ...(site.rangedUnitsHereRangeBonus === 1
          ? { rangedUnitsHereRangeBonus: 1 as const }
          : {}),
        ...(site.sacrificeToDestroyNearbySite === true
          ? { sacrificeToDestroyNearbySite: true as const }
          : {}),
      };
    });
    playerDeck.spellbook.forEach((cardId) => {
      cards[cardId] = {
        airborne: facts.airborne ?? false,
        attack: facts.attack ?? 1,
        burrowing: facts.burrowing ?? false,
        cardType: 'minion',
        cannotAttackSites: facts.cannotAttackSites ?? false,
        cannotDefend: facts.cannotDefend ?? false,
        cannotDefendOrIntercept: facts.cannotDefendOrIntercept ?? false,
        charge: facts.charge ?? false,
        connectsTopBottom: facts.connectsTopBottom ?? false,
        ...(facts.deathriteDamageEachUnitHere
          ? { deathriteDamageEachUnitHere: facts.deathriteDamageEachUnitHere }
          : {}),
        deathriteDrawSite: facts.deathriteDrawSite ?? false,
        ...(facts.deathriteDrawSpells === true ? { deathriteDrawSpells: true as const } : {}),
        ...(facts.deathriteHeal ? { deathriteHeal: facts.deathriteHeal } : {}),
        ...(facts.deathriteMillSites === true ? { deathriteMillSites: true as const } : {}),
        ...(facts.deathriteMillSpells === true ? { deathriteMillSpells: true as const } : {}),
        ...(facts.deathriteLoseLifePerNearbySiteControlled === 1
          ? { deathriteLoseLifePerNearbySiteControlled: 1 as const }
          : {}),
        defense: facts.defense ?? 1,
        ...(facts.discardSpellToDamageRandomOtherUnitHere !== undefined
          ? {
            discardSpellToDamageRandomOtherUnitHere:
              facts.discardSpellToDamageRandomOtherUnitHere,
          }
          : {}),
        ...(facts.discardRandomCardInsteadOfMana === true
          ? { discardRandomCardInsteadOfMana: true as const }
          : {}),
        ...(facts.diesAtEndOfControllerTurn === true
          ? { diesAtEndOfControllerTurn: true as const }
          : {}),
        gainsStealthAtEndOfTurn: facts.gainsStealthAtEndOfTurn ?? false,
        gainsStealthAtEndOfTurnIfNoEnemiesNearby:
          facts.gainsStealthAtEndOfTurnIfNoEnemiesNearby ?? false,
        ...(facts.genesisDrawSpells !== undefined
          ? { genesisDrawSpells: facts.genesisDrawSpells }
          : {}),
        genesisDrawSite: facts.genesisDrawSite ?? false,
        ...(facts.genesisHealController === 2 ? { genesisHealController: 2 as const } : {}),
        ...(facts.genesisLoseControllerLife === 2 ? { genesisLoseControllerLife: 2 as const } : {}),
        ...(facts.genesisMayDamageTargetAdjacentUnit === 2
          ? { genesisMayDamageTargetAdjacentUnit: 2 as const }
          : {}),
        ...(facts.genesisDisableSelfUntilDamaged === true
          ? { genesisDisableSelfUntilDamaged: true as const }
          : {}),
        ...(facts.genesisStrikeEachEnemyHere === true
          ? { genesisStrikeEachEnemyHere: true as const }
          : {}),
        ...(facts.genesisUntapAdjacentAllies === true
          ? { genesisUntapAdjacentAllies: true as const }
          : {}),
        ...(facts.genesisEachPlayerControlledByPreviousPlayerNextTurn === true
          ? { genesisEachPlayerControlledByPreviousPlayerNextTurn: true as const }
          : {}),
        ...(facts.genesisGainControlOfTappedMinionsHereUntilThisLeaves === true
          ? { genesisGainControlOfTappedMinionsHereUntilThisLeaves: true as const }
          : {}),
        ...(facts.nearbyAvatarsMayDiscardCardToGainControlOfThis === true
          ? { nearbyAvatarsMayDiscardCardToGainControlOfThis: true as const }
          : {}),
        immobile: facts.immobile ?? false,
        ...(facts.lanceCount ? { lanceCount: facts.lanceCount } : {}),
        lethal: facts.lethal ?? false,
        manaCost: facts.manaCost,
        ...(facts.mayRangedStrikeOnceDuringBasicMovement === true
          ? { mayRangedStrikeOnceDuringBasicMovement: true as const }
          : {}),
        ...(facts.mayStepAfterRangedStrike === true
          ? { mayStepAfterRangedStrike: true as const }
          : {}),
        ...(facts.movementBonus ? { movementBonus: facts.movementBonus } : {}),
        ...(facts.nearbyEnemiesPermanentlyLoseStealth === true
          ? { nearbyEnemiesPermanentlyLoseStealth: true as const }
          : {}),
        ...(facts.otherNearbyAlliesPowerBonus === 1
          ? { otherNearbyAlliesPowerBonus: 1 as const }
          : {}),
        movesOnlyForward: facts.movesOnlyForward ?? false,
        movesOnlySideways: facts.movesOnlySideways ?? false,
        mustBeCastBurrowed: facts.mustBeCastBurrowed ?? false,
        mustBeCastSubmerged: facts.mustBeCastSubmerged ?? false,
        mustBeCastToWaterSite: facts.mustBeCastToWaterSite ?? false,
        ...(facts.provides ? { provides: facts.provides } : {}),
        ...(facts.preventsDamageFromUnitsWithPowerAtLeast !== undefined
          ? {
            preventsDamageFromUnitsWithPowerAtLeast:
              facts.preventsDamageFromUnitsWithPowerAtLeast,
          }
          : {}),
        ranged: facts.ranged ?? false,
        ...(facts.sacrificeMinionAtSummoningLocationForManaDiscount === 2
          ? { sacrificeMinionAtSummoningLocationForManaDiscount: 2 as const }
          : {}),
        shootsDragProjectile: facts.shootsDragProjectile ?? false,
        stealth: facts.stealth ?? false,
        strikesFirstWhileAttacking: facts.strikesFirstWhileAttacking ?? false,
        strikesFirstWhileDefending: facts.strikesFirstWhileDefending ?? false,
        submerge: facts.submerge ?? false,
        summonToAnySite: facts.summonToAnySite ?? false,
        mustBeCastToOuterColumn: facts.mustBeCastToOuterColumn ?? false,
        ...(facts.tapToShootProjectileDamage !== undefined
          ? { tapToShootProjectileDamage: facts.tapToShootProjectileDamage }
          : {}),
        ...(facts.tapForMana ? { tapForMana: facts.tapForMana } : {}),
        ...(facts.takesLessDamage === 1 ? { takesLessDamage: 1 as const } : {}),
        thresholds: { ...facts.thresholds },
        ...(facts.doesNotUntapDuringControllersStartPhase === true
          ? { doesNotUntapDuringControllersStartPhase: true as const }
          : {}),
        ...(facts.untapsAtEndOfControllerTurn === true
          ? { untapsAtEndOfControllerTurn: true as const }
          : {}),
        voidwalk: facts.voidwalk ?? false,
        landbound: facts.landbound ?? false,
        waterbound: facts.waterbound ?? false,
        ward: facts.ward ?? false,
      };
    });
  }
  return cards;
}

// Retained original whole test ordinal 1; body is copied from the pinned AST statement.
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
  assert.throws(() => assertCanonicalGameManifest({
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
  const waterboundEndTurnStealth = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        gainsStealthAtEndOfTurnIfNoEnemiesNearby: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundEndTurnStealth.cards[firstSpell]?.cardType === 'minion'
      && waterboundEndTurnStealth.cards[firstSpell].gainsStealthAtEndOfTurnIfNoEnemiesNearby
      && waterboundEndTurnStealth.cards[firstSpell].waterbound,
    true,
  );
  const waterboundUnconditionalEndTurnStealth = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        gainsStealthAtEndOfTurn: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundUnconditionalEndTurnStealth.cards[firstSpell]?.cardType === 'minion'
      && waterboundUnconditionalEndTurnStealth.cards[firstSpell].gainsStealthAtEndOfTurn
      && waterboundUnconditionalEndTurnStealth.cards[firstSpell].waterbound,
    true,
  );
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
  const airborneGrantManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantAirborneToAllyThisTurn: true,
        manaCost: 1,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(airborneGrantManifest.cards[firstSpell], {
    cardType: 'magic',
    grantAirborneToAllyThisTurn: true,
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantAirborneToAllyThisTurn: false,
        manaCost: 1,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantAirborneToAllyThisTurn/);
  const rangedGrantManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantRangedToAllyThisTurn: true,
        manaCost: 1,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(rangedGrantManifest.cards[firstSpell], {
    cardType: 'magic',
    grantRangedToAllyThisTurn: true,
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantRangedToAllyThisTurn: false,
        manaCost: 1,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantRangedToAllyThisTurn/);
  const temporaryControlManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        gainControlOfTargetEnemyMinionThisTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(temporaryControlManifest.cards[firstSpell], {
    cardType: 'magic',
    gainControlOfTargetEnemyMinionThisTurn: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        gainControlOfTargetEnemyMinionThisTurn: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /gainControlOfTargetEnemyMinionThisTurn/);
  const stealthBoundControlManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        gainControlOfTargetEnemyMinionUntilStealthLost: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(stealthBoundControlManifest.cards[firstSpell], {
    cardType: 'magic',
    gainControlOfTargetEnemyMinionUntilStealthLost: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        gainControlOfTargetEnemyMinionUntilStealthLost: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /gainControlOfTargetEnemyMinionUntilStealthLost/);
  const lethalGrantManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantLethalToAllyThisTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(lethalGrantManifest.cards[firstSpell], {
    cardType: 'magic',
    grantLethalToAllyThisTurn: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantLethalToAllyThisTurn: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantLethalToAllyThisTurn/);
  const nextStrikeDoubleGrantManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantDoubleDamageToAllyNextStrikeThisTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(nextStrikeDoubleGrantManifest.cards[firstSpell], {
    cardType: 'magic',
    grantDoubleDamageToAllyNextStrikeThisTurn: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantDoubleDamageToAllyNextStrikeThisTurn: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantDoubleDamageToAllyNextStrikeThisTurn/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantDoubleDamageToAllyNextStrikeThisTurn: true,
        grantLethalToAllyThisTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const firstStrikeGrantManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantFirstStrikeToAllyThisTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(firstStrikeGrantManifest.cards[firstSpell], {
    cardType: 'magic',
    grantFirstStrikeToAllyThisTurn: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantFirstStrikeToAllyThisTurn: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantFirstStrikeToAllyThisTurn/);
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
  const cemeteryMagicManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnTargetMagicFromOwnCemetery: true,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(cemeteryMagicManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 3,
    returnTargetMagicFromOwnCemetery: true,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnTargetMagicFromOwnCemetery: false,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnTargetMagicFromOwnCemetery/);
  const cemeteryArtifactManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnTargetArtifactFromOwnCemetery: true,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(cemeteryArtifactManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 3,
    returnTargetArtifactFromOwnCemetery: true,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnTargetArtifactFromOwnCemetery: false,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnTargetArtifactFromOwnCemetery/);
  const cemeterySiteManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnTargetSiteFromOwnCemetery: true,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(cemeterySiteManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 3,
    returnTargetSiteFromOwnCemetery: true,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        returnTargetSiteFromOwnCemetery: false,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnTargetSiteFromOwnCemetery/);
  const chosenDiscardManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        discardCardAsAdditionalCost: true,
        drawSites: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(chosenDiscardManifest.cards[firstSpell], {
    cardType: 'magic',
    discardCardAsAdditionalCost: true,
    drawSites: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        discardCardAsAdditionalCost: false,
        drawSites: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /discardCardAsAdditionalCost/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        discardCardAsAdditionalCost: true,
        discardSiteAsAdditionalCost: true,
        destroyTargetSite: true,
        damageUnitsAboveAndBelowTargetSiteByManhattanDistance: [1, 2, 3, 4, 5],
        manaCost: 1,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /competing additional discard costs/);
  const payLifeManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 1,
        manaCost: 0,
        payLifeAsAdditionalCost: 2,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(payLifeManifest.cards[firstSpell], {
    cardType: 'magic',
    healController: 1,
    manaCost: 0,
    payLifeAsAdditionalCost: 2,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 1,
        manaCost: 0,
        payLifeAsAdditionalCost: 0,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /payLifeAsAdditionalCost/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        discardCardAsAdditionalCost: true,
        drawSites: 1,
        manaCost: 0,
        payLifeAsAdditionalCost: 2,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /competing additional costs/);
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
  const sleepManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        disableTargetMinionWithinTwoStepsUntilDamaged: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(sleepManifest.cards[firstSpell], {
    cardType: 'magic',
    disableTargetMinionWithinTwoStepsUntilDamaged: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        disableTargetMinionWithinTwoStepsUntilDamaged: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /disableTargetMinionWithinTwoStepsUntilDamaged/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        disableTargetMinionWithinTwoStepsUntilDamaged: true,
        disableTargetNearbyMinionUntilNextTurn: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const giftManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantPowerTwoToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(giftManifest.cards[firstSpell], {
    cardType: 'magic',
    grantPowerTwoToAllyThisTurnThenDrawSpell: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantPowerTwoToAllyThisTurnThenDrawSpell: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantPowerTwoToAllyThisTurnThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantPowerToAllyThisTurn: 2,
        grantPowerTwoToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const serpentManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantLethalToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(serpentManifest.cards[firstSpell], {
    cardType: 'magic',
    grantLethalToAllyThisTurnThenDrawSpell: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantLethalToAllyThisTurnThenDrawSpell: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantLethalToAllyThisTurnThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantLethalToAllyThisTurn: true,
        grantLethalToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const flightManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantAirborneToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(flightManifest.cards[firstSpell], {
    cardType: 'magic',
    grantAirborneToAllyThisTurnThenDrawSpell: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantAirborneToAllyThisTurnThenDrawSpell: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantAirborneToAllyThisTurnThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantAirborneToAllyThisTurn: true,
        grantAirborneToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const dashManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantMovementOneToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(dashManifest.cards[firstSpell], {
    cardType: 'magic',
    grantMovementOneToAllyThisTurnThenDrawSpell: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantMovementOneToAllyThisTurnThenDrawSpell: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantMovementOneToAllyThisTurnThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantPowerToAllyThisTurn: 2,
        grantMovementOneToAllyThisTurnThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const vanishManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToAlliedMinionsThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(vanishManifest.cards[firstSpell], {
    cardType: 'magic',
    grantStealthToAlliedMinionsThenDrawSpell: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToAlliedMinionsThenDrawSpell: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantStealthToAlliedMinionsThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToAlliedMinionsThenDrawSpell: true,
        grantStealthToTargetMinion: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const fadeManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(fadeManifest.cards[firstSpell], {
    cardType: 'magic',
    grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToAlliedMinionOccupyingEnemySiteThenDrawSpell: true,
        grantStealthToAlliedMinionsThenDrawSpell: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const frogTokenId = 'synthetic-frog-token';
  const frogManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        summonTokenToAlliedMinionThenDrawSpell: frogTokenId,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
      [frogTokenId]: {
        attack: 0,
        cardType: 'minion',
        defense: 0,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        token: true,
      },
    },
  });
  assert.deepEqual(frogManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 2,
    summonTokenToAlliedMinionThenDrawSpell: frogTokenId,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.equal(frogManifest.cards[frogTokenId]?.cardType === 'minion'
    && frogManifest.cards[frogTokenId].token, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        summonTokenToAlliedMinionThenDrawSpell: frogTokenId,
        summonTokenToEachControlledSiteBorderingEnemySite: frogTokenId,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
      [frogTokenId]: {
        attack: 0,
        cardType: 'minion',
        defense: 0,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        token: true,
      },
    },
  }), /exactly one supported Magic effect/);
  const baptizeManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        wardEachAlliedMinionAtTargetWaterSite: true,
      },
    },
  });
  assert.deepEqual(baptizeManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 3,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    wardEachAlliedMinionAtTargetWaterSite: true,
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        wardEachAlliedMinionAtTargetWaterSite: false,
      } as unknown as GameCardDefinition,
    },
  }), /wardEachAlliedMinionAtTargetWaterSite/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantWardToTargetMinion: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        wardEachAlliedMinionAtTargetWaterSite: true,
      },
    },
  }), /exactly one supported Magic effect/);
  const riptideManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        pullAdjacentAbovegroundUnitToTargetWaterSiteThenDrawSpell: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(riptideManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 2,
    pullAdjacentAbovegroundUnitToTargetWaterSiteThenDrawSpell: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        pullAdjacentAbovegroundUnitToTargetWaterSiteThenDrawSpell: false,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /pullAdjacentAbovegroundUnitToTargetWaterSiteThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        lureEnemyMinionOneStepCloser: true,
        manaCost: 2,
        pullAdjacentAbovegroundUnitToTargetWaterSiteThenDrawSpell: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const natureManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        returnUpToThreeCemeteryCardsToDeckBottomThenDrawSpell: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(natureManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 2,
    returnUpToThreeCemeteryCardsToDeckBottomThenDrawSpell: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        returnUpToThreeCemeteryCardsToDeckBottomThenDrawSpell: false,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnUpToThreeCemeteryCardsToDeckBottomThenDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        returnMinionFromOwnCemetery: true,
        returnUpToThreeCemeteryCardsToDeckBottomThenDrawSpell: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const blessManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        wardNearbyMinionOrSite: true,
      },
    },
  });
  assert.deepEqual(blessManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    wardNearbyMinionOrSite: true,
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        wardNearbyMinionOrSite: false,
      } as unknown as GameCardDefinition,
    },
  }), /wardNearbyMinionOrSite/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantWardToTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        wardNearbyMinionOrSite: true,
      },
    },
  }), /exactly one supported Magic effect/);
  const insultManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        silenceAndTapNearbyMinionThenMayDrawSpell: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(insultManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 2,
    silenceAndTapNearbyMinionThenMayDrawSpell: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        silenceAndTapNearbyMinionThenMayDrawSpell: false,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /silenceAndTapNearbyMinionThenMayDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        silenceAndTapNearbyMinionThenMayDrawSpell: true,
        tapTargetMinion: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const trialManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allySubmergesTargetNearbyMinion: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      },
    },
  });
  assert.deepEqual(trialManifest.cards[firstSpell], {
    cardType: 'magic',
    allySubmergesTargetNearbyMinion: true,
    manaCost: 3,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allySubmergesTargetNearbyMinion: false,
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      } as unknown as GameCardDefinition,
    },
  }), /allySubmergesTargetNearbyMinion/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allySubmergesTargetNearbyMinion: true,
        manaCost: 3,
        submergeTargetMinion: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      },
    },
  }), /exactly one supported Magic effect/);
  const spinManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allyStrikesEachEnemyAtItsLocation: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(spinManifest.cards[firstSpell], {
    cardType: 'magic',
    allyStrikesEachEnemyAtItsLocation: true,
    manaCost: 3,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allyStrikesEachEnemyAtItsLocation: false,
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /allyStrikesEachEnemyAtItsLocation/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allyStrikesEachEnemyAtItsLocation: true,
        leapAttackAlly: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const tacticalManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allyTakesUpToTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(tacticalManifest.cards[firstSpell], {
    cardType: 'magic',
    allyTakesUpToTwoSteps: true,
    manaCost: 2,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allyTakesUpToTwoSteps: false,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /allyTakesUpToTwoSteps/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        allyTakesUpToTwoSteps: true,
        leapAttackAlly: true,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const displaceManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        teleportTargetMinionArtifactOrAuraOneDiagonal: true,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(displaceManifest.cards[firstSpell], {
    cardType: 'magic',
    manaCost: 2,
    teleportTargetMinionArtifactOrAuraOneDiagonal: true,
    thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        teleportTargetMinionArtifactOrAuraOneDiagonal: false,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /teleportTargetMinionArtifactOrAuraOneDiagonal/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 2,
        teleportAllyToTargetSite: true,
        teleportTargetMinionArtifactOrAuraOneDiagonal: true,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const mortalityManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        killMortalMinionsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(mortalityManifest.cards[firstSpell], {
    cardType: 'magic',
    killMortalMinionsAtLocationWithinTwoSteps: true,
    manaCost: 2,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        killMortalMinionsAtLocationWithinTwoSteps: false,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /killMortalMinionsAtLocationWithinTwoSteps/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageEachUnitAtLocationWithinTwoSteps: 3,
        killMortalMinionsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const exorcismManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        banishDemonAndUndeadMinionsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(exorcismManifest.cards[firstSpell], {
    cardType: 'magic',
    banishDemonAndUndeadMinionsAtLocationWithinTwoSteps: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        banishDemonAndUndeadMinionsAtLocationWithinTwoSteps: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /banishDemonAndUndeadMinionsAtLocationWithinTwoSteps/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        banishDemonAndUndeadMinionsAtLocationWithinTwoSteps: true,
        killMortalMinionsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const destroyRelicsHereManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyArtifactsAndAurasAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(destroyRelicsHereManifest.cards[firstSpell], {
    cardType: 'magic',
    destroyArtifactsAndAurasAtLocationWithinTwoSteps: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyArtifactsAndAurasAtLocationWithinTwoSteps: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /destroyArtifactsAndAurasAtLocationWithinTwoSteps/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyArtifactsAndAurasAtLocationWithinTwoSteps: true,
        destroyTargetArtifact: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const boilManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyMinionsAtWaterSiteWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(boilManifest.cards[firstSpell], {
    cardType: 'magic',
    destroyMinionsAtWaterSiteWithinTwoSteps: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyMinionsAtWaterSiteWithinTwoSteps: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /destroyMinionsAtWaterSiteWithinTwoSteps/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        damageEachUnitAtLocationWithinTwoSteps: 3,
        destroyMinionsAtWaterSiteWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyMinionsAtWaterSiteWithinTwoSteps: true,
        killMortalMinionsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const unravelManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(unravelManifest.cards[firstSpell], {
    cardType: 'magic',
    destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps: true,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyArtifactsAndAurasAtLocationWithinTwoSteps: true,
        destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyUndeadMinionsAndArtifactsAtLocationWithinTwoSteps: true,
        killMortalMinionsAtLocationWithinTwoSteps: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const detonateManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyOwnArtifactAtLocationForAreaDamage: 3,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  });
  assert.deepEqual(detonateManifest.cards[firstSpell], {
    cardType: 'magic',
    destroyOwnArtifactAtLocationForAreaDamage: 3,
    manaCost: 2,
    thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyOwnArtifactAtLocationForAreaDamage: 2,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /destroyOwnArtifactAtLocationForAreaDamage must be 3/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyOwnArtifactAtLocationForAreaDamage: 3,
        destroyTargetArtifact: true,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const adjacentBuryManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        burrowTargetAdjacentMinion: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(adjacentBuryManifest.cards[firstSpell], {
    cardType: 'magic',
    burrowTargetAdjacentMinion: true,
    manaCost: 3,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        burrowTargetAdjacentMinion: false,
        manaCost: 3,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /burrowTargetAdjacentMinion/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        burrowTargetAdjacentMinion: true,
        burrowTargetMinionOrArtifact: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
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
  const healTarget = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healTargetMinion: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(healTarget.cards[firstSpell]?.cardType === 'magic'
    && healTarget.cards[firstSpell].healTargetMinion, 1);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healTargetMinion: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /healTargetMinion/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healTargetMinion: 1.5,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /healTargetMinion/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healTargetMinion: 101,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /healTargetMinion/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 7,
        healTargetMinion: 1,
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
        drawSpells: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /drawSpells/);
  const drawn = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSpells: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(drawn.cards[firstSpell]?.cardType === 'magic'
    && drawn.cards[firstSpell].drawSpells, 2);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        killTargetMinion: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /killTargetMinion must be true/);
  const killMinion = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        killTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(killMinion.cards[firstSpell]?.cardType === 'magic'
    && killMinion.cards[firstSpell].killTargetMinion, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSites: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /drawSites/);
  const drawnSites = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSites: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(drawnSites.cards[firstSpell]?.cardType === 'magic'
    && drawnSites.cards[firstSpell].drawSites, 2);
  const landmass = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSiteThenMayPlayLandSite: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(landmass.cards[firstSpell]?.cardType === 'magic'
    && landmass.cards[firstSpell].drawSiteThenMayPlayLandSite, true);
  const overflow = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSiteThenMayPlayWaterSite: true,
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(overflow.cards[firstSpell]?.cardType === 'magic'
    && overflow.cards[firstSpell].drawSiteThenMayPlayWaterSite, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSiteThenMayPlayLandSite: false,
        drawSites: 1,
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /drawSiteThenMayPlayLandSite must be true when defined/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSiteThenMayPlayWaterSite: false,
        drawSites: 1,
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /drawSiteThenMayPlayWaterSite must be true when defined/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSites: 1,
        drawSiteThenMayPlayLandSite: true,
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
        drawSites: 2,
        drawSpells: 2,
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
        returnTargetMinionToOwnerHand: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnTargetMinionToOwnerHand must be true/);
  const bounced = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        returnTargetMinionToOwnerHand: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(bounced.cards[firstSpell]?.cardType === 'magic'
    && bounced.cards[firstSpell].returnTargetMinionToOwnerHand, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        returnTargetSiteToOwnerHand: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnTargetSiteToOwnerHand must be true/);
  const bouncedSite = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        returnTargetSiteToOwnerHand: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(bouncedSite.cards[firstSpell]?.cardType === 'magic'
    && bouncedSite.cards[firstSpell].returnTargetSiteToOwnerHand, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyTargetArtifact: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /destroyTargetArtifact must be true/);
  const destroyedArtifact = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyTargetArtifact: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(destroyedArtifact.cards[firstSpell]?.cardType === 'magic'
    && destroyedArtifact.cards[firstSpell].destroyTargetArtifact, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        returnTargetArtifactToOwnerHand: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /returnTargetArtifactToOwnerHand must be true/);
  const bouncedArtifact = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        returnTargetArtifactToOwnerHand: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(bouncedArtifact.cards[firstSpell]?.cardType === 'magic'
    && bouncedArtifact.cards[firstSpell].returnTargetArtifactToOwnerHand, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyTargetArtifact: true,
        returnTargetArtifactToOwnerHand: true,
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
        destroyTargetSite: true,
        returnTargetSiteToOwnerHand: true,
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
        targetPlayerLosesLife: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /targetPlayerLosesLife/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 2,
        targetPlayerLosesLife: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const lifeLoss = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        targetPlayerLosesLife: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(lifeLoss.cards[firstSpell]?.cardType === 'magic'
    && lifeLoss.cards[firstSpell].targetPlayerLosesLife, 2);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        targetPlayerGainsLife: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /targetPlayerGainsLife/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 2,
        targetPlayerGainsLife: 2,
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
        targetPlayerGainsLife: 2,
        targetPlayerLosesLife: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const lifeGain = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        targetPlayerGainsLife: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(lifeGain.cards[firstSpell]?.cardType === 'magic'
    && lifeGain.cards[firstSpell].targetPlayerGainsLife, 2);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        untapTargetMinion: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /untapTargetMinion must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 2,
        untapTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const untap = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        untapTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(untap.cards[firstSpell]?.cardType === 'magic'
    && untap.cards[firstSpell].untapTargetMinion, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        tapTargetMinion: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /tapTargetMinion must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 2,
        tapTargetMinion: true,
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
        tapTargetMinion: true,
        untapTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const tap = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        tapTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(tap.cards[firstSpell]?.cardType === 'magic'
    && tap.cards[firstSpell].tapTargetMinion, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantWardToTargetMinion: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantWardToTargetMinion must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        healController: 2,
        grantWardToTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const grantWard = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantWardToTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(grantWard.cards[firstSpell]?.cardType === 'magic'
    && grantWard.cards[firstSpell].grantWardToTargetMinion, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToTargetMinion: false,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantStealthToTargetMinion must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantWardToTargetMinion: true,
        grantStealthToTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const grantStealth = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        grantStealthToTargetMinion: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(grantStealth.cards[firstSpell]?.cardType === 'magic'
    && grantStealth.cards[firstSpell].grantStealthToTargetMinion, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        millSpells: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /millSpells/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        drawSpells: 2,
        millSpells: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const milledSpells = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        millSpells: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(milledSpells.cards[firstSpell]?.cardType === 'magic'
    && milledSpells.cards[firstSpell].millSpells, 2);
  for (const targetPlayerDrawsSpells of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          cardType: 'magic',
          manaCost: 1,
          targetPlayerDrawsSpells,
          thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
        } as unknown as GameCardDefinition,
      },
    }), /targetPlayerDrawsSpells must be a safe integer between 1 and 200/);
  }
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        millSpells: 2,
        manaCost: 1,
        targetPlayerDrawsSpells: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const drawnSpells = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 1,
        targetPlayerDrawsSpells: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(drawnSpells.cards[firstSpell]?.cardType === 'magic'
    && drawnSpells.cards[firstSpell].targetPlayerDrawsSpells, 1);
  for (const targetPlayerDrawsSites of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          cardType: 'magic',
          manaCost: 1,
          targetPlayerDrawsSites,
          thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
        } as unknown as GameCardDefinition,
      },
    }), /targetPlayerDrawsSites must be a safe integer between 1 and 200/);
  }
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 1,
        targetPlayerDrawsSites: 1,
        targetPlayerDrawsSpells: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const targetedSiteDraw = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 1,
        targetPlayerDrawsSites: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(targetedSiteDraw.cards[firstSpell]?.cardType === 'magic'
    && targetedSiteDraw.cards[firstSpell].targetPlayerDrawsSites, 1);
  for (const targetPlayerDiscardsCards of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          cardType: 'magic',
          manaCost: 1,
          targetPlayerDiscardsCards,
          thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
        } as unknown as GameCardDefinition,
      },
    }), /targetPlayerDiscardsCards must be a safe integer between 1 and 200/);
  }
  const discardCards = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        manaCost: 1,
        targetPlayerDiscardsCards: 2,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(discardCards.cards[firstSpell]?.cardType === 'magic'
    && discardCards.cards[firstSpell].targetPlayerDiscardsCards, 2);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        millSites: 0,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /millSites/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        millSites: 2,
        millSpells: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /exactly one supported Magic effect/);
  const milledSites = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        millSites: 2,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(milledSites.cards[firstSpell]?.cardType === 'magic'
    && milledSites.cards[firstSpell].millSites, 2);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyTargetSite: true,
        discardSiteAsAdditionalCost: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  }), /site-destruction grid damage facts must be defined together/);
  const destroyedSite = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        destroyTargetSite: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.equal(destroyedSite.cards[firstSpell]?.cardType === 'magic'
    && destroyedSite.cards[firstSpell].destroyTargetSite, true);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        cardType: 'magic',
        returnMinionFromOwnCemetery: true,
        returnTargetMinionToOwnerHand: true,
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
      [firstSpell]: { ...cards[firstSpell]!, movementBonus: 4 } as unknown as GameCardDefinition,
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
        takesLessDamage: 101,
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
  for (const atStartOfControllerTurnDrawSpells of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          atStartOfControllerTurnDrawSpells,
        } as unknown as GameCardDefinition,
      },
    }), /atStartOfControllerTurnDrawSpells must be a safe integer between 1 and 200/);
  }
  for (const atStartOfControllerTurnDrawSites of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          atStartOfControllerTurnDrawSites,
        } as unknown as GameCardDefinition,
      },
    }), /atStartOfControllerTurnDrawSites must be a safe integer between 1 and 200/);
  }
  for (const atStartOfControllerTurnMillSpells of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          atStartOfControllerTurnMillSpells,
        } as unknown as GameCardDefinition,
      },
    }), /atStartOfControllerTurnMillSpells must be a safe integer between 1 and 200/);
  }
  for (const atStartOfControllerTurnMillSites of [0, 1.5, 201]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          atStartOfControllerTurnMillSites,
        } as unknown as GameCardDefinition,
      },
    }), /atStartOfControllerTurnMillSites must be a safe integer between 1 and 200/);
  }
  const startTurnDrawManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSpells: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnDrawManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnDrawManifest.cards[firstSpell].atStartOfControllerTurnDrawSpells,
    2,
  );
  const startTurnAtlasDrawManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSites: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnAtlasDrawManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnAtlasDrawManifest.cards[firstSpell].atStartOfControllerTurnDrawSites,
    2,
  );
  const startTurnMillManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnMillSpells: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnMillManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnMillManifest.cards[firstSpell].atStartOfControllerTurnMillSpells,
    2,
  );
  const startTurnAtlasMillManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnMillSites: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnAtlasMillManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnAtlasMillManifest.cards[firstSpell].atStartOfControllerTurnMillSites,
    2,
  );
  const startTurnDrawMillStackManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSpells: 1,
        atStartOfControllerTurnMillSpells: 1,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnDrawMillStackManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnDrawMillStackManifest.cards[firstSpell].atStartOfControllerTurnDrawSpells === 1
      && startTurnDrawMillStackManifest.cards[firstSpell].atStartOfControllerTurnMillSpells === 1,
    true,
  );
  const startTurnDrawTeleportStackManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSpells: 1,
        atStartOfControllerTurnTeleportToRandomSiteOrVoid: true,
        voidwalk: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnDrawTeleportStackManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnDrawTeleportStackManifest.cards[firstSpell].atStartOfControllerTurnDrawSpells === 1
      && startTurnDrawTeleportStackManifest.cards[firstSpell].atStartOfControllerTurnTeleportToRandomSiteOrVoid === true,
    true,
  );
  const startTurnDrawStackManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSites: 1,
        atStartOfControllerTurnDrawSpells: 1,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnDrawStackManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnDrawStackManifest.cards[firstSpell].atStartOfControllerTurnDrawSites === 1
      && startTurnDrawStackManifest.cards[firstSpell].atStartOfControllerTurnDrawSpells === 1,
    true,
  );
  const startTurnLureManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnLureNearbyEnemyMinion: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnLureManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnLureManifest.cards[firstSpell].atStartOfControllerTurnLureNearbyEnemyMinion,
    true,
  );
  const startTurnDrawLureStackManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSpells: 1,
        atStartOfControllerTurnLureNearbyEnemyMinion: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnDrawLureStackManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnDrawLureStackManifest.cards[firstSpell].atStartOfControllerTurnDrawSpells === 1
      && startTurnDrawLureStackManifest.cards[firstSpell].atStartOfControllerTurnLureNearbyEnemyMinion === true,
    true,
  );
  const startTurnDrawLifeGainStackManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSpells: 1,
        atStartOfControllerTurnControllerGainsLife: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnDrawLifeGainStackManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnDrawLifeGainStackManifest.cards[firstSpell].atStartOfControllerTurnDrawSpells === 1
      && startTurnDrawLifeGainStackManifest.cards[firstSpell].atStartOfControllerTurnControllerGainsLife === 2,
    true,
  );
  const startTurnDrawLifeMixedStackManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnDrawSpells: 1,
        atStartOfControllerTurnControllerGainsLife: 2,
        atStartOfControllerTurnControllerLosesLife: 1,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    startTurnDrawLifeMixedStackManifest.cards[firstSpell]?.cardType === 'minion'
      && startTurnDrawLifeMixedStackManifest.cards[firstSpell].atStartOfControllerTurnDrawSpells === 1
      && startTurnDrawLifeMixedStackManifest.cards[firstSpell].atStartOfControllerTurnControllerGainsLife === 2
      && startTurnDrawLifeMixedStackManifest.cards[firstSpell].atStartOfControllerTurnControllerLosesLife === 1,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atStartOfControllerTurnLureNearbyEnemyMinion: false,
      } as unknown as GameCardDefinition,
    },
  }), /atStartOfControllerTurnLureNearbyEnemyMinion must be true when defined/);
  for (const atEndOfControllerTurnDamageEachOtherUnitHere of [0, 1.5, 101]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          atEndOfControllerTurnDamageEachOtherUnitHere,
        } as unknown as GameCardDefinition,
      },
    }), /atEndOfControllerTurnDamageEachOtherUnitHere must be a safe integer between 1 and 100/);
  }
  const endTurnHereDamageManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atEndOfControllerTurnDamageEachOtherUnitHere: 1,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    endTurnHereDamageManifest.cards[firstSpell]?.cardType === 'minion'
      && endTurnHereDamageManifest.cards[firstSpell].atEndOfControllerTurnDamageEachOtherUnitHere,
    1,
  );
  for (const atEndOfControllerTurnControllerGainsLife of [0, 1.5, 101]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          atEndOfControllerTurnControllerGainsLife,
        } as unknown as GameCardDefinition,
      },
    }), /atEndOfControllerTurnControllerGainsLife must be a safe integer between 1 and 100/);
  }
  for (const atEndOfControllerTurnControllerLosesLife of [0, 1.5, 101]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [firstSpell]: {
          ...cards[firstSpell]!,
          atEndOfControllerTurnControllerLosesLife,
        } as unknown as GameCardDefinition,
      },
    }), /atEndOfControllerTurnControllerLosesLife must be a safe integer between 1 and 100/);
  }
  const endTurnLifeGainManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atEndOfControllerTurnControllerGainsLife: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    endTurnLifeGainManifest.cards[firstSpell]?.cardType === 'minion'
      && endTurnLifeGainManifest.cards[firstSpell].atEndOfControllerTurnControllerGainsLife,
    2,
  );
  const endTurnLifeLossManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atEndOfControllerTurnControllerLosesLife: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    endTurnLifeLossManifest.cards[firstSpell]?.cardType === 'minion'
      && endTurnLifeLossManifest.cards[firstSpell].atEndOfControllerTurnControllerLosesLife,
    2,
  );
  const endTurnLifeGainLossManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atEndOfControllerTurnControllerGainsLife: 2,
        atEndOfControllerTurnControllerLosesLife: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    endTurnLifeGainLossManifest.cards[firstSpell]?.cardType === 'minion'
      && endTurnLifeGainLossManifest.cards[firstSpell].atEndOfControllerTurnControllerGainsLife,
    2,
  );
  assert.equal(
    endTurnLifeGainLossManifest.cards[firstSpell]?.cardType === 'minion'
      && endTurnLifeGainLossManifest.cards[firstSpell].atEndOfControllerTurnControllerLosesLife,
    2,
  );
  const endTurnLifeGainHereDamageManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        atEndOfControllerTurnControllerGainsLife: 2,
        atEndOfControllerTurnDamageEachOtherUnitHere: 1,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    endTurnLifeGainHereDamageManifest.cards[firstSpell]?.cardType === 'minion'
      && endTurnLifeGainHereDamageManifest.cards[firstSpell].atEndOfControllerTurnControllerGainsLife,
    2,
  );
  assert.equal(
    endTurnLifeGainHereDamageManifest.cards[firstSpell]?.cardType === 'minion'
      && endTurnLifeGainHereDamageManifest.cards[firstSpell].atEndOfControllerTurnDamageEachOtherUnitHere,
    1,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        doesNotUntapDuringControllersStartPhase: false,
      } as unknown as GameCardDefinition,
    },
  }), /doesNotUntapDuringControllersStartPhase must be true when defined/);
  const doesNotUntapManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        doesNotUntapDuringControllersStartPhase: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    doesNotUntapManifest.cards[firstSpell]?.cardType === 'minion'
      && doesNotUntapManifest.cards[firstSpell].doesNotUntapDuringControllersStartPhase,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        mustAttackAUnitIfAble: false,
      } as unknown as GameCardDefinition,
    },
  }), /mustAttackAUnitIfAble must be true when defined/);
  const mustAttackManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        mustAttackAUnitIfAble: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    mustAttackManifest.cards[firstSpell]?.cardType === 'minion'
      && mustAttackManifest.cards[firstSpell].mustAttackAUnitIfAble,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        enemiesMustAttackThisIfAble: false,
      } as unknown as GameCardDefinition,
    },
  }), /enemiesMustAttackThisIfAble must be true when defined/);
  const forcedSourceManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        enemiesMustAttackThisIfAble: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    forcedSourceManifest.cards[firstSpell]?.cardType === 'minion'
      && forcedSourceManifest.cards[firstSpell].enemiesMustAttackThisIfAble,
    true,
  );
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
  const oversizedDiscard = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        discardSpellToDamageRandomOtherUnitHere: 3,
        occupiesSquareArea: 2,
      } as unknown as GameCardDefinition,
    },
  });
  assert.equal(
    oversizedDiscard.cards[firstSpell]?.cardType === 'minion'
      && oversizedDiscard.cards[firstSpell].occupiesSquareArea,
    2,
  );
  assert.equal(
    oversizedDiscard.cards[firstSpell]?.cardType === 'minion'
      && oversizedDiscard.cards[firstSpell].discardSpellToDamageRandomOtherUnitHere,
    3,
  );
  const stackedDrawGenesis = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSite: true,
        genesisDrawSpells: 1,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    stackedDrawGenesis.cards[firstSpell]?.cardType === 'minion'
      && stackedDrawGenesis.cards[firstSpell].genesisDrawSite
      && stackedDrawGenesis.cards[firstSpell].genesisDrawSpells === 1,
    true,
  );
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
  const stackedLossDrawGenesis = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSite: true,
        genesisLoseControllerLife: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    stackedLossDrawGenesis.cards[firstSpell]?.cardType === 'minion'
      && stackedLossDrawGenesis.cards[firstSpell].genesisDrawSite
      && stackedLossDrawGenesis.cards[firstSpell].genesisLoseControllerLife === 2,
    true,
  );
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
  const hobManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisUntapAdjacentAllies: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    hobManifest.cards[firstSpell]?.cardType === 'minion'
      && hobManifest.cards[firstSpell].genesisUntapAdjacentAllies,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisUntapAdjacentAllies: false,
      } as unknown as GameCardDefinition,
    },
  }), /genesisUntapAdjacentAllies must be true when defined/);
  const stackedHealDrawGenesis = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSite: true,
        genesisHealController: 2,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    stackedHealDrawGenesis.cards[firstSpell]?.cardType === 'minion'
      && stackedHealDrawGenesis.cards[firstSpell].genesisDrawSite
      && stackedHealDrawGenesis.cards[firstSpell].genesisHealController === 2,
    true,
  );
  const waterboundHealGenesis = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisHealController: 2,
        waterbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundHealGenesis.cards[firstSpell]?.cardType === 'minion'
      && waterboundHealGenesis.cards[firstSpell].genesisHealController === 2
      && waterboundHealGenesis.cards[firstSpell].waterbound,
    true,
  );
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
  const landboundManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        landbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    landboundManifest.cards[firstSpell]?.cardType === 'minion'
      && landboundManifest.cards[firstSpell].landbound,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        landbound: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /landbound must be boolean/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        landbound: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  }), /Landbound with Waterbound is unsupported/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        deathriteDrawSpells: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /deathriteDrawSpells must be boolean/);
  const deathriteDrawSpellsManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        deathriteDrawSpells: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    deathriteDrawSpellsManifest.cards[firstSpell]?.cardType === 'minion'
      && deathriteDrawSpellsManifest.cards[firstSpell].deathriteDrawSpells,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        deathriteMillSpells: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /deathriteMillSpells must be boolean/);
  const deathriteMillSpellsManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        deathriteMillSpells: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    deathriteMillSpellsManifest.cards[firstSpell]?.cardType === 'minion'
      && deathriteMillSpellsManifest.cards[firstSpell].deathriteMillSpells,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        deathriteMillSites: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /deathriteMillSites must be boolean/);
  const deathriteMillSitesManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        deathriteMillSites: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    deathriteMillSitesManifest.cards[firstSpell]?.cardType === 'minion'
      && deathriteMillSitesManifest.cards[firstSpell].deathriteMillSites,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        strikesFirstWhileDefending: 'yes',
      } as unknown as GameCardDefinition,
    },
  }), /strikesFirstWhileDefending must be boolean/);
  const defendingFirstStrike = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        strikesFirstWhileAttacking: true,
        strikesFirstWhileDefending: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    defendingFirstStrike.cards[firstSpell]?.cardType === 'minion'
      && defendingFirstStrike.cards[firstSpell].strikesFirstWhileAttacking
      && defendingFirstStrike.cards[firstSpell].strikesFirstWhileDefending,
    true,
  );
  const waterboundWard = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        waterbound: true,
        ward: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundWard.cards[firstSpell]?.cardType === 'minion'
      && waterboundWard.cards[firstSpell].waterbound
      && waterboundWard.cards[firstSpell].ward,
    true,
  );
  const waterboundStealthToken = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [decks.north.atlas[0]!]: {
        ...cards[decks.north.atlas[0]!]!,
        genesisPayOneManaToSummonToken: 'bound-scout',
      } as GameCardDefinition,
      'bound-scout': {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 0,
        occupiesSquareArea: 2,
        stealth: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        token: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundStealthToken.cards['bound-scout']?.cardType === 'minion'
      && waterboundStealthToken.cards['bound-scout'].occupiesSquareArea === 2
      && waterboundStealthToken.cards['bound-scout'].stealth
      && waterboundStealthToken.cards['bound-scout'].token
      && waterboundStealthToken.cards['bound-scout'].waterbound,
    true,
  );
  const waterboundSpellGenesis = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSpells: 1,
        waterbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundSpellGenesis.cards[firstSpell]?.cardType === 'minion'
      && waterboundSpellGenesis.cards[firstSpell].genesisDrawSpells === 1
      && waterboundSpellGenesis.cards[firstSpell].waterbound,
    true,
  );
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
  const mixedGenesisManaManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSite]: {
        ...cards[firstSite]!,
        genesisGainMana: 2,
        genesisGainManaIfOnlyControlledCopy: 1,
      } as GameCardDefinition,
    },
  });
  assert.deepEqual(mixedGenesisManaManifest.cards[firstSite], {
    cardType: 'site',
    elements: ['earth'],
    genesisGainMana: 2,
    genesisGainManaIfOnlyControlledCopy: 1,
  });
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

// Retained original whole test ordinal 128; body is copied from the pinned AST statement.
test('RULE-03 sacrifice-to-steal Artifact facts are exclusive and true-only', () => {
  const cards: Record<string, GameCardDefinition> = {
    'potion-north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'potion-north-site': { cardType: 'site', elements: ['earth'] },
    'potion-south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'potion-south-site': { cardType: 'site', elements: ['earth'] },
    'love-potion': {
      cardType: 'artifact',
      manaCost: 0,
      sacrificeThisToGainControlOfTargetEnemyMinionHereUntilBearerLeaves: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-sacrifice-control-artifact-v1',
    },
    cards,
    decks: {
      north: {
        atlas: Array(6).fill('potion-north-site'),
        avatar: 'potion-north-avatar',
        spellbook: Array(6).fill('love-potion'),
      },
      south: {
        atlas: Array(6).fill('potion-south-site'),
        avatar: 'potion-south-avatar',
        spellbook: Array(6).fill('love-potion'),
      },
    },
    firstSeat: 'north' as const,
    seed: 1,
  };
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards['love-potion'], cards['love-potion']);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'love-potion': {
        cardType: 'artifact',
        manaCost: 0,
        sacrificeThisToGainControlOfTargetEnemyMinionHereUntilBearerLeaves: false,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /sacrificeThisToGainControlOfTargetEnemyMinionHereUntilBearerLeaves must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'love-potion': {
        cardType: 'artifact',
        grantsBearerLethal: true,
        manaCost: 0,
        sacrificeThisToGainControlOfTargetEnemyMinionHereUntilBearerLeaves: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /exactly one supported Artifact effect/);
});

// Retained original whole test ordinal 199; body is copied from the pinned AST statement.
test('RULE-03 Craterize discards a site, destroys its target, and applies the printed damage grid', () => {
  const craterizeId = 'craterize';
  const targetSiteId = 'craterize-water-site';
  const landSiteId = 'craterize-land-site';
  const unitIds = [
    'craterize-center',
    'craterize-seven',
    'craterize-four',
    'craterize-two',
    'craterize-one',
    'craterize-oversized',
    'craterize-void',
  ] as const;
  const north: GameDeckSpec = {
    atlas: Array(8).fill(landSiteId),
    avatar: 'craterize-north-avatar',
    spellbook: Array(7).fill(craterizeId),
  };
  const south: GameDeckSpec = {
    atlas: [targetSiteId, targetSiteId, ...Array(8).fill(landSiteId)],
    avatar: 'craterize-south-avatar',
    spellbook: unitIds,
  };
  const thresholds = { air: 0, earth: 2, fire: 0, water: 0 } as const;
  const cards: Record<string, GameCardDefinition> = {
    [craterizeId]: {
      cardType: 'magic',
      damageUnitsAboveAndBelowTargetSiteByManhattanDistance: [10, 7, 4, 2, 1],
      destroyTargetSite: true,
      discardSiteAsAdditionalCost: true,
      manaCost: 8,
      thresholds,
    },
    [landSiteId]: { cardType: 'site', elements: ['earth'] },
    [targetSiteId]: { cardType: 'site', elements: ['water'] },
    'craterize-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'craterize-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    ...Object.fromEntries(unitIds.map((cardId) => [cardId, {
      attack: 1,
      ...(cardId === 'craterize-center' || cardId === 'craterize-seven'
        ? { burrowing: true }
        : {}),
      cardType: 'minion' as const,
      defense: 40,
      ...(cardId === 'craterize-oversized' ? { occupiesSquareArea: 2 as const } : {}),
      ...(cardId === 'craterize-two' ? { stealth: true } : {}),
      ...(cardId === 'craterize-center' ? { submerge: true } : {}),
      ...(cardId === 'craterize-void' ? { voidwalk: true } : {}),
      ...(cardId === 'craterize-one' ? { ward: true } : {}),
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    }])),
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-craterize-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
    seed: 197,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [craterizeId]: {
        ...cards[craterizeId]!,
        destroyTargetSite: false,
      } as unknown as GameCardDefinition,
    },
  }), /destroyTargetSite must be true/);

  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards[craterizeId], cards[craterizeId]);
});

// Retained original whole test ordinal 200; body is copied from the pinned AST statement.
test('token minions preserve an absent printed mana cost without accepting missing or non-token null', () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const absentToken: Extract<GameCardDefinition, { cardType: 'minion' }> = {
    attack: 1, cardType: 'minion', defense: 1, elements: ['water'], manaCost: null,
    subtypes: ['Beast'], thresholds, token: true,
  };
  const decks: GameDeckSpec = {
    atlas: ['printed-cost-site', 'printed-cost-site', 'printed-cost-site'],
    avatar: 'printed-cost-avatar',
    spellbook: ['printed-cost-spell', 'printed-cost-spell', 'printed-cost-spell'],
  };
  const baseCards: Record<string, GameCardDefinition> = {
    'printed-cost-avatar': {
      attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20,
    },
    'printed-cost-site': { cardType: 'site', elements: ['earth'] },
    'printed-cost-spell': {
      cardType: 'magic', manaCost: 1, summonTokenToAlliedMinionThenDrawSpell: 'printed-cost-token',
      thresholds,
    },
    'printed-cost-token': {
      ...absentToken,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-printed-cost-v1',
    },
    cards: baseCards,
    decks: { north: decks, south: decks },
    firstSeat: 'north' as const,
    seed: 17,
  };
  const absentCostManifest = createGameManifest(input);
  const absentTokenManifestCard = absentCostManifest.cards['printed-cost-token'];
  assert.equal(absentTokenManifestCard?.cardType, 'minion');
  assert.equal(absentTokenManifestCard?.cardType === 'minion' ? absentTokenManifestCard.manaCost : undefined, null);
  assert.deepEqual(absentTokenManifestCard?.cardType === 'minion' ? absentTokenManifestCard.elements : undefined, ['water']);
  assert.deepEqual(absentTokenManifestCard?.cardType === 'minion' ? absentTokenManifestCard.subtypes : undefined, ['Beast']);

  const zeroCostManifest = createGameManifest({
    ...input,
    cards: {
      ...baseCards,
      'printed-cost-token': { ...absentToken, manaCost: 0 },
    },
  });
  const zeroTokenManifestCard = zeroCostManifest.cards['printed-cost-token'];
  assert.equal(zeroTokenManifestCard?.cardType === 'minion' ? zeroTokenManifestCard.manaCost : undefined, 0);

  const emptyMetadataManifest = createGameManifest({
    ...input,
    cards: {
      ...baseCards,
      'printed-cost-token': { ...absentToken, elements: [], subtypes: [], manaCost: 0 },
    },
  });
  const emptyMetadataCard = emptyMetadataManifest.cards['printed-cost-token'];
  assert.deepEqual(emptyMetadataCard?.cardType === 'minion' ? emptyMetadataCard.elements : undefined, []);
  assert.deepEqual(emptyMetadataCard?.cardType === 'minion' ? emptyMetadataCard.subtypes : undefined, []);

  const bareToken = { ...absentToken } as unknown as Record<string, unknown>;
  delete bareToken.elements;
  delete bareToken.subtypes;
  const bareManifest = createGameManifest({
    ...input,
    cards: { ...baseCards, 'printed-cost-token': bareToken as GameCardDefinition },
  });
  const bareCard = bareManifest.cards['printed-cost-token'];
  assert.equal(bareCard?.cardType === 'minion' && 'elements' in bareCard, false);
  assert.equal(bareCard?.cardType === 'minion' && 'subtypes' in bareCard, false);

  assert.throws(() => createGameManifest({
    ...input,
    cards: { ...baseCards, 'printed-cost-token': { ...absentToken, mortal: true } },
  }), /mortal must agree with subtypes/);

  for (const subtype of ['\uFEFFBeast', 'Beast\uFEFF']) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: { ...baseCards, 'printed-cost-token': { ...absentToken, subtypes: [subtype] } },
    }), /subtypes must contain trimmed strings/);
  }

  const missingCostToken = { ...absentToken } as unknown as Record<string, unknown>;
  delete missingCostToken.manaCost;
  assert.throws(() => createGameManifest({
    ...input,
    cards: { ...baseCards, 'printed-cost-token': missingCostToken as GameCardDefinition },
  }), /manaCost must be defined/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...baseCards,
      'printed-cost-token': {
        ...absentToken, manaCost: null, token: undefined,
      } as unknown as GameCardDefinition,
    },
  }), /manaCost may be null only for token minions/);
});


type ShapeRole = 'artifact' | 'aura' | 'magic' | 'minion' | 'site';
type ShapeCase = Readonly<{
  id: string;
  title: string;
  role: ShapeRole;
  validFacts: Readonly<Record<string, unknown>>;
  invalidFacts?: Readonly<Record<string, unknown>>;
  error: string;
  validExtraCards?: Readonly<Record<string, GameCardDefinition>>;
}>;

const zeroThresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
const shapeAuthority = {
  contentHash: SYNTHETIC_AUTHORITY_HASH,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-manifest-shape-v1',
};

function shapeDefinition(role: ShapeRole, facts: Readonly<Record<string, unknown>>): GameCardDefinition {
  const common = { ...facts };
  switch (role) {
    case 'artifact':
      return { cardType: 'artifact', manaCost: 1, thresholds: zeroThresholds, ...common } as unknown as GameCardDefinition;
    case 'aura':
      return { cardType: 'aura', manaCost: 1, thresholds: zeroThresholds, ...common } as unknown as GameCardDefinition;
    case 'magic':
      return { cardType: 'magic', manaCost: 1, thresholds: zeroThresholds, ...common } as unknown as GameCardDefinition;
    case 'minion':
      return {
        attack: 1, cardType: 'minion', defense: 1, manaCost: 1, thresholds: zeroThresholds, ...common,
      } as unknown as GameCardDefinition;
    case 'site':
      return { cardType: 'site', elements: ['earth'], ...common } as unknown as GameCardDefinition;
  }
}

function shapeManifest(
  cardId: string,
  role: ShapeRole,
  definition: GameCardDefinition,
  extraCards: Readonly<Record<string, GameCardDefinition>> = {},
): GameManifest {
  const north: GameDeckSpec = {
    atlas: Array.from({ length: 30 }, (_, index) => index === 0 && role === 'site'
      ? cardId
      : `shape-north-site-${index}`),
    avatar: 'shape-north-avatar',
    spellbook: Array.from({ length: 50 }, (_, index) => index === 0 && role !== 'site'
      ? cardId
      : `shape-north-spell-${index}`),
  };
  const south: GameDeckSpec = {
    atlas: Array.from({ length: 30 }, (_, index) => `shape-south-site-${index}`),
    avatar: 'shape-south-avatar',
    spellbook: Array.from({ length: 50 }, (_, index) => `shape-south-spell-${index}`),
  };
  const cards: Record<string, GameCardDefinition> = {
    'shape-north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'shape-south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    ...Object.fromEntries([...north.atlas, ...south.atlas].filter((id) => id !== cardId).map((id) => [
      id, { cardType: 'site', elements: ['earth'] } satisfies GameCardDefinition,
    ])),
    ...Object.fromEntries([...north.spellbook, ...south.spellbook].filter((id) => id !== cardId).map((id) => [
      id, {
        attack: 1, cardType: 'minion', defense: 1, manaCost: 1, thresholds: zeroThresholds,
      } satisfies GameCardDefinition,
    ])),
    [cardId]: definition,
    ...extraCards,
  };
  return createGameManifest({
    authority: shapeAuthority,
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 1,
  });
}

const embeddedShapeCases: readonly ShapeCase[] = [
  { id: 'SHAPE-01', title: 'site sacrifice flag rejects false', role: 'site', validFacts: { sacrificeToDestroyNearbySite: true }, invalidFacts: { sacrificeToDestroyNearbySite: false }, error: 'sacrificeToDestroyNearbySite' },
  { id: 'SHAPE-02', title: 'protected site flag rejects false', role: 'site', validFacts: { cannotBeMovedDestroyedOrModified: true }, invalidFacts: { cannotBeMovedDestroyedOrModified: false }, error: 'cannotBeMovedDestroyedOrModified must be true when defined' },
  { id: 'SHAPE-03', title: 'Spellcaster fact requires boolean', role: 'minion', validFacts: { spellcaster: true }, invalidFacts: { spellcaster: 'yes' }, error: 'spellcaster must be boolean' },
  { id: 'SHAPE-04', title: 'Duel effect is true-only', role: 'magic', validFacts: { fightAllyWithAdjacentEnemy: true }, invalidFacts: { fightAllyWithAdjacentEnemy: false }, error: 'fightAllyWithAdjacentEnemy must be true' },
  { id: 'SHAPE-05', title: 'Leap Attack effect is true-only', role: 'magic', validFacts: { leapAttackAlly: true }, invalidFacts: { leapAttackAlly: false }, error: 'leapAttackAlly must be true' },
  { id: 'SHAPE-06', title: 'chain damage effect rejects false', role: 'magic', validFacts: { damageChainNearbyUnits: true }, invalidFacts: { damageChainNearbyUnits: false }, error: 'damageChainNearbyUnits' },
  { id: 'SHAPE-07', title: 'Overpower requires its exact value', role: 'magic', validFacts: { grantPowerToAllyThisTurn: 2 }, invalidFacts: { grantPowerToAllyThisTurn: 1 }, error: 'grantPowerToAllyThisTurn' },
  { id: 'SHAPE-08', title: 'nearby allies power bonus is exactly one', role: 'minion', validFacts: { otherNearbyAlliesPowerBonus: 1 }, invalidFacts: { otherNearbyAlliesPowerBonus: 2 }, error: 'otherNearbyAlliesPowerBonus must be 1' },
  { id: 'SHAPE-09', title: 'Mortal fact agrees with supported true-only schema', role: 'minion', validFacts: { mortal: true }, invalidFacts: { mortal: false }, error: 'mortal must be true' },
  { id: 'SHAPE-10', title: 'Demon fact agrees with supported true-only schema', role: 'minion', validFacts: { mortal: true, demon: true }, invalidFacts: { mortal: true, demon: false }, error: 'demon must be true' },
  { id: 'SHAPE-11', title: 'Undead fact agrees with supported true-only schema', role: 'minion', validFacts: { mortal: true, demon: true, undead: true }, invalidFacts: { mortal: true, demon: true, undead: false }, error: 'undead must be true' },
  { id: 'SHAPE-12', title: 'controlled Mortals power bonus is exactly one', role: 'minion', validFacts: { mortal: true, otherControlledMortalsPowerBonus: 1 }, invalidFacts: { mortal: true, otherControlledMortalsPowerBonus: 2 }, error: 'otherControlledMortalsPowerBonus must be 1' },
  { id: 'SHAPE-13', title: 'Cave-In effect is true-only', role: 'magic', validFacts: { burrowAllMinionsAndArtifactsAtTargetLandSite: true }, invalidFacts: { burrowAllMinionsAndArtifactsAtTargetLandSite: false }, error: 'burrowAllMinionsAndArtifactsAtTargetLandSite must be true when defined' },
  { id: 'SHAPE-14', title: 'Drown submerge effect rejects false', role: 'magic', validFacts: { submergeTargetMinion: true }, invalidFacts: { submergeTargetMinion: false }, error: 'submergeTargetMinion' },
  { id: 'SHAPE-15', title: 'Genesis area damage is exactly one', role: 'minion', validFacts: { genesisDamageEachOtherUnitHere: 1 }, invalidFacts: { genesisDamageEachOtherUnitHere: false }, error: 'genesisDamageEachOtherUnitHere must be 1' },
  { id: 'SHAPE-16', title: 'Genesis strike effect is true-only', role: 'minion', validFacts: { genesisStrikeEachEnemyHere: true }, invalidFacts: { genesisStrikeEachEnemyHere: false }, error: 'genesisStrikeEachEnemyHere must be true when defined' },
  { id: 'SHAPE-17', title: 'Genesis control effect is true-only', role: 'minion', validFacts: { genesisEachPlayerControlledByPreviousPlayerNextTurn: true }, invalidFacts: { genesisEachPlayerControlledByPreviousPlayerNextTurn: false }, error: 'genesisEachPlayerControlledByPreviousPlayerNextTurn must be true when defined' },
  { id: 'SHAPE-18', title: 'Genesis tapped-minion control effect is true-only', role: 'minion', validFacts: { genesisGainControlOfTappedMinionsHereUntilThisLeaves: true }, invalidFacts: { genesisGainControlOfTappedMinionsHereUntilThisLeaves: false }, error: 'genesisGainControlOfTappedMinionsHereUntilThisLeaves must be true when defined' },
  { id: 'SHAPE-19', title: 'Avatar discard control effect is true-only', role: 'minion', validFacts: { nearbyAvatarsMayDiscardCardToGainControlOfThis: true }, invalidFacts: { nearbyAvatarsMayDiscardCardToGainControlOfThis: false }, error: 'nearbyAvatarsMayDiscardCardToGainControlOfThis must be true when defined' },
  { id: 'SHAPE-20', title: 'Genesis self-disable effect is true-only', role: 'minion', validFacts: { genesisDisableSelfUntilDamaged: true }, invalidFacts: { genesisDisableSelfUntilDamaged: false }, error: 'genesisDisableSelfUntilDamaged must be true when defined' },
  { id: 'SHAPE-21', title: 'token spell cannot carry the Genesis self-disable fact', role: 'minion', validFacts: { genesisDisableSelfUntilDamaged: true }, invalidFacts: { token: true, genesisDisableSelfUntilDamaged: true }, error: 'unsupported spell' },
  { id: 'SHAPE-22', title: 'nearby Avatar Genesis healing is exactly three', role: 'site', validFacts: { genesisHealNearbyAvatars: 3 }, invalidFacts: { genesisHealNearbyAvatars: 2 }, error: 'genesisHealNearbyAvatars must be 3' },
  { id: 'SHAPE-23', title: 'Genesis immobilize effect rejects false', role: 'site', validFacts: { genesisImmobilizeNearbyUntilNextTurn: true }, invalidFacts: { genesisImmobilizeNearbyUntilNextTurn: false }, error: 'genesisImmobilizeNearbyUntilNextTurn' },
  { id: 'SHAPE-24', title: 'Aura grounding effect rejects false', role: 'aura', validFacts: { immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: true }, invalidFacts: { immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: false }, error: 'immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns' },
  { id: 'SHAPE-25', title: 'oversized footprint is exactly two', role: 'minion', validFacts: { occupiesSquareArea: 2 }, invalidFacts: { occupiesSquareArea: 3 }, error: 'occupiesSquareArea must be 2' },
  { id: 'SHAPE-26', title: 'Genesis token effect requires its referenced token minion', role: 'site', validFacts: { genesisPayOneManaToSummonToken: 'foot-soldier' }, error: 'token effect must reference a token minion', validExtraCards: { 'foot-soldier': { attack: 1, cardType: 'minion', defense: 1, manaCost: 0, thresholds: zeroThresholds, token: true } } },
  { id: 'SHAPE-27', title: 'Hunter’s Lodge effect is true-only', role: 'site', validFacts: { genesisEnemiesLoseStealth: true }, invalidFacts: { genesisEnemiesLoseStealth: false }, error: 'genesisEnemiesLoseStealth must be true' },
  { id: 'SHAPE-28', title: 'next-spell bottom effect is true-only', role: 'site', validFacts: { genesisMayBottomNextSpell: true }, invalidFacts: { genesisMayBottomNextSpell: false }, error: 'genesisMayBottomNextSpell must be true' },
  { id: 'SHAPE-29', title: 'Genesis next-spell reorder is exactly three', role: 'site', validFacts: { genesisReorderNextSpells: 3 }, invalidFacts: { genesisReorderNextSpells: 2 }, error: 'genesisReorderNextSpells must be 3' },
  { id: 'SHAPE-30', title: 'Vikings adjacent-location damage is exactly two', role: 'minion', validFacts: { tapToDamageEachUnitAtAdjacentLocation: 2 }, invalidFacts: { tapToDamageEachUnitAtAdjacentLocation: 1 }, error: 'tapToDamageEachUnitAtAdjacentLocation must be 2' },
  { id: 'SHAPE-31', title: 'site threshold suppression is true-only', role: 'minion', validFacts: { siteProvidesNoThreshold: true }, invalidFacts: { siteProvidesNoThreshold: false }, error: 'siteProvidesNoThreshold must be true when defined' },
  { id: 'SHAPE-32', title: 'Airborne site departure effect is true-only', role: 'site', validFacts: { airborneMinionsAtopMoveFreelyAway: true }, invalidFacts: { airborneMinionsAtopMoveFreelyAway: false }, error: 'airborneMinionsAtopMoveFreelyAway must be true when defined' },
  { id: 'SHAPE-33', title: 'Mountain Pass entry effect is true-only', role: 'site', validFacts: { blocksGroundMinionEntryWhileMinionAtop: true }, invalidFacts: { blocksGroundMinionEntryWhileMinionAtop: false }, error: 'blocksGroundMinionEntryWhileMinionAtop must be true when defined' },
  { id: 'SHAPE-34', title: 'controller-turn untap effect is true-only', role: 'minion', validFacts: { untapsAtEndOfControllerTurn: true }, invalidFacts: { untapsAtEndOfControllerTurn: false }, error: 'untapsAtEndOfControllerTurn must be true when defined' },
  { id: 'SHAPE-35', title: 'controller-turn death effect is true-only', role: 'minion', validFacts: { diesAtEndOfControllerTurn: true }, invalidFacts: { diesAtEndOfControllerTurn: false }, error: 'diesAtEndOfControllerTurn must be true when defined' },
  { id: 'SHAPE-36', title: 'Deathrite nearby-site life loss is exactly one', role: 'minion', validFacts: { deathriteLoseLifePerNearbySiteControlled: 1 }, invalidFacts: { deathriteLoseLifePerNearbySiteControlled: 2 }, error: 'deathriteLoseLifePerNearbySiteControlled must be 1' },
  { id: 'SHAPE-37', title: 'Deathrite damage requires a positive integer', role: 'minion', validFacts: { deathriteDamageEachUnitHere: 1 }, invalidFacts: { deathriteDamageEachUnitHere: 0 }, error: 'deathriteDamageEachUnitHere must be a safe integer between 1 and' },
  { id: 'SHAPE-38', title: 'Artifact bearer power is exactly two', role: 'artifact', validFacts: { grantsBearerPower: 2 }, invalidFacts: { grantsBearerPower: 1 }, error: 'grantsBearerPower must be 2' },
  { id: 'SHAPE-39', title: 'Artifact Lethal effect is true-only', role: 'artifact', validFacts: { grantsBearerLethal: true }, invalidFacts: { grantsBearerLethal: false }, error: 'grantsBearerLethal must be true' },
  { id: 'SHAPE-40', title: 'Artifact accepts exactly one supported effect', role: 'artifact', validFacts: { grantsBearerLethal: true }, invalidFacts: { grantsBearerLethal: true, grantsBearerPower: 2 }, error: 'exactly one supported Artifact effect' },
  { id: 'SHAPE-41', title: 'Siege Ballista damage is exactly three', role: 'artifact', validFacts: { tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3 }, invalidFacts: { tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 4 }, error: 'tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps must be 3' },
  { id: 'SHAPE-42', title: 'Siege Ballista cannot combine Lethal with its damage effect', role: 'artifact', validFacts: { tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3 }, invalidFacts: { tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3, grantsBearerLethal: true }, error: 'exactly one supported Artifact effect' },
  { id: 'SHAPE-43', title: 'Payload Trebuchet effect is true-only', role: 'artifact', validFacts: { tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps: true }, invalidFacts: { tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps: false }, error: 'tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps must be true' },
  { id: 'SHAPE-44', title: 'Rolling Boulder damage is exactly four', role: 'artifact', validFacts: { tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4 }, invalidFacts: { tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 5 }, error: 'tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath must be 4' },
  { id: 'SHAPE-45', title: 'Rolling Boulder cannot combine Lethal with its roll effect', role: 'artifact', validFacts: { tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4 }, invalidFacts: { tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4, grantsBearerLethal: true }, error: 'exactly one supported Artifact effect' },
  { id: 'SHAPE-46', title: 'Mesmerism control effect is true-only', role: 'magic', validFacts: { gainControlOfTargetNearbyMinion: true }, invalidFacts: { gainControlOfTargetNearbyMinion: false }, error: 'gainControlOfTargetNearbyMinion must be true' },
  { id: 'SHAPE-47', title: 'Mesmerism accepts exactly one supported Magic effect', role: 'magic', validFacts: { gainControlOfTargetNearbyMinion: true }, invalidFacts: { gainControlOfTargetNearbyMinion: true, healController: 1 }, error: 'exactly one supported Magic effect' },
  { id: 'SHAPE-48', title: 'Fatality requires its true-only wounded-minion effect', role: 'magic', validFacts: { killTargetWoundedMinion: true }, invalidFacts: { killTargetWoundedMinion: false }, error: 'killTargetWoundedMinion must be true' },
  { id: 'SHAPE-49', title: 'Fatality cannot combine healing with its kill effect', role: 'magic', validFacts: { killTargetWoundedMinion: true }, invalidFacts: { killTargetWoundedMinion: true, healController: 1 }, error: 'exactly one supported Magic effect' },
  { id: 'SHAPE-50', title: 'kill-minion effect is true-only', role: 'magic', validFacts: { killTargetMinion: true }, invalidFacts: { killTargetMinion: false }, error: 'killTargetMinion must be true' },
  { id: 'SHAPE-51', title: 'Magic cannot combine its two kill effects', role: 'magic', validFacts: { killTargetMinion: true }, invalidFacts: { killTargetMinion: true, killTargetWoundedMinion: true }, error: 'exactly one supported Magic effect' },
  { id: 'SHAPE-52', title: 'nearby-minion attack effect is true-only', role: 'artifact', validFacts: { nearbyMinionsMustAttackIfAble: true }, invalidFacts: { nearbyMinionsMustAttackIfAble: false }, error: 'nearbyMinionsMustAttackIfAble must be true' },
  { id: 'SHAPE-53', title: 'Tower marker is true-only', role: 'site', validFacts: { isTower: true }, invalidFacts: { isTower: false }, error: 'isTower must be true' },
  { id: 'SHAPE-54', title: 'Tower power bonus is exactly two', role: 'minion', validFacts: { gainsPowerRangedAndSpellcasterAtopTower: 2 }, invalidFacts: { gainsPowerRangedAndSpellcasterAtopTower: 1 }, error: 'gainsPowerRangedAndSpellcasterAtopTower must be 2' },
  { id: 'SHAPE-55', title: 'site-controller life loss requires a positive integer', role: 'artifact', validFacts: { atEndOfEachTurnSiteControllerLosesLife: 1 }, invalidFacts: { atEndOfEachTurnSiteControllerLosesLife: 0 }, error: 'atEndOfEachTurnSiteControllerLosesLife must be a safe integer between 1 and' },
  { id: 'SHAPE-56', title: 'Raise Dead effect is true-only', role: 'magic', validFacts: { summonRandomMinionFromAnyCemetery: true }, invalidFacts: { summonRandomMinionFromAnyCemetery: false }, error: 'summonRandomMinionFromAnyCemetery must be true' },
];

for (const shape of embeddedShapeCases) {
  test(`${shape.id} ${shape.title}`, () => {
    const cardId = `shape-card-${shape.id.toLowerCase()}`;
    const valid = shapeDefinition(shape.role, shape.validFacts);
    assert.doesNotThrow(() => shapeManifest(cardId, shape.role, valid, shape.validExtraCards));
    if (!shape.invalidFacts) {
      assert.throws(() => shapeManifest(cardId, shape.role, valid), new RegExp(shape.error));
      return;
    }
    const invalid = shapeDefinition(shape.role, { ...shape.validFacts, ...shape.invalidFacts });
    assert.throws(() => shapeManifest(cardId, shape.role, invalid), new RegExp(shape.error));
  });
}
