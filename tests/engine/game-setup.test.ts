import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  parseGameCheckpoint,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import { opaqueActionId, type EngineActionDescriptor } from '../../src/engine/contract.ts';
import {
  assertCanonicalGameManifest,
  createGameManifest,
  hashGameState,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
  type GameStepResult,
} from '../../src/engine/game.ts';
import { SetupCtx, findOpeningManifest, withFork, withPreview, withSetup } from './rust-setup-session.ts';

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
  attack?: number;
  burrowing?: boolean;
  cannotAttackSites?: boolean;
  cannotDefend?: boolean;
  cannotDefendOrIntercept?: boolean;
  charge?: boolean;
  connectsTopBottom?: boolean;
  deathriteDamageEachUnitHere?: number;
  deathriteDrawSite?: boolean;
  deathriteHeal?: number;
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
  genesisStrikeEachEnemyHere?: true;
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
  submerge?: boolean;
  summonToAnySite?: boolean;
  mustBeCastToOuterColumn?: boolean;
  tapForMana?: number;
  takesLessDamage?: 1;
  thresholds: Readonly<{ air: number; earth: number; fire: number; water: number }>;
  untapsAtEndOfControllerTurn?: true;
  voidwalk?: boolean;
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
        ...(facts.deathriteHeal ? { deathriteHeal: facts.deathriteHeal } : {}),
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
        submerge: facts.submerge ?? false,
        summonToAnySite: facts.summonToAnySite ?? false,
        mustBeCastToOuterColumn: facts.mustBeCastToOuterColumn ?? false,
        ...(facts.tapToShootProjectileDamage !== undefined
          ? { tapToShootProjectileDamage: facts.tapToShootProjectileDamage }
          : {}),
        ...(facts.tapForMana ? { tapForMana: facts.tapForMana } : {}),
        ...(facts.takesLessDamage === 1 ? { takesLessDamage: 1 as const } : {}),
        thresholds: { ...facts.thresholds },
        ...(facts.untapsAtEndOfControllerTurn === true
          ? { untapsAtEndOfControllerTurn: true as const }
          : {}),
        voidwalk: facts.voidwalk ?? false,
        waterbound: facts.waterbound ?? false,
        ward: facts.ward ?? false,
      };
    });
  }
  return cards;
}

function manifest(
  seed = 1,
  options: Readonly<{
    avatar?: AvatarFacts;
    north?: GameDeckSpec;
    northSpell?: SpellFacts;
    site?: SiteFacts;
    south?: GameDeckSpec;
    southSpell?: SpellFacts;
    spell?: SpellFacts;
  }> = {},
): GameManifest {
  const decks = {
    north: options.north ?? deck('north'),
    south: options.south ?? deck('south'),
  };
  return createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-setup-fixture-v1',
    },
    cards: cardsFor(decks, options.spell, options.avatar, options.site, {
      ...(options.northSpell ? { north: options.northSpell } : {}),
      ...(options.southSpell ? { south: options.southSpell } : {}),
    }),
    decks,
    firstSeat: 'north',
    seed,
  });
}

test('RULE-01 setup shuffles two decks, deals split hidden hands, and places Avatars', async () => {
  await withSetup(manifest(7), async (ctx) => {
    const session = ctx.session;
    const north = session.state.players.north;
    const southView = ctx.observe('south');

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
    assert.deepEqual(await ctx.legalActions('north'), northActions);
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
        stealth: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        token: true,
        waterbound: true,
      } as GameCardDefinition,
    },
  });
  assert.equal(
    waterboundStealthToken.cards['bound-scout']?.cardType === 'minion'
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

test('RULE-01 one mulligan returns at most three chosen cards to their deck bottoms and redraws', async () => {
  await withSetup(manifest(11), async (ctx) => {
    const returned = ctx.state.players.north.hand.atlas[0];
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
    const session = ctx.session;

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
    await ctx.accept(site);
    assert.equal(ctx.state.realm.sites.C4?.instanceId, siteId);
    assert.equal(ctx.state.realm.sites.C4?.controller, 'north');
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.players.north.domainEstablished, true);
    assert.equal(ctx.state.players.north.mana, 1);
    const afterSiteKinds = (await ctx.legalActions('north')).map(({ descriptor }) => descriptor.kind);
    assert.equal(afterSiteKinds.includes('play-site'), false);
    assert.equal(afterSiteKinds.includes('draw-site'), false);
    assert.equal(afterSiteKinds.includes('end-turn'), true);

    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    assert.equal(ctx.state.turnNumber, 2);
    assert.equal(ctx.state.activeSeat, 'south');
    assert.equal(ctx.state.phase, 'draw');
    assert.deepEqual(
      (await ctx.legalActions('south')).map(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone),
      ['atlas', 'spellbook'],
    );

    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.players.south.hand.atlas.length, 4);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events[0]?.payload, { seat: 'south', zone: 'atlas' });
    assert.doesNotMatch(canonicalJson(ctx.session.transcript.at(-1)?.events[0]?.payload ?? null), /south-site-/);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

async function withNorthSecondMain(
  seed: number,
  shortDecks: boolean,
  spell: SpellFacts | undefined,
  run: (ctx: SetupCtx) => Promise<void>,
): Promise<void> {
  const options = shortDecks
    ? { north: deck('north', 3, 4), south: deck('south', 3, 4), ...(spell ? { spell } : {}) }
    : spell ? { spell } : {};
  await withSetup(manifest(seed, options), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    await run(ctx);
  });
}

test('RULE-02 sites expand through unoccupied orthogonal cells controlled by their player', async () => {
  await withNorthSecondMain(23, false, undefined, async (ctx) => {
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
  await withPreview(base, async (preview) => {
    const sourceCardId = preview.state.players.north.hand.atlas[0]?.cardId;
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

      const southSite = ctx.state.realm.sites.C1;
      assert.ok(southSite && !('rubble' in southSite));
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
      if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
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
      assert.equal(ctx.observe('north').players.north.affinity.earth, 1);
      assert.equal(ctx.state.players.north.avatar.tapped, true);
      assert.equal(ctx.state.players.north.hand.atlas.length, handBefore - 1);
      assert.equal(ctx.state.players.north.mana, 1);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-02 the Avatar may draw a private site instead of playing one', async () => {
  await withNorthSecondMain(29, false, undefined, async (ctx) => {
    const before = ctx.state.players.north;
    const drawn = before.atlas[0];
    assert.ok(drawn);
    const result = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'draw-site'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;

    assert.equal(ctx.state.players.north.atlas.length, before.atlas.length - 1);
    assert.equal(ctx.state.players.north.hand.atlas.length, before.hand.atlas.length + 1);
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.deepEqual(result.receipt.events[0]?.payload, { seat: 'north' });
    assert.equal(result.receipt.events[0]?.type, 'site-drawn');
    assert.equal(canonicalJson(result.receipt.events[0]?.payload ?? null).includes(drawn.cardId), false);
    assert.equal(canonicalJson(ctx.observe('south')).includes(drawn.cardId), false);
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
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

    const before = ctx.state.players.north;
    const drawn = before.spellbook[0];
    assert.ok(drawn);
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'draw-spell'));
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.players.north.spellbook.length, before.spellbook.length - 1);
    assert.equal(ctx.state.players.north.hand.spellbook.length, before.hand.spellbook.length + 1);
    assert.equal(ctx.session.transcript.at(-1)?.events[0]?.type, 'spell-drawn');
    assert.doesNotMatch(canonicalJson(ctx.session.transcript.at(-1)?.events[0]?.payload ?? null), /north-spell-/);
    assert.doesNotMatch(canonicalJson(ctx.observe('south')), new RegExp(drawn.cardId));
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02 drawing a site from an empty Atlas pays the tap cost and loses', async () => {
  await withNorthSecondMain(31, true, undefined, async (ctx) => {
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
  await withNorthSecondMain(37, false, undefined, async (ctx) => {
    const beforeState = canonicalJson(ctx.state);
    const transcriptLength = ctx.session.transcript.length;
    const result = await ctx.stepRequest({
      actionId: 'sha256:2222222222222222222222222222222222222222222222222222222222222222',
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });

    assert.equal(result.accepted, false);
    if (result.accepted) return;
    assert.equal(result.reason.code, 'unknown_action');
    assert.equal(canonicalJson(result.session.state), beforeState);
    assert.equal(result.session.transcript.length, transcriptLength);
  });
});

test('RULE-03 Sinkhole sacrifices sites into neutral Rubble and preserves relative subsurface', async () => {
  const base = manifest(244);
  await withPreview(base, async (preview) => {
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
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const sourceCard = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === sourceCardId);
      const protectedCard = ctx.state.players.north.hand.atlas.find(({ cardId }) =>
        cardId === protectedCardId);
      const targetCard = ctx.state.players.south.hand.atlas.find(({ cardId }) => cardId === targetCardId);
      const replacementCard = ctx.state.players.south.hand.atlas.find(({ cardId }) =>
        cardId === replacementCardId);
      const drownedCard = ctx.state.players.south.hand.spellbook.find(({ cardId }) =>
        cardId === drownedCardId);
      const survivorCard = ctx.state.players.south.hand.spellbook.find(({ cardId }) =>
        cardId === survivorCardId);
      const artifactCard = ctx.state.players.south.hand.spellbook.find(({ cardId }) =>
        cardId === artifactCardId);
      assert.ok(sourceCard);
      assert.ok(protectedCard);
      assert.ok(targetCard);
      assert.ok(replacementCard);
      assert.ok(drownedCard);
      assert.ok(survivorCard);
      assert.ok(artifactCard);

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === protectedCard.instanceId
          && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId !== targetCard.instanceId
          && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === sourceCard.instanceId
          && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === targetCard.instanceId
          && descriptor.cell === 'C2');
      const secondDrownedCard = ctx.state.players.south.hand.spellbook.find(({ cardId }) =>
        cardId === secondDrownedCardId);
      assert.ok(secondDrownedCard);
      for (const card of [drownedCard, secondDrownedCard]) {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === card.instanceId
            && descriptor.cell === 'C2'
            && descriptor.region === 'underwater');
      }
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === survivorCard.instanceId
          && descriptor.cell === 'C2'
          && descriptor.region === 'underwater');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'cast-artifact'
          && descriptor.cardInstanceId === artifactCard.instanceId
          && descriptor.bearer?.instanceId === survivorCard.instanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'drop-artifacts'
          && descriptor.unit.instanceId === survivorCard.instanceId
          && descriptor.artifactInstanceIds[0] === artifactCard.instanceId);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'atlas');

      const checkpointSiteC4 = ctx.state.realm.sites.C4;
      const checkpointVersion = ctx.state.stateVersion;
      const actions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'activate-site-destruction'
          && descriptor.sourceSiteInstanceId === sourceCard.instanceId);
      assert.deepEqual(actions.flatMap(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
        ? [descriptor.targetCell]
        : []), ['C2', 'C3', 'C4']);
      const protectedActivation = actions.find(({ descriptor }) =>
        descriptor.kind === 'activate-site-destruction' && descriptor.targetCell === 'C4');
      assert.ok(protectedActivation);
      await withFork(ctx, async (protectedFork) => {
        const protectedResult = await protectedFork.step(protectedActivation);
        assert.equal(protectedResult.accepted, true);
        if (!protectedResult.accepted) return;
        assert.deepEqual(protectedResult.receipt.events.map(({ type }) => type), [
          'site-sacrificed',
          'site-destruction-prevented',
          'rubble-created',
        ]);
        assert.deepEqual(protectedResult.session.state.realm.sites.C4, checkpointSiteC4);
        assert.equal(protectedResult.session.state.players.north.cemetery.some(({ instanceId }) =>
          instanceId === sourceCard.instanceId), true);
        assert.equal(protectedResult.session.state.players.north.cemetery.some(({ instanceId }) =>
          instanceId === protectedCard.instanceId), false);
        assert.equal(await protectedFork.verifyReplay(), true);
      });
      const activation = actions.find(({ descriptor }) =>
        descriptor.kind === 'activate-site-destruction' && descriptor.targetCell === 'C2');
      assert.ok(activation);
      const result = await ctx.step(activation);
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      assert.equal(ctx.state.stateVersion, checkpointVersion + 1);
      const stale = await ctx.step(activation);
      assert.equal(stale.accepted, false);
      assert.equal(stale.reason.code, 'stale_version');
      assert.deepEqual(result.receipt.events.map(({ type }) => type), [
        'site-sacrificed',
        'site-destroyed',
        'rubble-created',
        'rubble-created',
      ]);
      const drownedInstanceIds = [drownedCard.instanceId, secondDrownedCard.instanceId].sort();
      assert.equal(ctx.state.phase, 'deathrite-order');
      assert.equal(ctx.state.decisionSeat, 'south');
      assert.equal(drownedInstanceIds.every((instanceId) => !ctx.state.realm.units
        .some((unit) => unit.instanceId === instanceId)), true);
      assert.equal(drownedInstanceIds.every((instanceId) => !ctx.state.players.south.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      const orderActions = (await ctx.legalActions('south')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
        descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(),
      drownedInstanceIds);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === sourceCard.instanceId), true);
      assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
        instanceId === targetCard.instanceId), true);
      await ctx.accept(orderActions[0]!);
      assert.equal(drownedInstanceIds.every((instanceId) => ctx.state.players.south.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      const survivor = ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === survivorCard.instanceId);
      assert.equal(survivor?.region, 'underground');
      const buriedArtifact = ctx.state.realm.artifacts?.find(({ instanceId }) =>
        instanceId === artifactCard.instanceId);
      assert.deepEqual(buriedArtifact, {
        cardId: artifactCard.cardId,
        instanceId: artifactCard.instanceId,
        location: 'C2',
        owner: 'south',
        region: 'underground',
        source: artifactCard.source,
      });
      assert.deepEqual(ctx.observe('north').realm.sites.C2, {
        cardId: 'rubble',
        controller: null,
        elements: [],
        instanceId: ctx.state.realm.sites.C2?.instanceId,
        rubble: true,
      });
      assert.equal(ctx.observe('north').players.north.affinity.earth, 1);
      assert.equal(ctx.observe('north').players.south.affinity.water, 0);

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'rubble-replaced',
        'site-played',
      ]);
      assert.equal(ctx.state.realm.sites.C3?.controller, 'north');
      assert.equal('rubble' in ctx.state.realm.sites.C3!, false);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const replacement = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === replacementCard.instanceId
          && descriptor.cell === 'C2'));
      assert.equal(replacement.accepted, true);
      if (!replacement.accepted) return;
      assert.deepEqual(replacement.receipt.events.map(({ type }) => type), [
        'rubble-replaced',
        'site-played',
      ]);
      assert.deepEqual(ctx.state.realm.artifacts?.find(({ instanceId }) =>
        instanceId === artifactCard.instanceId), { ...buriedArtifact, region: 'underwater' });
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03/04 a Spellcaster pays mana and summons a minion atop a controlled site', async () => {
  await withSetup(manifest(41), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    const before = ctx.state.players.north;
    const summons = (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'summon-minion');
    assert.equal(ctx.observe('north').players.north.affinity.earth, 1);
    assert.equal(summons.length, 3);
    assert.ok(summons.every(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cell === 'C4'
        && descriptor.casterInstanceId === before.avatar.card.instanceId
        && descriptor.manaCost === 1));

    const summon = summons[0];
    assert.ok(summon);
    const result = await ctx.step(summon);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const unit = ctx.state.realm.units[0];
    assert.ok(unit);
    assert.equal(ctx.state.players.north.hand.spellbook.length, before.hand.spellbook.length - 1);
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
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const emptyCast = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target === undefined));
    assert.equal(emptyCast.accepted, true);
    if (!emptyCast.accepted) return;
    assert.deepEqual(emptyCast.receipt.events.map(({ type }) => type), ['magic-cast', 'magic-resolved']);
    assert.equal(ctx.state.realm.units.some(({ source }) => source === 'token'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'B2');
    const attacker = ctx.state.realm.units.find(({ controller, location }) =>
      controller === 'south' && location === 'B2');
    assert.ok(attacker);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

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
    assert.deepEqual(ctx.observe('south').realm.units
      .filter(({ token }) => token)
      .map(({ attack, defense, instanceId }) => ({ attack, defense, instanceId })),
    tokens.map(({ instanceId }) => ({ attack: 1, defense: 1, instanceId })));
    assert.deepEqual(summonEvents.map(({ payload }) =>
      (payload as { instanceId: string }).instanceId), tokens.map(({ instanceId }) => instanceId));
    assert.deepEqual(summoned.receipt.randomDraws, []);
    const killed = tokens[0]!;
    const survivor = tokens[1]!;

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attacker.instanceId
      && descriptor.to.cell === 'B3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
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
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    assert.equal(ctx.state.players.north.mana, 1);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
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
    const southBefore = canonicalJson(ctx.observe('south'));
    assert.ok(eligible.every(({ cardId, instanceId }) =>
      !southBefore.includes(cardId) && !southBefore.includes(instanceId)));
    await withFork(ctx, async (firstFork) => {
      const first = await firstFork.step(cast);
      await withFork(ctx, async (secondFork) => {
        const second = await secondFork.step(cast);
        assert.equal(first.accepted, true);
        assert.equal(second.accepted, true);
        if (!first.accepted || !second.accepted) return;
        assert.deepEqual(second.receipt.randomDraws, first.receipt.randomDraws);
        assert.deepEqual(second.receipt.events, first.receipt.events);
        assert.equal(secondFork.stateHash(), firstFork.stateHash());
      });
      assert.equal(first.accepted, true);
      if (!first.accepted) return;

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
      assert.equal(firstFork.state.players.north.mana, 2);
      assert.equal(firstFork.state.players.north.hand.atlas.length,
        handBefore.atlas.length - (discardedZone === 'atlas' ? 1 : 0));
      assert.equal(firstFork.state.players.north.hand.spellbook.length,
        handBefore.spellbook.length - 1 - (discardedZone === 'spellbook' ? 1 : 0));
      assert.equal(firstFork.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === discardedInstanceId), true);
      assert.equal(firstFork.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === castInstanceId), false);
      assert.ok(first.receipt.randomDraws.length > 0);
      assert.ok(first.receipt.randomDraws.every(({ domain, purpose }) => {
        if (typeof domain !== 'object' || domain === null || Array.isArray(domain)) return false;
        const drawDomain = domain as Readonly<Record<string, unknown>>;
        return purpose === 'summon_random_card_discard_cost'
          && drawDomain.kind === 'card_index_candidate'
          && drawDomain.exclusiveMaximum === eligible.length;
      }));
      const southAfter = canonicalJson(firstFork.observe('south'));
      assert.ok(southAfter.includes(String(discardedCardId)));
      assert.ok(southAfter.includes(String(discardedInstanceId)));
      assert.equal(await firstFork.verifyReplay(), true);
    });
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

  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players;
      return [wendigoId, localMinionId, submergedMinionId].every((cardId) =>
        opening.north.hand.spellbook.some((card) => card.cardId === cardId))
        && opening.north.spellbook[0]?.cardId === secondLocalMinionId
        && opening.south.hand.spellbook.some((card) => card.cardId === enemyMinionId);
    },
  );
  assert.equal(
    gameManifest.cards[wendigoId]?.cardType === 'minion'
      && gameManifest.cards[wendigoId].sacrificeMinionAtSummoningLocationForManaDiscount,
    2,
  );

  const readyToSummon = async (ctx: SetupCtx): Promise<void> => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === localMinionId
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === submergedMinionId
      && descriptor.cell === 'C4'
      && descriptor.region === 'underwater');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === enemyMinionId
      && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
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

    const checkpointHash = ctx.stateHash();
    const checkpointVersion = ctx.state.stateVersion;
    const transcriptLength = ctx.session.transcript.length;
    const forged = await ctx.stepRequest({
      actionId: `${cast.actionId}:forged`,
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    assert.equal(forged.reason?.code, 'unknown_action');
    assert.equal(ctx.stateHash(), checkpointHash);
    assert.equal(forged.session.transcript.length, transcriptLength);

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
    assert.equal(ctx.state.stateVersion, checkpointVersion + 1);
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

    const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
      ctx.checkpoint(),
    )));
    assert.equal(
      canonicalJson(restored as unknown as JsonValue),
      canonicalJson(ctx.session as unknown as JsonValue),
    );
    await withFork(ctx, async (restoredFork) => {
      assert.deepEqual(
        (await restoredFork.legalActions('north')).map(({ actionId }) => actionId),
        orderActions.map(({ actionId }) => actionId),
      );
    });

    const branchHashes: string[] = [];
    for (const orderAction of orderActions) {
      assert.equal(orderAction.descriptor.kind, 'order-deathrites');
      if (orderAction.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
      const chosenInstanceId = orderAction.descriptor.sourceInstanceId;
      const otherInstanceId = orderedLocals.find(({ instanceId }) =>
        instanceId !== chosenInstanceId)?.instanceId;
      assert.ok(otherInstanceId);
      await withFork(ctx, async (fork) => {
        const ordered = await fork.step(orderAction);
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
        assert.equal(fork.state.phase, 'main');
        assert.equal(fork.state.decisionSeat, 'north');
        assert.deepEqual(fork.state.terminal, { status: 'active' });
        assert.equal(fork.state.pendingDeathrites, undefined);
        assert.equal(fork.state.players.north.mana, 2);
        assert.equal(fork.state.players.north.atlas.length, atlasBefore - 2);
        assert.equal(fork.state.players.north.hand.atlas.length, atlasHandBefore + 2);
        assert.equal(orderedLocals.every(({ instanceId }) => fork.state.players.north.cemetery
          .some((card) => card.instanceId === instanceId)), true);
        assert.equal(fork.state.players.north.hand.spellbook.some(({ cardId }) =>
          cardId === wendigoId), false);
        assert.equal(fork.state.players.north.cemetery.some(({ cardId }) =>
          cardId === wendigoId), false);
        const summonedWendigos = fork.state.realm.units.filter(({ cardId }) => cardId === wendigoId);
        assert.equal(summonedWendigos.length, 1);
        assert.deepEqual(summonedWendigos.map(({ location, region }) => ({ location, region })), [{
          location: 'C4',
          region: 'surface',
        }]);
        assert.equal(await fork.verifyReplay(), true);
        branchHashes.push(fork.stateHash());
      });
    }
    assert.equal(new Set(branchHashes).size, 1);
  });

  await withSetup(deathriteManifest, async (ctx) => {
    await readyToSummon(ctx);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    assert.equal(ctx.state.players.north.atlas.length, 1);
    const terminalLocals = ctx.state.realm.units.filter(({ cardId }) =>
      cardId === localMinionId || cardId === secondLocalMinionId);
    assert.equal(terminalLocals.length, 2);
    const terminalManaBefore = ctx.state.players.north.mana;
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardId === wendigoId
        && descriptor.cell === 'C4'
        && descriptor.manaCost === 2
        && descriptor.sacrificedMinionInstanceIds?.length === 2);
    assert.equal(ctx.state.phase, 'deathrite-order');
    const terminalResult = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'order-deathrites'));
    assert.equal(terminalResult.accepted, true);
    if (!terminalResult.accepted) throw new Error('expected terminal Deathrite order to resolve');
    assert.deepEqual(terminalResult.receipt.events.map(({ type }) => type), [
      'deathrite-order-committed',
      'site-drawn',
      'minion-died',
      'minion-died',
      'game-ended',
    ]);
    assert.deepEqual(ctx.state.terminal, {
      loser: 'north',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'south',
    });
    assert.equal(ctx.state.phase, 'terminal');
    assert.equal(ctx.state.pendingDeathrites, undefined);
    assert.equal(ctx.state.players.north.mana, terminalManaBefore - 2);
    assert.equal(ctx.state.realm.units.some(({ cardId }) =>
      cardId === wendigoId), false);
    assert.equal(terminalResult.receipt.events.some(({ type }) => type === 'minion-summoned'), false);
    assert.equal(terminalLocals.every(({ instanceId }) => ctx.state.players.north.cemetery
      .some((card) => card.instanceId === instanceId)), true);
  });
});

test('RULE-03 Hamlet reduces only Ordinary minion mana payments at that site', async () => {
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

  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => session.state.players.north.hand.spellbook
      .some(({ cardId }) => cardId === 'ordinary-one'),
    { from: 1, to: 512 },
  );
  assert.deepEqual(gameManifest.cards.hamlet,
    { cardType: 'site', elements: ['earth'], ordinaryMinionManaDiscount: 1 });
  assert.equal(gameManifest.cards['ordinary-one']?.cardType === 'minion'
    && gameManifest.cards['ordinary-one'].ordinary, true);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === 'hamlet' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === 'hamlet' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === 'ordinary-site' && descriptor.cell === 'C3');

    const ordinary = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === 'ordinary-one'
        ? [descriptor]
        : []);
    assert.deepEqual(ordinary.map(({ cell, manaCost }) => ({ cell, manaCost })), [
      { cell: 'C3', manaCost: 1 },
      { cell: 'C4', manaCost: 0 },
    ]);

    const manaBefore = ctx.state.players.north.mana;
    const cast = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'ordinary-one'
        && descriptor.cell === 'C4'));
    assert.equal(cast.accepted, true);
    if (!cast.accepted) throw new Error('expected Hamlet-discounted summon to be accepted');
    const summoned = cast.receipt.events.find(({ type }) => type === 'minion-summoned');
    assert.ok(summoned && typeof summoned.payload === 'object' && !Array.isArray(summoned.payload));
    assert.equal((summoned.payload as Readonly<Record<string, unknown>>).manaPaid, 0);
    assert.equal(ctx.state.players.north.mana, manaBefore);
    assert.equal(await ctx.verifyReplay(), true);
  });
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

  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const preview = session.state.players;
      const northAvailable = [
        ...preview.north.hand.spellbook,
        ...preview.north.spellbook.slice(0, 2),
      ].map(({ cardId }) => cardId);
      const southOpening = preview.south.hand.spellbook.map(({ cardId }) => cardId);
      return [activeCasterId, disabledCasterId, freezeId, summonId, tappedCasterId]
        .every((cardId) => northAvailable.includes(cardId))
        && preview.north.hand.spellbook.some(({ cardId }) => cardId === tappedCasterId)
        && southOpening.includes(targetId)
        && southOpening.includes(stealthedTargetId);
    },
    { from: 980, to: 2_979 },
  );
  assert.equal(
    gameManifest.cards[activeCasterId]?.cardType === 'minion'
      && (gameManifest.cards[activeCasterId] as unknown as { spellcaster?: boolean }).spellcaster,
    true,
  );
  assert.equal('spellcaster' in gameManifest.cards[summonId]!, false);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === tappedCasterId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetId && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === stealthedTargetId && descriptor.cell === 'C1');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetId);
    const stealthedTarget = ctx.state.realm.units.find(({ cardId }) =>
      cardId === stealthedTargetId);
    assert.ok(target);
    assert.ok(stealthedTarget);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const tappedCaster = ctx.state.realm.units.find(({ cardId }) => cardId === tappedCasterId);
    assert.ok(tappedCaster);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === tappedCaster.instanceId
      && descriptor.from.cell === 'C4'
      && descriptor.to.cell === 'C3'
      && descriptor.path.length === 2);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === activeCasterId
      && descriptor.casterInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
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
    await ctx.take(({ descriptor }) => descriptor.kind === 'activate-mana'
      && descriptor.unitInstanceId === tappedCaster.instanceId);
    assert.deepEqual({
      stealthed: activeCaster.stealthed,
      summoningSickness: activeCaster.summoningSickness,
      tapped: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === activeCaster.instanceId)?.tapped,
    }, { stealthed: true, summoningSickness: true, tapped: false });
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === tappedCaster.instanceId)?.tapped, true);
    assert.equal(ctx.observe('north').realm.units.find(({ instanceId }) =>
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
      ctx.state.players.north.avatar.card.instanceId,
      tappedCaster.instanceId,
    ].sort());
    assert.equal(summonCasters.has(disabledCaster.instanceId), false);

    await withFork(ctx, async (frozen) => {
      await frozen.accept(targetFreezes[0]!);
      assert.deepEqual(frozen.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'magic-cast',
        'stealth-lost',
        'minion-disabled',
        'magic-resolved',
      ]);
      assert.equal((frozen.session.transcript.at(-1)?.events[0]?.payload as unknown as
        Readonly<Record<string, unknown>>).casterInstanceId,
      activeCaster.instanceId);
      assert.equal((frozen.session.transcript.at(-1)?.events[2]?.payload as unknown as
        Readonly<Record<string, unknown>>).sourceInstanceId, freeze.instanceId);
      const casterAfterMagic = frozen.state.realm.units.find(({ instanceId }) =>
        instanceId === activeCaster.instanceId);
      assert.deepEqual({
        stealthed: casterAfterMagic?.stealthed,
        summoningSickness: casterAfterMagic?.summoningSickness,
        tapped: casterAfterMagic?.tapped,
      }, { stealthed: false, summoningSickness: true, tapped: false });
      assert.equal(await frozen.verifyReplay(), true);
    });

    await withFork(ctx, async (summoned) => {
      await summoned.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === summonedCard.instanceId
          && descriptor.casterInstanceId === activeCaster.instanceId
          && descriptor.cell === 'C2');
      assert.deepEqual(summoned.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'stealth-lost',
        'minion-summoned',
      ]);
      assert.equal((summoned.session.transcript.at(-1)?.events[1]?.payload as unknown as
        Readonly<Record<string, unknown>>).casterInstanceId,
      activeCaster.instanceId);
      assert.equal(summoned.state.realm.units.find(({ instanceId }) =>
        instanceId === summonedCard.instanceId)?.location, 'C2');
      assert.equal(await summoned.verifyReplay(), true);
    });
  });
});

test('RULE-03/05 targeted Magic is a non-unit source and resolves damage, Deathrite, and cemetery entry', async () => {
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
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const target = ctx.state.realm.units.find(({ controller }) => controller === 'south');
    assert.ok(target);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const spell = ctx.state.players.north.hand.spellbook[0];
    assert.ok(spell);
    const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
    assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target
      ? [`${descriptor.target.kind}:${descriptor.target.seat}:${descriptor.target.instanceId}`]
      : []).sort(), [
      `avatar:north:${ctx.state.players.north.avatar.card.instanceId}`,
      `avatar:south:${ctx.state.players.south.avatar.card.instanceId}`,
      `minion:south:${target.instanceId}`,
    ].sort());
    const before = ctx.state.players;
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === spell.instanceId
        && descriptor.target !== undefined
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === target.instanceId);

    assert.equal(ctx.state.players.north.mana, before.north.mana - 1);
    assert.equal(ctx.state.players.north.hand.spellbook.length, before.north.hand.spellbook.length - 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === spell.instanceId), true);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === target.instanceId), false);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === target.instanceId), true);
    assert.equal(ctx.state.players.south.atlas.length, before.south.atlas.length - 1);
    assert.equal(ctx.state.players.south.hand.atlas.length, before.south.hand.atlas.length + 1);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'magic-damage-allocated',
      'damage-dealt',
      'site-drawn',
      'minion-died',
      'magic-resolved',
    ]);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.target !== undefined
        && descriptor.target.kind === 'avatar'
        && descriptor.target.seat === 'south');
    assert.equal(ctx.state.players.south.avatar.life, 0);
    assert.deepEqual(ctx.state.terminal, { status: 'active' });

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const finalSpell = ctx.state.players.north.hand.spellbook[0];
    assert.ok(finalSpell);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === finalSpell.instanceId
        && descriptor.target !== undefined
        && descriptor.target.kind === 'avatar'
        && descriptor.target.seat === 'south');
    assert.deepEqual(ctx.state.terminal, {
      loser: 'south',
      reason: 'avatar_defeated',
      status: 'finished',
      winner: 'north',
    });
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === finalSpell.instanceId), true);
    assert.equal(ctx.session.transcript.at(-1)?.events.at(-1)?.type, 'game-ended');
    assert.equal(ctx.session.transcript.at(-1)?.events.at(-2)?.type, 'magic-resolved');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 Duel makes a chosen ally fight a same-square targeted enemy', async () => {
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

  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players.north.hand.spellbook;
      return [duelId, allyId, casterId].every((cardId) =>
        opening.some((card) => card.cardId === cardId));
    },
    { from: 1, to: 99 },
  );
  assert.equal(gameManifest.cards[duelId]?.cardType === 'magic'
    && gameManifest.cards[duelId].fightAllyWithAdjacentEnemy, true);
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === allyId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === casterId && descriptor.cell === 'C4'
      && descriptor.region === 'underground');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    for (const targetId of [normalTargetId, wardedTargetId, stealthedTargetId, disabledTargetId]) {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === targetId && descriptor.cell === 'C4');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === allyId);
    const caster = ctx.state.realm.units.find(({ cardId }) => cardId === casterId);
    const normalTarget = ctx.state.realm.units.find(({ cardId }) => cardId === normalTargetId);
    const wardedTarget = ctx.state.realm.units.find(({ cardId }) => cardId === wardedTargetId);
    const stealthedTarget = ctx.state.realm.units.find(({ cardId }) => cardId === stealthedTargetId);
    const disabledTarget = ctx.state.realm.units.find(({ cardId }) => cardId === disabledTargetId);
    assert.ok(ally);
    assert.ok(caster);
    assert.ok(normalTarget);
    assert.ok(wardedTarget);
    assert.ok(stealthedTarget);
    assert.ok(disabledTarget);
    const duelActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
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
    assert.equal(targetIds.includes(ctx.state.players.south.avatar.card.instanceId), false);
    const normalAction = duelActions.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === normalTarget.instanceId);
    const wardedAction = duelActions.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === wardedTarget.instanceId);
    const disabledAction = duelActions.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === disabledTarget.instanceId);
    assert.ok(normalAction);
    assert.ok(wardedAction);
    assert.ok(disabledAction);

    await withFork(ctx, async (foughtFork) => {
      const fought = await foughtFork.step(normalAction);
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
      const fightingAlly = foughtFork.state.realm.units.find(({ instanceId }) =>
        instanceId === ally.instanceId);
      assert.ok(fightingAlly);
      assert.deepEqual({
        damage: fightingAlly.damage,
        location: fightingAlly.location,
        tapped: fightingAlly.tapped,
      }, { damage: 2, location: 'C4', tapped: false });
      assert.equal(foughtFork.state.realm.units.some(({ instanceId }) =>
        instanceId === normalTarget.instanceId), false);
      assert.equal(foughtFork.state.players.south.cemetery.some(({ instanceId }) =>
        instanceId === normalTarget.instanceId), true);
      assert.equal(foughtFork.state.players.north.mana, 0);
      assert.equal(foughtFork.state.players.north.cemetery.some(({ cardId }) => cardId === duelId), true);
      assert.equal(await foughtFork.verifyReplay(), true);
    });

    await withFork(ctx, async (wardedFork) => {
      const warded = await wardedFork.step(wardedAction);
      assert.equal(warded.accepted, true);
      assert.deepEqual(warded.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'ward-broken',
        'magic-resolved',
      ]);
      assert.equal(wardedFork.state.realm.units.find(({ instanceId }) =>
        instanceId === ally.instanceId)?.damage, 0);
      assert.equal(wardedFork.state.realm.units.find(({ instanceId }) =>
        instanceId === wardedTarget.instanceId)?.warded, false);
      assert.equal(await wardedFork.verifyReplay(), true);
    });

    await withFork(ctx, async (disabledFork) => {
      const disabled = await disabledFork.step(disabledAction);
      assert.equal(disabled.accepted, true);
      assert.equal(disabledFork.state.realm.units.find(({ instanceId }) =>
        instanceId === ally.instanceId)?.damage, 0);
      assert.equal(disabledFork.state.realm.units.some(({ instanceId }) =>
        instanceId === disabledTarget.instanceId), false);
      assert.equal(await disabledFork.verifyReplay(), true);
    });
  });
});

test('RULE-03/04 Leap Attack optionally steps an ally before it strikes every enemy there', async () => {
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
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players.north.hand.spellbook;
      return [leapId, allyId].every((cardId) => opening.some((card) => card.cardId === cardId));
    },
    { from: 1, to: 99 },
  );
  assert.equal(gameManifest.cards[leapId]?.cardType === 'magic'
    && gameManifest.cards[leapId].leapAttackAlly, true);
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === allyId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === originEnemyId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === immobileId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === disabledId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    for (const enemyId of [normalEnemyId, wardedEnemyId, stealthedEnemyId]) {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === enemyId && descriptor.cell === 'C3');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === allyId);
    const immobile = ctx.state.realm.units.find(({ cardId }) => cardId === immobileId);
    const disabled = ctx.state.realm.units.find(({ cardId }) => cardId === disabledId);
    const originEnemy = ctx.state.realm.units.find(({ cardId }) => cardId === originEnemyId);
    const normalEnemy = ctx.state.realm.units.find(({ cardId }) => cardId === normalEnemyId);
    const wardedEnemy = ctx.state.realm.units.find(({ cardId }) => cardId === wardedEnemyId);
    const stealthedEnemy = ctx.state.realm.units.find(({ cardId }) => cardId === stealthedEnemyId);
    assert.ok(ally);
    assert.ok(immobile);
    assert.ok(disabled);
    assert.ok(originEnemy);
    assert.ok(normalEnemy);
    assert.ok(wardedEnemy);
    assert.ok(stealthedEnemy);
    const actionsFor = async (instanceId: string): Promise<readonly GameLegalAction[]> =>
      (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardId === leapId
          && descriptor.ally?.instanceId === instanceId);
    assert.deepEqual((await actionsFor(ally.instanceId)).flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyDestination
        ? [`${descriptor.allyDestination.cell}/${descriptor.allyDestination.region}`]
        : []), ['C3/surface', 'C4/surface']);
    assert.deepEqual((await actionsFor(immobile.instanceId)).flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyDestination
        ? [descriptor.allyDestination.cell]
        : []), ['C4']);
    assert.deepEqual((await actionsFor(disabled.instanceId)).flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyDestination
        ? [descriptor.allyDestination.cell]
        : []), ['C4']);
    const noStepAction = (await actionsFor(ally.instanceId)).find(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyDestination?.cell === 'C4');
    const stepAction = (await actionsFor(ally.instanceId)).find(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyDestination?.cell === 'C3');
    assert.ok(noStepAction);
    assert.ok(stepAction);
    if (stepAction.descriptor.kind !== 'cast-magic') throw new Error('expected Leap Attack cast');
    const leapInstanceId = stepAction.descriptor.cardInstanceId;

    await withFork(ctx, async (stayedFork) => {
      const stayed = await stayedFork.step(noStepAction);
      assert.equal(stayed.accepted, true);
      assert.equal(stayed.receipt.events.some(({ type }) => type === 'unit-stepped'), false);
      assert.equal(stayed.receipt.events.filter(({ type }) => type === 'strike-damage-allocated').length, 1);
      assert.equal(stayedFork.state.realm.units.some(({ instanceId }) =>
        instanceId === originEnemy.instanceId), false);
      assert.equal(stayedFork.state.realm.units.some(({ instanceId }) =>
        instanceId === normalEnemy.instanceId), true);
      assert.equal(await stayedFork.verifyReplay(), true);
    });

    const leaped = await ctx.step(stepAction);
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
    const leapedAlly = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === ally.instanceId);
    assert.deepEqual({
      damage: leapedAlly?.damage,
      location: leapedAlly?.location,
      tapped: leapedAlly?.tapped,
    }, { damage: 0, location: 'C3', tapped: false });
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === normalEnemy.instanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === stealthedEnemy.instanceId), false);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === wardedEnemy.instanceId)?.warded, false);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === originEnemy.instanceId), true);
    assert.equal(ctx.state.players.north.mana, 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ cardId }) => cardId === leapId), true);
    assert.equal(leaped.receipt.events.at(-1)?.type, 'magic-resolved');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 Leap Attack resumes its strike after ordered movement Deathrites', async () => {
  const decks = {
    north: deck('leap-order-north', 6, 8),
    south: deck('leap-order-south', 6, 8),
  };
  const cards = cardsFor(decks, {
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['fire'] });
  const leapId = decks.north.spellbook[0]!;
  const sourceId = decks.north.spellbook[1]!;
  const fragileIds = decks.north.spellbook.slice(2, 4);
  const rainId = decks.north.spellbook[4]!;
  const enemyId = decks.south.spellbook[0]!;
  cards[leapId] = {
    cardType: 'magic',
    leapAttackAlly: true,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  cards[sourceId] = {
    ...cards[sourceId]!,
    attack: 3,
    otherNearbyAlliesPowerBonus: 1,
  } as GameCardDefinition;
  for (const fragileId of fragileIds) {
    cards[fragileId] = {
      ...cards[fragileId]!,
      deathriteDrawSite: true,
      defense: 1,
    } as GameCardDefinition;
  }
  cards[rainId] = {
    cardType: 'magic',
    damageEachAbovegroundMinion: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  cards[enemyId] = {
    ...cards[enemyId]!,
    attack: 1,
    defense: 3,
    summonToAnySite: true,
  } as GameCardDefinition;
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-leap-attack-ordered-deathrites-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
  };
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players;
      const northOpening = new Set(opening.north.hand.spellbook.map(({ cardId }) => cardId));
      const northBySecondTurn = new Set([
        ...opening.north.hand.spellbook,
        ...opening.north.spellbook.slice(0, 1),
      ].map(({ cardId }) => cardId));
      const northByThirdTurn = new Set([
        ...opening.north.hand.spellbook,
        ...opening.north.spellbook.slice(0, 2),
      ].map(({ cardId }) => cardId));
      const southBySecondTurn = new Set([
        ...opening.south.hand.spellbook,
        ...opening.south.spellbook.slice(0, 2),
      ].map(({ cardId }) => cardId));
      return fragileIds.every((cardId) => northOpening.has(cardId))
        && northBySecondTurn.has(sourceId)
        && [leapId, rainId].every((cardId) => northByThirdTurn.has(cardId))
        && southBySecondTurn.has(enemyId);
    },
  );
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    for (const fragileId of fragileIds) {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === fragileId
        && descriptor.cell === 'C4');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === sourceId
      && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === enemyId
      && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic' && descriptor.cardId === rainId);
    const fragiles = ctx.state.realm.units.filter(({ cardId }) => fragileIds.includes(cardId));
    assert.equal(fragiles.length, 2);
    assert.equal(fragiles.every(({ damage }) => damage === 1), true);
    const source = ctx.state.realm.units.find(({ cardId }) => cardId === sourceId);
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === enemyId);
    assert.ok(source && enemy);
    assert.equal(source.location, 'C3');
    assert.deepEqual(fragiles.map(({ instanceId }) => ctx.observe('north').realm.units
      .find((unit) => unit.instanceId === instanceId)?.defense), [2, 2]);
    const atlasBefore = ctx.state.players.north.atlas.length;
    const atlasHandBefore = ctx.state.players.north.hand.atlas.length;
    const interrupted = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardId === leapId
        && descriptor.ally?.instanceId === source.instanceId
        && descriptor.allyDestination?.cell === 'C2'));
    assert.equal(interrupted.accepted, true);
    if (!interrupted.accepted) throw new Error('expected Leap Attack to reach Deathrite ordering');
    assert.deepEqual(interrupted.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'unit-stepped',
    ]);
    assert.equal(ctx.state.phase, 'deathrite-order');
    assert.equal(ctx.state.decisionSeat, 'north');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === source.instanceId)?.location, 'C2');
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === enemy.instanceId), true);
    assert.equal(fragiles.every(({ instanceId }) => !ctx.state.realm.units
      .some((unit) => unit.instanceId === instanceId)), true);
    assert.equal(fragiles.every(({ instanceId }) => !ctx.state.players.north.cemetery
      .some((card) => card.instanceId === instanceId)), true);
    const orderActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(),
    fragiles.map(({ instanceId }) => instanceId).sort());

    const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
      ctx.checkpoint(),
    )));
    assert.equal(
      canonicalJson(restored as unknown as JsonValue),
      canonicalJson(ctx.session as unknown as JsonValue),
    );
    await withFork(ctx, async (restoredFork) => {
      assert.deepEqual(
        (await restoredFork.legalActions('north')).map(({ actionId }) => actionId),
        orderActions.map(({ actionId }) => actionId),
      );
    });
    const branchHashes: string[] = [];
    for (const orderAction of orderActions) {
      assert.equal(orderAction.descriptor.kind, 'order-deathrites');
      if (orderAction.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
      const chosenInstanceId = orderAction.descriptor.sourceInstanceId;
      const otherInstanceId = fragiles.find(({ instanceId }) =>
        instanceId !== chosenInstanceId)?.instanceId;
      assert.ok(otherInstanceId);
      await withFork(ctx, async (fork) => {
        const ordered = await fork.step(orderAction);
        assert.equal(ordered.accepted, true);
        if (!ordered.accepted) throw new Error('expected Leap Attack to resume after Deathrites');
        const types = ordered.receipt.events.map(({ type }) => type);
        assert.deepEqual(types.slice(0, 5), [
          'deathrite-order-committed',
          'site-drawn',
          'site-drawn',
          'minion-died',
          'minion-died',
        ]);
        assert.equal(types.at(-1), 'magic-resolved');
        const strikeIndex = types.indexOf('strike-damage-allocated');
        assert.ok(strikeIndex > 4);
        assert.ok(strikeIndex < types.lastIndexOf('minion-died'));
        assert.deepEqual(ordered.receipt.events.filter(({ type }) => type === 'site-drawn')
          .map(({ payload }) => payload !== null && typeof payload === 'object'
            && 'sourceInstanceId' in payload ? payload.sourceInstanceId : undefined), [
          chosenInstanceId,
          otherInstanceId,
        ]);
        assert.equal(fork.state.phase, 'main');
        assert.equal(fork.state.pendingDeathrites, undefined);
        assert.equal(fork.state.realm.units.find(({ instanceId }) =>
          instanceId === source.instanceId)?.location, 'C2');
        assert.equal(fork.state.realm.units.some(({ instanceId }) =>
          instanceId === enemy.instanceId), false);
        assert.equal(fork.state.players.south.cemetery.some(({ instanceId }) =>
          instanceId === enemy.instanceId), true);
        assert.equal(fragiles.every(({ instanceId }) => fork.state.players.north.cemetery
          .some((card) => card.instanceId === instanceId)), true);
        assert.equal(fork.state.players.north.atlas.length, atlasBefore - 2);
        assert.equal(fork.state.players.north.hand.atlas.length, atlasHandBefore + 2);
        assert.equal(await fork.verifyReplay(), true);
        branchHashes.push(fork.stateHash());
      });
    }
    assert.equal(new Set(branchHashes).size, 1);
  });
});

test('RULE-03 Magic targets stay in the caster region and exclude enemy Stealth', async () => {
  const targetIsLegal = async (
    spell: SpellFacts,
    region: 'surface' | 'underground',
    targetNearby = false,
  ): Promise<boolean> => {
    const decks = { north: deck('target-north', 4, 6), south: deck('target-south', 4, 6) };
    const cards = cardsFor(decks, spell, undefined, { elements: ['air'] });
    for (const cardId of decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 1,
        ...(targetNearby ? { targetNearby: true } : {}),
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      };
    }
    let legal = false;
    await withSetup(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-magic-target-${region}-v1`,
      },
      cards,
      decks,
      firstSeat: 'north',
      seed: region === 'surface' ? 149 : 150,
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && (descriptor.region ?? 'surface') === region);
      const target = ctx.state.realm.units.find(({ controller }) => controller === 'south');
      assert.ok(target);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      assert.equal(await ctx.verifyReplay(), true);
      const actions = await ctx.legalActions('north');
      if (targetNearby) {
        assert.equal(actions.some(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.target !== undefined
            && descriptor.target.kind === 'avatar'
            && descriptor.target.seat === 'north'), true);
      }
      legal = actions.some(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.target !== undefined
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === target.instanceId);
    });
    return legal;
  };

  assert.equal(await targetIsLegal({
    manaCost: 1,
    stealth: true,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, 'surface'), false);
  assert.equal(await targetIsLegal({
    burrowing: true,
    manaCost: 1,
    mustBeCastBurrowed: true,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, 'underground'), false);
  assert.equal(await targetIsLegal({
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, 'surface', true), false);
});

test('RULE-03 Lash damages then untaps only a surviving nearby minion target', async () => {
  const decks = { north: deck('lash-north'), south: deck('lash-south') };
  const cards = cardsFor(decks, {
    defense: 2,
    manaCost: 0,
    summonToAnySite: true,
    tapForMana: 1,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  for (const cardId of decks.north.spellbook) {
    cards[cardId] = {
      cardType: 'magic',
      damageTargetUnit: 1,
      manaCost: 1,
      targetNearby: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      untapTargetMinionAfterDamage: true,
    };
  }
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-lash-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 230,
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const nearbyTarget = ctx.state.realm.units.find(({ controller, location }) =>
      controller === 'south' && location === 'C4');
    const distantTarget = ctx.state.realm.units.find(({ controller, location }) =>
      controller === 'south' && location === 'C1');
    assert.ok(nearbyTarget);
    assert.ok(distantTarget);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'activate-mana'
      && descriptor.unitInstanceId === nearbyTarget.instanceId);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === nearbyTarget.instanceId)?.tapped, true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const spell = ctx.state.players.north.hand.spellbook.find(({ cardId }) => {
      const definition = gameManifest.cards[cardId];
      return definition?.cardType === 'magic' && definition.untapTargetMinionAfterDamage === true;
    });
    assert.ok(spell);
    const lashActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
    assert.deepEqual(lashActions.map(({ descriptor }) =>
      descriptor.kind === 'cast-magic' ? descriptor.target?.instanceId : undefined),
    [nearbyTarget.instanceId]);
    assert.equal(lashActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.kind === 'avatar'), false);
    assert.equal(lashActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId === distantTarget.instanceId), false);

    const manaBefore = ctx.state.players.north.mana;
    const versionBefore = ctx.state.stateVersion;
    const result = await ctx.step(lashActions[0]!);
    assert.equal(result.accepted, true);
    const targetAfter = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === nearbyTarget.instanceId);
    assert.equal(targetAfter?.damage, 1);
    assert.equal(targetAfter?.tapped, false);
    assert.equal(ctx.state.players.north.mana, manaBefore - 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === spell.instanceId), true);
    assert.equal(ctx.state.stateVersion, versionBefore + 1);
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'magic-damage-allocated',
      'damage-dealt',
      'minion-untapped',
      'magic-resolved',
    ]);
    assert.deepEqual(result.receipt.events[3]?.payload, {
      instanceId: nearbyTarget.instanceId,
      seat: 'south',
      sourceInstanceId: spell.instanceId,
    });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Freeze disables a nearby minion until the caster next Start Phase', async () => {
  const decks = {
    north: deck('freeze-north', 6, 6),
    south: deck('freeze-south', 6, 6),
  };
  const cards = cardsFor(decks, {
    defense: 5,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const allyCardId = decks.north.spellbook[0]!;
  const targetCardId = decks.south.spellbook[0]!;
  const stealthCardId = decks.south.spellbook[1]!;
  const wardCardId = decks.south.spellbook[2]!;
  cards[allyCardId] = {
    ...cards[allyCardId]!,
    provides: 'water',
    stealth: true,
    ward: true,
  } as GameCardDefinition;
  cards[targetCardId] = {
    ...cards[targetCardId]!,
    movementBonus: 1,
    provides: 'air',
    ranged: true,
    tapForMana: 1,
  } as GameCardDefinition;
  cards[stealthCardId] = { ...cards[stealthCardId]!, stealth: true } as GameCardDefinition;
  cards[wardCardId] = { ...cards[wardCardId]!, ward: true } as GameCardDefinition;
  for (const cardId of decks.north.spellbook.slice(1)) {
    cards[cardId] = {
      cardType: 'magic',
      disableTargetNearbyMinionUntilNextTurn: true,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    };
  }
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: 'synthetic-freeze-fixture-v1',
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    }),
    (session) => {
      const preview = session.state.players;
      return preview.north.hand.spellbook.some(({ cardId }) => cardId === allyCardId)
        && preview.south.hand.spellbook.some(({ cardId }) => cardId === targetCardId)
        && preview.south.hand.spellbook.some(({ cardId }) => cardId === stealthCardId)
        && preview.south.hand.spellbook.some(({ cardId }) => cardId === wardCardId);
    },
    { from: 156, to: 555 },
  );
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === allyCardId && descriptor.cell === 'C4');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === allyCardId);
    assert.ok(ally);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetCardId && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === stealthCardId && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === wardCardId && descriptor.cell === 'C2');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetCardId);
    const stealthed = ctx.state.realm.units.find(({ cardId }) => cardId === stealthCardId);
    const wardedTarget = ctx.state.realm.units.find(({ cardId }) => cardId === wardCardId);
    assert.ok(target);
    assert.ok(stealthed);
    assert.ok(wardedTarget);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.from.cell === 'C4'
      && descriptor.to.cell === 'C3'
      && descriptor.path.length === 2);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    const freezeCards = ctx.state.players.north.hand.spellbook.filter(({ cardId }) =>
      gameManifest.cards[cardId]?.cardType === 'magic');
    assert.equal(freezeCards.length, 2);
    const firstTargets = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === freezeCards[0]?.instanceId
        && descriptor.target
        ? [descriptor.target.instanceId]
        : []);
    assert.equal(firstTargets.includes(ally.instanceId), true);
    assert.equal(firstTargets.includes(target.instanceId), true);
    assert.equal(firstTargets.includes(wardedTarget.instanceId), true);
    assert.equal(firstTargets.includes(stealthed.instanceId), false);
    assert.equal(firstTargets.includes(ctx.state.players.north.avatar.card.instanceId), false);

    await withFork(ctx, async (unfrozen) => {
      await unfrozen.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await unfrozen.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      const unfrozenKinds = (await unfrozen.legalActions('south')).flatMap(({ descriptor }) =>
        'unitInstanceId' in descriptor && descriptor.unitInstanceId === target.instanceId
          ? [descriptor.kind]
          : 'shooterInstanceId' in descriptor && descriptor.shooterInstanceId === target.instanceId
            ? [descriptor.kind]
            : []);
      assert.equal(unfrozenKinds.includes('move-and-attack'), true);
      assert.equal(unfrozenKinds.includes('shoot-projectile'), true);
      assert.equal(unfrozenKinds.includes('activate-mana'), true);
    });

    await withFork(ctx, async (alliedFork) => {
      await alliedFork.take(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === freezeCards[0]?.instanceId
          && descriptor.target?.instanceId === ally.instanceId);
      const disabledAlly = alliedFork.state.realm.units.find(({ instanceId }) =>
        instanceId === ally.instanceId);
      assert.equal(alliedFork.observe('north').realm.units.find(({ instanceId }) =>
        instanceId === ally.instanceId)?.disabled, true);
      assert.deepEqual({ stealthed: disabledAlly?.stealthed, warded: disabledAlly?.warded }, {
        stealthed: false,
        warded: false,
      });
      assert.deepEqual(alliedFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'magic-cast',
        'minion-disabled',
        'magic-resolved',
      ]);
      assert.deepEqual(alliedFork.session.transcript.at(-1)?.events[1]?.payload, {
        expiresAtSeat: 'north',
        instanceId: ally.instanceId,
        seat: 'north',
        sourceInstanceId: freezeCards[0]!.instanceId,
        stealthRemoved: true,
        wardRemoved: true,
      });
      assert.equal(await alliedFork.verifyReplay(), true);
    });

    await withFork(ctx, async (wardedFork) => {
      const warded = await wardedFork.step(await wardedFork.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === freezeCards[0]?.instanceId
          && descriptor.target?.instanceId === wardedTarget.instanceId));
      assert.equal(warded.accepted, true);
      if (!warded.accepted) return;
      assert.deepEqual(warded.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'ward-broken',
        'magic-resolved',
      ]);
      assert.equal(wardedFork.state.realm.units.find(({ instanceId }) =>
        instanceId === wardedTarget.instanceId)?.disableEffects, undefined);
      assert.equal(await wardedFork.verifyReplay(), true);
    });

    const firstFreeze = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === freezeCards[0]?.instanceId
        && descriptor.target?.instanceId === target.instanceId));
    assert.equal(firstFreeze.accepted, true);
    if (!firstFreeze.accepted) return;
    const freeze = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === freezeCards[1]?.instanceId
        && descriptor.target?.instanceId === target.instanceId));
    assert.equal(freeze.accepted, true);
    if (!freeze.accepted) return;
    const disabledTarget = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId);
    assert.deepEqual(disabledTarget?.disableEffects, [{
      expiresAtSeat: 'north',
      sourceInstanceId: freezeCards[0]!.instanceId,
    }, {
      expiresAtSeat: 'north',
      sourceInstanceId: freezeCards[1]!.instanceId,
    }]);
    assert.equal(ctx.observe('south').realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.disabled, true);
    assert.equal(ctx.observe('south').players.south.affinity.air, 0);
    assert.deepEqual(freeze.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-disabled',
      'magic-resolved',
    ]);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) =>
      type === 'minion-disable-expired'), false);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    assert.equal(ctx.observe('south').realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.disabled, true);
    const disabledKinds = (await ctx.legalActions('south')).flatMap(({ descriptor }) =>
      'unitInstanceId' in descriptor && descriptor.unitInstanceId === target.instanceId
        ? [descriptor.kind]
        : 'shooterInstanceId' in descriptor && descriptor.shooterInstanceId === target.instanceId
          ? [descriptor.kind]
          : []);
    assert.deepEqual(disabledKinds, []);
    const expiration = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(expiration.accepted, true);
    if (!expiration.accepted) return;
    assert.deepEqual(expiration.receipt.events.map(({ type }) => type), [
      'turn-ended',
      'minion-disable-expired',
      'minion-disable-expired',
      'turn-started',
    ]);
    assert.deepEqual(expiration.receipt.events[1]?.payload, {
      instanceId: target.instanceId,
      seat: 'south',
      sourceInstanceId: freezeCards[0]!.instanceId,
    });
    assert.deepEqual(expiration.receipt.events[2]?.payload, {
      instanceId: target.instanceId,
      seat: 'south',
      sourceInstanceId: freezeCards[1]!.instanceId,
    });
    const expiredTarget = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId);
    assert.equal(expiredTarget?.disableEffects, undefined);
    assert.deepEqual({ warded: expiredTarget?.warded }, { warded: false });
    assert.equal(ctx.observe('north').realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.disabled, false);
    assert.equal(ctx.observe('north').players.south.affinity.air, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 disabling a subsurface minion immediately settles its region', async () => {
  const decks = {
    north: deck('disable-region-north', 3, 3),
    south: deck('disable-region-south', 3, 3),
  };
  const cards = cardsFor(decks, {
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['water'] });
  const [casterCardId, targetCardId, freezeCardId] = decks.north.spellbook;
  assert.ok(casterCardId && targetCardId && freezeCardId);
  cards[casterCardId] = {
    ...cards[casterCardId]!,
    spellcaster: true,
    submerge: true,
    voidwalk: true,
  } as GameCardDefinition;
  cards[targetCardId] = {
    ...cards[targetCardId]!,
    submerge: true,
    voidwalk: true,
  } as GameCardDefinition;
  cards[freezeCardId] = {
    cardType: 'magic',
    disableTargetNearbyMinionUntilNextTurn: true,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-disable-region-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 0,
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');

    const freezeAt = async (region: 'underwater' | 'void', cell: 'C4' | 'B4') =>
      withFork(ctx, async (fork) => {
        await fork.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === casterCardId && descriptor.cell === cell
          && descriptor.region === region);
        await fork.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === targetCardId && descriptor.cell === cell
          && descriptor.region === region);
        const caster = fork.state.realm.units.find(({ cardId }) => cardId === casterCardId);
        const target = fork.state.realm.units.find(({ cardId }) => cardId === targetCardId);
        const freeze = fork.state.players.north.hand.spellbook.find(({ cardId }) =>
          cardId === freezeCardId);
        assert.ok(caster && target && freeze);
        const result = await fork.step(await fork.action(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.cardInstanceId === freeze.instanceId
            && descriptor.casterInstanceId === caster.instanceId
            && descriptor.target?.instanceId === target.instanceId));
        assert.equal(result.accepted, true);
        if (!result.accepted) throw new Error('expected Freeze to resolve');
        assert.equal(await fork.verifyReplay(), true);
        return { caster, freeze, result, target };
      });

    const underwater = await freezeAt('underwater', 'C4');
    assert.deepEqual(underwater.result.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-disabled',
      'minion-died',
      'magic-resolved',
    ]);
    assert.equal(underwater.result.session.state.realm.units.some(({ instanceId }) =>
      instanceId === underwater.target.instanceId), false);
    assert.equal(underwater.result.session.state.realm.units.some(({ instanceId, region }) =>
      instanceId === underwater.caster.instanceId && region === 'underwater'), true);
    assert.equal(underwater.result.session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === underwater.target.instanceId), true);
    assert.equal(underwater.result.session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === underwater.freeze.instanceId), true);
    assert.equal(underwater.result.receipt.randomDraws.length, 0);

    const voided = await freezeAt('void', 'B4');
    assert.deepEqual(voided.result.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-disabled',
      'minion-banished',
      'magic-resolved',
    ]);
    assert.equal(voided.result.session.state.realm.units.some(({ instanceId }) =>
      instanceId === voided.target.instanceId), false);
    assert.equal(voided.result.session.state.realm.units.some(({ instanceId, region }) =>
      instanceId === voided.caster.instanceId && region === 'void'), true);
    assert.equal(voided.result.session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === voided.target.instanceId), false);
    assert.equal(voided.result.session.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === voided.target.instanceId), false);
    assert.equal(voided.result.session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === voided.freeze.instanceId), true);
    assert.equal(voided.result.receipt.randomDraws.length, 0);
  });
});

test('RULE-03 Lightning Bolt targets a location and deterministically damages one random unit there', async () => {
  const decks = { north: deck('bolt-north', 4, 6), south: deck('bolt-south', 4, 6) };
  const cards = cardsFor(decks, {
    defense: 5,
    manaCost: 0,
    stealth: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['air'] });
  for (const cardId of decks.north.spellbook) {
    cards[cardId] = {
      cardType: 'magic',
      damageRandomUnitAtLocation: 3,
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    };
  }
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-lightning-bolt-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 153,
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    for (let count = 0; count < 2; count += 1) {
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const bolt = ctx.state.players.north.hand.spellbook[0];
    assert.ok(bolt);
    const occupants = [
      ctx.state.players.south.avatar.card.instanceId,
      ...ctx.state.realm.units
        .filter(({ location, region }) => location === 'C1' && region === 'surface')
        .map(({ instanceId }) => instanceId),
    ].sort();
    assert.equal(occupants.length, 3);
    assert.equal(ctx.state.realm.units.every(({ stealthed }) => stealthed), true);
    const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === bolt.instanceId);
    assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.targetLocation
      ? [`${descriptor.targetLocation.cell}:${descriptor.targetLocation.region}`]
      : []), ['C1:surface', 'C4:surface']);
    assert.equal(casts.some(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target !== undefined), false);

    const versionBefore = ctx.state.stateVersion;
    const result = await ctx.step(casts.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.targetLocation?.cell === 'C1')!);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const random = result.receipt.randomDraws.at(-1);
    assert.equal(result.receipt.randomDraws.length, 1);
    assert.equal(random?.purpose, 'magic_random_unit_at_location');
    assert.deepEqual(random?.domain, {
      accepted: true,
      exclusiveMaximum: occupants.length,
      kind: 'unit_index_candidate',
    });
    assert.equal(typeof random?.result, 'number');
    const selectedId = occupants[(random!.result as number) % occupants.length]!;
    const allocation = result.receipt.events.find(({ type }) => type === 'magic-damage-allocated');
    assert.equal(allocation?.payload !== null
      && typeof allocation?.payload === 'object'
      && 'targetInstanceId' in allocation.payload
      && allocation.payload.targetInstanceId === selectedId, true);
    const selectedDamage = selectedId === ctx.state.players.south.avatar.card.instanceId
      ? 20 - ctx.state.players.south.avatar.life
      : ctx.state.realm.units.find(({ instanceId }) => instanceId === selectedId)?.damage;
    assert.equal(selectedDamage, 3);
    assert.equal(ctx.state.stateVersion, versionBefore + 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) => instanceId === bolt.instanceId), true);
    assert.equal(result.receipt.events.at(-1)?.type, 'magic-resolved');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Lucky Charm commits the random action before exposing two deterministic outcomes', async () => {
  const north = deck('charm-north', 4, 6);
  const south = deck('charm-south', 4, 6);
  const luckyCharmId = north.spellbook[0]!;
  const cards = cardsFor({ north, south }, {
    defense: 5,
    manaCost: 0,
    stealth: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['air'] });
  cards[luckyCharmId] = {
    bearerControllerChoosesExtraRandomOutcome: true,
    cardType: 'artifact',
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  for (const cardId of north.spellbook.slice(1)) {
    cards[cardId] = {
      cardType: 'magic',
      damageRandomUnitAtLocation: 3,
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    };
  }
  const build = (seed: number): GameManifest => createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-lucky-charm-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed,
  });
  const ctx = await SetupCtx.open(build(1));
  try {
    let found = false;
    for (let seed = 1; seed <= 100; seed += 1) {
      if (seed !== 1) await ctx.reset(build(seed));
      await ctx.keep();
      await ctx.keep();
      if (!ctx.state.players.north.hand.spellbook.some(({ cardId }) =>
        cardId === luckyCharmId)) continue;
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'cast-artifact'
          && descriptor.cardId === luckyCharmId
          && descriptor.bearer?.kind === 'avatar');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      for (let count = 0; count < 2; count += 1) {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const bolt = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId !== luckyCharmId);
      if (!bolt) continue;
      const beforeActions = await ctx.legalActions('north');
      assert.equal(beforeActions.some(({ descriptor }) =>
        descriptor.kind === 'resolve-random-outcome'), false);
      const casts = beforeActions.filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === bolt.instanceId
          && descriptor.targetLocation?.cell === 'C1');
      if (casts.length !== 1) continue;
      const committed = await ctx.step(casts[0]!);
      if (!committed.accepted) continue;
      const choices = (await ctx.legalActions('north')).filter(
        ({ descriptor }) => descriptor.kind === 'resolve-random-outcome',
      );
      if (choices.length !== 2) continue;
      found = true;
      assert.equal(ctx.state.phase, 'random-choice');
      assert.equal(choices.every(({ descriptor, label }) =>
        descriptor.kind === 'resolve-random-outcome'
          && label.includes('Lucky Charm chooses')), true);
      const committedReceipt = ctx.session.transcript.at(-1)!;
      assert.equal(committedReceipt.events.length, 0);
      assert.equal(committedReceipt.randomDraws.length, 2);
      assert.equal(committedReceipt.randomDraws.every(({ purpose }) =>
        purpose === 'magic_random_unit_at_location'), true);

      const chosen = choices[1]!;
      assert.equal(chosen.descriptor.kind, 'resolve-random-outcome');
      if (chosen.descriptor.kind !== 'resolve-random-outcome') return;
      const chosenId = chosen.descriptor.outcomeInstanceId;
      const occupants = [
        ctx.state.players.south.avatar.card.instanceId,
        ...ctx.state.realm.units
          .filter(({ location, region }) => location === 'C1' && region === 'surface')
          .map(({ instanceId }) => instanceId),
      ].sort();
      const offeredIds = choices.flatMap(({ descriptor }) =>
        descriptor.kind === 'resolve-random-outcome'
          ? [descriptor.outcomeInstanceId]
          : []);
      const unofferedId = occupants.find((instanceId) => !offeredIds.includes(instanceId));
      assert.ok(unofferedId);
      const forgedDescriptor = {
        kind: 'resolve-random-outcome' as const,
        outcomeInstanceId: unofferedId,
      };
      const forged = await ctx.stepRequest({
        actionId: opaqueActionId(
          'sorcery-core-v1',
          'north',
          ctx.state.stateVersion,
          forgedDescriptor,
        ),
        seat: 'north',
        stateVersion: ctx.state.stateVersion,
      });
      assert.equal(forged.accepted, false);
      if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');

      const result = await ctx.step(chosen);
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      assert.equal(result.receipt.randomDraws.length, 0);
      const allocation = result.receipt.events.find(({ type }) => type === 'magic-damage-allocated');
      assert.equal(allocation?.payload !== null
        && typeof allocation?.payload === 'object'
        && 'targetInstanceId' in allocation.payload
        && allocation.payload.targetInstanceId === chosenId, true);
      assert.equal(await ctx.verifyReplay(), true);
      break;
    }
    assert.equal(found, true);
  } finally {
    await ctx.close();
  }
});

test('RULE-03/04 Minor Explosion damages every unit at a location up to two cardinal steps away', async () => {
  const decks = { north: deck('explosion-north', 4, 6), south: deck('explosion-south', 4, 6) };
  const baseCards = cardsFor(decks, {
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['fire'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-minor-explosion-v1',
  };
  const seed = 249;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const allyCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const explosionCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    const deathriteCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
    const wardedCardId = preview.state.players.south.hand.spellbook[1]?.cardId;
    const stealthCardId = preview.state.players.south.hand.spellbook[2]?.cardId;
    assert.ok(allyCardId);
    assert.ok(explosionCardId);
    assert.ok(deathriteCardId);
    assert.ok(wardedCardId);
    assert.ok(stealthCardId);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    for (const cardId of decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        damageEachUnitAtLocationWithinTwoSteps: 3,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      };
    }
    cards[allyCardId] = {
      ...baseCards[allyCardId]!,
      summonToAnySite: true,
    } as GameCardDefinition;
    cards[deathriteCardId] = {
      ...baseCards[deathriteCardId]!,
      deathriteHeal: 3,
    } as GameCardDefinition;
    cards[wardedCardId] = { ...baseCards[wardedCardId]!, ward: true } as GameCardDefinition;
    cards[stealthCardId] = {
      ...baseCards[stealthCardId]!,
      deathriteDrawSite: true,
      stealth: true,
    } as GameCardDefinition;
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      for (const cardId of [deathriteCardId, wardedCardId, stealthCardId]) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === cardId && descriptor.cell === 'C2');
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ctx.state.players.south.avatar.card.instanceId
        && descriptor.to.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === allyCardId && descriptor.cell === 'C2');

      const explosion = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === explosionCardId);
      const ally = ctx.state.realm.units.find(({ cardId }) => cardId === allyCardId);
      const deathrite = ctx.state.realm.units.find(({ cardId }) => cardId === deathriteCardId);
      const warded = ctx.state.realm.units.find(({ cardId }) => cardId === wardedCardId);
      const stealthed = ctx.state.realm.units.find(({ cardId }) => cardId === stealthCardId);
      assert.ok(explosion);
      assert.ok(ally);
      assert.ok(deathrite);
      assert.ok(warded);
      assert.ok(stealthed);
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === explosion.instanceId);
      assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.targetLocation ? [descriptor.targetLocation.cell] : []), ['C2', 'C3', 'C4']);
      assert.equal(casts.some(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.target !== undefined), false);

      const beforeMana = ctx.state.players.north.mana;
      const versionBefore = ctx.state.stateVersion;
      const southAvatarId = ctx.state.players.south.avatar.card.instanceId;
      const result = await ctx.step(casts.find(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.targetLocation?.cell === 'C2')!);
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      const affectedIds = [
        southAvatarId,
        ally.instanceId,
        deathrite.instanceId,
        warded.instanceId,
        stealthed.instanceId,
      ].sort();
      const allocations = result.receipt.events.filter(({ type }) => type === 'magic-damage-allocated');
      assert.deepEqual(allocations.map(({ payload }) => payload).sort((left, right) =>
        String((left as { targetInstanceId: string }).targetInstanceId)
          .localeCompare(String((right as { targetInstanceId: string }).targetInstanceId))), affectedIds.map((targetInstanceId) => ({
        amount: 3,
        sourceInstanceId: explosion.instanceId,
        targetInstanceId,
      })));
      assert.equal(result.receipt.randomDraws.length, 0);
      assert.equal(ctx.state.stateVersion, versionBefore + 1);
      assert.equal(ctx.state.players.north.mana, beforeMana - 1);
      assert.equal(ctx.state.players.south.avatar.life, 17);
      assert.equal(result.receipt.events.some(({ payload, type }) => type === 'avatar-life-lost'
        && (payload as { amount: number }).amount === 3), true);
      assert.equal(ctx.state.phase, 'deathrite-order');
      assert.equal(ctx.state.decisionSeat, 'south');
      assert.equal(result.receipt.events.some(({ type }) => type === 'magic-resolved'), false);
      assert.equal(result.receipt.events.some(({ type }) =>
        type === 'avatar-healed' || type === 'site-drawn' || type === 'minion-died'), false);
      const survivingWard = ctx.state.realm.units.find(({ instanceId }) => instanceId === warded.instanceId);
      assert.deepEqual({ damage: survivingWard?.damage, warded: survivingWard?.warded }, {
        damage: 0,
        warded: false,
      });
      const deadIds = [ally.instanceId, deathrite.instanceId, stealthed.instanceId];
      assert.equal(deadIds.every((instanceId) => !ctx.state.realm.units.some((unit) =>
        unit.instanceId === instanceId)), true);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === explosion.instanceId), true);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === ally.instanceId), false);
      assert.equal([deathrite.instanceId, stealthed.instanceId].every((instanceId) =>
        !ctx.state.players.south.cemetery.some((card) => card.instanceId === instanceId)), true);
      const orderActions = (await ctx.legalActions('south')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
        descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(),
      [deathrite.instanceId, stealthed.instanceId].sort());
      const resolved = await ctx.step(orderActions.find(({ descriptor }) =>
        descriptor.kind === 'order-deathrites'
          && descriptor.sourceInstanceId === deathrite.instanceId)!);
      assert.equal(resolved.accepted, true);
      if (!resolved.accepted) return;
      assert.equal(ctx.state.stateVersion, versionBefore + 2);
      assert.equal(ctx.state.players.south.avatar.life, 20);
      assert.equal(resolved.receipt.events.some(({ payload, type }) => type === 'avatar-healed'
        && (payload as { amount: number }).amount === 3
        && (payload as { sourceInstanceId: string }).sourceInstanceId === deathrite.instanceId), true);
      assert.equal(resolved.receipt.events.some(({ payload, type }) => type === 'site-drawn'
        && (payload as { sourceInstanceId: string }).sourceInstanceId === stealthed.instanceId), true);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === ally.instanceId), true);
      assert.equal([deathrite.instanceId, stealthed.instanceId].every((instanceId) =>
        ctx.state.players.south.cemetery.some((card) => card.instanceId === instanceId)), true);
      const firstDeath = resolved.receipt.events.findIndex(({ type }) => type === 'minion-died');
      assert.equal(resolved.receipt.events.findIndex(({ type }) => type === 'avatar-healed') < firstDeath, true);
      assert.equal(resolved.receipt.events.findIndex(({ type }) => type === 'site-drawn') < firstDeath, true);
      assert.equal(resolved.receipt.events.at(-1)?.type, 'magic-resolved');
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03/04 Chain Magic stages distinct nearby hops and damages all chosen units simultaneously', async () => {
  const decks = { north: deck('chain-north', 8, 8), south: deck('chain-south', 8, 8) };
  const baseCards = cardsFor(decks, {
    defense: 2,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['air'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-chain-magic-v1',
  };
  const seed = 271;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const [chainCard, firstTargetCard, secondTargetCard] = preview.state.players.north.hand.spellbook;
    assert.ok(chainCard);
    assert.ok(firstTargetCard);
    assert.ok(secondTargetCard);

    const cards: Record<string, GameCardDefinition> = {
      ...baseCards,
      [chainCard.cardId]: {
        cardType: 'magic',
        damageChainNearbyUnits: true,
        manaCost: 2,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      },
    };
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [chainCard.cardId]: {
          cardType: 'magic',
          damageChainNearbyUnits: false,
          manaCost: 2,
          thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /damageChainNearbyUnits/);
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });
    assert.deepEqual(gameManifest.cards[chainCard.cardId], {
      cardType: 'magic',
      damageChainNearbyUnits: true,
      manaCost: 2,
      thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
    });

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === firstTargetCard.cardId && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === secondTargetCard.cardId && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');

      const checkpointLength = ctx.session.transcript.length;
      const chain = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === chainCard.cardId);
      const firstTarget = ctx.state.realm.units.find(({ cardId }) =>
        cardId === firstTargetCard.cardId);
      const secondTarget = ctx.state.realm.units.find(({ cardId }) =>
        cardId === secondTargetCard.cardId);
      assert.ok(chain);
      assert.ok(firstTarget);
      assert.ok(secondTarget);
      const avatarId = ctx.state.players.north.avatar.card.instanceId;
      const southAvatarId = ctx.state.players.south.avatar.card.instanceId;
      const starts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'begin-chain-magic' && descriptor.cardInstanceId === chain.instanceId);
      assert.deepEqual(starts.flatMap(({ descriptor }) => descriptor.kind === 'begin-chain-magic'
        ? [descriptor.target.instanceId]
        : []).sort(), [avatarId, firstTarget.instanceId].sort());
      assert.equal(starts.some(({ descriptor }) => descriptor.kind === 'begin-chain-magic'
        && descriptor.target.instanceId === secondTarget.instanceId), false);
      assert.equal(starts.some(({ descriptor }) => descriptor.kind === 'begin-chain-magic'
        && descriptor.target.instanceId === southAvatarId), false);

      const begin = starts.find(({ descriptor }) => descriptor.kind === 'begin-chain-magic'
        && descriptor.target.instanceId === firstTarget.instanceId);
      assert.ok(begin);
      const beforeMana = ctx.state.players.north.mana;
      const beginResult = await ctx.step(begin);
      assert.equal(beginResult.accepted, true);
      if (!beginResult.accepted) return;
      assert.equal(beginResult.receipt.events.length, 0);
      assert.equal(ctx.state.phase, 'chain-magic');
      assert.equal(ctx.state.players.north.mana, beforeMana);
      assert.equal(ctx.state.players.north.hand.spellbook.some(({ instanceId }) =>
        instanceId === chain.instanceId), true);

      const staged = await ctx.legalActions('north');
      const extensions = staged.filter(({ descriptor }) =>
        descriptor.kind === 'extend-chain-magic');
      assert.deepEqual(extensions.flatMap(({ descriptor }) => descriptor.kind === 'extend-chain-magic'
        ? [descriptor.target.instanceId]
        : []).sort(), [avatarId, secondTarget.instanceId].sort());
      assert.equal(extensions.some(({ descriptor }) => descriptor.kind === 'extend-chain-magic'
        && descriptor.target.instanceId === firstTarget.instanceId), false);
      assert.equal(extensions.some(({ descriptor }) => descriptor.kind === 'extend-chain-magic'
        && descriptor.target.instanceId === southAvatarId), false);
      assert.equal(staged.some(({ descriptor, label }) =>
        descriptor.kind === 'resolve-chain-magic' && /1 chosen unit \(2 mana\)/.test(label)), true);

      const forgedDescriptor = {
        kind: 'extend-chain-magic',
        target: { instanceId: firstTarget.instanceId, kind: 'minion', seat: 'north' },
      };
      const forged = await ctx.stepRequest({
        actionId: opaqueActionId(
          'sorcery-core-v1',
          'north',
          ctx.state.stateVersion,
          forgedDescriptor as unknown as EngineActionDescriptor,
        ),
        seat: 'north',
        stateVersion: ctx.state.stateVersion,
      });
      assert.equal(forged.accepted, false);
      assert.equal(forged.reason.code, 'unknown_action');

      const extend = extensions.find(({ descriptor }) => descriptor.kind === 'extend-chain-magic'
        && descriptor.target.instanceId === secondTarget.instanceId);
      assert.ok(extend);
      const extendResult = await ctx.step(extend);
      assert.equal(extendResult.accepted, true);
      if (!extendResult.accepted) return;
      assert.equal(extendResult.receipt.events.length, 0);
      assert.equal(ctx.state.players.north.mana, beforeMana);
      const finalActions = await ctx.legalActions('north');
      assert.equal(finalActions.some(({ descriptor }) => descriptor.kind === 'extend-chain-magic'), false);
      const finish = finalActions.find(({ descriptor }) => descriptor.kind === 'resolve-chain-magic');
      assert.ok(finish);
      assert.match(finish.label, /2 chosen units \(4 mana\)/);

      const result = await ctx.step(finish);
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      assert.equal(ctx.state.players.north.mana, beforeMana - 4);
      assert.deepEqual(result.receipt.events.filter(({ type }) => type === 'magic-damage-allocated')
        .map(({ payload }) => payload), [firstTarget.instanceId, secondTarget.instanceId].map((instanceId) => ({
        amount: 2,
        sourceInstanceId: chain.instanceId,
        targetInstanceId: instanceId,
      })));
      const firstDeath = result.receipt.events.findIndex(({ type }) => type === 'minion-died');
      assert.equal(firstDeath > 0, true);
      assert.equal(result.receipt.events.filter(({ type }) => type === 'damage-dealt').every((event) =>
        result.receipt.events.indexOf(event) < firstDeath), true);
      assert.equal([firstTarget.instanceId, secondTarget.instanceId].every((instanceId) =>
        ctx.state.players.north.cemetery.some((card) => card.instanceId === instanceId)), true);
      assert.equal(result.receipt.events.at(-1)?.type, 'magic-resolved');
      assert.equal(result.receipt.randomDraws.length, 0);
      assert.equal(ctx.session.transcript.length, checkpointLength + 3);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03/04 Rain of Arrows simultaneously damages every aboveground minion', async () => {
  const decks = { north: deck('rain-north', 4, 6), south: deck('rain-south', 4, 6) };
  const baseCards = cardsFor(decks, {
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['air'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-rain-of-arrows-v1',
  };
  const seed = 263;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const rainCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const deathriteCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    const burrowedCardId = preview.state.players.north.hand.spellbook[2]?.cardId;
    const voidCardId = preview.state.players.north.spellbook[0]?.cardId;
    const stealthedCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
    const wardedCardId = preview.state.players.south.hand.spellbook[1]?.cardId;
    const submergedCardId = preview.state.players.south.hand.spellbook[2]?.cardId;
    assert.ok(rainCardId);
    assert.ok(deathriteCardId);
    assert.ok(burrowedCardId);
    assert.ok(voidCardId);
    assert.ok(stealthedCardId);
    assert.ok(wardedCardId);
    assert.ok(submergedCardId);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    for (const cardId of decks.south.atlas) {
      cards[cardId] = { cardType: 'site', elements: ['water', 'air'] };
    }
    cards[rainCardId] = {
      cardType: 'magic',
      damageEachAbovegroundMinion: 1,
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    };
    cards[deathriteCardId] = {
      ...baseCards[deathriteCardId]!,
      deathriteDrawSite: true,
    } as GameCardDefinition;
    cards[burrowedCardId] = {
      ...baseCards[burrowedCardId]!,
      burrowing: true,
    } as GameCardDefinition;
    cards[voidCardId] = { ...baseCards[voidCardId]!, voidwalk: true } as GameCardDefinition;
    cards[stealthedCardId] = { ...baseCards[stealthedCardId]!, stealth: true } as GameCardDefinition;
    cards[wardedCardId] = { ...baseCards[wardedCardId]!, ward: true } as GameCardDefinition;
    cards[submergedCardId] = { ...baseCards[submergedCardId]!, submerge: true } as GameCardDefinition;

    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [rainCardId]: {
          cardType: 'magic',
          damageEachAbovegroundMinion: 0,
          manaCost: 1,
          thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /damageEachAbovegroundMinion/);
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });
    assert.deepEqual(gameManifest.cards[rainCardId], {
      cardType: 'magic',
      damageEachAbovegroundMinion: 1,
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    });

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === deathriteCardId && descriptor.cell === 'C4' && !descriptor.region);
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === burrowedCardId && descriptor.cell === 'C4'
        && descriptor.region === 'underground');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === stealthedCardId && descriptor.cell === 'C1' && !descriptor.region);
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === wardedCardId && descriptor.cell === 'C1' && !descriptor.region);
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === submergedCardId && descriptor.cell === 'C1'
        && descriptor.region === 'underwater');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === voidCardId && descriptor.cell === 'B4' && descriptor.region === 'void');

      const rain = ctx.state.players.north.hand.spellbook.find(({ cardId }) => cardId === rainCardId);
      const deathrite = ctx.state.realm.units.find(({ cardId }) => cardId === deathriteCardId);
      const burrowed = ctx.state.realm.units.find(({ cardId }) => cardId === burrowedCardId);
      const voidwalker = ctx.state.realm.units.find(({ cardId }) => cardId === voidCardId);
      const stealthed = ctx.state.realm.units.find(({ cardId }) => cardId === stealthedCardId);
      const warded = ctx.state.realm.units.find(({ cardId }) => cardId === wardedCardId);
      const submerged = ctx.state.realm.units.find(({ cardId }) => cardId === submergedCardId);
      assert.ok(rain);
      assert.ok(deathrite);
      assert.ok(burrowed);
      assert.ok(voidwalker);
      assert.ok(stealthed);
      assert.ok(warded);
      assert.ok(submerged);
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === rain.instanceId);
      assert.equal(casts.length, 1);
      assert.equal(casts[0]?.descriptor.kind === 'cast-magic'
        && casts[0].descriptor.target === undefined
        && casts[0].descriptor.targetLocation === undefined, true);

      const beforeMana = ctx.state.players.north.mana;
      const versionBefore = ctx.state.stateVersion;
      const result = await ctx.step(casts[0]!);
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      const affected = [deathrite.instanceId, stealthed.instanceId, warded.instanceId].sort();
      const allocations = result.receipt.events.filter(({ type }) => type === 'magic-damage-allocated');
      assert.deepEqual(allocations.map(({ payload }) => payload).sort((left, right) =>
        String((left as { targetInstanceId: string }).targetInstanceId)
          .localeCompare(String((right as { targetInstanceId: string }).targetInstanceId))),
      affected.map((targetInstanceId) => ({
        amount: 1,
        sourceInstanceId: rain.instanceId,
        targetInstanceId,
      })));
      assert.equal(result.receipt.events[0]?.type, 'magic-cast');
      assert.equal(result.receipt.events.at(-1)?.type, 'magic-resolved');
      const firstDeath = result.receipt.events.findIndex(({ type }) => type === 'minion-died');
      assert.equal(firstDeath > 0, true);
      assert.equal(result.receipt.events.filter(({ type }) => type === 'damage-dealt').every((event) =>
        result.receipt.events.indexOf(event) < firstDeath), true);
      assert.equal(result.receipt.events.findIndex(({ type }) => type === 'site-drawn') < firstDeath, true);
      assert.equal(result.receipt.events.some(({ type }) => type === 'stealth-lost'), false);
      assert.equal(result.receipt.randomDraws.length, 0);
      assert.equal(ctx.state.stateVersion, versionBefore + 1);
      assert.equal(ctx.state.players.north.mana, beforeMana - 1);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === rain.instanceId), true);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === deathrite.instanceId), true);
      assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
        instanceId === stealthed.instanceId), true);
      const survivingWard = ctx.state.realm.units.find(({ instanceId }) => instanceId === warded.instanceId);
      assert.deepEqual({ damage: survivingWard?.damage, warded: survivingWard?.warded }, {
        damage: 0,
        warded: false,
      });
      for (const excluded of [burrowed, voidwalker, submerged]) {
        assert.deepEqual(ctx.state.realm.units.find(({ instanceId }) =>
          instanceId === excluded.instanceId)?.damage, 0);
      }
      assert.equal(ctx.state.players.north.avatar.life, 20);
      assert.equal(ctx.state.players.south.avatar.life, 20);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03 Charge Magic grants an untargeted ally Charge only for the current turn', async () => {
  const decks = { north: deck('charge-north', 4, 6), south: deck('charge-south', 4, 6) };
  const baseCards = cardsFor(decks, {
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['fire'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-charge-magic-v1',
  };
  const seed = 250;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const printedChargeCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const chargeCardIds = preview.state.players.north.hand.spellbook.slice(1).map(({ cardId }) => cardId);
    const summonedCardId = preview.state.players.north.spellbook[0]?.cardId;
    const enemyCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
    assert.ok(printedChargeCardId);
    assert.equal(chargeCardIds.length, 2);
    assert.ok(summonedCardId);
    assert.ok(enemyCardId);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    for (const cardId of decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        grantChargeToAllyThisTurn: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      };
    }
    cards[printedChargeCardId] = {
      ...baseCards[printedChargeCardId]!,
      burrowing: true,
      charge: true,
      stealth: true,
      ward: true,
    } as GameCardDefinition;
    cards[summonedCardId] = { ...baseCards[summonedCardId]! } as GameCardDefinition;
    cards[enemyCardId] = {
      ...baseCards[enemyCardId]!,
      stealth: true,
      ward: true,
    } as GameCardDefinition;
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === printedChargeCardId
        && descriptor.cell === 'C4'
        && descriptor.region === 'underground');
      const printedCharge = ctx.state.realm.units.find(({ cardId }) => cardId === printedChargeCardId);
      assert.ok(printedCharge);
      assert.equal(printedCharge.summoningSickness, true);
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === printedCharge.instanceId), true);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === enemyCardId && descriptor.cell === 'C1');
      const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === enemyCardId);
      assert.ok(enemy);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === summonedCardId && descriptor.cell === 'C4');

      const summoned = ctx.state.realm.units.find(({ cardId }) => cardId === summonedCardId);
      const chargeCards = ctx.state.players.north.hand.spellbook.filter(({ cardId }) =>
        chargeCardIds.includes(cardId));
      assert.ok(summoned);
      assert.equal(chargeCards.length, 2);
      assert.deepEqual({
        region: ctx.state.realm.units.find(({ instanceId }) =>
          instanceId === printedCharge.instanceId)?.region,
        stealthed: ctx.state.realm.units.find(({ instanceId }) =>
          instanceId === printedCharge.instanceId)?.stealthed,
        warded: ctx.state.realm.units.find(({ instanceId }) =>
          instanceId === printedCharge.instanceId)?.warded,
      }, { region: 'underground', stealthed: true, warded: true });
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === chargeCards[0]?.instanceId);
      const allyIds = casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally ? [descriptor.ally.instanceId] : []).sort();
      assert.deepEqual(allyIds, [
        ctx.state.players.north.avatar.card.instanceId,
        printedCharge.instanceId,
        summoned.instanceId,
      ].sort());
      assert.equal(allyIds.includes(enemy.instanceId), false);
      assert.equal(casts.every(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.target === undefined), true);
      assert.equal(casts.find(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally?.instanceId === summoned.instanceId)?.label.includes('grant Charge'), true);
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === summoned.instanceId), false);

      const versionBefore = ctx.state.stateVersion;
      await withFork(ctx, async (avatarFork) => {
        await avatarFork.take(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.cardInstanceId === chargeCards[0]?.instanceId
            && descriptor.ally?.kind === 'avatar');
        assert.deepEqual(avatarFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'charge-granted',
          'magic-resolved',
        ]);
        assert.equal(avatarFork.state.realm.units.every(({ temporaryChargeSources }) =>
          temporaryChargeSources === undefined), true);
        assert.equal(await avatarFork.verifyReplay(), true);
      });

      const first = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === chargeCards[0]?.instanceId
          && descriptor.ally?.instanceId === summoned.instanceId));
      assert.equal(first.accepted, true);
      if (!first.accepted) return;
      assert.deepEqual(first.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'charge-granted',
        'magic-resolved',
      ]);
      assert.deepEqual(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === summoned.instanceId)?.temporaryChargeSources, [chargeCards[0]!.instanceId]);
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === summoned.instanceId), true);

      const second = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === chargeCards[1]?.instanceId
          && descriptor.ally?.instanceId === summoned.instanceId));
      assert.equal(second.accepted, true);
      if (!second.accepted) return;
      assert.deepEqual(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === summoned.instanceId)?.temporaryChargeSources, chargeCards.map(({ instanceId }) => instanceId));
      assert.equal(ctx.state.players.north.mana, 0);
      assert.equal(ctx.state.players.north.cemetery.filter(({ instanceId }) =>
        chargeCards.some((card) => card.instanceId === instanceId)).length, 2);
      assert.equal(ctx.state.stateVersion, versionBefore + 2);
      assert.equal(first.receipt.randomDraws.length + second.receipt.randomDraws.length, 0);

      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === summoned.instanceId
        && descriptor.to.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      const ended = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
      assert.equal(ended.accepted, true);
      if (!ended.accepted) return;
      assert.deepEqual(ended.receipt.events.map(({ type }) => type), [
        'charge-expired',
        'charge-expired',
        'turn-ended',
        'turn-started',
      ]);
      assert.deepEqual(ended.receipt.events.slice(0, 2).map(({ payload }) => payload), chargeCards.map((card) => ({
        instanceId: summoned.instanceId,
        seat: 'north',
        sourceInstanceId: card.instanceId,
      })));
      const expired = ctx.state.realm.units.find(({ instanceId }) => instanceId === summoned.instanceId);
      assert.equal(expired?.temporaryChargeSources, undefined);
      assert.equal(expired?.tapped, true);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === printedCharge.instanceId)?.temporaryChargeSources, undefined);
      assert.equal(gameManifest.cards[printedChargeCardId]?.cardType === 'minion'
        && gameManifest.cards[printedChargeCardId].charge, true);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03 Overpower changes current power for source-aware prevention until the current End Phase', async () => {
  const decks = { north: deck('overpower-north', 4, 6), south: deck('overpower-south', 4, 6) };
  const baseCards = cardsFor(decks, {
    attack: 1,
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-overpower-v1',
  };
  const seed = 264;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const overpowerCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const fighterCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    const hiddenCardId = preview.state.players.north.hand.spellbook[2]?.cardId;
    const disabledCardId = preview.state.players.north.spellbook[0]?.cardId;
    const enemyCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
    assert.ok(overpowerCardId);
    assert.ok(fighterCardId);
    assert.ok(hiddenCardId);
    assert.ok(disabledCardId);
    assert.ok(enemyCardId);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    cards[overpowerCardId] = {
      cardType: 'magic',
      grantPowerToAllyThisTurn: 2,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    } as unknown as GameCardDefinition;
    cards[fighterCardId] = {
      ...baseCards[fighterCardId]!,
      attack: 2,
      defense: 2,
    } as GameCardDefinition;
    cards[hiddenCardId] = {
      ...baseCards[hiddenCardId]!,
      burrowing: true,
      stealth: true,
      ward: true,
    } as GameCardDefinition;
    cards[disabledCardId] = {
      ...baseCards[disabledCardId]!,
      waterbound: true,
    } as GameCardDefinition;
    cards[enemyCardId] = {
      ...baseCards[enemyCardId]!,
      attack: 2,
      defense: 2,
      preventsDamageFromUnitsWithPowerAtLeast: 4,
      summonToAnySite: true,
    } as GameCardDefinition;
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [overpowerCardId]: {
          cardType: 'magic',
          grantPowerToAllyThisTurn: 1,
          manaCost: 1,
          thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /grantPowerToAllyThisTurn/);
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });
    assert.deepEqual(gameManifest.cards[overpowerCardId], {
      cardType: 'magic',
      grantPowerToAllyThisTurn: 2,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    });

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === fighterCardId && descriptor.cell === 'C4' && !descriptor.region);
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === hiddenCardId && descriptor.cell === 'C4'
        && descriptor.region === 'underground');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === enemyCardId && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === disabledCardId && descriptor.cell === 'C4');

      const overpower = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === overpowerCardId);
      const fighter = ctx.state.realm.units.find(({ cardId }) => cardId === fighterCardId);
      const hidden = ctx.state.realm.units.find(({ cardId }) => cardId === hiddenCardId);
      const disabled = ctx.state.realm.units.find(({ cardId }) => cardId === disabledCardId);
      const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === enemyCardId);
      assert.ok(overpower);
      assert.ok(fighter);
      assert.ok(hidden);
      assert.ok(disabled);
      assert.ok(enemy);
      const northAvatarId = ctx.state.players.north.avatar.card.instanceId;
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === overpower.instanceId);
      assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally ? [descriptor.ally.instanceId] : []).sort(), [
        northAvatarId,
        disabled.instanceId,
        fighter.instanceId,
        hidden.instanceId,
      ].sort());
      assert.equal(casts.some(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally?.instanceId === enemy.instanceId), false);
      assert.equal(casts.every(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.target === undefined && descriptor.targetLocation === undefined), true);
      assert.equal(casts.find(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally?.instanceId === fighter.instanceId)?.label.includes('grant +2 power'), true);

      await withFork(ctx, async (avatarFork) => {
        await avatarFork.take(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.cardInstanceId === overpower.instanceId
            && descriptor.ally?.kind === 'avatar');
        assert.deepEqual({
          attack: avatarFork.observe('north').players.north.avatar.attack,
          defense: avatarFork.observe('north').players.north.avatar.defense,
        }, { attack: 3, defense: 3 });
        assert.deepEqual(avatarFork.session.transcript.at(-1)?.events.slice(1, 2).map(({ payload, type }) => ({
          payload,
          type,
        })), [{
          payload: {
            amount: 2,
            instanceId: northAvatarId,
            seat: 'north',
            sourceInstanceId: overpower.instanceId,
          },
          type: 'power-granted',
        }]);
        assert.equal(await avatarFork.verifyReplay(), true);
      });

      await withFork(ctx, async (disabledFork) => {
        await disabledFork.take(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.cardInstanceId === overpower.instanceId
            && descriptor.ally?.instanceId === disabled.instanceId);
        const disabledView = disabledFork.observe('north').realm.units.find(({ instanceId }) =>
          instanceId === disabled.instanceId);
        assert.deepEqual({
          attack: disabledView?.attack,
          defense: disabledView?.defense,
          disabled: disabledView?.disabled,
        }, { attack: 3, defense: 3, disabled: true });
        assert.equal(await disabledFork.verifyReplay(), true);
      });

      const powered = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === overpower.instanceId
          && descriptor.ally?.instanceId === fighter.instanceId));
      assert.equal(powered.accepted, true);
      if (!powered.accepted) return;
      assert.deepEqual(powered.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'power-granted',
        'magic-resolved',
      ]);
      const poweredView = ctx.observe('north').realm.units.find(({ instanceId }) =>
        instanceId === fighter.instanceId);
      assert.deepEqual({ attack: poweredView?.attack, defense: poweredView?.defense }, {
        attack: 4,
        defense: 4,
      });
      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === fighter.instanceId && descriptor.path.length === 1);
      await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion' && descriptor.target.instanceId === enemy.instanceId);
      const fought = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
      assert.equal(fought.accepted, true);
      if (!fought.accepted) return;
      assert.deepEqual(fought.receipt.events.find(({ payload, type }) => type === 'damage-dealt'
        && (payload as { instanceId?: string }).instanceId === enemy.instanceId)?.payload, {
        accumulated: 0,
        amount: 0,
        attemptedAmount: 4,
        direct: true,
        instanceId: enemy.instanceId,
        prevented: true,
        seat: 'south',
      });
      const survivor = ctx.state.realm.units.find(({ instanceId }) => instanceId === fighter.instanceId);
      assert.equal(survivor?.damage, 2);
      assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === enemy.instanceId), true);

      const ended = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
      assert.equal(ended.accepted, true);
      if (!ended.accepted) return;
      const expiryIndex = ended.receipt.events.findIndex(({ type }) => type === 'power-expired');
      const turnEndedIndex = ended.receipt.events.findIndex(({ type }) => type === 'turn-ended');
      assert.equal(expiryIndex >= 0 && expiryIndex < turnEndedIndex, true);
      assert.deepEqual(ended.receipt.events[expiryIndex]?.payload, {
        amount: 2,
        instanceId: fighter.instanceId,
        seat: 'north',
        sourceInstanceId: overpower.instanceId,
      });
      const expired = ctx.observe('north').realm.units.find(({ instanceId }) =>
        instanceId === fighter.instanceId);
      assert.deepEqual({
        attack: expired?.attack,
        damage: expired?.damage,
        defense: expired?.defense,
      }, { attack: 2, damage: 0, defense: 2 });
      assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === fighter.instanceId), true);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === overpower.instanceId), true);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-04 nearby-allies power is derived and settles deaths when its source dies', async () => {
  const decks = { north: deck('banner-north', 4, 6), south: deck('banner-south', 4, 6) };
  const baseCards = cardsFor(decks, {
    attack: 1,
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-nearby-power-v1',
  };
  const seed = 265;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const [allyCard, sourceCard, disabledSourceCard] = preview.state.players.north.hand.spellbook;
    const rainCard = preview.state.players.south.hand.spellbook[0];
    assert.ok(allyCard);
    assert.ok(sourceCard);
    assert.ok(disabledSourceCard);
    assert.ok(rainCard);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    cards[sourceCard.cardId] = {
      ...baseCards[sourceCard.cardId]!,
      otherNearbyAlliesPowerBonus: 1,
    } as GameCardDefinition;
    cards[disabledSourceCard.cardId] = {
      ...baseCards[disabledSourceCard.cardId]!,
      otherNearbyAlliesPowerBonus: 1,
      waterbound: true,
    } as GameCardDefinition;
    cards[rainCard.cardId] = {
      cardType: 'magic',
      damageEachAbovegroundMinion: 1,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    };
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [sourceCard.cardId]: {
          ...baseCards[sourceCard.cardId]!,
          otherNearbyAlliesPowerBonus: 2,
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /otherNearbyAlliesPowerBonus must be 1/);
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });
    const sourceDefinition = gameManifest.cards[sourceCard.cardId];
    assert.equal(sourceDefinition?.cardType, 'minion');
    assert.equal(sourceDefinition?.cardType === 'minion'
      ? sourceDefinition.otherNearbyAlliesPowerBonus
      : undefined, 1);

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      for (const card of [allyCard, sourceCard, disabledSourceCard]) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === card.cardId
          && descriptor.cell === 'C4');
      }
      const ally = ctx.state.realm.units.find(({ cardId }) => cardId === allyCard.cardId);
      const source = ctx.state.realm.units.find(({ cardId }) => cardId === sourceCard.cardId);
      const disabledSource = ctx.state.realm.units.find(({ cardId }) =>
        cardId === disabledSourceCard.cardId);
      assert.ok(ally);
      assert.ok(source);
      assert.ok(disabledSource);
      const view = ctx.observe('north');
      const status = (instanceId: string) => view.realm.units.find((unit) =>
        unit.instanceId === instanceId);
      assert.deepEqual({
        ally: [status(ally.instanceId)?.attack, status(ally.instanceId)?.defense],
        avatar: [view.players.north.avatar.attack, view.players.north.avatar.defense],
        disabledSource: [
          status(disabledSource.instanceId)?.attack,
          status(disabledSource.instanceId)?.defense,
          status(disabledSource.instanceId)?.disabled,
        ],
        source: [status(source.instanceId)?.attack, status(source.instanceId)?.defense],
      }, {
        ally: [2, 2],
        avatar: [2, 2],
        disabledSource: [2, 2, true],
        source: [1, 1],
      });

      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      const damaged = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardId === rainCard.cardId));
      assert.equal(damaged.accepted, true);
      if (!damaged.accepted) return;
      assert.equal(ctx.state.realm.units.some(({ owner }) => owner === 'north'), false);
      assert.deepEqual(ctx.state.players.north.cemetery.map(({ instanceId }) => instanceId), [
        source.instanceId,
        ally.instanceId,
        disabledSource.instanceId,
      ]);
      assert.equal(damaged.receipt.events.filter(({ type }) => type === 'minion-died').length, 3);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-04 controlled Mortal power follows current control and settles deaths', async () => {
  const decks = { north: deck('king-north', 4, 6), south: deck('king-south', 4, 6) };
  const baseCards = cardsFor(decks, {
    attack: 1,
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-controlled-mortal-power-v1',
  };
  const seed = 266;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const [northMortalCard, northKingACard, northKingBCard] =
      preview.state.players.north.hand.spellbook;
    const [southMortalCard, southNonMortalCard, mesmerismCard] =
      preview.state.players.south.hand.spellbook;
    const damageCard = preview.state.players.south.spellbook[0];
    assert.ok(northMortalCard);
    assert.ok(northKingACard);
    assert.ok(northKingBCard);
    assert.ok(southMortalCard);
    assert.ok(southNonMortalCard);
    assert.ok(mesmerismCard);
    assert.ok(damageCard);

    const mortal = (cardId: string): GameCardDefinition => ({
      ...baseCards[cardId]!,
      mortal: true,
    } as GameCardDefinition);
    const king = (cardId: string): GameCardDefinition => ({
      ...mortal(cardId),
      otherControlledMortalsPowerBonus: 1,
    } as GameCardDefinition);
    const cards: Record<string, GameCardDefinition> = {
      ...baseCards,
      [northMortalCard.cardId]: mortal(northMortalCard.cardId),
      [northKingACard.cardId]: king(northKingACard.cardId),
      [northKingBCard.cardId]: king(northKingBCard.cardId),
      [southMortalCard.cardId]: {
        ...mortal(southMortalCard.cardId),
        spellcaster: true,
      } as GameCardDefinition,
      [mesmerismCard.cardId]: {
        cardType: 'magic',
        gainControlOfTargetNearbyMinion: true,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
      [damageCard.cardId]: {
        cardType: 'magic',
        damageTargetUnit: 2,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    };
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [northMortalCard.cardId]: {
          ...baseCards[northMortalCard.cardId]!,
          mortal: false,
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /mortal must be true/);
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [northKingACard.cardId]: {
          ...mortal(northKingACard.cardId),
          otherControlledMortalsPowerBonus: 2,
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /otherControlledMortalsPowerBonus must be 1/);
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });
    assert.deepEqual(gameManifest.cards[northKingACard.cardId], {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      mortal: true,
      otherControlledMortalsPowerBonus: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    });

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      for (const card of [northMortalCard, northKingACard, northKingBCard]) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === card.cardId
          && descriptor.cell === 'C4');
      }
      const northMortal = ctx.state.realm.units.find(({ cardId }) =>
        cardId === northMortalCard.cardId);
      const northKingA = ctx.state.realm.units.find(({ cardId }) =>
        cardId === northKingACard.cardId);
      const northKingB = ctx.state.realm.units.find(({ cardId }) =>
        cardId === northKingBCard.cardId);
      assert.ok(northMortal);
      assert.ok(northKingA);
      assert.ok(northKingB);
      const northOpening = ctx.observe('north');
      const openingStatus = (instanceId: string) => northOpening.realm.units.find((unit) =>
        unit.instanceId === instanceId);
      assert.deepEqual({
        avatar: [northOpening.players.north.avatar.attack, northOpening.players.north.avatar.defense],
        kingA: [openingStatus(northKingA.instanceId)?.attack, openingStatus(northKingA.instanceId)?.defense],
        kingB: [openingStatus(northKingB.instanceId)?.attack, openingStatus(northKingB.instanceId)?.defense],
        mortal: [
          openingStatus(northMortal.instanceId)?.attack,
          openingStatus(northMortal.instanceId)?.defense,
        ],
      }, {
        avatar: [1, 1],
        kingA: [2, 2],
        kingB: [2, 2],
        mortal: [3, 3],
      });

      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      for (const card of [southMortalCard, southNonMortalCard]) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === card.cardId
          && descriptor.cell === 'C1');
      }
      const southMortal = ctx.state.realm.units.find(({ cardId }) =>
        cardId === southMortalCard.cardId);
      const southNonMortal = ctx.state.realm.units.find(({ cardId }) =>
        cardId === southNonMortalCard.cardId);
      assert.ok(southMortal);
      assert.ok(southNonMortal);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northKingA.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      const separated = ctx.observe('north').realm.units.find(({ instanceId }) =>
        instanceId === northMortal.instanceId);
      assert.deepEqual([separated?.attack, separated?.defense], [3, 3]);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === southMortal.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.cardId === damageCard.cardId
        && descriptor.target?.instanceId === northMortal.instanceId);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === northMortal.instanceId)?.damage, 2);
      const controlled = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardId === mesmerismCard.cardId
          && descriptor.target?.instanceId === northKingA.instanceId));
      assert.equal(controlled.accepted, true);
      if (!controlled.accepted) return;
      assert.deepEqual(controlled.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'minion-control-changed',
        'minion-died',
        'magic-resolved',
      ]);
      assert.equal(controlled.receipt.randomDraws.length, 0);
      assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
        instanceId === northMortal.instanceId), false);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === northMortal.instanceId), true);

      const finalView = ctx.observe('north');
      const status = (instanceId: string) => finalView.realm.units.find((unit) =>
        unit.instanceId === instanceId);
      assert.deepEqual({
        kingA: [
          status(northKingA.instanceId)?.attack,
          status(northKingA.instanceId)?.controller,
          status(northKingA.instanceId)?.defense,
        ],
        kingB: [status(northKingB.instanceId)?.attack, status(northKingB.instanceId)?.defense],
        nonMortal: [
          status(southNonMortal.instanceId)?.attack,
          status(southNonMortal.instanceId)?.defense,
        ],
        southAvatar: [
          finalView.players.south.avatar.attack,
          finalView.players.south.avatar.defense,
        ],
        southMortal: [
          status(southMortal.instanceId)?.attack,
          status(southMortal.instanceId)?.defense,
        ],
      }, {
        kingA: [1, 'south', 1],
        kingB: [1, 1],
        nonMortal: [1, 1],
        southAvatar: [1, 1],
        southMortal: [2, 2],
      });
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-04 aura-loss Deathrite ends the game after its triggering Magic resolves', async () => {
  const north: GameDeckSpec = {
    atlas: Array(3).fill('banner-site'),
    avatar: 'banner-north-avatar',
    spellbook: [
      ...Array(2).fill('banner-source'),
      ...Array(2).fill('banner-ally'),
      ...Array(2).fill('banner-teleport'),
      ...Array(2).fill('banner-rain'),
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(3).fill('banner-site'),
    avatar: 'banner-south-avatar',
    spellbook: Array(6).fill('banner-source'),
  };
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const cards: Readonly<Record<string, GameCardDefinition>> = {
    'banner-ally': {
      attack: 1,
      cardType: 'minion',
      deathriteDrawSite: true,
      defense: 1,
      manaCost: 0,
      thresholds,
    },
    'banner-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'banner-rain': {
      cardType: 'magic',
      damageEachAbovegroundMinion: 1,
      manaCost: 0,
      thresholds,
    },
    'banner-site': { cardType: 'site', elements: [] },
    'banner-source': {
      attack: 1,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      otherNearbyAlliesPowerBonus: 1,
      thresholds,
    },
    'banner-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'banner-teleport': {
      cardType: 'magic',
      manaCost: 0,
      teleportAllyToTargetSite: true,
      thresholds,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-nearby-power-deathrite-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players.north;
      const openingNames = new Set(opening.hand.spellbook.map(({ cardId }) => cardId));
      const availableNames = new Set([
        ...opening.hand.spellbook,
        ...opening.spellbook.slice(0, 2),
      ].map(({ cardId }) => cardId));
      return openingNames.has('banner-ally')
        && openingNames.has('banner-source')
        && ['banner-rain', 'banner-teleport']
        .every((cardId) => availableNames.has(cardId));
    },
  );
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'banner-source');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'banner-ally');
    const source = ctx.state.realm.units.find(({ cardId }) => cardId === 'banner-source');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === 'banner-ally');
    assert.ok(source);
    assert.ok(ally);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A4');
    assert.equal(ctx.state.players.north.atlas.length, 0);
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'banner-rain');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === ally.instanceId)?.damage, 1);

    const result = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardId === 'banner-teleport'
        && descriptor.ally?.instanceId === source.instanceId
        && descriptor.targetLocation?.cell === 'A4'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'unit-teleported',
      'minion-died',
      'magic-resolved',
      'game-ended',
    ]);
    assert.deepEqual(ctx.state.terminal, {
      loser: 'north',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'south',
    });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 aura-loss deaths cannot restore stale combat during a defender path', async () => {
  const north: GameDeckSpec = {
    atlas: Array(3).fill('defend-site'),
    avatar: 'defend-north-avatar',
    spellbook: [
      ...Array(2).fill('defend-target'),
      ...Array(2).fill('defend-fragile'),
      ...Array(2).fill('defend-source'),
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(3).fill('defend-site'),
    avatar: 'defend-south-avatar',
    spellbook: [
      ...Array(3).fill('defend-attacker'),
      ...Array(3).fill('defend-rain'),
    ],
  };
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const cards: Readonly<Record<string, GameCardDefinition>> = {
    'defend-attacker': {
      attack: 1,
      cardType: 'minion',
      charge: true,
      defense: 10,
      manaCost: 0,
      summonToAnySite: true,
      thresholds,
    },
    'defend-fragile': {
      attack: 1,
      cardType: 'minion',
      deathriteHeal: 3,
      defense: 1,
      manaCost: 0,
      thresholds,
    },
    'defend-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'defend-rain': {
      cardType: 'magic',
      damageEachAbovegroundMinion: 1,
      manaCost: 0,
      thresholds,
    },
    'defend-site': { cardType: 'site', elements: [] },
    'defend-source': {
      attack: 1,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      movementBonus: 2,
      otherNearbyAlliesPowerBonus: 1,
      thresholds,
    },
    'defend-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'defend-target': {
      attack: 1,
      cardType: 'minion',
      defense: 3,
      manaCost: 0,
      thresholds,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-nearby-power-defend-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players;
      const northOpening = new Set(opening.north.hand.spellbook.map(({ cardId }) => cardId));
      const southBySecondTurn = new Set([
        ...opening.south.hand.spellbook,
        ...opening.south.spellbook.slice(0, 2),
      ].map(({ cardId }) => cardId));
      return ['defend-fragile', 'defend-source', 'defend-target']
        .every((cardId) => northOpening.has(cardId))
        && opening.north.spellbook[0]?.cardId === 'defend-fragile'
        && southBySecondTurn.has('defend-attacker')
        && new Set([
          ...opening.south.hand.spellbook,
          ...opening.south.spellbook.slice(0, 3),
        ].map(({ cardId }) => cardId)).has('defend-rain');
    },
  );
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    for (const cardId of ['defend-target', 'defend-fragile', 'defend-source']) {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === cardId
        && descriptor.cell === 'C4');
    }
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === 'defend-target');
    const fragile = ctx.state.realm.units.find(({ cardId }) => cardId === 'defend-fragile');
    const source = ctx.state.realm.units.find(({ cardId }) => cardId === 'defend-source');
    assert.ok(target);
    assert.ok(fragile);
    assert.ok(source);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'defend-fragile'
      && descriptor.cell === 'C4');
    const fragiles = ctx.state.realm.units.filter(({ cardId }) => cardId === 'defend-fragile');
    assert.equal(fragiles.length, 2);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === source.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,B4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'defend-attacker'
      && descriptor.cell === 'C4');
    const attacker = ctx.state.realm.units.find(({ cardId }) => cardId === 'defend-attacker');
    assert.ok(attacker);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'defend-rain');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attacker.instanceId
      && descriptor.path.length === 1);
    await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === target.instanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'defend'
      && descriptor.unitInstanceId === fragile.instanceId
      && descriptor.path.length === 1);

    const defended = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === source.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'B4,A4,B4,C4'));
    assert.equal(defended.accepted, true);
    if (!defended.accepted) return;
    assert.equal(ctx.state.phase, 'deathrite-order');
    assert.equal(ctx.state.decisionSeat, 'north');
    assert.deepEqual(defended.receipt.events.map(({ type }) => type), ['basic-movement-started']);
    assert.equal(fragiles.every(({ instanceId }) => !ctx.state.realm.units
      .some((unit) => unit.instanceId === instanceId)), true);
    assert.equal(fragiles.every(({ instanceId }) => !ctx.state.players.north.cemetery
      .some((card) => card.instanceId === instanceId)), true);
    const orderActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(),
    fragiles.map(({ instanceId }) => instanceId).sort());

    await ctx.resume(parseGameCheckpoint(serializeGameCheckpoint(ctx.checkpoint())));
    const ordered = await ctx.step(orderActions[0]!);
    assert.equal(ordered.accepted, true);
    if (!ordered.accepted) return;
    assert.equal(ctx.state.phase, 'movement');
    assert.equal(ordered.receipt.events.filter(({ type }) => type === 'minion-died').length, 2);
    assert.equal(fragiles.every(({ instanceId }) => ctx.state.players.north.cemetery
      .some((card) => card.instanceId === instanceId)), true);

    const events = [...defended.receipt.events, ...ordered.receipt.events];
    while (ctx.state.phase === 'movement') {
      const continued = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'continue-basic-movement'));
      assert.equal(continued.accepted, true);
      if (!continued.accepted) return;
      events.push(...continued.receipt.events);
    }
    assert.equal(ctx.state.phase, 'defend');
    assert.deepEqual(ctx.state.pendingCombat?.defenders.map(({ instanceId }) => instanceId), [
      source.instanceId,
    ]);
    assert.equal(events.filter(({ type }) => type === 'basic-movement-started').length, 1);
    assert.equal(events.filter(({ type }) => type === 'basic-movement-continued').length, 2);
    assert.equal(events.filter(({ type }) => type === 'defender-joined').length, 1);
    assert.ok(events.findIndex(({ type }) => type === 'defender-joined')
      > events.findLastIndex(({ type }) => type === 'minion-died'));
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Lure makes a chosen enemy minion take its own closer step', async () => {
  const decks = { north: deck('lure-north', 6, 8), south: deck('lure-south', 6, 8) };
  const baseCards = cardsFor(decks, {
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['water'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-lure-v1',
  };
  const seed = 251;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const mobileCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
    const immobileCardId = preview.state.players.south.hand.spellbook[1]?.cardId;
    assert.ok(mobileCardId);
    assert.ok(immobileCardId);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    for (const cardId of decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        lureEnemyMinionOneStepCloser: true,
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      };
    }
    cards[mobileCardId] = {
      ...baseCards[mobileCardId]!,
      charge: true,
      stealth: true,
      summonToAnySite: true,
      ward: true,
    } as GameCardDefinition;
    cards[immobileCardId] = {
      ...baseCards[immobileCardId]!,
      immobile: true,
      summonToAnySite: true,
    } as GameCardDefinition;
    const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === immobileCardId && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'D4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'D2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'D3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === mobileCardId && descriptor.cell === 'D3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ctx.state.realm.units.find(({ cardId }) =>
          cardId === mobileCardId)?.instanceId
        && descriptor.path.length === 1);
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

      const allyInstanceId = ctx.state.players.north.avatar.card.instanceId;
      const mobile = ctx.state.realm.units.find(({ cardId }) => cardId === mobileCardId);
      const immobile = ctx.state.realm.units.find(({ cardId }) => cardId === immobileCardId);
      const lureCards = ctx.state.players.north.hand.spellbook.slice(0, 3);
      assert.ok(mobile);
      assert.ok(immobile);
      assert.equal(lureCards.length, 3);
      assert.deepEqual({
        location: mobile.location,
        stealthed: mobile.stealthed,
        tapped: mobile.tapped,
        warded: mobile.warded,
      }, { location: 'D3', stealthed: true, tapped: true, warded: true });

      const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === lureCards[0]?.instanceId);
      assert.deepEqual(choices.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.temptedDestination
        ? [descriptor.temptedDestination.cell]
        : []).sort(), ['C3', 'D4']);
      assert.equal(choices.every(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally?.instanceId === allyInstanceId
        && descriptor.temptedEnemy?.instanceId === mobile.instanceId
        && descriptor.target === undefined), true);
      assert.equal(choices.some(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.temptedEnemy?.instanceId === immobile.instanceId), false);
      assert.equal(choices.every(({ label }) => label.includes('tempts minion')), true);

      const beforeMana = ctx.state.players.north.mana;
      const versionBefore = ctx.state.stateVersion;
      const first = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === lureCards[0]?.instanceId
          && descriptor.temptedDestination?.cell === 'C3'));
      assert.equal(first.accepted, true);
      if (!first.accepted) return;
      assert.deepEqual(first.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'unit-lured',
        'magic-resolved',
      ]);
      assert.deepEqual(first.receipt.events[1]?.payload, {
        allyInstanceId,
        from: { cell: 'D3', region: 'surface' },
        path: [
          { cell: 'D3', region: 'surface' },
          { cell: 'C3', region: 'surface' },
        ],
        seat: 'south',
        sourceInstanceId: lureCards[0]!.instanceId,
        steps: 1,
        targetInstanceId: mobile.instanceId,
        to: { cell: 'C3', region: 'surface' },
      });
      assert.deepEqual(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === mobile.instanceId), { ...mobile, location: 'C3' });

      const second = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === lureCards[1]?.instanceId
          && descriptor.temptedEnemy?.instanceId === mobile.instanceId
          && descriptor.temptedDestination?.cell === 'C4'));
      assert.equal(second.accepted, true);
      if (!second.accepted) return;
      const noChoiceActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === lureCards[2]?.instanceId);
      assert.equal(noChoiceActions.length, 1);
      assert.equal(noChoiceActions[0]?.descriptor.kind === 'cast-magic'
        && noChoiceActions[0].descriptor.ally === undefined
        && noChoiceActions[0].descriptor.temptedEnemy === undefined
        && noChoiceActions[0].descriptor.temptedDestination === undefined, true);

      const noOp = await ctx.step(noChoiceActions[0]!);
      assert.equal(noOp.accepted, true);
      if (!noOp.accepted) return;
      assert.deepEqual(noOp.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'magic-resolved',
      ]);
      assert.deepEqual(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === mobile.instanceId), { ...mobile, location: 'C4' });
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === immobile.instanceId)?.location, 'C3');
      assert.equal(ctx.state.players.north.mana, beforeMana - 3);
      assert.equal(ctx.state.players.north.cemetery.filter(({ instanceId }) =>
        lureCards.some((card) => card.instanceId === instanceId)).length, 3);
      assert.equal(ctx.state.stateVersion, versionBefore + 3);
      assert.equal([
        ...first.receipt.randomDraws,
        ...second.receipt.randomDraws,
        ...noOp.receipt.randomDraws,
      ].length, 0);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03 Teleport forcefully moves a chosen ally to a target site surface', async () => {
  const allyFacts = {
    defense: 5,
    immobile: true,
    manaCost: 0,
    stealth: true,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    ward: true,
  } as const;
  const base = manifest(154, {
    northSpell: allyFacts,
    site: { elements: ['water', 'air'] },
  });
  await withPreview(base, async (preview) => {
    const allyCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const teleportCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    assert.ok(allyCardId);
    assert.ok(teleportCardId);
    const cards: Record<string, GameCardDefinition> = { ...base.cards };
    for (const cardId of base.decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        manaCost: 2,
        teleportAllyToTargetSite: true,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      };
    }
    cards[allyCardId] = base.cards[allyCardId]!;
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const allyCard = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === allyCardId);
      const teleportCard = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === teleportCardId);
      assert.ok(allyCard);
      assert.ok(teleportCard);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === allyCard.instanceId
          && descriptor.cell === 'C4'
          && descriptor.region === 'underwater');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C3');

      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === teleportCard.instanceId);
      assert.equal(casts.length, 6);
      assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally
        ? [descriptor.ally.instanceId]
        : []))].sort(), [
        allyCard.instanceId,
        ctx.state.players.north.avatar.card.instanceId,
      ].sort());
      assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.targetLocation
        ? [descriptor.targetLocation.cell]
        : []))].sort(), ['C1', 'C3', 'C4']);
      assert.equal(casts.every(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.target === undefined
          && descriptor.targetLocation?.region === 'surface'
          && descriptor.targetSiteInstanceId !== undefined), true);

      await withFork(ctx, async (noMove) => {
        await noMove.accept(casts.find(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.ally?.kind === 'avatar'
            && descriptor.targetLocation?.cell === 'C4')!);
        assert.deepEqual(noMove.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'magic-resolved',
        ]);
        assert.equal(noMove.state.players.north.avatar.location, 'C4');
        assert.equal(await noMove.verifyReplay(), true);
      });

      const destinationSite = ctx.state.realm.sites.C1;
      assert.ok(destinationSite);
      const versionBefore = ctx.state.stateVersion;
      await ctx.accept(casts.find(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.ally?.instanceId === allyCard.instanceId
          && descriptor.targetLocation?.cell === 'C1')!);
      const moved = ctx.state.realm.units.find(({ instanceId }) => instanceId === allyCard.instanceId);
      assert.deepEqual({
        controller: moved?.controller,
        damage: moved?.damage,
        location: moved?.location,
        owner: moved?.owner,
        region: moved?.region,
        stealthed: moved?.stealthed,
        tapped: moved?.tapped,
        warded: moved?.warded,
      }, {
        controller: 'north',
        damage: 0,
        location: 'C1',
        owner: 'north',
        region: 'surface',
        stealthed: true,
        tapped: false,
        warded: true,
      });
      assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'magic-cast',
        'unit-teleported',
        'magic-resolved',
      ]);
      const teleportEvent = ctx.session.transcript.at(-1)?.events[1];
      assert.equal(canonicalJson(teleportEvent?.payload ?? null).includes(
        `\"sourceInstanceId\":\"${teleportCard.instanceId}\"`), true);
      assert.equal(canonicalJson(teleportEvent?.payload ?? null).includes(
        `\"targetSiteInstanceId\":\"${destinationSite.instanceId}\"`), true);
      assert.equal(ctx.state.stateVersion, versionBefore + 1);
      assert.equal(ctx.state.players.north.mana, 0);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === teleportCard.instanceId), true);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03 Blink teleports a nearby ally before deaths and a private chosen-deck draw', async () => {
  const decks = { north: deck('blink-north', 5, 8), south: deck('blink-south', 5, 6) };
  const baseCards = cardsFor(decks, {
    attack: 1,
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-blink-v1',
    },
    decks,
    firstSeat: 'north' as const,
    seed: 266,
  };
  await withPreview(createGameManifest({ ...input, cards: baseCards }), async (preview) => {
    const [sourceCard, beneficiaryCard, rainCard] = preview.state.players.north.hand.spellbook;
    const blinkCard = preview.state.players.north.spellbook[0];
    assert.ok(sourceCard);
    assert.ok(beneficiaryCard);
    assert.ok(rainCard);
    assert.ok(blinkCard);
    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    cards[sourceCard.cardId] = {
      ...baseCards[sourceCard.cardId]!,
      defense: 2,
      otherNearbyAlliesPowerBonus: 1,
    } as GameCardDefinition;
    cards[beneficiaryCard.cardId] = {
      ...baseCards[beneficiaryCard.cardId]!,
      waterbound: true,
    } as GameCardDefinition;
    cards[rainCard.cardId] = {
      cardType: 'magic',
      damageEachAbovegroundMinion: 1,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    };
    cards[blinkCard.cardId] = {
      cardType: 'magic',
      manaCost: 0,
      teleportNearbyAllyThenDrawCard: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    };
    const gameManifest = createGameManifest({ ...input, cards });
    const blinkDefinition = gameManifest.cards[blinkCard.cardId];
    assert.equal(blinkDefinition?.cardType === 'magic'
      ? blinkDefinition.teleportNearbyAllyThenDrawCard
      : undefined, true);

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const sourceInstanceId = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === sourceCard.cardId)?.instanceId;
      const beneficiaryInstanceId = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === beneficiaryCard.cardId)?.instanceId;
      const rainInstanceId = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === rainCard.cardId)?.instanceId;
      assert.ok(sourceInstanceId);
      assert.ok(beneficiaryInstanceId);
      assert.ok(rainInstanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === sourceInstanceId
          && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'B4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === beneficiaryInstanceId
          && descriptor.cell === 'B4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'D4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === rainInstanceId);

      const beneficiary = ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === beneficiaryInstanceId);
      assert.ok(beneficiary);
      assert.equal(ctx.observe('north').realm.units.find(({ instanceId }) =>
        instanceId === beneficiaryInstanceId)?.disabled, true);
      assert.equal(beneficiary.damage, 1);
      const blinkInstanceId = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === blinkCard.cardId)?.instanceId;
      assert.ok(blinkInstanceId);
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === blinkInstanceId);
      assert.equal(casts.some(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally?.instanceId === beneficiaryInstanceId), true);
      assert.equal(casts.every(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.ally?.seat === 'north'
        && descriptor.target === undefined), true);
      assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.drawZone ? [descriptor.drawZone] : []))].sort(), [
        'atlas',
        'spellbook',
      ]);
      assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.ally?.instanceId === sourceInstanceId
          && descriptor.targetLocation
          ? [descriptor.targetLocation.cell]
          : []))].sort(), ['B4', 'C4', 'D4']);

      const drawnSpell = ctx.state.players.north.spellbook[0];
      const drawnSite = ctx.state.players.north.atlas[0];
      assert.ok(drawnSpell);
      assert.ok(drawnSite);
      const beforeHandCount = ctx.state.players.north.hand.spellbook.length;
      await withFork(ctx, async (spellFork) => {
        await spellFork.accept(casts.find(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.ally?.instanceId === sourceInstanceId
            && descriptor.drawZone === 'spellbook'
            && descriptor.targetLocation?.cell === 'D4')!);
        assert.deepEqual(spellFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'unit-teleported',
          'minion-died',
          'spell-drawn',
          'magic-resolved',
        ]);
        assert.equal(spellFork.state.realm.units.find(({ instanceId }) =>
          instanceId === sourceInstanceId)?.location, 'D4');
        assert.equal(spellFork.state.players.north.cemetery.some(({ instanceId }) =>
          instanceId === beneficiaryInstanceId), true);
        assert.equal(spellFork.state.players.north.hand.spellbook.length, beforeHandCount);
        assert.equal(spellFork.state.players.north.hand.spellbook.some(({ instanceId }) =>
          instanceId === drawnSpell.instanceId), true);
        assert.equal(typeof spellFork.observe('south').players.north.hand.spellbook, 'number');
        assert.equal(await spellFork.verifyReplay(), true);
      });

      await withFork(ctx, async (siteFork) => {
        await siteFork.accept(casts.find(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.ally?.instanceId === sourceInstanceId
            && descriptor.drawZone === 'atlas'
            && descriptor.targetLocation?.cell === 'D4')!);
        assert.deepEqual(siteFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'unit-teleported',
          'minion-died',
          'site-drawn',
          'magic-resolved',
        ]);
        assert.equal(siteFork.state.players.north.hand.atlas.some(({ instanceId }) =>
          instanceId === drawnSite.instanceId), true);
        assert.equal(typeof siteFork.observe('south').players.north.hand.atlas, 'number');
        assert.equal(await siteFork.verifyReplay(), true);
      });
    });
  });
});

test('RULE-03 Rescue returns a chosen own cemetery minion to hidden hand or resolves with none', async () => {
  const decks = {
    north: deck('rescue-north', 4, 6),
    south: deck('rescue-south', 4, 6),
  };
  const cards = cardsFor(decks);
  for (const cardId of [...decks.north.atlas, ...decks.south.atlas]) {
    cards[cardId] = { ...cards[cardId]!, genesisDiscardTopSpells: 2 } as GameCardDefinition;
  }
  for (const cardId of decks.north.spellbook.slice(2)) {
    cards[cardId] = {
      cardType: 'magic',
      manaCost: 0,
      returnMinionFromOwnCemetery: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    };
  }
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: 'synthetic-rescue-fixture-v1',
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    }),
    (session) => {
      const preview = session.state.players.north;
      const topTypes = preview.spellbook.slice(0, 2).map(({ cardId }) =>
        session.manifest.cards[cardId]?.cardType);
      return topTypes.includes('minion')
        && topTypes.includes('magic')
        && preview.hand.spellbook.some(({ cardId }) => session.manifest.cards[cardId]?.cardType === 'magic');
    },
    { from: 155, to: 174 },
  );

  const noChoiceCards = { ...cards };
  for (const cardId of [...decks.north.atlas, ...decks.south.atlas]) {
    noChoiceCards[cardId] = { cardType: 'site', elements: ['earth'] };
  }
  const noChoiceManifest = createGameManifest({
    authority: gameManifest.authority,
    cards: noChoiceCards,
    decks,
    firstSeat: 'north',
    seed: gameManifest.seed,
  });
  await withSetup(noChoiceManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const noChoiceCard = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      noChoiceManifest.cards[cardId]?.cardType === 'magic');
    assert.ok(noChoiceCard);
    const noChoiceActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === noChoiceCard.instanceId);
    assert.equal(noChoiceActions.length, 1);
    assert.equal(noChoiceActions[0]?.descriptor.kind === 'cast-magic'
      && noChoiceActions[0].descriptor.cemeteryMinionInstanceId, undefined);
    assert.equal(noChoiceActions[0]?.descriptor.kind === 'cast-magic'
      && noChoiceActions[0].descriptor.target, undefined);
    await ctx.accept(noChoiceActions[0]!);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'magic-resolved',
    ]);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === noChoiceCard.instanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const ownMinion = ctx.state.players.north.cemetery.find(({ cardId }) =>
      gameManifest.cards[cardId]?.cardType === 'minion');
    const ownMagic = ctx.state.players.north.cemetery.find(({ cardId }) =>
      gameManifest.cards[cardId]?.cardType === 'magic');
    assert.ok(ownMinion);
    assert.ok(ownMagic);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    const opposingMinion = ctx.state.players.south.cemetery.find(({ cardId }) =>
      gameManifest.cards[cardId]?.cardType === 'minion');
    assert.ok(opposingMinion);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const rescueCard = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      gameManifest.cards[cardId]?.cardType === 'magic');
    assert.ok(rescueCard);
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === rescueCard.instanceId);
    assert.deepEqual(choices.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cemeteryMinionInstanceId
      ? [descriptor.cemeteryMinionInstanceId]
      : []), [ownMinion.instanceId]);
    assert.equal(choices.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cemeteryMinionInstanceId === ownMagic.instanceId), false);
    assert.equal(choices.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cemeteryMinionInstanceId === opposingMinion.instanceId), false);

    const before = ctx.state;
    const result = await ctx.step(choices[0]!);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.equal(ctx.state.stateVersion, before.stateVersion + 1);
    assert.equal(ctx.state.players.north.hand.spellbook.length, before.players.north.hand.spellbook.length);
    assert.equal(ctx.state.players.north.hand.spellbook.some(({ instanceId }) =>
      instanceId === ownMinion.instanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === ownMinion.instanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === rescueCard.instanceId), true);
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-returned-to-hand',
      'magic-resolved',
    ]);
    assert.deepEqual(result.receipt.events[1]?.payload, {
      cardId: ownMinion.cardId,
      instanceId: ownMinion.instanceId,
      owner: 'north',
      seat: 'north',
      sourceInstanceId: rescueCard.instanceId,
    });
    assert.equal(canonicalJson(result.receipt.events[0]?.payload ?? null).includes(
      `\"cemeteryMinionInstanceId\":\"${ownMinion.instanceId}\"`), true);
    const northView = ctx.observe('north');
    const southView = ctx.observe('south');
    assert.equal(Array.isArray(northView.players.north.hand.spellbook)
      && northView.players.north.hand.spellbook.some(({ instanceId }) =>
        instanceId === ownMinion.instanceId), true);
    assert.equal(southView.players.north.hand.spellbook, ctx.state.players.north.hand.spellbook.length);
    assert.doesNotMatch(canonicalJson(southView), new RegExp(ownMinion.instanceId));
    assert.doesNotMatch(canonicalJson(southView), new RegExp(ownMinion.cardId));
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 Bury forcefully burrows minions if able and immediately resolves survival', async () => {
  const castBury = async (
    targetFacts: Pick<SpellFacts, 'burrowing' | 'ward'>,
    waterTarget: boolean,
    seed: number,
  ): Promise<Readonly<{
    beforeCast: GameSession['state'];
    session: GameSession;
    targetInstanceId: string;
  }>> => {
    const decks = { north: deck(`bury-north-${seed}`, 4, 6), south: deck(`bury-south-${seed}`, 4, 6) };
    const cards = cardsFor(decks, {
      ...targetFacts,
      defense: 2,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    });
    for (const cardId of decks.north.spellbook) {
      cards[cardId] = {
        burrowTargetMinionOrArtifact: true,
        cardType: 'magic',
        manaCost: 1,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    if (waterTarget) {
      for (const cardId of decks.south.atlas) {
        cards[cardId] = { cardType: 'site', elements: ['water'] };
      }
    }
    return withSetup(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-bury-${seed}-v1`,
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cell === 'C1'
          && descriptor.region === undefined);
      const target = ctx.state.realm.units.find(({ controller }) => controller === 'south');
      assert.ok(target);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const spell = ctx.state.players.north.hand.spellbook[0];
      assert.ok(spell);
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
      assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.target
        ? [`${descriptor.target.kind}:${descriptor.target.instanceId}`]
        : []), [`minion:${target.instanceId}`]);
      const beforeCast = ctx.state;
      await ctx.accept(casts[0]!);
      assert.equal(ctx.state.stateVersion, beforeCast.stateVersion + 1);
      assert.equal(ctx.state.players.north.mana, beforeCast.players.north.mana - 1);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === spell.instanceId), true);
      assert.equal(await ctx.verifyReplay(), true);
      return { beforeCast, session: ctx.session, targetInstanceId: target.instanceId };
    });
  };

  const survivor = await castBury({ burrowing: true }, false, 152);
  assert.deepEqual(survivor.session.state.realm.units.find(({ instanceId }) =>
    instanceId === survivor.targetInstanceId), {
    ...survivor.beforeCast.realm.units.find(({ instanceId }) => instanceId === survivor.targetInstanceId),
    region: 'underground',
  });
  assert.deepEqual(survivor.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'minion-burrowed',
    'magic-resolved',
  ]);

  const dead = await castBury({}, false, 153);
  assert.equal(dead.session.state.realm.units.some(({ instanceId }) =>
    instanceId === dead.targetInstanceId), false);
  assert.equal(dead.session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === dead.targetInstanceId), true);
  assert.deepEqual(dead.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'minion-burrowed',
    'minion-died',
    'magic-resolved',
  ]);

  const warded = await castBury({ ward: true }, false, 154);
  assert.deepEqual(warded.session.state.realm.units.find(({ instanceId }) =>
    instanceId === warded.targetInstanceId), {
    ...warded.beforeCast.realm.units.find(({ instanceId }) => instanceId === warded.targetInstanceId),
    warded: false,
  });
  assert.deepEqual(warded.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'ward-broken',
    'magic-resolved',
  ]);

  const water = await castBury({}, true, 155);
  assert.deepEqual(water.session.state.realm.units.find(({ instanceId }) =>
    instanceId === water.targetInstanceId), water.beforeCast.realm.units.find(({ instanceId }) =>
    instanceId === water.targetInstanceId));
  assert.deepEqual(water.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
});

test('RULE-03/04 Bury detaches and burrows Artifacts if able', async () => {
  const castBury = async (
    carried: boolean,
    waterTarget: boolean,
  ): Promise<Readonly<{
    artifactInstanceId: string;
    beforeCast: GameSession['state'];
    session: GameSession;
  }>> => {
    const north: GameDeckSpec = {
      atlas: Array(4).fill('bury-artifact-north-site'),
      avatar: 'bury-artifact-north-avatar',
      spellbook: Array(4).fill('bury-artifact-magic'),
    };
    const south: GameDeckSpec = {
      atlas: Array(4).fill('bury-artifact-south-site'),
      avatar: 'bury-artifact-south-avatar',
      spellbook: Array(4).fill('bury-artifact-target'),
    };
    const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
    const cards: Record<string, GameCardDefinition> = {
      'bury-artifact-magic': {
        burrowTargetMinionOrArtifact: true,
        cardType: 'magic',
        manaCost: 1,
        thresholds,
      },
      'bury-artifact-north-avatar': {
        attack: 1,
        cardType: 'avatar',
        defense: 1,
        drawSpell: false,
        life: 20,
      },
      'bury-artifact-north-site': {
        cardType: 'site',
        elements: ['earth'],
        genesisGainMana: 6,
      },
      'bury-artifact-south-avatar': {
        attack: 1,
        cardType: 'avatar',
        defense: 1,
        drawSpell: false,
        life: 20,
      },
      'bury-artifact-south-site': {
        cardType: 'site',
        elements: waterTarget ? ['water'] : ['earth'],
        genesisGainMana: 6,
      },
      'bury-artifact-target': {
        cardType: 'artifact',
        grantsBearerPower: 2,
        manaCost: 0,
        thresholds,
      },
    };
    return withSetup(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-bury-artifact-${carried ? 'carried' : 'uncarried'}-${waterTarget ? 'water' : 'land'}-v1`,
      },
      cards,
      decks: { north, south },
      firstSeat: 'north',
      seed: 156,
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
        && descriptor.cardId === 'bury-artifact-target'
        && (carried
          ? descriptor.bearer?.kind === 'avatar'
          : descriptor.bearer === undefined && descriptor.cell === 'C1'));
      const artifact = ctx.state.realm.artifacts?.[0];
      assert.ok(artifact);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const bury = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === 'bury-artifact-magic');
      assert.ok(bury);
      const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === bury.instanceId
          && descriptor.targetArtifactInstanceId === artifact.instanceId);
      assert.equal(choices.length, 1);
      assert.match(choices[0]!.label, /artifact/);
      const beforeCast = ctx.state;
      await ctx.accept(choices[0]!);
      assert.equal(await ctx.verifyReplay(), true);
      return { artifactInstanceId: artifact.instanceId, beforeCast, session: ctx.session };
    });
  };

  const uncarried = await castBury(false, false);
  assert.deepEqual(uncarried.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === uncarried.artifactInstanceId), {
    ...uncarried.beforeCast.realm.artifacts?.find(({ instanceId }) =>
      instanceId === uncarried.artifactInstanceId),
    region: 'underground',
  });
  assert.deepEqual(uncarried.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'artifact-burrowed',
    'magic-resolved',
  ]);
  assert.equal(canonicalJson(uncarried.session.transcript.at(-1)?.events[0]?.payload ?? null)
    .includes(`"targetArtifactInstanceId":"${uncarried.artifactInstanceId}"`), true);

  const carried = await castBury(true, false);
  assert.deepEqual(carried.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === carried.artifactInstanceId), {
    cardId: 'bury-artifact-target',
    instanceId: carried.artifactInstanceId,
    location: 'C1',
    owner: 'south',
    region: 'underground',
    source: 'spellbook',
  });
  assert.deepEqual(carried.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'artifact-burrowed',
    'magic-resolved',
  ]);

  const water = await castBury(false, true);
  assert.deepEqual(water.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === water.artifactInstanceId), water.beforeCast.realm.artifacts?.find(({ instanceId }) =>
    instanceId === water.artifactInstanceId));
  assert.deepEqual(water.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
});

test('RULE-03/04 Cave-In burrows every surface minion and Artifact at one Land Site together', async () => {
  const north: GameDeckSpec = {
    atlas: Array(4).fill('cave-in-water-site'),
    avatar: 'cave-in-north-avatar',
    spellbook: Array(4).fill('cave-in-magic'),
  };
  const south: GameDeckSpec = {
    atlas: Array(4).fill('cave-in-land-site'),
    avatar: 'cave-in-south-avatar',
    spellbook: ['cave-in-burrower', 'cave-in-victim', 'cave-in-artifact'],
  };
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const cards: Record<string, GameCardDefinition> = {
    'cave-in-artifact': {
      cardType: 'artifact',
      grantsBearerPower: 2,
      manaCost: 0,
      thresholds,
    },
    'cave-in-burrower': {
      attack: 1,
      burrowing: true,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      stealth: true,
      thresholds,
      ward: true,
    },
    'cave-in-land-site': { cardType: 'site', elements: ['earth'] },
    'cave-in-magic': {
      burrowAllMinionsAndArtifactsAtTargetLandSite: true,
      cardType: 'magic',
      manaCost: 0,
      thresholds,
    },
    'cave-in-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'cave-in-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'cave-in-victim': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds,
    },
    'cave-in-water-site': { cardType: 'site', elements: ['water'] },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-cave-in-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'cave-in-magic': {
        ...cards['cave-in-magic'],
        burrowAllMinionsAndArtifactsAtTargetLandSite: false,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /burrowAllMinionsAndArtifactsAtTargetLandSite must be true when defined/);
  const gameManifest = createGameManifest({ ...input, seed: 157 });
  assert.equal((gameManifest.cards['cave-in-magic'] as Extract<GameCardDefinition, {
    cardType: 'magic';
  }>).burrowAllMinionsAndArtifactsAtTargetLandSite, true);
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'cave-in-burrower'
      && descriptor.cell === 'C1'
      && descriptor.region === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'cave-in-victim' && descriptor.cell === 'C1');
    const burrower = ctx.state.realm.units.find(({ cardId }) => cardId === 'cave-in-burrower');
    const victim = ctx.state.realm.units.find(({ cardId }) => cardId === 'cave-in-victim');
    const artifactCard = ctx.state.players.south.hand.spellbook
      .find(({ cardId }) => cardId === 'cave-in-artifact');
    assert.ok(burrower && victim && artifactCard);

    const castCaveIn = async (fork: SetupCtx, bearer: 'avatar' | 'minion'): Promise<void> => {
      await fork.take(({ descriptor }) =>
        descriptor.kind === 'cast-artifact'
          && descriptor.cardInstanceId === artifactCard.instanceId
          && descriptor.bearer?.kind === bearer
          && (bearer === 'avatar' || descriptor.bearer.instanceId === burrower.instanceId));
      const artifact = fork.state.realm.artifacts?.find(({ instanceId }) =>
        instanceId === artifactCard.instanceId);
      assert.ok(artifact);
      await fork.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await fork.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const spell = fork.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === 'cave-in-magic');
      assert.ok(spell);
      const casts = (await fork.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
      assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.targetLocation
        ? [`${descriptor.targetLocation.cell}:${descriptor.targetLocation.region}`]
        : []), ['C1:surface']);
      const result = await fork.step(casts[0]!);
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      const events = result.receipt.events;
      const burrowEvents = events.filter(({ type }) =>
        type === 'minion-burrowed' || type === 'artifact-burrowed');
      assert.deepEqual(burrowEvents.map(({ payload }) => (payload as { instanceId: string }).instanceId), [
        artifact.instanceId,
        burrower.instanceId,
        victim.instanceId,
      ].sort());
      assert.ok(events.findIndex(({ type }) => type === 'minion-died')
        > events.findLastIndex(({ type }) => type === 'minion-burrowed' || type === 'artifact-burrowed'));
      const survivingBurrower = fork.state.realm.units.find(({ instanceId }) =>
        instanceId === burrower.instanceId);
      assert.deepEqual({
        region: survivingBurrower?.region,
        stealthed: survivingBurrower?.stealthed,
        warded: survivingBurrower?.warded,
      }, { region: 'underground', stealthed: true, warded: true });
      assert.equal(fork.state.players.south.cemetery.some(({ instanceId }) =>
        instanceId === victim.instanceId), true);
      assert.equal(fork.state.players.south.avatar.region, 'surface');
      const movedArtifact = fork.state.realm.artifacts?.find(({ instanceId }) =>
        instanceId === artifact.instanceId);
      if (bearer === 'minion') {
        assert.deepEqual(movedArtifact, artifact);
      } else {
        assert.deepEqual(movedArtifact, {
          cardId: artifact.cardId,
          instanceId: artifact.instanceId,
          location: 'C1',
          owner: artifact.owner,
          region: 'underground',
          source: artifact.source,
        });
      }
      assert.equal(fork.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
      assert.equal(await fork.verifyReplay(), true);
    };

    await withFork(ctx, async (fork) => {
      await castCaveIn(fork, 'minion');
    });
    await withFork(ctx, async (fork) => {
      await castCaveIn(fork, 'avatar');
    });
  });
});

test('RULE-03/04 Drown forcefully submerges a target minion if able', async () => {
  const decks = { north: deck('drown-north', 4, 8), south: deck('drown-south', 4, 8) };
  const baseCards = cardsFor(decks, {
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['water'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-drown-v1',
  };
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed: 246,
  }), async (preview) => {
    const waterSiteId = preview.state.players.south.hand.atlas[0]?.cardId;
    const landSiteId = preview.state.players.south.hand.atlas[1]?.cardId;
    const ordinaryId = preview.state.players.south.hand.spellbook[0]?.cardId;
    const survivorId = preview.state.players.south.hand.spellbook[1]?.cardId;
    const wardedId = preview.state.players.south.hand.spellbook[2]?.cardId;
    const landTargetId = preview.state.players.south.spellbook[0]?.cardId;
    const stealthId = preview.state.players.south.spellbook[1]?.cardId;
    assert.ok(waterSiteId);
    assert.ok(landSiteId);
    assert.ok(ordinaryId);
    assert.ok(survivorId);
    assert.ok(wardedId);
    assert.ok(landTargetId);
    assert.ok(stealthId);
    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    for (const cardId of decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        manaCost: 1,
        submergeTargetMinion: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
      };
    }
    cards[landSiteId] = { cardType: 'site', elements: ['earth'] };
    cards[survivorId] = { ...cards[survivorId]!, submerge: true } as GameCardDefinition;
    cards[wardedId] = { ...cards[wardedId]!, ward: true } as GameCardDefinition;
    cards[stealthId] = { ...cards[stealthId]!, stealth: true } as GameCardDefinition;
    const drownCardId = decks.north.spellbook[0]!;
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [drownCardId]: {
          ...cards[drownCardId]!,
          submergeTargetMinion: false,
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed: 246,
    }), /submergeTargetMinion/);
    const gameManifest = createGameManifest({
      authority,
      cards,
      decks,
      firstSeat: 'north',
      seed: 246,
    });
    assert.equal(gameManifest.cards[drownCardId]?.cardType === 'magic'
      && gameManifest.cards[drownCardId].submergeTargetMinion, true);
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const instance = (cardId: string) => [
        ...ctx.state.players.south.hand.atlas,
        ...ctx.state.players.south.hand.spellbook,
        ...ctx.state.players.south.atlas,
        ...ctx.state.players.south.spellbook,
      ].find((card) => card.cardId === cardId)!;
      const waterSite = instance(waterSiteId);
      const landSite = instance(landSiteId);
      const ordinary = instance(ordinaryId);
      const survivor = instance(survivorId);
      const warded = instance(wardedId);
      const landTarget = instance(landTargetId);
      const stealth = instance(stealthId);

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === waterSite.instanceId
          && descriptor.cell === 'C1');
      for (const target of [ordinary, survivor, warded]) {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === target.instanceId
            && descriptor.cell === 'C1'
            && descriptor.region === undefined);
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === landSite.instanceId
          && descriptor.cell === 'C2');
      for (const target of [landTarget, stealth]) {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === target.instanceId
            && descriptor.cell === 'C2'
            && descriptor.region === undefined);
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

      const drown = ctx.state.players.north.hand.spellbook.find(({ cardId }) => cardId === drownCardId);
      assert.ok(drown);
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === drown.instanceId);
      const targetIds = casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.target?.kind === 'minion' ? [descriptor.target.instanceId] : []);
      assert.deepEqual([...targetIds].sort(), [ordinary, survivor, warded, landTarget]
        .map(({ instanceId }) => instanceId).sort());
      assert.equal(targetIds.includes(stealth.instanceId), false);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === ordinary.instanceId)?.location, 'C1');
      assert.equal(ctx.state.players.north.avatar.location, 'C4');
      const verifyCast = async (fork: SetupCtx): Promise<void> => {
        assert.equal(fork.state.stateVersion, ctx.state.stateVersion + 1);
        assert.equal(fork.state.players.north.mana, ctx.state.players.north.mana - 1);
        assert.equal(fork.state.players.north.cemetery.some(({ instanceId }) =>
          instanceId === drown.instanceId), true);
        assert.equal(await fork.verifyReplay(), true);
      };

      await withFork(ctx, async (deadFork) => {
        await deadFork.accept(casts.find(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.target?.instanceId === ordinary.instanceId)!);
        await verifyCast(deadFork);
        assert.equal(deadFork.state.realm.units.some(({ instanceId }) =>
          instanceId === ordinary.instanceId), false);
        assert.equal(deadFork.state.players.south.cemetery.some(({ instanceId }) =>
          instanceId === ordinary.instanceId), true);
        assert.deepEqual(deadFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'minion-submerged',
          'minion-died',
          'magic-resolved',
        ]);
      });

      await withFork(ctx, async (submergedFork) => {
        await submergedFork.accept(casts.find(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.target?.instanceId === survivor.instanceId)!);
        await verifyCast(submergedFork);
        assert.equal(submergedFork.state.realm.units.find(({ instanceId }) =>
          instanceId === survivor.instanceId)?.region, 'underwater');
        assert.deepEqual(submergedFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'minion-submerged',
          'magic-resolved',
        ]);
      });

      await withFork(ctx, async (wardedFork) => {
        await wardedFork.accept(casts.find(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.target?.instanceId === warded.instanceId)!);
        await verifyCast(wardedFork);
        const wardedUnit = wardedFork.state.realm.units.find(({ instanceId }) =>
          instanceId === warded.instanceId);
        assert.equal(wardedUnit?.region, 'surface');
        assert.equal(wardedUnit?.warded, false);
        assert.deepEqual(wardedFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'ward-broken',
          'magic-resolved',
        ]);
      });

      await withFork(ctx, async (unableFork) => {
        await unableFork.accept(casts.find(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.target?.instanceId === landTarget.instanceId)!);
        await verifyCast(unableFork);
        assert.equal(unableFork.state.realm.units.find(({ instanceId }) =>
          instanceId === landTarget.instanceId)?.region, 'surface');
        assert.deepEqual(unableFork.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'magic-cast',
          'magic-resolved',
        ]);
      });
    });
  });
});

test("RULE-03/04 healing Magic is targetless, capped, and cannot leave Death's Door", async () => {
  const healAfterDamage = async (maximumLife: number, seed: number): Promise<Readonly<{
    beforeCast: GameSession['state'];
    session: GameSession;
    spellInstanceId: string;
  }>> => {
    const decks = { north: deck('heal-north', 4, 6), south: deck('heal-south', 4, 6) };
    const cards = cardsFor(
      decks,
      { manaCost: 1, thresholds: { air: 1, earth: 0, fire: 0, water: 0 } },
      { attack: 1, defense: 1, drawSpell: false, life: maximumLife },
      { elements: ['air'] },
    );
    for (const cardId of decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        healController: 7,
        manaCost: 1,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      };
    }
    for (const cardId of decks.south.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        damageTargetUnit: 4,
        manaCost: 1,
        thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      };
    }
    return withSetup(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-healing-magic-${maximumLife}-v1`,
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.target !== undefined
          && descriptor.target.kind === 'avatar'
          && descriptor.target.seat === 'north');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const spell = ctx.state.players.north.hand.spellbook[0];
      assert.ok(spell);
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
      assert.equal(casts.length, 1);
      assert.equal(casts[0]?.descriptor.kind === 'cast-magic'
        && casts[0].descriptor.target === undefined, true);
      const beforeCast = ctx.state;
      await ctx.accept(casts[0]!);
      assert.equal(await ctx.verifyReplay(), true);
      return { beforeCast, session: ctx.session, spellInstanceId: spell.instanceId };
    });
  };

  const capped = await healAfterDamage(20, 151);
  assert.equal(capped.beforeCast.players.north.avatar.life, 16);
  assert.equal(capped.session.state.players.north.avatar.life, 20);
  assert.equal(capped.session.state.players.north.mana, capped.beforeCast.players.north.mana - 1);
  assert.equal(capped.session.state.stateVersion, capped.beforeCast.stateVersion + 1);
  assert.equal(capped.session.state.players.north.hand.spellbook.length,
    capped.beforeCast.players.north.hand.spellbook.length - 1);
  assert.equal(capped.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === capped.spellInstanceId), true);
  assert.deepEqual(capped.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'avatar-healed',
    'magic-resolved',
  ]);
  assert.deepEqual(capped.session.transcript.at(-1)?.events[1]?.payload, {
    amount: 4,
    attemptedAmount: 7,
    life: 20,
    seat: 'north',
    sourceInstanceId: capped.spellInstanceId,
  });

  const deathDoor = await healAfterDamage(4, 152);
  assert.equal(deathDoor.beforeCast.players.north.avatar.life, 0);
  assert.equal(deathDoor.session.state.players.north.avatar.life, 0);
  assert.equal(deathDoor.session.state.players.north.avatar.deathDoorTurn,
    deathDoor.beforeCast.players.north.avatar.deathDoorTurn);
  assert.deepEqual(deathDoor.session.state.terminal, { status: 'active' });
  assert.deepEqual(deathDoor.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
});

test('RULE-03 explicit permission allows a minion to be summoned to any site', async () => {
  const summonCells = async (summonToAnySite: boolean): Promise<readonly string[]> => {
    let cells: readonly string[] = [];
    await withNorthSecondMain(111, false, {
      manaCost: 1,
      summonToAnySite,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    }, async (ctx) => {
      const cardInstanceId = ctx.state.players.north.hand.spellbook[0]?.instanceId;
      assert.ok(cardInstanceId);
      cells = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === cardInstanceId
          ? [descriptor.cell]
          : []);
      if (summonToAnySite) {
        await ctx.accept(await ctx.action(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === cardInstanceId
            && descriptor.cell === 'C1'));
        assert.equal(ctx.state.realm.units[0]?.location, 'C1');
        assert.equal(await ctx.verifyReplay(), true);
      }
    });
    return cells;
  };

  assert.deepEqual(await summonCells(false), ['C4']);
  assert.deepEqual(await summonCells(true), ['C1', 'C4']);
});

test('RULE-03 a Water-site cast restriction filters unrestricted summons by terrain', async () => {
  const decks = {
    north: deck('water-cast-north', 3, 6),
    south: deck('water-cast-south', 3, 6),
  };
  const cards = cardsFor(decks);
  const northWaterId = decks.north.atlas[0]!;
  const northLandId = decks.north.atlas[1]!;
  const southLandId = decks.south.atlas[0]!;
  const southWaterId = decks.south.atlas[1]!;
  decks.north.spellbook.forEach((cardId) => {
    cards[cardId] = {
      attack: 4,
      cardType: 'minion',
      defense: 4,
      manaCost: 1,
      mustBeCastToWaterSite: true,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
    };
  });
  cards[northWaterId] = { cardType: 'site', elements: ['water'] };
  cards[southWaterId] = { cardType: 'site', elements: ['water'] };
  const restrictedManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-water-site-cast-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 144,
  });
  await withSetup(restrictedManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();

    const northWater = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === northWaterId);
    const northLand = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === northLandId);
    const southLand = ctx.state.players.south.hand.atlas.find(({ cardId }) => cardId === southLandId);
    const southWater = ctx.state.players.south.hand.atlas.find(({ cardId }) => cardId === southWaterId);
    const featuredId = ctx.state.players.north.hand.spellbook[0]?.cardId;
    const ordinaryId = ctx.state.players.south.hand.spellbook[0]?.cardId;
    assert.ok(northWater);
    assert.ok(northLand);
    assert.ok(southLand);
    assert.ok(southWater);
    assert.ok(featuredId);
    assert.ok(ordinaryId);

    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northWater.instanceId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === southLand.instanceId && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northLand.instanceId && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === southWater.instanceId && descriptor.cell === 'B1');
    const southSummons = await ctx.legalActions('south');
    assert.equal(southSummons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === ordinaryId && descriptor.cell === 'C4'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const summons = await ctx.legalActions('north');
    const cellsFor = (cardId: string): readonly string[] => summons.flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === cardId ? [descriptor.cell] : []);
    assert.deepEqual(cellsFor(featuredId), ['B1', 'C4']);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === featuredId && descriptor.cell === 'B1');
    assert.equal(ctx.state.realm.units[0]?.location, 'B1');
    assert.equal(ctx.state.realm.units[0]?.controller, 'north');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 Waterbound derives Disabled from terrain and survives only with active abilities', async () => {
  const base = manifest(228);
  await withPreview(base, async (preview) => {
    const waterSiteId = preview.state.players.north.hand.atlas[0]?.cardId;
    const sinkholeId = preview.state.players.north.hand.atlas[1]?.cardId;
    const waterboundId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const teleportId = preview.state.players.north.hand.spellbook[1]?.cardId;
    assert.ok(waterSiteId);
    assert.ok(sinkholeId);
    assert.ok(waterboundId);
    assert.ok(teleportId);
    const cards: Record<string, GameCardDefinition> = { ...base.cards };
    cards[waterSiteId] = { cardType: 'site', elements: ['water', 'air'] };
    cards[sinkholeId] = {
      ...cards[sinkholeId]!,
      sacrificeToDestroyNearbySite: true,
    } as GameCardDefinition;
    for (const cardId of base.decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        manaCost: 0,
        teleportAllyToTargetSite: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    for (const cardId of base.decks.south.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    cards[waterboundId] = {
      attack: 2,
      cardType: 'minion',
      deathriteDrawSite: true,
      defense: 2,
      manaCost: 0,
      provides: 'water',
      stealth: true,
      submerge: true,
      tapForMana: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      waterbound: true,
    };
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const waterSite = ctx.state.players.north.hand.atlas.find(({ cardId }) =>
        cardId === waterSiteId);
      const sinkhole = ctx.state.players.north.hand.atlas.find(({ cardId }) =>
        cardId === sinkholeId);
      const waterbound = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === waterboundId);
      const teleport = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === teleportId);
      assert.ok(waterSite);
      assert.ok(sinkhole);
      assert.ok(waterbound);
      assert.ok(teleport);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === waterSite.instanceId
          && descriptor.cell === 'C4');
      const summonRegions = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === waterbound.instanceId
          ? [descriptor.region ?? 'surface']
          : []);
      assert.deepEqual(summonRegions, ['underwater', 'surface']);

      const continueToSinkhole = async (target: SetupCtx): Promise<void> => {
        await target.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await target.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await target.take(({ descriptor }) =>
          descriptor.kind === 'play-site' && descriptor.cell === 'C1');
        await target.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await target.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await target.take(({ descriptor }) =>
          descriptor.kind === 'play-site'
            && descriptor.cardInstanceId === sinkhole.instanceId
            && descriptor.cell === 'C3');
      };

      await withFork(ctx, async (underwater) => {
        await underwater.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === waterbound.instanceId
            && (descriptor.region ?? 'surface') === 'underwater');
        assert.equal(underwater.observe('north').realm.units[0]?.disabled, false);
        assert.equal(underwater.observe('north').realm.units[0]?.stealthed, true);
        assert.equal(underwater.observe('north').players.north.affinity.water, 2);
        assert.equal(await underwater.verifyReplay(), true);

        await continueToSinkhole(underwater);
        const atlasBefore = underwater.state.players.north.atlas.length;
        const atlasHandBefore = underwater.state.players.north.hand.atlas.length;
        const destroyed = await underwater.step(await underwater.action(({ descriptor }) =>
          descriptor.kind === 'activate-site-destruction'
            && descriptor.sourceSiteInstanceId === sinkhole.instanceId
            && descriptor.targetCell === 'C4'));
        assert.equal(destroyed.accepted, true);
        assert.deepEqual(destroyed.receipt.events.map(({ type }) => type), [
          'site-sacrificed',
          'site-destroyed',
          'stealth-lost',
          'minion-died',
          'rubble-created',
          'rubble-created',
        ]);
        assert.equal(destroyed.session.state.realm.units.some(({ instanceId }) =>
          instanceId === waterbound.instanceId), false);
        assert.equal(destroyed.session.state.players.north.atlas.length, atlasBefore);
        assert.equal(destroyed.session.state.players.north.hand.atlas.length, atlasHandBefore);
        assert.equal(destroyed.receipt.events.some(({ type }) => type === 'site-drawn'), false);
        assert.equal(await underwater.verifyReplay(), true);
      });

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === waterbound.instanceId
          && (descriptor.region ?? 'surface') === 'surface');
      assert.equal(ctx.observe('north').realm.units[0]?.stealthed, true);
      await withFork(ctx, async (activeTarget) => {
        await activeTarget.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await activeTarget.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        assert.equal((await activeTarget.legalActions('south')).some(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.target?.instanceId === waterbound.instanceId), false);
      });
      await continueToSinkhole(ctx);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === waterbound.instanceId
          && descriptor.to.cell === 'C3');
      assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'move-and-attack-activated',
        'stealth-lost',
      ]);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === waterbound.instanceId)?.stealthed, false);
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      assert.equal(ctx.observe('north').realm.units[0]?.disabled, true);
      assert.equal(ctx.observe('north').realm.units[0]?.stealthed, false);
      assert.equal(ctx.observe('north').players.north.affinity.water, 1);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.target?.instanceId === waterbound.instanceId), true);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const disabledActions = await ctx.legalActions('north');
      assert.equal(disabledActions.some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === waterbound.instanceId), false);
      assert.equal(disabledActions.some(({ descriptor }) =>
        descriptor.kind === 'activate-mana'
          && descriptor.unitInstanceId === waterbound.instanceId), false);
      const beforeTeleport = ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === waterbound.instanceId);
      assert.ok(beforeTeleport);
      assert.deepEqual({
        damage: beforeTeleport.damage,
        summoningSickness: beforeTeleport.summoningSickness,
        tapped: beforeTeleport.tapped,
      }, { damage: 0, summoningSickness: false, tapped: false });
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === teleport.instanceId
          && descriptor.ally?.instanceId === waterbound.instanceId
          && descriptor.targetLocation?.cell === 'C4');
      const returned = ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === waterbound.instanceId);
      assert.deepEqual({
        damage: returned?.damage,
        location: returned?.location,
        region: returned?.region,
        summoningSickness: returned?.summoningSickness,
        tapped: returned?.tapped,
      }, {
        damage: 0,
        location: 'C4',
        region: 'surface',
        summoningSickness: false,
        tapped: false,
      });
      assert.equal(ctx.observe('north').realm.units[0]?.disabled, false);
      assert.equal(ctx.observe('north').realm.units[0]?.stealthed, false);
      assert.equal(ctx.observe('north').players.north.affinity.water, 2);
      const enabledActions = await ctx.legalActions('north');
      assert.equal(enabledActions.some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === waterbound.instanceId), true);
      assert.equal(enabledActions.some(({ descriptor }) =>
        descriptor.kind === 'activate-mana'
          && descriptor.unitInstanceId === waterbound.instanceId), true);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.target?.instanceId === waterbound.instanceId), true);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-02/04 region settlement kills inhospitable minions and banishes them from the void', async () => {
  const base = manifest(229, {
    northSpell: {
      burrowing: true,
      deathriteDrawSite: true,
      manaCost: 0,
      movementBonus: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
      waterbound: true,
    },
    southSpell: {
      charge: true,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
    },
  });
  await withPreview(base, async (preview) => {
    const waterSiteId = preview.state.players.north.hand.atlas[0]?.cardId;
    const landSiteId = preview.state.players.north.hand.atlas[1]?.cardId;
    const featuredId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const targetId = preview.state.players.north.hand.spellbook[1]?.cardId;
    assert.ok(waterSiteId);
    assert.ok(landSiteId);
    assert.ok(featuredId);
    assert.ok(targetId);
    const cards: Record<string, GameCardDefinition> = { ...base.cards };
    cards[waterSiteId] = { cardType: 'site', elements: ['water'] };
    cards[targetId] = {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
    };
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const waterSite = ctx.state.players.north.hand.atlas.find(({ cardId }) =>
        cardId === waterSiteId);
      const landSite = ctx.state.players.north.hand.atlas.find(({ cardId }) =>
        cardId === landSiteId);
      const featured = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === featuredId);
      const target = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === targetId);
      assert.ok(waterSite);
      assert.ok(landSite);
      assert.ok(featured);
      assert.ok(target);

      await withFork(ctx, async (land) => {
        await land.take(({ descriptor }) =>
          descriptor.kind === 'play-site'
            && descriptor.cardInstanceId === landSite.instanceId
            && descriptor.cell === 'C4');
        const atlasBeforeDeath = land.state.players.north.atlas.length;
        const landVersion = land.state.stateVersion;
        await withFork(land, async (diedCtx) => {
          const died = await diedCtx.step(await diedCtx.action(({ descriptor }) =>
            descriptor.kind === 'summon-minion'
              && descriptor.cardInstanceId === featured.instanceId
              && descriptor.cell === 'C4'
              && descriptor.region === 'underground'));
          assert.equal(died.accepted, true);
          assert.deepEqual(died.receipt.events.map(({ type }) => type), [
            'minion-summoned',
            'minion-died',
          ]);
          assert.equal(died.session.state.realm.units.some(({ instanceId }) =>
            instanceId === featured.instanceId), false);
          assert.equal(died.session.state.players.north.cemetery.some(({ instanceId }) =>
            instanceId === featured.instanceId), true);
          assert.equal(died.session.state.players.north.atlas.length, atlasBeforeDeath);
          assert.equal(died.receipt.events.some(({ type }) => type === 'site-drawn'), false);
          assert.equal(died.session.state.stateVersion, landVersion + 1);
          assert.equal(await diedCtx.verifyReplay(), true);
        });
        const banished = await land.step(await land.action(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === featured.instanceId
            && descriptor.cell === 'A4'
            && descriptor.region === 'void'));
        assert.equal(banished.accepted, true);
        assert.deepEqual(banished.receipt.events.map(({ type }) => type), [
          'minion-summoned',
          'minion-banished',
        ]);
        assert.equal(banished.session.state.realm.units.some(({ instanceId }) =>
          instanceId === featured.instanceId), false);
        assert.equal(banished.session.state.players.north.cemetery.some(({ instanceId }) =>
          instanceId === featured.instanceId), false);
        assert.equal(banished.session.state.stateVersion, landVersion + 1);
        assert.equal(await land.verifyReplay(), true);
      });

      await withFork(ctx, async (movement) => {
        await movement.take(({ descriptor }) =>
          descriptor.kind === 'play-site'
            && descriptor.cardInstanceId === waterSite.instanceId
            && descriptor.cell === 'C4');
        await movement.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === featured.instanceId
            && descriptor.cell === 'C4'
            && descriptor.region === undefined);
        await movement.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await movement.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await movement.take(({ descriptor }) =>
          descriptor.kind === 'play-site' && descriptor.cell === 'C1');
        await movement.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await movement.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        const beforeMoveVersion = movement.state.stateVersion;
        const moved = await movement.step(await movement.action(({ descriptor }) =>
          descriptor.kind === 'move-and-attack'
            && descriptor.unitInstanceId === featured.instanceId
            && descriptor.path.length === 3
            && descriptor.path[0]?.cell === 'C4'
            && descriptor.path[0]?.region === 'surface'
            && descriptor.path[1]?.cell === 'B4'
            && descriptor.path[1]?.region === 'void'
            && descriptor.path[2]?.cell === 'A4'
            && descriptor.path[2]?.region === 'void'));
        assert.equal(moved.accepted, true);
        assert.deepEqual(moved.receipt.events.map(({ type }) => type), [
          'move-and-attack-activated',
          'minion-banished',
        ]);
        assert.deepEqual(moved.receipt.events[0]?.payload, {
          from: { cell: 'C4', region: 'surface' },
          path: [
            { cell: 'C4', region: 'surface' },
            { cell: 'B4', region: 'void' },
          ],
          seat: 'north',
          steps: 1,
          to: { cell: 'B4', region: 'void' },
          unitInstanceId: featured.instanceId,
        });
        assert.equal(moved.session.state.realm.units.some(({ instanceId }) =>
          instanceId === featured.instanceId), false);
        assert.equal(moved.session.state.players.north.cemetery.some(({ instanceId }) =>
          instanceId === featured.instanceId), false);
        assert.equal(moved.session.state.phase, 'main');
        assert.equal(moved.session.state.pendingCombat, null);
        assert.equal(moved.session.state.stateVersion, beforeMoveVersion + 1);
        assert.equal(await movement.verifyReplay(), true);
      });

      await withFork(ctx, async (defense) => {
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'play-site'
            && descriptor.cardInstanceId === waterSite.instanceId
            && descriptor.cell === 'C4');
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === featured.instanceId
            && descriptor.cell === 'C4'
            && descriptor.region === undefined);
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === target.instanceId
            && descriptor.cell === 'A4'
            && descriptor.region === 'void');
        await defense.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'play-site' && descriptor.cell === 'C1');
        const southAttacker = defense.state.players.south.hand.spellbook[0];
        assert.ok(southAttacker);
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === southAttacker.instanceId
            && descriptor.cell === 'A4'
            && descriptor.region === 'void');
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'move-and-attack'
            && descriptor.unitInstanceId === southAttacker.instanceId
            && descriptor.path.length === 1);
        await defense.take(({ descriptor }) =>
          descriptor.kind === 'declare-attack'
            && descriptor.target.kind === 'minion'
            && descriptor.target.instanceId === target.instanceId);
        const beforeDefendVersion = defense.state.stateVersion;
        const defended = await defense.step(await defense.action(({ descriptor }) =>
          descriptor.kind === 'defend'
            && descriptor.unitInstanceId === featured.instanceId
            && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,B4,A4'));
        assert.equal(defended.accepted, true);
        assert.deepEqual(defended.receipt.events.map(({ type }) => type), [
          'defender-moved',
          'minion-banished',
        ]);
        assert.deepEqual(defended.receipt.events[0]?.payload, {
          from: { cell: 'C4', region: 'surface' },
          instanceId: featured.instanceId,
          path: [
            { cell: 'C4', region: 'surface' },
            { cell: 'B4', region: 'void' },
          ],
          seat: 'north',
          steps: 1,
          to: { cell: 'B4', region: 'void' },
        });
        assert.deepEqual(defended.session.state.pendingCombat?.defenders, []);
        assert.equal(defended.session.state.pendingCombat?.targetRemoved, false);
        assert.equal(defended.session.state.stateVersion, beforeDefendVersion + 1);
        assert.equal(await defense.verifyReplay(), true);
      });
    });
  });
});

test('RULE-02/04 playing a site surfaces uncarried Artifacts from the covered void', async () => {
  const north: GameDeckSpec = {
    atlas: Array(6).fill('artifact-site'),
    avatar: 'artifact-avatar',
    spellbook: [...Array(4).fill('voidwalker'), ...Array(4).fill('sword')],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('artifact-site'),
    avatar: 'artifact-avatar',
    spellbook: Array(8).fill('voidwalker'),
  };
  const cards: Record<string, GameCardDefinition> = {
    'artifact-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'artifact-site': { cardType: 'site', elements: ['earth'], genesisGainMana: 6 },
    sword: {
      cardType: 'artifact',
      grantsBearerPower: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    voidwalker: {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
    },
  };
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: 'synthetic-artifact-site-entry-v1',
      },
      cards,
      decks: { north, south },
      firstSeat: 'north',
      seed,
    }),
    (session) => {
      const hand = session.state.players.north.hand.spellbook;
      return hand.some(({ cardId }) => cardId === 'voidwalker')
        && hand.filter(({ cardId }) => cardId === 'sword').length >= 2;
    },
    { from: 1, to: 63 },
  );
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'voidwalker' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'voidwalker');
    assert.ok(bearer);
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword'
      && descriptor.bearer?.instanceId === bearer.instanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword'
      && descriptor.bearer?.instanceId === bearer.instanceId);
    const [carried, stillCarried] = ctx.state.realm.artifacts ?? [];
    assert.ok(carried);
    assert.ok(stillCarried && 'bearer' in stillCarried);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === bearer.instanceId
      && descriptor.to.cell === 'B4'
      && descriptor.to.region === 'void');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'drop-artifacts'
      && descriptor.unit.instanceId === bearer.instanceId
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === carried.instanceId);
    const dropped = ctx.state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === carried.instanceId);
    assert.deepEqual(dropped, {
      cardId: carried.cardId,
      instanceId: carried.instanceId,
      location: 'B4',
      owner: 'north',
      region: 'void',
      source: carried.source,
    });

    const result = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B4'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.deepEqual(result.receipt.events.map(({ type }) => type), ['site-played', 'mana-gained']);
    assert.deepEqual(result.receipt.randomDraws, []);
    assert.deepEqual(result.session.state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === carried.instanceId), { ...dropped, region: 'surface' });
    assert.deepEqual(result.session.state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === stillCarried.instanceId), stillCarried);
    assert.deepEqual(ctx.observe('north').realm.artifacts
      ?.find(({ instanceId }) => instanceId === stillCarried.instanceId), {
      bearer: stillCarried.bearer,
      cardId: stillCarried.cardId,
      controller: 'north',
      instanceId: stillCarried.instanceId,
      location: 'B4',
      owner: 'north',
      region: 'surface',
    });
    assert.equal(result.session.state.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId)?.region, 'surface');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Charge allows a summoned minion to Move and Attack immediately', async () => {
  await withSetup(manifest(42, {
    spell: {
      charge: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'));
    const unit = ctx.state.realm.units[0];
    assert.ok(unit);
    assert.equal(unit.summoningSickness, true);
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unit.instanceId
        && descriptor.to.cell === 'C4'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'decline-attack'));
    assert.equal(ctx.state.realm.units[0]?.tapped, true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Genesis draws a hidden site and an empty Atlas loses after summoning', async () => {
  const spell: SpellFacts = {
    genesisDrawSite: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  await withSetup(manifest(40, { spell }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    const before = ctx.state.players.north;
    const drawn = before.atlas[0];
    assert.ok(drawn);
    const result = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.equal(ctx.state.players.north.atlas.length, before.atlas.length - 1);
    assert.equal(ctx.state.players.north.hand.atlas.length, before.hand.atlas.length + 1);
    assert.deepEqual(result.receipt.events.map(({ type }) => type), ['minion-summoned', 'site-drawn']);
    assert.doesNotMatch(canonicalJson(result.receipt.events[1]?.payload ?? null), new RegExp(drawn.cardId));
    assert.doesNotMatch(canonicalJson(ctx.observe('south')), new RegExp(drawn.cardId));
    assert.equal(await ctx.verifyReplay(), true);
  });

  const short = deck('genesis-short', 3, 3);
  await withSetup(manifest(40, { north: short, south: short, spell }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'));
    assert.equal(ctx.state.realm.units.length, 1);
    assert.deepEqual(ctx.state.terminal, {
      loser: 'north',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'south',
    });
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['minion-summoned', 'game-ended'],
    );
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Genesis draws a hidden spell and an empty Spellbook loses after summoning', async () => {
  const spell: SpellFacts = {
    genesisDrawSpells: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  await withSetup(manifest(127, { spell }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    const before = ctx.state.players.north;
    const drawn = before.spellbook[0];
    assert.ok(drawn);
    const result = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.equal(ctx.state.players.north.spellbook.length, before.spellbook.length - 1);
    assert.equal(ctx.state.players.north.hand.spellbook.length, before.hand.spellbook.length);
    assert.equal(ctx.state.players.north.hand.spellbook.some(({ instanceId }) => instanceId === drawn.instanceId), true);
    assert.deepEqual(result.receipt.events.map(({ type }) => type), ['minion-summoned', 'spell-drawn']);
    assert.doesNotMatch(canonicalJson(result.receipt.events[1]?.payload ?? null), new RegExp(drawn.cardId));
    assert.doesNotMatch(canonicalJson(ctx.observe('south')), new RegExp(drawn.cardId));
    assert.equal(await ctx.verifyReplay(), true);
  });

  const short = deck('genesis-spell-short', 3, 3);
  await withSetup(manifest(127, { north: short, south: short, spell }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'));
    assert.equal(ctx.state.realm.units.length, 1);
    assert.deepEqual(ctx.state.terminal, {
      loser: 'north',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'south',
    });
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['minion-summoned', 'game-ended'],
    );
    assert.equal(await ctx.verifyReplay(), true);
  });
});

async function withNumericGenesis(
  remainingCount: number,
  seed: number,
  run: (ctx: SetupCtx) => Promise<void>,
): Promise<void> {
  const decks = {
    north: deck(`genesis-spells-north-${remainingCount}`, 5, 3 + remainingCount),
    south: deck(`genesis-spells-south-${remainingCount}`, 5, 3 + remainingCount),
  };
  const cards = cardsFor(decks, {
    attack: 0,
    defense: 0,
    genesisDrawSpells: 3,
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['air'] });
  await withSetup(createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: `synthetic-genesis-spells-${remainingCount}-v1`,
    },
    cards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await run(ctx);
  });
}

test('RULE-03 numeric Genesis spell draw counts draw ordered hidden cards', async () => {
  await withNumericGenesis(3, 128, async (ctx) => {
    const before = ctx.state.players.north;
    const expectedDraws = before.spellbook.slice(0, 3);
    assert.equal(expectedDraws.length, 3);

    const result = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const after = ctx.state.players.north;
    assert.equal(after.spellbook.length, before.spellbook.length - 3);
    assert.equal(after.hand.spellbook.length, before.hand.spellbook.length + 2);
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'spell-drawn',
      'spell-drawn',
      'spell-drawn',
    ]);
    for (const drawn of expectedDraws) {
      assert.equal(after.hand.spellbook.some(({ instanceId }) => instanceId === drawn.instanceId), true);
      assert.doesNotMatch(canonicalJson(result.receipt.events), new RegExp(drawn.cardId));
      assert.doesNotMatch(canonicalJson(ctx.observe('south')), new RegExp(drawn.cardId));
    }
    assert.equal(ctx.observe('north').realm.units.some(({ attack, damage, defense }) =>
      attack === 0 && damage === 0 && defense === 0), true);
    assert.deepEqual(result.receipt.randomDraws, []);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 numeric Genesis spell draws exhaust 2, 1, or 0 remaining cards before deck loss', async () => {
  for (const remainingCount of [2, 1, 0]) {
    await withNumericGenesis(remainingCount, 129 + remainingCount, async (ctx) => {
      const before = ctx.state.players.north;
      assert.equal(before.spellbook.length, remainingCount);
      const expectedDraws = before.spellbook.slice();

      const result = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'summon-minion'));
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      assert.deepEqual(ctx.state.terminal, {
        loser: 'north',
        reason: 'deck_empty',
        status: 'finished',
        winner: 'south',
      });
      assert.equal(ctx.state.players.north.spellbook.length, 0);
      assert.deepEqual(result.receipt.events.map(({ type }) => type), [
        'minion-summoned',
        ...Array.from({ length: remainingCount }, () => 'spell-drawn' as const),
        'game-ended',
      ]);
      for (const drawn of expectedDraws) {
        assert.equal(ctx.state.players.north.hand.spellbook
          .some(({ instanceId }) => instanceId === drawn.instanceId), true);
        assert.doesNotMatch(canonicalJson(result.receipt.events), new RegExp(drawn.cardId));
        assert.doesNotMatch(
          canonicalJson(ctx.observe('south')),
          new RegExp(drawn.cardId),
        );
      }
      assert.deepEqual(result.receipt.randomDraws, []);
      assert.equal(await ctx.verifyReplay(), true);
    });
  }
});

test('RULE-03/04 Genesis resolves simultaneous area damage and enemy strikes', async () => {
  const decks = {
    north: deck('static-north', 5, 6),
    south: deck('static-south', 5, 6),
  };
  const cards = cardsFor(decks, {
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['air'] });
  const staticId = decks.north.spellbook[0]!;
  const alliedMinionId = decks.north.spellbook[1]!;
  const titanId = decks.north.spellbook[2]!;
  const teleportId = decks.south.spellbook[0]!;
  const enemyMinionId = decks.south.spellbook[1]!;
  const wardedMinionId = decks.south.spellbook[2]!;
  const undergroundMinionId = decks.south.spellbook[3]!;
  cards[staticId] = {
    ...cards[staticId]!,
    attack: 2,
    defense: 2,
    genesisDamageEachOtherUnitHere: 1,
  } as unknown as GameCardDefinition;
  cards[alliedMinionId] = {
    ...cards[alliedMinionId]!,
    deathriteDrawSite: true,
    defense: 1,
  } as GameCardDefinition;
  cards[titanId] = {
    ...cards[titanId]!,
    attack: 3,
    defense: 3,
    genesisStrikeEachEnemyHere: true,
  } as GameCardDefinition;
  cards[teleportId] = {
    cardType: 'magic',
    manaCost: 0,
    teleportAllyToTargetSite: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  for (const minionId of [enemyMinionId, wardedMinionId, undergroundMinionId]) {
    cards[minionId] = {
      ...cards[minionId]!,
      defense: 1,
      summonToAnySite: true,
    } as GameCardDefinition;
  }
  cards[wardedMinionId] = {
    ...cards[wardedMinionId]!,
    ward: true,
  } as GameCardDefinition;
  cards[undergroundMinionId] = {
    ...cards[undergroundMinionId]!,
    burrowing: true,
    mustBeCastBurrowed: true,
  } as GameCardDefinition;
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-static-servant-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [staticId]: {
        ...cards[staticId]!,
        genesisDamageEachOtherUnitHere: false,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /genesisDamageEachOtherUnitHere must be 1/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [titanId]: {
        ...cards[titanId]!,
        genesisStrikeEachEnemyHere: false,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /genesisStrikeEachEnemyHere must be true when defined/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [alliedMinionId]: {
        ...cards[alliedMinionId]!,
        genesisDisableSelfUntilDamaged: false,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /genesisDisableSelfUntilDamaged must be true when defined/);
  for (const incompatible of [{ stealth: true }, { token: true }]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [alliedMinionId]: {
          ...cards[alliedMinionId]!,
          ...incompatible,
          genesisDisableSelfUntilDamaged: true,
        } as unknown as GameCardDefinition,
      },
      seed: 1,
    }), /Genesis/);
  }
  const waterboundGenesisDisable = createGameManifest({
    ...input,
    cards: {
      ...cards,
      [alliedMinionId]: {
        ...cards[alliedMinionId]!,
        genesisDisableSelfUntilDamaged: true,
        waterbound: true,
      } as GameCardDefinition,
    },
    seed: 1,
  });
  assert.equal(
    waterboundGenesisDisable.cards[alliedMinionId]?.cardType === 'minion'
      && waterboundGenesisDisable.cards[alliedMinionId].genesisDisableSelfUntilDamaged
      && waterboundGenesisDisable.cards[alliedMinionId].waterbound,
    true,
  );
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [staticId]: {
        ...cards[staticId]!,
        genesisDrawSpells: 1,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /simultaneous Genesis/);
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players;
      const northReady = [staticId, alliedMinionId, titanId].every((cardId) =>
        opening.north.hand.spellbook.some((card) => card.cardId === cardId));
      const southAvailable = [
        ...opening.south.hand.spellbook,
        opening.south.spellbook[0],
      ].flatMap((card) => card ? [card.cardId] : []);
      return northReady && [teleportId, enemyMinionId, wardedMinionId, undergroundMinionId]
        .every((cardId) => southAvailable.includes(cardId));
    },
  );
  assert.equal((gameManifest.cards[staticId] as unknown as
    Readonly<Record<string, unknown>>).genesisDamageEachOtherUnitHere, 1);
  assert.equal((gameManifest.cards[titanId] as unknown as
    Readonly<Record<string, unknown>>).genesisStrikeEachEnemyHere, true);
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === alliedMinionId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === teleportId
      && descriptor.ally?.kind === 'avatar'
      && descriptor.allyDestination === undefined
      && descriptor.targetLocation?.cell === 'C4');
    for (const minionId of [enemyMinionId, wardedMinionId]) {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === minionId
        && descriptor.cell === 'C4'
        && descriptor.region === undefined);
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === undergroundMinionId
      && descriptor.cell === 'C4'
      && descriptor.region === 'underground');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');

    const beforeVersion = ctx.state.stateVersion;
    const beforeNorthAtlas = ctx.state.players.north.atlas.length;
    const alliedMinion = ctx.state.realm.units.find(({ cardId }) => cardId === alliedMinionId);
    const enemyMinion = ctx.state.realm.units.find(({ cardId }) => cardId === enemyMinionId);
    const wardedMinion = ctx.state.realm.units.find(({ cardId }) => cardId === wardedMinionId);
    const undergroundMinion = ctx.state.realm.units.find(({ cardId }) =>
      cardId === undergroundMinionId);
    assert.ok(alliedMinion);
    assert.ok(enemyMinion);
    assert.ok(wardedMinion);
    assert.ok(undergroundMinion);
    await withFork(ctx, async (staticCtx) => {
      const result = await staticCtx.step(await staticCtx.action(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardId === staticId
          && descriptor.cell === 'C4'));
      assert.equal(result.accepted, true);
      const source = result.session.state.realm.units.find(({ cardId }) => cardId === staticId);
      assert.ok(source);
      assert.equal(result.session.state.stateVersion, beforeVersion + 1);
      const allocations = result.receipt.events.filter(({ type }) =>
        type === 'genesis-damage-allocated');
      assert.deepEqual(allocations.map(({ payload }) => payload), [
        result.session.state.players.north.avatar.card,
        result.session.state.players.south.avatar.card,
        alliedMinion,
        enemyMinion,
        wardedMinion,
      ].map(({ instanceId }) => ({
        amount: 1,
        sourceInstanceId: source.instanceId,
        targetInstanceId: instanceId,
      })).sort((left, right) => left.targetInstanceId.localeCompare(right.targetInstanceId)));
      assert.equal(allocations.some(({ payload }) =>
        typeof payload === 'object'
          && payload !== null
          && 'targetInstanceId' in payload
          && payload.targetInstanceId === source.instanceId), false);
      assert.deepEqual({
        northLife: result.session.state.players.north.avatar.life,
        southLife: result.session.state.players.south.avatar.life,
        sourceDamage: source.damage,
        undergroundDamage: result.session.state.realm.units.find(({ instanceId }) =>
          instanceId === undergroundMinion.instanceId)?.damage,
        warded: result.session.state.realm.units.find(({ instanceId }) =>
          instanceId === wardedMinion.instanceId)?.warded,
      }, {
        northLife: 19,
        southLife: 19,
        sourceDamage: 0,
        undergroundDamage: 0,
        warded: false,
      });
      assert.equal(result.session.state.realm.units.some(({ instanceId }) =>
        instanceId === alliedMinion.instanceId), false);
      assert.equal(result.session.state.realm.units.some(({ instanceId }) =>
        instanceId === enemyMinion.instanceId), false);
      assert.equal(result.session.state.players.north.atlas.length, beforeNorthAtlas - 1);
      const avatarIds = new Set([
        result.session.state.players.north.avatar.card.instanceId,
        result.session.state.players.south.avatar.card.instanceId,
      ]);
      const targetIds = [
        ...avatarIds,
        alliedMinion.instanceId,
        enemyMinion.instanceId,
        wardedMinion.instanceId,
      ].sort();
      const resolutionTypes = targetIds.flatMap((instanceId) =>
        avatarIds.has(instanceId)
          ? ['damage-dealt', 'avatar-life-lost']
          : instanceId === wardedMinion.instanceId
            ? ['damage-dealt', 'ward-broken']
            : ['damage-dealt']);
      assert.deepEqual(result.receipt.events.map(({ type }) => type), [
        'minion-summoned',
        ...Array.from({ length: 5 }, () => 'genesis-damage-allocated'),
        ...resolutionTypes,
        'site-drawn',
        'minion-died',
        'minion-died',
      ]);
      assert.equal(result.receipt.randomDraws.length, 0);
      assert.equal(result.session.state.pendingCombat, null);
      assert.equal(result.session.state.terminal.status, 'active');
      assert.equal(await staticCtx.verifyReplay(), true);
    });

    const titanResult = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardId === titanId
        && descriptor.cell === 'C4'));
    assert.equal(titanResult.accepted, true);
    const titan = titanResult.session.state.realm.units.find(({ cardId }) => cardId === titanId);
    assert.ok(titan);
    const strikeAllocations = titanResult.receipt.events.filter(({ type }) =>
      type === 'strike-damage-allocated');
    assert.deepEqual(strikeAllocations.map(({ payload }) => payload), [
      titanResult.session.state.players.south.avatar.card,
      enemyMinion,
      wardedMinion,
    ].map(({ instanceId }) => ({
      amount: 3,
      strikerInstanceId: titan.instanceId,
      targetInstanceId: instanceId,
    })).sort((left, right) => left.targetInstanceId.localeCompare(right.targetInstanceId)));
    assert.deepEqual({
      alliedPresent: titanResult.session.state.realm.units.some(({ instanceId }) =>
        instanceId === alliedMinion.instanceId),
      enemyPresent: titanResult.session.state.realm.units.some(({ instanceId }) =>
        instanceId === enemyMinion.instanceId),
      southLife: titanResult.session.state.players.south.avatar.life,
      titanDamage: titan.damage,
      undergroundDamage: titanResult.session.state.realm.units.find(({ instanceId }) =>
        instanceId === undergroundMinion.instanceId)?.damage,
      warded: titanResult.session.state.realm.units.find(({ instanceId }) =>
        instanceId === wardedMinion.instanceId)?.warded,
    }, {
      alliedPresent: true,
      enemyPresent: false,
      southLife: 17,
      titanDamage: 0,
      undergroundDamage: 0,
      warded: false,
    });
    const titanTargetIds = [
      titanResult.session.state.players.south.avatar.card.instanceId,
      enemyMinion.instanceId,
      wardedMinion.instanceId,
    ].sort();
    const titanResolutionTypes = titanTargetIds.flatMap((instanceId) =>
      instanceId === titanResult.session.state.players.south.avatar.card.instanceId
        ? ['damage-dealt', 'avatar-life-lost']
        : instanceId === wardedMinion.instanceId
          ? ['damage-dealt', 'ward-broken']
          : ['damage-dealt']);
    assert.deepEqual(titanResult.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      ...Array.from({ length: 3 }, () => 'strike-damage-allocated'),
      ...titanResolutionTypes,
      'minion-died',
    ]);
    assert.equal(titanResult.receipt.randomDraws.length, 0);
    assert.equal(titanResult.session.state.pendingCombat, null);
    assert.equal(titanResult.session.state.terminal.status, 'active');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 Vile Imp may deal 2 damage to one adjacent unit or decline', async () => {
  const gameManifest = manifest(391, {
    northSpell: {
      attack: 2,
      defense: 2,
      genesisMayDamageTargetAdjacentUnit: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    southSpell: {
      defense: 2,
      manaCost: 0,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      ward: true,
    },
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const card = ctx.state.players.north.hand.spellbook[0];
    assert.ok(card);
    assert.equal((gameManifest.cards[card.cardId] as unknown as
      Readonly<Record<string, unknown>>).genesisMayDamageTargetAdjacentUnit, 2);
    const avatar = ctx.state.players.north.avatar.card;
    const wardedTarget = ctx.state.realm.units[0];
    assert.ok(wardedTarget);
    assert.equal(wardedTarget.warded, true);
    const summons = (await ctx.legalActions('north'))
      .flatMap(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === card.instanceId
        && descriptor.cell === 'C4' ? [descriptor] : []);
    assert.deepEqual(summons.map((descriptor) => ({
      choice: descriptor.genesisDamageChoice,
      target: descriptor.genesisDamageTarget?.instanceId,
    })).sort((left, right) => (left.target ?? '').localeCompare(right.target ?? '')), [
      { choice: 'decline', target: undefined },
      { choice: 'target', target: card.instanceId },
      { choice: 'target', target: avatar.instanceId },
      { choice: 'target', target: wardedTarget.instanceId },
    ].sort((left, right) => (left.target ?? '').localeCompare(right.target ?? '')));
    const checkpointVersion = ctx.state.stateVersion;

    await withFork(ctx, async (declineCtx) => {
      const decline = await declineCtx.step(await declineCtx.action(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === card.instanceId
          && descriptor.cell === 'C4'
          && descriptor.genesisDamageChoice === 'decline'));
      assert.equal(decline.accepted, true);
      assert.equal(decline.session.state.players.north.avatar.life, 20);
      assert.deepEqual(decline.receipt.events.map(({ type }) => type), ['minion-summoned']);
      assert.equal(await declineCtx.verifyReplay(), true);
    });

    await withFork(ctx, async (targetedCtx) => {
      const targeted = await targetedCtx.step(await targetedCtx.action(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === card.instanceId
          && descriptor.cell === 'C4'
          && descriptor.genesisDamageChoice === 'target'
          && descriptor.genesisDamageTarget?.instanceId === avatar.instanceId));
      assert.equal(targeted.accepted, true);
      const source = targeted.session.state.realm.units.find(({ instanceId }) =>
        instanceId === card.instanceId);
      assert.ok(source);
      assert.equal(source.damage, 0);
      assert.equal(targeted.session.state.players.north.avatar.life, 18);
      assert.deepEqual(targeted.receipt.events.map(({ type }) => type), [
        'minion-summoned',
        'genesis-damage-allocated',
        'damage-dealt',
        'avatar-life-lost',
      ]);
      assert.deepEqual(targeted.receipt.events[1]?.payload, {
        amount: 2,
        sourceInstanceId: source.instanceId,
        targetInstanceId: avatar.instanceId,
      });
      assert.equal(targeted.receipt.randomDraws.length, 0);
      assert.equal(targeted.session.state.stateVersion, checkpointVersion + 1);
      assert.equal(await targetedCtx.verifyReplay(), true);
    });

    const warded = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === card.instanceId
        && descriptor.cell === 'C4'
        && descriptor.genesisDamageChoice === 'target'
        && descriptor.genesisDamageTarget?.instanceId === wardedTarget.instanceId));
    assert.equal(warded.accepted, true);
    const wardedSurvivor = warded.session.state.realm.units.find(({ instanceId }) =>
      instanceId === wardedTarget.instanceId);
    assert.ok(wardedSurvivor);
    assert.equal(wardedSurvivor.damage, 0);
    assert.equal(wardedSurvivor.warded, false);
    assert.deepEqual(warded.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'genesis-damage-allocated',
      'damage-dealt',
      'ward-broken',
    ]);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test("RULE-03 Genesis life loss reaches but cannot cross Death's Door", async () => {
  const readyToSummon = async (ctx: SetupCtx): Promise<void> => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
  };
  const summon = async (ctx: SetupCtx) => {
    const beforeVersion = ctx.state.stateVersion;
    const result = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'));
    assert.equal(result.accepted, true);
    assert.equal(result.session.state.stateVersion, beforeVersion + 1);
    return result;
  };

  await withSetup(manifest(226, {
    avatar: { attack: 1, defense: 1, drawSpell: false, life: 3 },
    spell: {
      genesisLoseControllerLife: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await readyToSummon(ctx);
    const lifeThree = await summon(ctx);
    const lifeThreeSource = lifeThree.session.state.realm.units.at(-1)?.instanceId;
    assert.ok(lifeThreeSource);
    assert.equal(lifeThree.session.state.players.north.avatar.life, 1);
    assert.deepEqual(lifeThree.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'avatar-life-lost',
    ]);
    assert.deepEqual(lifeThree.receipt.events[1]?.payload, {
      amount: 2,
      life: 1,
      seat: 'north',
      sourceInstanceId: lifeThreeSource,
    });
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(227, {
    avatar: { attack: 1, defense: 1, drawSpell: false, life: 2 },
    spell: {
      genesisLoseControllerLife: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await readyToSummon(ctx);
    const lifeTwo = await summon(ctx);
    const lifeTwoSource = lifeTwo.session.state.realm.units.at(-1)?.instanceId;
    assert.ok(lifeTwoSource);
    assert.equal(lifeTwo.session.state.players.north.avatar.life, 0);
    assert.deepEqual(lifeTwo.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'avatar-life-lost',
      'avatar-reached-deaths-door',
    ]);
    assert.deepEqual(lifeTwo.receipt.events[1]?.payload, {
      amount: 2,
      life: 0,
      seat: 'north',
      sourceInstanceId: lifeTwoSource,
    });
    assert.deepEqual(lifeTwo.receipt.events[2]?.payload, {
      seat: 'north',
      sourceInstanceId: lifeTwoSource,
      turnNumber: 1,
    });
    assert.deepEqual(lifeTwo.session.state.terminal, { status: 'active' });
    assert.equal(await ctx.verifyReplay(), true);

    const deathDoorTurn = ctx.state.players.north.avatar.deathDoorTurn;
    const lifeZero = await summon(ctx);
    assert.equal(lifeZero.session.state.players.north.avatar.life, 0);
    assert.equal(lifeZero.session.state.players.north.avatar.deathDoorTurn, deathDoorTurn);
    assert.deepEqual(lifeZero.receipt.events.map(({ type }) => type), ['minion-summoned']);
    assert.deepEqual(lifeZero.session.state.terminal, { status: 'active' });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Grain Sparrow Genesis gains controller life through shared healing semantics', async () => {
  const decks = { north: deck('grain-north'), south: deck('grain-south') };
  const cards = cardsFor(decks, {
    airborne: true,
    genesisHealController: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  for (const cardId of decks.south.spellbook) {
    cards[cardId] = {
      cardType: 'magic',
      damageTargetUnit: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    };
  }
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-grain-sparrow-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 229,
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.kind === 'avatar'
      && descriptor.target.seat === 'north');
    assert.equal(ctx.state.players.north.avatar.life, 19);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const beforeVersion = ctx.state.stateVersion;
    const result = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
    assert.equal(result.accepted, true);
    assert.equal(result.session.state.stateVersion, beforeVersion + 1);
    assert.equal(result.session.state.players.north.avatar.life, 20);
    const sourceInstanceId = result.session.state.realm.units.at(-1)?.instanceId;
    assert.ok(sourceInstanceId);
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'avatar-healed',
    ]);
    assert.deepEqual(result.receipt.events[1]?.payload, {
      amount: 1,
      attemptedAmount: 2,
      life: 20,
      seat: 'north',
      sourceInstanceId,
    });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 site Genesis grants temporary mana once, pays a summon, and expires', async () => {
  await withSetup(manifest(55, {
    site: { genesisGainMana: 1 },
    spell: {
      manaCost: 2,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    assert.equal(ctx.state.players.north.mana, 2);
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['site-played', 'mana-gained'],
    );
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'));
    assert.equal(ctx.state.players.north.mana, 0);
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    assert.equal(ctx.state.players.north.mana, 0);
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    assert.equal(ctx.state.players.north.mana, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 site Genesis heals every nearby Avatar through shared life caps', async () => {
  const base = manifest(247, {
    avatar: { attack: 1, defense: 1, drawSpell: false, life: 10 },
  });
  await withPreview(base, async (preview) => {
    const [plainC4, plainC3, holyGround] =
      preview.state.players.north.hand.atlas.map(({ cardId }) => cardId);
    const [plainC1, plainC2] =
      preview.state.players.south.hand.atlas.map(({ cardId }) => cardId);
    assert.ok(plainC4);
    assert.ok(plainC3);
    assert.ok(holyGround);
    assert.ok(plainC1);
    assert.ok(plainC2);

    const cards: Record<string, GameCardDefinition> = { ...base.cards };
    for (const cardId of base.decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        damageTargetUnit: 1,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    for (const cardId of base.decks.south.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        damageTargetUnit: 3,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    cards[holyGround] = {
      cardType: 'site',
      elements: ['earth'],
      genesisHealNearbyAvatars: 3,
    };
    const input = {
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    };
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [holyGround]: {
          ...cards[holyGround]!,
          genesisHealNearbyAvatars: 2,
        } as unknown as GameCardDefinition,
      },
    }), /genesisHealNearbyAvatars must be 3/);
    const gameManifest = createGameManifest(input);
    assert.equal(gameManifest.cards[holyGround]?.cardType === 'site'
      && gameManifest.cards[holyGround].genesisHealNearbyAvatars, 3);

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === plainC4 && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === plainC1 && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.target?.kind === 'avatar' && descriptor.target.seat === 'south');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === plainC3 && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.target?.kind === 'avatar' && descriptor.target.seat === 'north');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === plainC2 && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId
        && descriptor.from.cell === 'C4' && descriptor.to.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      assert.deepEqual({
        north: ctx.state.players.north.avatar.life,
        south: ctx.state.players.south.avatar.life,
      }, { north: 7, south: 9 });

      await withFork(ctx, async (far) => {
        await far.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await far.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        const farResult = await far.step(await far.action(({ descriptor }) =>
          descriptor.kind === 'play-site'
            && descriptor.cardId === holyGround
            && descriptor.cell === 'B3'));
        assert.equal(farResult.accepted, true);
        const farSource = farResult.session.state.realm.sites.B3;
        assert.ok(farSource);
        assert.deepEqual({
          north: farResult.session.state.players.north.avatar.life,
          south: farResult.session.state.players.south.avatar.life,
        }, { north: 10, south: 9 });
        assert.deepEqual(farResult.receipt.events.map(({ type }) => type), [
          'site-played',
          'avatar-healed',
        ]);
        assert.deepEqual(farResult.receipt.events[1]?.payload, {
          amount: 3,
          attemptedAmount: 3,
          life: 10,
          seat: 'north',
          sourceInstanceId: farSource.instanceId,
        });
        assert.deepEqual(farResult.receipt.randomDraws, []);
        assert.equal(await far.verifyReplay(), true);
      });

      await withFork(ctx, async (near) => {
        await near.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === near.state.players.south.avatar.card.instanceId
          && descriptor.from.cell === 'C1'
          && descriptor.to.cell === 'C2');
        await near.take(({ descriptor }) => descriptor.kind === 'decline-attack');
        await near.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await near.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        const nearResult = await near.step(await near.action(({ descriptor }) =>
          descriptor.kind === 'play-site'
            && descriptor.cardId === holyGround
            && descriptor.cell === 'B3'));
        assert.equal(nearResult.accepted, true);
        const nearSource = nearResult.session.state.realm.sites.B3;
        assert.ok(nearSource);
        assert.deepEqual({
          north: nearResult.session.state.players.north.avatar.life,
          south: nearResult.session.state.players.south.avatar.life,
        }, { north: 10, south: 10 });
        assert.deepEqual(nearResult.receipt.events.map(({ type }) => type), [
          'site-played',
          'avatar-healed',
          'avatar-healed',
        ]);
        assert.deepEqual(nearResult.receipt.events.slice(1).map(({ payload }) => payload), [
          {
            amount: 3,
            attemptedAmount: 3,
            life: 10,
            seat: 'north',
            sourceInstanceId: nearSource.instanceId,
          },
          {
            amount: 1,
            attemptedAmount: 3,
            life: 10,
            seat: 'south',
            sourceInstanceId: nearSource.instanceId,
          },
        ]);
        assert.deepEqual(nearResult.receipt.randomDraws, []);
        assert.equal(await near.verifyReplay(), true);
      });
    });
  });
});

test('RULE-03 site Genesis makes units at nearby sites Immobile until its controller next turn', async () => {
  const base = manifest(246);
  await withPreview(base, async (preview) => {
    const [plainC4, plainC3, quagmire] =
      preview.state.players.north.hand.atlas.map(({ cardId }) => cardId);
    const sinkhole = preview.state.players.south.hand.atlas[0]?.cardId;
    const [casterA, casterB] =
      preview.state.players.north.hand.spellbook.map(({ cardId }) => cardId);
    const disabledEnemy = preview.state.players.south.hand.spellbook[0]?.cardId;
    assert.ok(plainC4);
    assert.ok(plainC3);
    assert.ok(quagmire);
    assert.ok(sinkhole);
    assert.ok(casterA);
    assert.ok(casterB);
    assert.ok(disabledEnemy);

    const cards: Record<string, GameCardDefinition> = { ...base.cards };
    for (const cardId of base.decks.north.spellbook) {
      cards[cardId] = {
        cardType: 'magic',
        manaCost: 0,
        teleportAllyToTargetSite: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    for (const cardId of [casterA, casterB]) {
      cards[cardId] = {
        attack: 2,
        cardType: 'minion',
        defense: 2,
        manaCost: 0,
        ...(cardId === casterA ? { movementBonus: 2 as const } : {}),
        spellcaster: true,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    cards[disabledEnemy] = {
      attack: 2,
      cardType: 'minion',
      defense: 2,
      immobile: true,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      waterbound: true,
    };
    cards[quagmire] = {
      cardType: 'site',
      elements: ['earth'],
      genesisImmobilizeNearbyUntilNextTurn: true,
    };
    cards[sinkhole] = {
      cardType: 'site',
      elements: ['earth'],
      sacrificeToDestroyNearbySite: true,
    };
    const input = {
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    };
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [quagmire]: {
          ...cards[quagmire]!,
          genesisImmobilizeNearbyUntilNextTurn: false,
        } as unknown as GameCardDefinition,
      },
    }), /genesisImmobilizeNearbyUntilNextTurn/);
    const gameManifest = createGameManifest(input);
    assert.equal(gameManifest.cards[quagmire]?.cardType === 'site'
      && gameManifest.cards[quagmire].genesisImmobilizeNearbyUntilNextTurn, true);

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === plainC4 && descriptor.cell === 'C4');
      for (const cardId of [casterA, casterB]) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === cardId && descriptor.cell === 'C4');
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === sinkhole && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === disabledEnemy && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === plainC3 && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === quagmire && descriptor.cell === 'C2');

      const quagmireSite = ctx.state.realm.sites.C2;
      const firstCaster = ctx.state.realm.units.find(({ cardId }) => cardId === casterA);
      const secondCaster = ctx.state.realm.units.find(({ cardId }) => cardId === casterB);
      const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === disabledEnemy);
      assert.ok(quagmireSite);
      assert.ok(firstCaster);
      assert.ok(secondCaster);
      assert.ok(enemy);
      let view = ctx.observe('north');
      assert.deepEqual(view.realm.immobileAreas, [{
        cells: ['C1', 'C2', 'C3'],
        expiresAtSeat: 'north',
        sourceInstanceId: quagmireSite.instanceId,
      }]);
      assert.equal(view.players.north.avatar.immobile, false);
      assert.equal(view.players.south.avatar.immobile, true);
      const observedEnemy = view.realm.units.find(({ instanceId }) => instanceId === enemy.instanceId);
      assert.ok(observedEnemy);
      assert.deepEqual({
        disabled: observedEnemy.disabled,
        immobile: observedEnemy.immobile,
      }, { disabled: true, immobile: true });
      const areaCells = new Set(['C1', 'C2', 'C3']);
      const outsidePaths = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
        descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === firstCaster.instanceId
          ? [descriptor.path]
          : []);
      assert.equal(outsidePaths.some((path) => path.at(-1)?.cell === 'C3'), true);
      assert.equal(outsidePaths.every((path) => {
        const entryIndex = path.findIndex(({ cell }, index) => index > 0 && areaCells.has(cell));
        return entryIndex < 0 || entryIndex === path.length - 1;
      }), true);

      const teleports = [...ctx.state.players.north.hand.spellbook];
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === teleports[0]?.instanceId
        && descriptor.casterInstanceId === firstCaster.instanceId
        && descriptor.ally?.instanceId === secondCaster.instanceId
        && descriptor.targetLocation?.cell === 'C3');
      view = ctx.observe('north');
      assert.equal(view.realm.units.find(({ instanceId }) =>
        instanceId === secondCaster.instanceId)?.immobile, true);
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === teleports[1]?.instanceId
        && descriptor.casterInstanceId === secondCaster.instanceId
        && descriptor.ally?.instanceId === secondCaster.instanceId
        && descriptor.targetLocation?.cell === 'C4');
      view = ctx.observe('north');
      assert.equal(view.realm.units.find(({ instanceId }) =>
        instanceId === secondCaster.instanceId)?.immobile, false);

      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const southAvatarId = ctx.state.players.south.avatar.card.instanceId;
      const avatarPaths = (await ctx.legalActions('south')).flatMap(({ descriptor }) =>
        descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === southAvatarId
          ? [descriptor.path]
          : []);
      assert.equal(avatarPaths.length > 0 && avatarPaths.every((path) => path.length === 1), true);
      await ctx.take(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
        && descriptor.sourceSiteInstanceId === ctx.state.realm.sites.C1?.instanceId
        && descriptor.targetCell === 'C2');
      view = ctx.observe('south');
      assert.deepEqual(view.realm.immobileAreas?.[0]?.cells, ['C1', 'C2', 'C3']);
      assert.equal(view.players.south.avatar.immobile, true);
      assert.equal(view.realm.units.find(({ instanceId }) => instanceId === enemy.instanceId)?.immobile, true);

      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      view = ctx.observe('north');
      assert.equal(view.realm.immobileAreas, undefined);
      assert.equal(view.players.south.avatar.immobile, false);
      assert.deepEqual({
        disabled: view.realm.units.find(({ instanceId }) => instanceId === enemy.instanceId)?.disabled,
        immobile: view.realm.units.find(({ instanceId }) => instanceId === enemy.instanceId)?.immobile,
      }, { disabled: true, immobile: false });
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === southAvatarId
          && descriptor.path.length > 1), true);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03 Aura occupies any canonical 2x2 area and grounds site minions for three controller turns', async () => {
  const base = manifest(247);
  await withPreview(base, async (preview) => {
  const [auraCardId, airborneCardId, burrowingCardId] =
    preview.state.players.north.hand.spellbook.map(({ cardId }) => cardId);
  const northSiteId = preview.state.players.north.hand.atlas[0]?.cardId;
  const southSiteId = preview.state.players.south.hand.atlas[0]?.cardId;
  const voidwalkCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
  assert.ok(auraCardId);
  assert.ok(airborneCardId);
  assert.ok(burrowingCardId);
  assert.ok(northSiteId);
  assert.ok(southSiteId);
  assert.ok(voidwalkCardId);

  const cards: Record<string, GameCardDefinition> = {
    ...base.cards,
    [auraCardId]: {
      cardType: 'aura',
      immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: true,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [airborneCardId]: {
      airborne: true,
      attack: 2,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [burrowingCardId]: {
      attack: 2,
      burrowing: true,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [voidwalkCardId]: {
      attack: 2,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
    },
  };
  const input = {
    authority: base.authority,
    cards,
    decks: base.decks,
    firstSeat: base.firstSeat,
    seed: base.seed,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [auraCardId]: {
        ...cards[auraCardId]!,
        immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: false,
      } as unknown as GameCardDefinition,
    },
  }), /immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns/);
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards[auraCardId], cards[auraCardId]);

  await withSetup(gameManifest, async (ctx) => {
  await ctx.keep();
  await ctx.keep();
  await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === northSiteId && descriptor.cell === 'C4');
  await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === airborneCardId && descriptor.cell === 'C4'
    && descriptor.region === undefined);
  await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === burrowingCardId && descriptor.cell === 'C4'
    && descriptor.region === 'underground');

  const auraCasts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
    descriptor.kind === 'cast-aura' && descriptor.cardId === auraCardId);
  assert.equal(auraCasts.length, 12);
  assert.deepEqual(auraCasts.flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-aura' ? [descriptor.cells] : []), [
    ['A1', 'A2', 'B1', 'B2'],
    ['A2', 'A3', 'B2', 'B3'],
    ['A3', 'A4', 'B3', 'B4'],
    ['B1', 'B2', 'C1', 'C2'],
    ['B2', 'B3', 'C2', 'C3'],
    ['B3', 'B4', 'C3', 'C4'],
    ['C1', 'C2', 'D1', 'D2'],
    ['C2', 'C3', 'D2', 'D3'],
    ['C3', 'C4', 'D3', 'D4'],
    ['D1', 'D2', 'E1', 'E2'],
    ['D2', 'D3', 'E2', 'E3'],
    ['D3', 'D4', 'E3', 'E4'],
  ]);
  const affectedCells = ['B3', 'B4', 'C3', 'C4'] as const;
  await ctx.take(({ descriptor }) => descriptor.kind === 'cast-aura'
    && descriptor.cells.every((cell, index) => cell === affectedCells[index]));

  const auraInstance = ctx.state.realm.auras?.[0];
  const airborne = ctx.state.realm.units.find(({ cardId }) => cardId === airborneCardId);
  const burrowing = ctx.state.realm.units.find(({ cardId }) => cardId === burrowingCardId);
  assert.ok(auraInstance);
  assert.ok(airborne);
  assert.ok(burrowing);
  let view = ctx.observe('north');
  assert.deepEqual(view.realm.auras, [{
    cardId: auraCardId,
    cells: affectedCells,
    controller: 'north',
    instanceId: auraInstance.instanceId,
    owner: 'north',
    turnCounters: 0,
  }]);
  assert.deepEqual(view.realm.immobileAreas, [{
    cells: affectedCells,
    minionsAtSitesOnly: true,
    sourceInstanceId: auraInstance.instanceId,
    suppressesAirborne: true,
  }]);
  assert.equal(view.players.north.avatar.immobile, false);
  assert.deepEqual(view.realm.units
    .filter(({ instanceId }) => instanceId === airborne.instanceId
      || instanceId === burrowing.instanceId)
    .map(({ airborne: observedAirborne, immobile, region }) => ({
      airborne: observedAirborne,
      immobile,
      region,
    })), [
    { airborne: false, immobile: true, region: 'surface' },
    { airborne: false, immobile: true, region: 'underground' },
  ]);
  assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'aura-conjured'), true);

  await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
  assert.equal(ctx.observe('north').realm.auras?.[0]?.turnCounters, 1);
  await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === southSiteId && descriptor.cell === 'C1');
  await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === voidwalkCardId && descriptor.cell === 'B3'
    && descriptor.region === 'void');
  const voidwalk = ctx.state.realm.units.find(({ cardId }) => cardId === voidwalkCardId);
  assert.ok(voidwalk);
  view = ctx.observe('south');
  assert.equal(view.realm.units.find(({ instanceId }) =>
    instanceId === voidwalk.instanceId)?.immobile, false);

  await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
  await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
  assert.equal(ctx.observe('north').realm.auras?.[0]?.turnCounters, 2);
  await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
  await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

  view = ctx.observe('north');
  assert.equal(view.realm.auras, undefined);
  assert.equal(view.realm.immobileAreas, undefined);
  assert.deepEqual(view.realm.units
    .filter(({ instanceId }) => instanceId === airborne.instanceId
      || instanceId === burrowing.instanceId)
    .map(({ airborne: observedAirborne, immobile, region }) => ({
      airborne: observedAirborne,
      immobile,
      region,
    })), [
    { airborne: true, immobile: false, region: 'surface' },
    { airborne: false, immobile: false, region: 'underground' },
  ]);
  assert.equal(view.players.north.cemetery.some(({ instanceId }) =>
    instanceId === auraInstance.instanceId), true);
  assert.deepEqual(ctx.session.transcript.at(-1)?.events
    .filter(({ type }) => type.startsWith('aura-'))
    .map(({ type }) => type), ['aura-turn-counted', 'aura-dispelled']);
  assert.equal(await ctx.verifyReplay(), true);
  });
  });
});

test('RULE-03 an end-turn Aura damages a random affected unit before its optional step', async () => {
  const ctx = await SetupCtx.open(manifest(2));
  try {
    for (let seed = 2; seed <= 100; seed += 1) {
      const base = manifest(seed);
      await ctx.reset(base);
      const [luckyCharmId, auraCardId, minionCardId] =
        ctx.state.players.north.hand.spellbook.map(({ cardId }) => cardId);
      const siteCardId = ctx.state.players.north.hand.atlas[0]?.cardId;
      if (!luckyCharmId || !auraCardId || !minionCardId || !siteCardId) continue;
      const cards: Record<string, GameCardDefinition> = {
        ...base.cards,
        [luckyCharmId]: {
          bearerControllerChoosesExtraRandomOutcome: true,
          cardType: 'artifact',
          manaCost: 0,
          thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        },
        [auraCardId]: {
          atEndOfControllerTurnDamageRandomUnitAtAffectedSitesThenMayMoveOneStep: 3,
          cardType: 'aura',
          manaCost: 0,
          thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        },
        [minionCardId]: {
          attack: 1,
          cardType: 'minion',
          defense: 10,
          manaCost: 0,
          thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        },
      };
      await ctx.reset(createGameManifest({
        authority: base.authority,
        cards,
        decks: base.decks,
        firstSeat: base.firstSeat,
        seed,
      }));
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === siteCardId && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
        && descriptor.cardId === luckyCharmId && descriptor.bearer?.kind === 'avatar');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === minionCardId && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-aura'
        && descriptor.cardId === auraCardId
        && descriptor.cells.join(',') === 'B3,B4,C3,C4');
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'resolve-end-turn-aura-random'), false);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      const randomOutcomes = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'resolve-end-turn-aura-random');
      if (randomOutcomes.length !== 2) continue;
      assert.equal(ctx.state.phase, 'end-turn-aura');
      const committed = ctx.session.transcript.at(-1)!;
      assert.equal(committed.events.some(({ type }) => type === 'damage-dealt'), false);
      assert.equal(committed.events.some(({ type }) => type === 'turn-ended'), false);
      assert.equal(committed.randomDraws.length, 2);
      assert.equal(committed.randomDraws.every(({ purpose }) =>
        purpose === 'aura_end_turn_random_unit_at_affected_sites'), true);

      const aura = ctx.state.realm.auras?.[0];
      assert.ok(aura);
      const chosen = await ctx.action(({ descriptor }) =>
        descriptor.kind === 'resolve-end-turn-aura-random');
      assert.equal(chosen.descriptor.kind, 'resolve-end-turn-aura-random');
      if (chosen.descriptor.kind !== 'resolve-end-turn-aura-random') return;
      const chosenId = chosen.descriptor.outcomeInstanceId;
      const result = await ctx.step(chosen);
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      assert.equal(result.receipt.randomDraws.length, 0);
      assert.equal(result.receipt.events.some(({ type, payload }) =>
        type === 'aura-end-turn-damage-allocated'
          && payload !== null
          && typeof payload === 'object'
          && 'targetInstanceId' in payload
          && payload.targetInstanceId === chosenId), true);
      assert.equal(ctx.state.phase, 'end-turn-aura');

      const moves = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'resolve-end-turn-aura-move');
      assert.equal(moves.length, 4);
      const forgedDescriptor = {
        auraInstanceId: aura.instanceId,
        cells: ['A1', 'A2', 'B1', 'B2'] as const,
        kind: 'resolve-end-turn-aura-move' as const,
      };
      const forged = await ctx.stepRequest({
        actionId: opaqueActionId('sorcery-core-v1', 'north', ctx.state.stateVersion, forgedDescriptor),
        seat: 'north',
        stateVersion: ctx.state.stateVersion,
      });
      assert.equal(forged.accepted, false);
      if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
      assert.equal(hashGameState(forged.session.state), ctx.stateHash());

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'resolve-end-turn-aura-move'
          && descriptor.cells?.join(',') === 'C3,C4,D3,D4');
      assert.deepEqual(ctx.state.realm.auras?.[0]?.cells, ['C3', 'C4', 'D3', 'D4']);
      assert.equal(ctx.state.phase, 'draw');
      assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'turn-ended'), true);

      for (let controllerTurn = 2; controllerTurn <= 3; controllerTurn += 1) {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        if (!ctx.state.players.south.domainEstablished) {
          await ctx.take(({ descriptor }) =>
            descriptor.kind === 'play-site' && descriptor.cell === 'C1');
        }
        await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'resolve-end-turn-aura-random');
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'resolve-end-turn-aura-move' && descriptor.cells === undefined);
      }
      assert.equal(ctx.state.realm.auras, undefined);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === aura.instanceId), true);
      assert.deepEqual(ctx.session.transcript.at(-1)?.events
        .filter(({ type }) => type.startsWith('aura-'))
        .map(({ type }) => type), [
        'aura-move-declined',
        'aura-turn-counted',
        'aura-dispelled',
      ]);
      assert.equal(await ctx.verifyReplay(), true);
      return;
    }
    assert.fail('expected Lucky Charm end-turn Aura seed with two outcomes');
  } finally {
    await ctx.close();
  }
});

test('RULE-03 oversized minions occupy one canonical 2x2 footprint for movement, combat, Auras, and terrain', async () => {
  const base = manifest(248);
  await withPreview(base, async (preview) => {
  const [giantCardId, auraCardId, artifactCardId] = preview.state.players.north.hand.spellbook
    .map(({ cardId }) => cardId);
  const [enemyCardId, caveInCardId] = preview.state.players.south.hand.spellbook
    .map(({ cardId }) => cardId);
  const remoteEnemyCardId = preview.state.players.south.spellbook[0]?.cardId;
  const [teleportCardId, blinkCardId, leapCardId] = preview.state.players.north.spellbook
    .slice(0, 3)
    .map(({ cardId }) => cardId);
  const waterSiteCardId = preview.state.players.north.hand.atlas[0]?.cardId;
  assert.ok(giantCardId);
  assert.ok(auraCardId);
  assert.ok(artifactCardId);
  assert.ok(enemyCardId);
  assert.ok(caveInCardId);
  assert.ok(remoteEnemyCardId);
  assert.ok(teleportCardId);
  assert.ok(blinkCardId);
  assert.ok(leapCardId);
  assert.ok(waterSiteCardId);

  const cards: Record<string, GameCardDefinition> = {
    ...base.cards,
    [giantCardId]: {
      attack: 8,
      cardType: 'minion',
      defense: 8,
      manaCost: 0,
      occupiesSquareArea: 2,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [auraCardId]: {
      cardType: 'aura',
      immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: true,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [artifactCardId]: {
      cardType: 'artifact',
      grantsBearerPower: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [enemyCardId]: {
      attack: 2,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [remoteEnemyCardId]: {
      attack: 2,
      cardType: 'minion',
      charge: true,
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [caveInCardId]: {
      burrowAllMinionsAndArtifactsAtTargetLandSite: true,
      cardType: 'magic',
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [teleportCardId]: {
      cardType: 'magic',
      manaCost: 0,
      teleportAllyToTargetSite: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [blinkCardId]: {
      cardType: 'magic',
      manaCost: 0,
      teleportNearbyAllyThenDrawCard: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [leapCardId]: {
      cardType: 'magic',
      leapAttackAlly: true,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    [waterSiteCardId]: {
      cardType: 'site',
      elements: ['water'],
    },
  };
  const input = {
    authority: base.authority,
    cards,
    decks: base.decks,
    firstSeat: base.firstSeat,
    seed: base.seed,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [giantCardId]: {
        ...cards[giantCardId]!,
        occupiesSquareArea: 3,
      } as unknown as GameCardDefinition,
    },
  }), /occupiesSquareArea must be 2/);
  for (const incompatibleFact of [
    { connectsTopBottom: true as const },
    { siteProvidesNoThreshold: true as const },
    { spellcaster: true as const },
  ]) {
    assert.throws(() => createGameManifest({
      ...input,
      cards: {
        ...cards,
        [giantCardId]: {
          ...cards[giantCardId]!,
          ...incompatibleFact,
        },
      },
    }), /occupiesSquareArea has an unsupported ability combination/);
  }
  const gameManifest = createGameManifest(input);
  assert.equal(gameManifest.cards[giantCardId]?.cardType === 'minion'
    && gameManifest.cards[giantCardId].occupiesSquareArea, 2);

  await withSetup(gameManifest, async (ctx) => {
  await ctx.keep();
  await ctx.keep();
  const northSites = [
    ...ctx.state.players.north.hand.atlas,
    ctx.state.players.north.atlas[0]!,
  ];
  const southSites = [
    ...ctx.state.players.south.hand.atlas,
    ctx.state.players.south.atlas[0]!,
  ];
  const play = async (
    seat: 'north' | 'south',
    index: number,
    cell: 'B1' | 'B2' | 'B3' | 'B4' | 'C1' | 'C2' | 'C3' | 'C4',
  ): Promise<void> => {
    const card = (seat === 'north' ? northSites : southSites)[index];
    assert.ok(card);
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === card.instanceId && descriptor.cell === cell);
  };
  const endAndDraw = async (zone: 'atlas' | 'spellbook' = 'spellbook'): Promise<void> => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === zone);
  };

  await play('north', 0, 'C4');
  assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === giantCardId), false);
  await endAndDraw();
  await play('south', 0, 'C1');
  await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === enemyCardId && descriptor.cell === 'C1');
  await endAndDraw();
  await play('north', 1, 'B4');
  await endAndDraw('atlas');
  await play('south', 1, 'B1');
  await endAndDraw();
  await play('north', 2, 'C3');
  await endAndDraw('atlas');
  await play('south', 2, 'C2');
  const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === enemyCardId);
  assert.ok(enemy);
  await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === enemy.instanceId
    && descriptor.to.cell === 'C2');
  await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
  await endAndDraw('atlas');

  assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === giantCardId), false);
  await play('north', 3, 'B3');
  const giantSummons = (await ctx.legalActions('north')).filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === giantCardId);
  assert.deepEqual(giantSummons.flatMap(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cells ? [descriptor.cells] : []), [
    ['B3', 'B4', 'C3', 'C4'],
  ]);
  await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === giantCardId
    && descriptor.cells?.every((cell, index) =>
      cell === ['B3', 'B4', 'C3', 'C4'][index]) === true);
  const giant = ctx.state.realm.units.find(({ cardId }) => cardId === giantCardId);
  assert.ok(giant);
  assert.deepEqual(giant.occupiedCells, ['B3', 'B4', 'C3', 'C4']);
  assert.deepEqual(ctx.observe('south').realm.units
    .find(({ instanceId }) => instanceId === giant.instanceId)?.occupiedCells,
  ['B3', 'B4', 'C3', 'C4']);
  await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === artifactCardId
    && descriptor.bearer?.instanceId === giant.instanceId
    && descriptor.bearerCell === 'C4');
  const artifact = ctx.state.realm.artifacts?.find(({ cardId }) =>
    cardId === artifactCardId);
  assert.ok(artifact);
  assert.deepEqual(ctx.observe('north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === artifact.instanceId)?.location, 'C4');

  await ctx.take(({ descriptor }) => descriptor.kind === 'cast-aura'
    && descriptor.cardId === auraCardId
    && descriptor.cells.every((cell, index) =>
      cell === ['A2', 'A3', 'B2', 'B3'][index]));
  assert.equal(ctx.observe('north').realm.units
    .find(({ instanceId }) => instanceId === giant.instanceId)?.immobile, true);
  assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === giant.instanceId
      && descriptor.path.length > 1), false);

  for (let round = 0; round < 3; round += 1) {
    await endAndDraw(round === 2 ? 'atlas' : 'spellbook');
    if (round === 0) {
      await play('south', 3, 'B2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === remoteEnemyCardId && descriptor.cell === 'B2');
    }
    await endAndDraw('spellbook');
  }
  const remoteInterceptor = ctx.state.realm.units.find(({ cardId, location }) =>
    cardId === remoteEnemyCardId && location === 'B2');
  assert.ok(remoteInterceptor);
  const teleportTargetSite = ctx.state.realm.sites.C3;
  assert.ok(teleportTargetSite);
  const destinationAreas = [
    ['B2', 'B3', 'C2', 'C3'],
    ['B3', 'B4', 'C3', 'C4'],
  ];
  await withFork(ctx, async (teleportFork) => {
    const teleportActions = (await teleportFork.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardId === teleportCardId
        && descriptor.ally?.instanceId === giant.instanceId
        && descriptor.targetLocation?.cell === 'C3'
        && descriptor.targetSiteInstanceId === teleportTargetSite.instanceId);
    assert.deepEqual(teleportActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyDestinationCells
        ? [descriptor.allyDestinationCells]
        : []), destinationAreas);
    assert.equal(new Set(teleportActions.map(({ actionId }) => actionId)).size, 2);
    const shiftedTeleport = teleportActions.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.allyDestinationCells?.[0] === 'B2');
    assert.ok(shiftedTeleport);
    const teleported = await teleportFork.step(shiftedTeleport);
    assert.equal(teleported.accepted, true);
    if (!teleported.accepted) return;
    assert.deepEqual(teleported.session.state.realm.units
      .find(({ instanceId }) => instanceId === giant.instanceId)?.occupiedCells,
    destinationAreas[0]);
    const teleportPayload = teleported.receipt.events
      .find(({ type }) => type === 'unit-teleported')?.payload;
    assert.ok(teleportPayload
      && typeof teleportPayload === 'object'
      && !Array.isArray(teleportPayload)
      && 'cells' in teleportPayload);
    if (teleportPayload
      && typeof teleportPayload === 'object'
      && !Array.isArray(teleportPayload)
      && 'cells' in teleportPayload) {
      assert.deepEqual(teleportPayload.cells, destinationAreas[0]);
    }
    assert.equal(await teleportFork.verifyReplay(), true);
    const leapActions = (await teleportFork.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.cardId === leapCardId
        && descriptor.ally?.instanceId === giant.instanceId
        && descriptor.allyDestination?.cell === 'B2');
    assert.deepEqual(leapActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyStrikeLocation
        ? [descriptor.allyStrikeLocation.cell]
        : []), ['B2', 'B3', 'C2', 'C3']);
    assert.equal(new Set(leapActions.map(({ actionId }) => actionId)).size, 4);
    const focusedLeap = leapActions.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyStrikeLocation?.cell === 'B2');
    assert.ok(focusedLeap);
    if (focusedLeap.descriptor.kind !== 'cast-magic') throw new Error('expected Leap Attack cast');
    const leaped = await teleportFork.step(focusedLeap);
    assert.equal(leaped.accepted, true);
    if (!leaped.accepted) return;
    assert.equal(leaped.receipt.events[0]?.type, 'magic-cast');
    assert.deepEqual(leaped.receipt.events[0]?.payload, {
      allyDestination: { cell: 'B2', region: 'surface' },
      allyInstanceId: giant.instanceId,
      allySeat: 'north',
      allyStrikeLocation: { cell: 'B2', region: 'surface' },
      cardId: leapCardId,
      casterInstanceId: focusedLeap.descriptor.casterInstanceId,
      instanceId: focusedLeap.descriptor.cardInstanceId,
      manaPaid: 0,
      seat: 'north',
    });
    assert.deepEqual(leaped.receipt.events
      .filter(({ type }) => type === 'strike-damage-allocated')
      .map(({ payload }) => payload), [{
      amount: 10,
      strikerInstanceId: giant.instanceId,
      targetInstanceId: remoteInterceptor.instanceId,
    }]);
    assert.equal(leaped.session.state.realm.units.some(({ instanceId }) =>
      instanceId === remoteInterceptor.instanceId), false);
    assert.equal(leaped.session.state.realm.units.some(({ instanceId }) =>
      instanceId === enemy.instanceId), true);
    assert.equal(await teleportFork.verifyReplay(), true);
  });

  await withFork(ctx, async (blinkFork) => {
    const blinkActions = (await blinkFork.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardId === blinkCardId
        && descriptor.ally?.instanceId === giant.instanceId
        && descriptor.targetLocation?.cell === 'C3'
        && descriptor.drawZone === 'spellbook');
    assert.deepEqual(blinkActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.allyDestinationCells
        ? [descriptor.allyDestinationCells]
        : []), destinationAreas);
    assert.equal(new Set(blinkActions.map(({ actionId }) => actionId)).size, 2);
    const shiftedBlink = blinkActions.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.allyDestinationCells?.[0] === 'B2');
    assert.ok(shiftedBlink);
    const blinked = await blinkFork.step(shiftedBlink);
    assert.equal(blinked.accepted, true);
    if (!blinked.accepted) return;
    assert.deepEqual(blinked.session.state.realm.units
      .find(({ instanceId }) => instanceId === giant.instanceId)?.occupiedCells,
    destinationAreas[0]);
    assert.equal(await blinkFork.verifyReplay(), true);
  });

  const translated = await ctx.action(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === giant.instanceId
      && descriptor.from.cell === 'B3' && descriptor.to.cell === 'B2');
  assert.deepEqual(translated.descriptor.kind === 'move-and-attack'
    ? translated.descriptor.path.map(({ cell }) => cell)
    : [], ['B3', 'B2']);
  await ctx.take(({ actionId }) => actionId === translated.actionId);
  assert.deepEqual(ctx.state.realm.units
    .find(({ instanceId }) => instanceId === giant.instanceId)?.occupiedCells,
  ['B2', 'B3', 'C2', 'C3']);
  assert.equal(ctx.observe('north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === artifact.instanceId)?.location, 'C3');
  await withFork(ctx, async (declinedFork) => {
    const declined = await declinedFork.step(await declinedFork.action(({ descriptor }) =>
      descriptor.kind === 'decline-attack'));
    assert.equal(declined.accepted, true);
    if (!declined.accepted) return;
    assert.equal(declined.session.state.pendingCombat?.cell, 'B2');
    assert.equal((await declinedFork.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept'
        && descriptor.unitInstanceId === enemy.instanceId), false);
    const intercepted = await declinedFork.step(await declinedFork.action(({ descriptor }) =>
      descriptor.kind === 'intercept'
        && descriptor.unitInstanceId === remoteInterceptor.instanceId));
    assert.equal(intercepted.accepted, true);
    if (!intercepted.accepted) return;
    assert.equal(intercepted.session.state.pendingCombat?.cell, 'B2');
    assert.equal(intercepted.receipt.events[0]?.type, 'interceptor-joined');
    assert.deepEqual(intercepted.receipt.events[0]?.payload, {
      cell: 'B2',
      instanceId: remoteInterceptor.instanceId,
      seat: 'south',
    });
    assert.equal(await declinedFork.verifyReplay(), true);
  });

  await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === enemy.instanceId);
  assert.equal(ctx.state.pendingCombat?.cell, 'C2');
  await ctx.take(({ descriptor }) => descriptor.kind === 'close-defend'
    && descriptor.originalTargetParticipates);
  assert.equal(ctx.state.realm.units.some(({ cardId }) => cardId === enemyCardId), false);
  assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
    instanceId === giant.instanceId)?.damage, 2);

  await endAndDraw();
  const targetSite = ctx.state.realm.sites.C2;
  assert.ok(targetSite);
  const caveIn = await ctx.step(await ctx.action(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardId === caveInCardId
      && descriptor.targetLocation?.cell === 'C2'
      && descriptor.targetSiteInstanceId === targetSite.instanceId));
  assert.equal(caveIn.accepted, true);
  if (!caveIn.accepted) return;
  assert.equal(caveIn.session.state.realm.units.some(({ instanceId }) =>
    instanceId === giant.instanceId), false);
  assert.deepEqual(caveIn.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === artifact.instanceId), {
    cardId: artifactCardId,
    instanceId: artifact.instanceId,
    location: 'C3',
    owner: 'north',
    region: 'underground',
    source: 'spellbook',
  });
  assert.deepEqual(caveIn.receipt.events
    .filter(({ type }) => type === 'minion-burrowed' || type === 'minion-died')
    .map(({ type }) => type), ['minion-burrowed', 'minion-died']);
  assert.deepEqual(caveIn.receipt.randomDraws, []);
  assert.equal(await ctx.verifyReplay(), true);
  });
  });
});

test('RULE-03 Tower Genesis grants mana only for the first controlled copy', async () => {
  const north = deck('tower-north');
  await withSetup(manifest(56, {
    north: { ...north, atlas: ['dark-tower', 'gothic-tower', 'dark-tower'] },
    site: { genesisGainManaIfOnlyControlledCopy: 1 },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await ctx.accept(await ctx.action(predicate));
    };
    const playNorthSite = async (cardId: string, gainsBonus: boolean): Promise<void> => {
      const instanceId = ctx.state.players.north.hand.atlas
        .find((card) => card.cardId === cardId)?.instanceId;
      assert.ok(instanceId);
      const manaBefore = ctx.state.players.north.mana;
      await take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cardInstanceId === instanceId);
      assert.equal(ctx.state.players.north.mana - manaBefore, gainsBonus ? 2 : 1);
      assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'site-played',
        ...(gainsBonus ? ['mana-gained'] : []),
      ]);
    };
    await playNorthSite('dark-tower', true);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await playNorthSite('gothic-tower', true);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await playNorthSite('dark-tower', false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Humble Village Genesis may spend its mana to summon one Foot Soldier', async () => {
  const north: GameDeckSpec = {
    atlas: Array(4).fill('humble-village'),
    avatar: 'avatar',
    spellbook: Array(4).fill('blank-minion'),
  };
  const south: GameDeckSpec = {
    atlas: Array(4).fill('plain-site'),
    avatar: 'avatar',
    spellbook: Array(4).fill('blank-minion'),
  };
  const cards: Record<string, GameCardDefinition> = {
    avatar: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'blank-minion': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'foot-soldier': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      token: true,
    },
    'humble-village': {
      cardType: 'site',
      elements: ['earth'],
      genesisPayOneManaToSummonToken: 'foot-soldier',
    },
    'plain-site': { cardType: 'site', elements: ['earth'] },
  };
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-humble-village-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 103,
  });
  assert.equal(gameManifest.cards['humble-village']?.cardType === 'site'
    && gameManifest.cards['humble-village'].genesisPayOneManaToSummonToken,
  'foot-soldier');
  const missingTokenCards = { ...cards };
  delete missingTokenCards['foot-soldier'];
  assert.throws(() => createGameManifest({
    ...gameManifest,
    cards: missingTokenCards,
  }), /token effect must reference a token minion/);
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const villageInstanceId = ctx.state.players.north.hand.atlas[0]!.instanceId;
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === villageInstanceId
        && descriptor.cell === 'C4');
    const declined = choices.find(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.genesisTokenChoice === 'decline');
    const paid = choices.find(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.genesisTokenChoice === 'pay-one-mana');
    assert.equal(choices.length, 2);
    assert.ok(declined);
    assert.ok(paid);
    assert.notEqual(declined.actionId, paid.actionId);
    const checkpointVersion = ctx.state.stateVersion;

    await withFork(ctx, async (declinedFork) => {
      const declinedResult = await declinedFork.step(declined);
      assert.equal(declinedResult.accepted, true);
      if (!declinedResult.accepted) return;
      assert.equal(declinedResult.session.state.stateVersion, checkpointVersion + 1);
      assert.equal(declinedResult.session.state.players.north.mana, 1);
      assert.equal(declinedResult.session.state.realm.units.length, 0);
      assert.deepEqual(declinedResult.receipt.events.map(({ type }) => type), ['site-played']);
      assert.equal(declinedResult.receipt.randomDraws.length, 0);
      assert.equal(await declinedFork.verifyReplay(), true);
    });

    await withFork(ctx, async (paidFork) => {
      const paidResult = await paidFork.step(paid);
      assert.equal(paidResult.accepted, true);
      if (!paidResult.accepted) return;
      assert.equal(paidResult.session.state.stateVersion, checkpointVersion + 1);
      const token = paidResult.session.state.realm.units[0];
      assert.ok(token);
      assert.equal(paidResult.session.state.players.north.mana, 0);
      assert.deepEqual(paidResult.receipt.events.map(({ type }) => type), [
        'site-played',
        'minion-summoned',
      ]);
      assert.deepEqual(paidResult.receipt.events[1]?.payload, {
        cardId: 'foot-soldier',
        cell: 'C4',
        instanceId: token.instanceId,
        manaPaid: 1,
        owner: 'north',
        seat: 'north',
        sourceInstanceId: paidResult.session.state.realm.sites.C4?.instanceId,
        token: true,
      });
      assert.deepEqual({
        controller: token.controller,
        damage: token.damage,
        location: token.location,
        owner: token.owner,
        region: token.region,
        source: token.source,
        summoningSickness: token.summoningSickness,
        tapped: token.tapped,
      }, {
        controller: 'north',
        damage: 0,
        location: 'C4',
        owner: 'north',
        region: 'surface',
        source: 'token',
        summoningSickness: true,
        tapped: false,
      });
      assert.equal(paidResult.receipt.randomDraws.length, 0);
      assert.equal(await paidFork.verifyReplay(), true);
    });
  });
});

test('RULE-02/03 Geomancer creates Rubble and privately replaces it with the top Atlas site', async () => {
  const northAtlas = Array.from({ length: 4 }, (_, index) => `rustic-village-${index + 1}`);
  const southAtlas = Array.from({ length: 4 }, (_, index) => `south-site-${index + 1}`);
  const north: GameDeckSpec = {
    atlas: northAtlas,
    avatar: 'geomancer',
    spellbook: Array(4).fill('north-minion'),
  };
  const south: GameDeckSpec = {
    atlas: southAtlas,
    avatar: 'south-avatar',
    spellbook: Array(4).fill('south-minion'),
  };
  const cards: Record<string, GameCardDefinition> = {
    geomancer: {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      earthSitePlayCreatesAdjacentRubble: true,
      life: 20,
      replaceAdjacentRubbleWithTopAtlasSite: true,
    },
    'foot-soldier': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      token: true,
    },
    'north-minion': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'south-minion': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  };
  northAtlas.forEach((cardId) => {
    cards[cardId] = {
      cardType: 'site',
      elements: ['earth'],
      genesisPayOneManaToSummonToken: 'foot-soldier',
    };
  });
  southAtlas.forEach((cardId) => {
    cards[cardId] = { cardType: 'site', elements: [] };
  });
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-geomancer-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 104,
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const firstPlay = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cell === 'C4'
        && descriptor.createRubbleAt === 'C3'
        && descriptor.genesisTokenChoice === 'decline');
    const firstResult = await ctx.step(firstPlay);
    assert.equal(firstResult.accepted, true);
    if (!firstResult.accepted) return;
    assert.deepEqual(firstResult.receipt.events.map(({ type }) => type), [
      'site-played',
      'rubble-created',
    ]);
    assert.equal('rubble' in ctx.state.realm.sites.C3!, true);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const top = ctx.state.players.north.atlas[0];
    assert.ok(top);
    const before = canonicalJson(ctx.observe('north') as unknown as JsonValue);
    assert.equal(before.includes(top.cardId), false);
    assert.equal(before.includes(top.instanceId), false);
    const replacement = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'replace-rubble-with-top-atlas-site'
        && descriptor.targetCell === 'C3');
    assert.equal(replacement.label, 'Replace Rubble at C3 with the top site of your Atlas');
    assert.equal(canonicalJson(replacement as unknown as JsonValue).includes(top.cardId), false);
    assert.equal(canonicalJson(replacement as unknown as JsonValue).includes(top.instanceId), false);

    const replaced = await ctx.step(replacement);
    assert.equal(replaced.accepted, true);
    if (!replaced.accepted) return;
    assert.deepEqual(replaced.receipt.events.map(({ type }) => type), [
      'rubble-replaced',
      'site-played',
    ]);
    assert.equal(ctx.state.phase, 'genesis');
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.players.north.atlas.length, 0);
    assert.equal(ctx.state.players.north.hand.atlas.length, 2);
    assert.equal(ctx.state.realm.sites.C3?.instanceId, top.instanceId);
    assert.equal(Object.values(ctx.state.realm.sites)
      .filter((site) => 'rubble' in site).length, 0);
    const revealed = canonicalJson(ctx.observe('south') as unknown as JsonValue);
    assert.equal(revealed.includes(top.cardId), true);
    assert.equal(revealed.includes(top.instanceId), true);

    const choices = await ctx.legalActions('north');
    assert.equal(choices.length, 2);
    assert.equal(choices.every(({ descriptor }) => descriptor.kind === 'resolve-genesis-token'), true);
    const paid = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'resolve-genesis-token' && descriptor.choice === 'pay-one-mana');
    const paidResult = await ctx.step(paid);
    assert.equal(paidResult.accepted, true);
    if (!paidResult.accepted) return;
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.pendingGenesisToken, null);
    assert.equal(ctx.state.players.north.mana, 1);
    assert.equal(ctx.state.realm.units.at(-1)?.cardId, 'foot-soldier');
    assert.deepEqual(paidResult.receipt.events.map(({ type }) => type), ['minion-summoned']);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02/03 site Genesis resumes after ordered terrain-replacement Deathrites', async () => {
  const base = manifest(245, {
    avatar: {
      attack: 1,
      defense: 1,
      drawSpell: false,
      life: 20,
      replaceAdjacentRubbleWithTopAtlasSite: true,
    },
  });
  await withPreview(base, async (preview) => {
    const sourceCardId = preview.state.players.north.hand.atlas[0]?.cardId;
    const targetCardId = preview.state.players.north.hand.atlas[1]?.cardId;
    const waterCardId = preview.state.players.north.atlas[0]?.cardId;
    const deathriteCardIds = preview.state.players.north.hand.spellbook.slice(0, 2).map(({ cardId }) => cardId);
    assert.ok(sourceCardId && targetCardId && waterCardId);
    assert.equal(deathriteCardIds.length, 2);
    const cards: Record<string, GameCardDefinition> = { ...base.cards };
    cards[sourceCardId] = {
      cardType: 'site',
      elements: ['earth'],
      sacrificeToDestroyNearbySite: true,
    };
    cards[targetCardId] = { cardType: 'site', elements: ['earth'] };
    cards[waterCardId] = { cardType: 'site', elements: ['water'], genesisGainMana: 1 };
    for (const cardId of deathriteCardIds) {
      cards[cardId] = {
        attack: 1,
        burrowing: true,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      };
    }
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });

    const playToReplacement = async (ctx: SetupCtx, deathriteCount: 1 | 2) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === sourceCardId
        && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
        && descriptor.cardId === targetCardId
        && descriptor.cell === 'C3');
      for (const cardId of deathriteCardIds.slice(0, deathriteCount)) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === cardId
          && descriptor.cell === 'C3'
          && descriptor.region === 'underground');
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
        && descriptor.targetCell === 'C3');
      const deathrites = ctx.state.realm.units.filter(({ cardId }) =>
        deathriteCardIds.includes(cardId));
      assert.equal(deathrites.length, deathriteCount);
      assert.equal(deathrites.every(({ region }) => region === 'underground'), true);
      const top = ctx.state.players.north.atlas[0];
      assert.ok(top);
      assert.equal(top.cardId, waterCardId);
      return { deathrites, top };
    };

    await withSetup(gameManifest, async (ctx) => {
      const { deathrites, top } = await playToReplacement(ctx, 2);
      const manaBefore = ctx.state.players.north.mana;
      const atlasBefore = ctx.state.players.north.atlas.length;
      const atlasHandBefore = ctx.state.players.north.hand.atlas.length;
      const replacement = await ctx.action(({ descriptor }) =>
        descriptor.kind === 'replace-rubble-with-top-atlas-site'
          && descriptor.targetCell === 'C3');
      const interrupted = await ctx.step(replacement);
      assert.equal(interrupted.accepted, true);
      if (!interrupted.accepted) throw new Error('expected terrain replacement to reach Deathrites');
      assert.deepEqual(interrupted.receipt.events.map(({ type }) => type), [
        'rubble-replaced',
        'site-played',
      ]);
      assert.deepEqual(interrupted.receipt.randomDraws, []);
      assert.equal(ctx.state.phase, 'deathrite-order');
      assert.equal(ctx.state.decisionSeat, 'north');
      assert.equal(ctx.state.realm.sites.C3?.instanceId, top.instanceId);
      assert.equal(ctx.state.players.north.avatar.tapped, true);
      assert.equal(ctx.state.players.north.mana, manaBefore + 1);
      assert.equal(ctx.state.players.north.atlas.length, atlasBefore - 1);
      assert.equal(deathrites.every(({ instanceId }) => !ctx.state.realm.units
        .some((unit) => unit.instanceId === instanceId)), true);
      assert.equal(deathrites.every(({ instanceId }) => !ctx.state.players.north.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
        ctx.checkpoint(),
      )));
      assert.equal(
        canonicalJson(restored as unknown as JsonValue),
        canonicalJson(ctx.session as unknown as JsonValue),
      );
      const orders = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.equal(orders.length, 2);
      assert.deepEqual(await ctx.legalActions('south'), []);
      assert.deepEqual(
        (await ctx.legalActions('north')).map(({ actionId }) => actionId),
        orders.map(({ actionId }) => actionId),
      );
      const branchHashes: string[] = [];
      for (const order of orders) {
        assert.equal(order.descriptor.kind, 'order-deathrites');
        if (order.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
        const chosenInstanceId = order.descriptor.sourceInstanceId;
        const otherInstanceId = deathrites.find(({ instanceId }) =>
          instanceId !== chosenInstanceId)?.instanceId;
        assert.ok(otherInstanceId);
        await withFork(ctx, async (branch) => {
          const resolved = await branch.step(order);
          assert.equal(resolved.accepted, true);
          if (!resolved.accepted) throw new Error('expected site Genesis to resume');
          assert.deepEqual(resolved.receipt.events.map(({ type }) => type), [
            'deathrite-order-committed',
            'site-drawn',
            'site-drawn',
            'minion-died',
            'minion-died',
            'mana-gained',
          ]);
          assert.deepEqual(resolved.receipt.events.filter(({ type }) => type === 'site-drawn')
            .map(({ payload }) => payload !== null && typeof payload === 'object'
              && 'sourceInstanceId' in payload ? payload.sourceInstanceId : undefined), [
            chosenInstanceId,
            otherInstanceId,
          ]);
          assert.deepEqual(resolved.receipt.events.at(-1)?.payload, {
            amount: 1,
            seat: 'north',
            sourceInstanceId: top.instanceId,
          });
          assert.equal(branch.state.phase, 'main');
          assert.equal(branch.state.pendingDeathrites, undefined);
          assert.equal(branch.state.realm.sites.C3?.instanceId, top.instanceId);
          assert.equal(deathrites.every(({ instanceId }) => branch.state.players.north.cemetery
            .some((card) => card.instanceId === instanceId)), true);
          assert.equal(branch.state.players.north.atlas.length, atlasBefore - 3);
          assert.equal(branch.state.players.north.hand.atlas.length, atlasHandBefore + 2);
          assert.equal(branch.state.players.north.mana, manaBefore + 2);
          assert.deepEqual(resolved.receipt.randomDraws, []);
          assert.equal(await branch.verifyReplay(), true);
          branchHashes.push(branch.stateHash());
        });
      }
      assert.equal(new Set(branchHashes).size, 1);
    });

    await withSetup(gameManifest, async (ctx) => {
      const { deathrites } = await playToReplacement(ctx, 1);
      const immediateResult = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'replace-rubble-with-top-atlas-site'
          && descriptor.targetCell === 'C3'));
      assert.equal(immediateResult.accepted, true);
      if (!immediateResult.accepted) throw new Error('expected synchronous site Genesis');
      assert.deepEqual(immediateResult.receipt.events.map(({ type }) => type), [
        'rubble-replaced',
        'site-played',
        'site-drawn',
        'minion-died',
        'mana-gained',
      ]);
      assert.equal(ctx.state.phase, 'main');
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === deathrites[0]!.instanceId), true);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03 Hunter\'s Lodge Genesis removes only enemy Stealth', async () => {
  const north: GameDeckSpec = {
    atlas: Array(4).fill('hunters-lodge'),
    avatar: 'lodge-north-avatar',
    spellbook: Array(4).fill('lodge-ally'),
  };
  const south: GameDeckSpec = {
    atlas: Array(5).fill('lodge-south-site'),
    avatar: 'lodge-south-avatar',
    spellbook: Array(4).fill('lodge-enemy'),
  };
  const cards: Record<string, GameCardDefinition> = {
    'hunters-lodge': {
      cardType: 'site',
      elements: ['earth'],
      genesisEnemiesLoseStealth: true,
    },
    'lodge-ally': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 1,
      stealth: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'lodge-enemy': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 1,
      stealth: true,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'lodge-north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'lodge-south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'lodge-south-site': { cardType: 'site', elements: ['earth'] },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-hunters-lodge-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
    seed: 97,
  };
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards['hunters-lodge'], cards['hunters-lodge']);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'hunters-lodge': {
        cardType: 'site',
        elements: ['earth'],
        genesisEnemiesLoseStealth: false,
      } as unknown as GameCardDefinition,
    },
  }), /genesisEnemiesLoseStealth must be true/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), ['site-played']);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'lodge-ally' && descriptor.cell === 'C4');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === 'lodge-ally')!;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'lodge-enemy' && descriptor.cell === 'C4');
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === 'lodge-enemy')!;
    assert.equal(ally.stealthed, true);
    assert.equal(enemy.stealthed, true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const play = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cardId === 'hunters-lodge' && descriptor.cell === 'C3');
    if (play.descriptor.kind !== 'play-site') throw new Error('expected Hunter\'s Lodge play');
    const result = await ctx.step(play);
    assert.equal(result.accepted, true);
    assert.deepEqual(result.receipt.events.map(({ payload, type }) => ({ payload, type })), [
      {
        payload: {
          cardId: 'hunters-lodge',
          cell: 'C3',
          instanceId: play.descriptor.cardInstanceId,
          seat: 'north',
        },
        type: 'site-played',
      },
      {
        payload: {
          instanceId: enemy.instanceId,
          seat: 'south',
          sourceInstanceId: play.descriptor.cardInstanceId,
        },
        type: 'stealth-lost',
      },
    ]);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === ally.instanceId)?.stealthed, true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === enemy.instanceId)?.stealthed, false);
    assert.deepEqual(result.receipt.randomDraws, []);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 seasonal River Genesis privately keeps or bottoms the next spell', async () => {
  const base = deck('river-north');
  const north = { ...base, atlas: base.atlas.map(() => 'river-site') };
  const gameManifest = manifest(160, {
    north,
    site: { genesisMayBottomNextSpell: true },
  });
  assert.equal(gameManifest.cards['river-site']?.cardType === 'site'
    && gameManifest.cards['river-site'].genesisMayBottomNextSpell, true);
  assert.throws(() => createGameManifest({
    ...gameManifest,
    cards: {
      ...gameManifest.cards,
      'river-site': {
        ...gameManifest.cards['river-site']!,
        genesisMayBottomNextSpell: false,
      } as unknown as GameCardDefinition,
    },
  }), /genesisMayBottomNextSpell must be true/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const before = ctx.state.players.north.spellbook;
    const [top, next] = before;
    const riverInstanceId = ctx.state.players.north.hand.atlas[0]?.instanceId;
    assert.ok(top);
    assert.ok(next);
    assert.ok(riverInstanceId);
    const southBefore = canonicalJson(ctx.observe('south') as unknown as JsonValue);
    assert.equal(southBefore.includes(top.cardId), false);
    assert.equal(southBefore.includes(top.instanceId), false);

    const plays = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === riverInstanceId
        && descriptor.cell === 'C4');
    const play = plays[0];
    assert.equal(plays.length, 1);
    assert.ok(play);
    assert.equal(play.label.includes(top.cardId), false);
    const played = await ctx.step(play);
    assert.equal(played.accepted, true);
    if (!played.accepted) return;
    assert.equal(played.session.state.phase, 'genesis');
    assert.deepEqual(played.session.state.players.north.spellbook, before);
    assert.deepEqual(played.receipt.events.map(({ type }) => type), ['site-played']);
    const pendingSouth = canonicalJson(ctx.observe('south') as unknown as JsonValue);
    assert.equal(pendingSouth.includes(top.cardId), false);
    assert.equal(pendingSouth.includes(top.instanceId), false);

    const choices = await ctx.legalActions('north');
    const keepNext = choices.find(({ descriptor }) =>
      descriptor.kind === 'resolve-genesis-spell' && descriptor.choice === 'keep-next');
    const bottomNext = choices.find(({ descriptor }) =>
      descriptor.kind === 'resolve-genesis-spell' && descriptor.choice === 'bottom-next');
    assert.equal(choices.length, 2);
    assert.ok(keepNext);
    assert.ok(bottomNext);
    assert.notEqual(keepNext.actionId, bottomNext.actionId);
    assert.equal(choices.every(({ descriptor, label }) =>
      label.includes(top.cardId)
        && !canonicalJson(descriptor as unknown as JsonValue).includes(top.cardId)
        && !canonicalJson(descriptor as unknown as JsonValue).includes(top.instanceId)), true);
    const checkpointVersion = ctx.state.stateVersion;

    await withFork(ctx, async (keptFork) => {
      const kept = await keptFork.step(keepNext);
      assert.equal(kept.accepted, true);
      if (!kept.accepted) return;
      assert.equal(kept.session.state.stateVersion, checkpointVersion + 1);
      assert.equal(kept.session.state.phase, 'main');
      assert.equal(kept.session.state.pendingGenesisSpell, null);
      assert.deepEqual(kept.session.state.players.north.spellbook, before);
      assert.deepEqual(kept.receipt.events.map(({ type }) => type), ['spell-kept']);
      assert.equal(await keptFork.verifyReplay(), true);
      await withFork(ctx, async (bottomedFork) => {
        const bottomed = await bottomedFork.step(bottomNext);
        assert.equal(bottomed.accepted, true);
        if (!bottomed.accepted) return;
        assert.equal(bottomed.session.state.stateVersion, checkpointVersion + 1);
        assert.equal(bottomed.session.state.phase, 'main');
        assert.equal(bottomed.session.state.pendingGenesisSpell, null);
        assert.deepEqual(bottomed.session.state.players.north.spellbook, [...before.slice(1), top]);
        assert.equal(bottomed.session.state.players.north.spellbook[0]?.instanceId, next.instanceId);
        assert.deepEqual(bottomed.receipt.events.map(({ payload, type }) => ({ payload, type })), [
          {
            payload: { seat: 'north', sourceInstanceId: riverInstanceId },
            type: 'spell-bottomed',
          },
        ]);
        assert.equal(canonicalJson(bottomed.receipt.events as unknown as JsonValue).includes(top.cardId), false);
        assert.equal(canonicalJson(bottomed.receipt.events as unknown as JsonValue).includes(top.instanceId), false);
        assert.deepEqual(bottomed.receipt.randomDraws, []);
        assert.equal(
          canonicalJson(keptFork.observe('south') as unknown as JsonValue),
          canonicalJson(bottomedFork.observe('south') as unknown as JsonValue),
        );
        assert.equal(await bottomedFork.verifyReplay(), true);
      });
    });
  });
});

test('RULE-03 Observatory privately reorders the next three spells without drawing', async () => {
  const base = deck('observatory-north');
  const north = { ...base, atlas: base.atlas.map(() => 'observatory-site') };
  const gameManifest = manifest(161, {
    north,
    site: { genesisReorderNextSpells: 3 },
  });
  assert.throws(() => createGameManifest({
    ...gameManifest,
    cards: {
      ...gameManifest.cards,
      'observatory-site': {
        ...gameManifest.cards['observatory-site']!,
        genesisReorderNextSpells: 2,
      } as unknown as GameCardDefinition,
    },
  }), /genesisReorderNextSpells must be 3/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const before = ctx.state.players.north.spellbook;
    const play = await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === 'observatory-site' && descriptor.cell === 'C4');
    if (play.descriptor.kind !== 'play-site') throw new Error('expected Observatory play');
    const sourceInstanceId = play.descriptor.cardInstanceId;
    const played = await ctx.step(play);
    assert.equal(played.accepted, true);
    if (!played.accepted) return;
    assert.equal(played.session.state.phase, 'genesis');
    assert.deepEqual(played.session.state.players.north.spellbook, before);

    const choices = await ctx.legalActions('north');
    assert.equal(choices.length, 6);
    assert.equal(choices.every(({ descriptor }) =>
      descriptor.kind === 'resolve-genesis-spell-order'
        && descriptor.order.length === 3
        && !canonicalJson(descriptor as unknown as JsonValue).includes(before[0]!.instanceId)), true);
    const identity = choices.find(({ descriptor }) =>
      descriptor.kind === 'resolve-genesis-spell-order'
        && descriptor.order.join(',') === '0,1,2');
    const reverse = choices.find(({ descriptor }) =>
      descriptor.kind === 'resolve-genesis-spell-order'
        && descriptor.order.join(',') === '2,1,0');
    assert.ok(identity);
    assert.ok(reverse);

    await withFork(ctx, async (keptFork) => {
      const kept = await keptFork.step(identity);
      assert.equal(kept.accepted, true);
      if (!kept.accepted) return;
      assert.deepEqual(kept.session.state.players.north.spellbook, before);
      assert.equal(await keptFork.verifyReplay(), true);
      await withFork(ctx, async (reversedFork) => {
        const reversed = await reversedFork.step(reverse);
        assert.equal(reversed.accepted, true);
        if (!reversed.accepted) return;
        assert.deepEqual(reversed.session.state.players.north.spellbook.slice(0, 3), before.slice(0, 3).reverse());
        assert.deepEqual(reversed.session.state.players.north.spellbook.slice(3), before.slice(3));
        assert.deepEqual(reversed.receipt.events.map(({ payload, type }) => ({ payload, type })), [{
          payload: { count: 3, seat: 'north', sourceInstanceId },
          type: 'spells-reordered',
        }]);
        assert.deepEqual(reversed.receipt.randomDraws, []);
        assert.equal(
          canonicalJson(keptFork.observe('south') as unknown as JsonValue),
          canonicalJson(reversedFork.observe('south') as unknown as JsonValue),
        );
        assert.equal(await reversedFork.verifyReplay(), true);
      });
    });
  });
});

test('RULE-03 adjacent matching sites trigger one spell draw apiece and a short deck loses', async () => {
  const base = deck('leyline-north');
  const north = {
    ...base,
    atlas: base.atlas.map(() => 'leyline-site'),
  };
  await withSetup(manifest(137, {
    north,
    site: { genesisDrawSpellPerAdjacentSameCard: true },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const firstHandSize = ctx.state.players.north.hand.spellbook.length;
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    assert.equal(ctx.state.players.north.hand.spellbook.length, firstHandSize);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), ['site-played']);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const beforeOne = ctx.state.players.north.hand.spellbook.length;
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    assert.equal(ctx.state.players.north.hand.spellbook.length, beforeOne + 1);
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['site-played', 'spell-drawn'],
    );
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    const beforeTwo = ctx.state.players.north.hand.spellbook.length;
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    assert.equal(ctx.state.players.north.hand.spellbook.length, beforeTwo + 2);
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['site-played', 'spell-drawn', 'spell-drawn'],
    );
    assert.equal(await ctx.verifyReplay(), true);
  });

  const shortBase = deck('leyline-short', 30, 6);
  const short = {
    ...shortBase,
    atlas: shortBase.atlas.map(() => 'leyline-short-site'),
  };
  await withSetup(manifest(138, {
    north: short,
    site: { genesisDrawSpellPerAdjacentSameCard: true },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    const beforePartial = ctx.state.players.north;
    assert.equal(beforePartial.spellbook.length, 1);
    const fourth = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    const partial = await ctx.step(fourth);
    assert.equal(partial.accepted, true);
    const fourthInstanceId = fourth.descriptor.kind === 'play-site'
      ? fourth.descriptor.cardInstanceId
      : '';
    assert.equal(ctx.state.players.north.spellbook.length, 0);
    assert.equal(
      ctx.state.players.north.hand.spellbook.length,
      beforePartial.hand.spellbook.length + 1,
    );
    assert.equal(ctx.state.realm.sites.B3?.instanceId, fourthInstanceId);
    assert.deepEqual(ctx.state.terminal, {
      loser: 'north',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'south',
    });
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['site-played', 'spell-drawn', 'game-ended'],
    );
    assert.equal(
      canonicalJson(ctx.session.transcript.at(-1)?.events[1]?.payload ?? null)
        .includes(fourthInstanceId),
      true,
    );
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 site Genesis discards up to two top spells publicly without deck-out', async () => {
  await withSetup(manifest(139, {
    site: { genesisDiscardTopSpells: 2 },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const beforeVersion = ctx.state.stateVersion;
    const [first, second, next] = ctx.state.players.north.spellbook;
    assert.ok(first);
    assert.ok(second);
    assert.ok(next);
    const southBefore = canonicalJson(ctx.observe('south') as unknown as JsonValue);
    for (const hidden of [first, second, next]) {
      assert.equal(southBefore.includes(hidden.cardId), false);
      assert.equal(southBefore.includes(hidden.instanceId), false);
    }
    const play = await ctx.action(({ descriptor }) => descriptor.kind === 'play-site');
    if (play.descriptor.kind !== 'play-site') throw new Error('expected site play');
    const sourceInstanceId = play.descriptor.cardInstanceId;
    const result = await ctx.step(play);
    assert.equal(result.accepted, true);

    assert.equal(ctx.state.stateVersion, beforeVersion + 1);
    assert.deepEqual(ctx.state.players.north.cemetery, [first, second]);
    assert.equal(ctx.state.players.north.spellbook[0]?.instanceId, next.instanceId);
    assert.deepEqual(result.receipt.events.map(({ payload, type }) => ({ payload, type })), [
      {
        payload: {
          cardId: play.descriptor.cardId,
          cell: play.descriptor.cell,
          instanceId: sourceInstanceId,
          seat: 'north',
        },
        type: 'site-played',
      },
      {
        payload: {
          cardId: first.cardId,
          instanceId: first.instanceId,
          owner: 'north',
          seat: 'north',
          sourceInstanceId,
        },
        type: 'spell-discarded',
      },
      {
        payload: {
          cardId: second.cardId,
          instanceId: second.instanceId,
          owner: 'north',
          seat: 'north',
          sourceInstanceId,
        },
        type: 'spell-discarded',
      },
    ]);
    const southAfter = canonicalJson(ctx.observe('south') as unknown as JsonValue);
    assert.equal(southAfter.includes(first.instanceId), true);
    assert.equal(southAfter.includes(second.instanceId), true);
    assert.equal(southAfter.includes(next.instanceId), false);
    assert.deepEqual(ctx.state.terminal, { status: 'active' });
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(140, {
    north: deck('shallow-short', 30, 4),
    site: { genesisDiscardTopSpells: 2 },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const only = ctx.state.players.north.spellbook[0];
    assert.ok(only);
    const partial = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    assert.equal(partial.accepted, true);
    assert.deepEqual(ctx.state.players.north.cemetery, [only]);
    assert.equal(ctx.state.players.north.spellbook.length, 0);
    assert.deepEqual(partial.receipt.events.map(({ type }) => type), ['site-played', 'spell-discarded']);
    assert.deepEqual(ctx.state.terminal, { status: 'active' });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 Vikings area damage uses bearer Lethal without becoming a strike', async () => {
  const north: GameDeckSpec = {
    atlas: Array(6).fill('vikings-north-site'),
    avatar: 'vikings-north-avatar',
    spellbook: ['vikings', 'disabled-vikings', 'vikings-ally', 'poisonous-dagger'],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('vikings-south-site'),
    avatar: 'vikings-south-avatar',
    spellbook: ['vikings-warded-enemy', 'vikings-stealthed-enemy', 'vikings-submerged-enemy'],
  };
  const cards: Record<string, GameCardDefinition> = {
    'disabled-vikings': {
      attack: 4,
      cardType: 'minion',
      defense: 4,
      manaCost: 1,
      tapToDamageEachUnitAtAdjacentLocation: 2,
      thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      waterbound: true,
    },
    'poisonous-dagger': {
      cardType: 'artifact',
      grantsBearerLethal: true,
      manaCost: 2,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    'vikings': {
      attack: 4,
      cardType: 'minion',
      charge: true,
      defense: 4,
      manaCost: 5,
      stealth: true,
      tapToDamageEachUnitAtAdjacentLocation: 2,
      thresholds: { air: 0, earth: 0, fire: 2, water: 0 },
    },
    'vikings-ally': {
      attack: 1,
      cardType: 'minion',
      defense: 2,
      manaCost: 1,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
    },
    'vikings-north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'vikings-north-site': { cardType: 'site', elements: ['fire'], genesisGainMana: 10 },
    'vikings-south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'vikings-south-site': { cardType: 'site', elements: ['fire', 'water'], genesisGainMana: 5 },
    'vikings-stealthed-enemy': {
      attack: 1,
      cardType: 'minion',
      defense: 3,
      manaCost: 1,
      stealth: true,
      thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
    },
    'vikings-submerged-enemy': {
      attack: 1,
      cardType: 'minion',
      defense: 2,
      manaCost: 1,
      submerge: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
    },
    'vikings-warded-enemy': {
      attack: 1,
      cardType: 'minion',
      defense: 2,
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
      ward: true,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-vikings-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
    seed: 101,
  };
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards.vikings, cards.vikings);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      vikings: {
        ...cards.vikings,
        tapToDamageEachUnitAtAdjacentLocation: 1,
      } as unknown as GameCardDefinition,
    },
  }), /tapToDamageEachUnitAtAdjacentLocation must be 2/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'vikings' && descriptor.cell === 'C3');
    const vikings = ctx.state.realm.units.find(({ cardId }) => cardId === 'vikings')!;
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === vikings.instanceId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'poisonous-dagger'
      && descriptor.bearer?.instanceId === vikings.instanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'disabled-vikings' && descriptor.cell === 'C3');
    const disabled = ctx.state.realm.units.find(({ cardId }) => cardId === 'disabled-vikings')!;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'vikings-warded-enemy' && descriptor.cell === 'C2');
    const warded = ctx.state.realm.units.find(({ cardId }) => cardId === 'vikings-warded-enemy')!;
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'vikings-stealthed-enemy' && descriptor.cell === 'C2');
    const stealthed = ctx.state.realm.units.find(({ cardId }) => cardId === 'vikings-stealthed-enemy')!;
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'vikings-submerged-enemy'
      && descriptor.cell === 'C2' && descriptor.region === 'underwater');
    const submerged = ctx.state.realm.units.find(({ cardId }) => cardId === 'vikings-submerged-enemy')!;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'vikings-ally' && descriptor.cell === 'C2');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === 'vikings-ally')!;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.to.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const activations = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === vikings.instanceId);
    assert.deepEqual(activations.flatMap(({ descriptor }) =>
      descriptor.kind === 'activate-area-damage' ? [descriptor.targetLocation.cell] : []), ['C2', 'C4']);
    assert.equal(activations.every(({ descriptor }) =>
      descriptor.kind === 'activate-area-damage' && descriptor.targetLocation.region === 'surface'), true);
    assert.equal(ctx.observe('north').realm.units.find(({ instanceId }) =>
      instanceId === disabled.instanceId)?.disabled, true);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === disabled.instanceId), false);

    const result = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-area-damage'
        && descriptor.sourceInstanceId === vikings.instanceId
        && descriptor.targetLocation.cell === 'C2'));
    assert.equal(result.accepted, true);
    const events = result.receipt.events;
    assert.equal(events.some(({ type }) => type === 'strike-damage-allocated'), false);
    assert.deepEqual(events.slice(0, 2).map(({ type }) => type), ['area-damage-activated', 'stealth-lost']);
    assert.equal(events.slice(2, 6).every(({ type }) => type === 'area-damage-allocated'), true);
    assert.deepEqual(events.filter(({ type }) => type === 'area-damage-allocated')
      .map(({ payload }) => (payload as unknown as Readonly<Record<string, unknown>>).targetInstanceId)
      .sort(), [
      ally.instanceId,
      ctx.state.players.south.avatar.card.instanceId,
      stealthed.instanceId,
      warded.instanceId,
    ].sort());
    assert.equal(events.filter(({ type }) => type === 'minion-died').length, 2);
    assert.equal(events.some(({ payload, type }) => type === 'damage-dealt'
      && (payload as unknown as Readonly<Record<string, unknown>>).instanceId === stealthed.instanceId
      && (payload as unknown as Readonly<Record<string, unknown>>).amount === 2), true);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === ally.instanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === stealthed.instanceId), false);
    assert.equal(ctx.state.realm.artifacts?.some((artifact) =>
      'bearer' in artifact && artifact.bearer.instanceId === vikings.instanceId), true);
    const wardedAfter = ctx.state.realm.units.find(({ instanceId }) => instanceId === warded.instanceId)!;
    assert.deepEqual({ damage: wardedAfter.damage, warded: wardedAfter.warded }, { damage: 0, warded: false });
    const submergedAfter = ctx.state.realm.units.find(({ instanceId }) => instanceId === submerged.instanceId)!;
    assert.deepEqual({ damage: submergedAfter.damage, region: submergedAfter.region }, {
      damage: 0,
      region: 'underwater',
    });
    assert.equal(ctx.state.players.south.avatar.life, 18);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === vikings.instanceId)?.tapped, true);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === vikings.instanceId), false);
    assert.deepEqual(result.receipt.randomDraws, []);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a minion mana ability requires readiness, taps, and expires at End Phase', async () => {
  await withSetup(manifest(39, {
    spell: {
      manaCost: 1,
      stealth: true,
      tapForMana: 2,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion');
    const unit = ctx.state.realm.units[0];
    assert.ok(unit);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === unit.instanceId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const before = ctx.state.players.north.mana;
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === unit.instanceId);
    assert.equal(ctx.state.players.north.mana, before + 2);
    assert.equal(ctx.state.realm.units[0]?.tapped, true);
    assert.equal(ctx.state.realm.units[0]?.stealthed, false);
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === unit.instanceId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    assert.equal(ctx.state.players.north.mana, 1);
    assert.equal(ctx.state.realm.units[0]?.tapped, false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 mana and every elemental threshold gate minion actions without being spent together', async () => {
  await withNorthSecondMain(43, false, {
    manaCost: 2,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  }, async (ctx) => {
    assert.equal(ctx.state.players.north.mana, 1);
    assert.equal(ctx.observe('north').players.north.affinity.earth, 1);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion'), false);

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    assert.equal(ctx.state.players.north.mana, 2);
    assert.equal(ctx.observe('north').players.north.affinity.earth, 2);
    const cardId = ctx.state.players.north.hand.spellbook[0]?.cardId;
    assert.ok(cardId);
    const cells = (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cardId === cardId)
      .map(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell);
    assert.deepEqual(cells, ['C3', 'C4']);

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === cardId && descriptor.cell === 'C4');
    assert.equal(ctx.state.players.north.mana, 0);
    assert.equal(ctx.observe('north').players.north.affinity.earth, 2);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

async function withNorthAttacksAtC2(
  seed: number,
  spell: SpellFacts | undefined,
  avatar: AvatarFacts | undefined,
  emptyAtlasAfterOpening: boolean,
  southSpell: SpellFacts | undefined,
  extraSouthMinionsAtC1: number,
  run: (setup: Readonly<{
    attackerInstanceId: string;
    defenderInstanceId: string;
    targetInstanceId: string;
    ctx: SetupCtx;
  }>) => Promise<void>,
): Promise<void> {
  const shortDecks = emptyAtlasAfterOpening
    ? { north: deck('north', 3), south: deck('south', 3) }
    : {};
  await withSetup(manifest(seed, {
    ...shortDecks,
    ...(spell ? { spell } : {}),
    ...(southSpell ? { southSpell } : {}),
    ...(avatar ? { avatar } : {}),
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const attackerInstanceId = ctx.state.realm.units.find(({ controller }) => controller === 'north')?.instanceId;
    assert.ok(attackerInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const defenderInstanceId = ctx.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(defenderInstanceId);
    for (let index = 0; index < extraSouthMinionsAtC1; index += 1) {
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C2');
    const targetInstanceId = ctx.state.realm.units
      .find(({ controller, location }) => controller === 'south' && location === 'C2')?.instanceId;
    assert.ok(targetInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.from.cell === 'C3'
        && descriptor.to.cell === 'C2');
    await run({ attackerInstanceId, defenderInstanceId, targetInstanceId, ctx });
  });
}

test('RULE-03/04 an undamaged 0/0 Genesis minion survives until it takes positive damage', async () => {
  await withNorthAttacksAtC2(131, {
    attack: 1,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, {
    attack: 0,
    defense: 0,
    genesisDrawSpells: 3,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, 0, async ({ ctx, targetInstanceId }) => {
    const target = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId);
    assert.ok(target);
    assert.deepEqual({
      attack: target.attack,
      damage: target.damage,
      defense: target.defense,
    }, {
      attack: 0,
      damage: 0,
      defense: 0,
    });

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    if (!fought.accepted) return;
    assert.equal(ctx.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === targetInstanceId), true);
    assert.deepEqual(fought.receipt.events.find(({ payload, type }) =>
      type === 'damage-dealt'
        && canonicalJson(payload).includes(targetInstanceId))?.payload, {
      accumulated: 1,
      amount: 1,
      direct: true,
      instanceId: targetInstanceId,
      seat: 'south',
    });
    assert.equal(ctx.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 an active minion prevents damage from a unit at its current-power threshold', async () => {
  for (const [seed, sourcePower, disabled, expectedDamage] of [
    [170, 4, false, 0],
    [172, 3, false, 3],
    [173, 4, true, 4],
  ] as const) {
    await withNorthAttacksAtC2(seed, {
      attack: sourcePower,
      defense: 5,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    }, undefined, false, {
      attack: 1,
      defense: 10,
      ...(disabled ? { genesisDisableSelfUntilDamaged: true as const } : {}),
      manaCost: 1,
      preventsDamageFromUnitsWithPowerAtLeast: 4,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    }, 0, async ({ ctx, targetInstanceId }) => {
      assert.equal(ctx.observe('south').realm.units.find(({ instanceId }) =>
        instanceId === targetInstanceId)?.disabled, disabled);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === targetInstanceId);
      const fought = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
      assert.equal(fought.accepted, true);
      if (!fought.accepted) return;
      const target = ctx.observe('south').realm.units.find(({ instanceId }) =>
        instanceId === targetInstanceId);
      assert.deepEqual({ damage: target?.damage, disabled: target?.disabled }, {
        damage: expectedDamage,
        disabled: false,
      });
      assert.equal(ctx.observe('north').realm.units.find(({ instanceId }) =>
        instanceId === targetInstanceId)?.damage, expectedDamage);
      const targetDamage = fought.receipt.events.find(({ payload, type }) =>
        type === 'damage-dealt' && canonicalJson(payload).includes(targetInstanceId));
      assert.equal((targetDamage?.payload as { amount?: number }).amount, expectedDamage);
      assert.equal((targetDamage?.payload as { prevented?: boolean }).prevented,
        expectedDamage < sourcePower ? true : undefined);
      assert.equal(fought.receipt.randomDraws.length, 0);
      assert.equal(await ctx.verifyReplay(), true);
    });
  }
});

test('RULE-04 Ranged damage retains its attacking unit source classification', async () => {
  const gameManifest = manifest(174, {
    northSpell: {
      attack: 4,
      defense: 4,
      manaCost: 1,
      ranged: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    southSpell: {
      attack: 1,
      defense: 5,
      manaCost: 1,
      preventsDamageFromUnitsWithPowerAtLeast: 4,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    site: { rangedUnitsHereRangeBonus: 1 },
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const shooter = ctx.state.realm.units.find(({ controller }) => controller === 'north');
    assert.ok(shooter);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const target = ctx.state.realm.units.find(({ controller }) => controller === 'south');
    assert.ok(target);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === shooter.instanceId && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const shot = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === shooter.instanceId
        && descriptor.hit?.instanceId === target.instanceId));
    assert.equal(shot.accepted, true);
    if (!shot.accepted) return;
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.damage, 0);
    assert.deepEqual(shot.receipt.events.map(({ type }) => type), [
      'projectile-shot',
      'strike-damage-allocated',
      'damage-dealt',
    ]);
    assert.equal(shot.receipt.randomDraws.length, 0);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a ready minion may tap to shoot a fixed-damage projectile to the first visible unit', async () => {
  const gameManifest = manifest(314, {
    northSpell: {
      airborne: true,
      attack: 4,
      defense: 4,
      manaCost: 1,
      tapToShootProjectileDamage: 4,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      ward: true,
    },
    southSpell: {
      attack: 1,
      defense: 5,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const shooter = ctx.state.realm.units.find(({ controller }) => controller === 'north');
    assert.ok(shooter);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'shoot-damage-projectile'
        && descriptor.shooterInstanceId === shooter.instanceId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const target = ctx.state.realm.units.find(({ controller }) => controller === 'south');
    assert.ok(target);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const shot = (await ctx.legalActions('north')).find(({ descriptor }) =>
      descriptor.kind === 'shoot-damage-projectile'
        && descriptor.shooterInstanceId === shooter.instanceId
        && descriptor.direction === 'south'
        && descriptor.hit?.instanceId === target.instanceId);
    assert.ok(shot);
    assert.deepEqual(
      shot.descriptor.kind === 'shoot-damage-projectile'
        ? shot.descriptor.path.map(({ cell }) => cell)
        : [],
      ['C4', 'C3', 'C2', 'C1'],
    );
    const fired = await ctx.step(shot);
    assert.equal(fired.accepted, true);
    if (!fired.accepted) return;
    assert.deepEqual(fired.receipt.events.map(({ type }) => type), [
      'projectile-shot',
      'projectile-damage-allocated',
      'damage-dealt',
    ]);
    assert.equal(canonicalJson(fired.receipt.events[1]!.payload), canonicalJson({
      amount: 4,
      sourceInstanceId: shooter.instanceId,
      targetInstanceId: target.instanceId,
    }));
    const firedShooter = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === shooter.instanceId);
    assert.deepEqual(firedShooter && {
      damage: firedShooter.damage,
      location: firedShooter.location,
      summoningSickness: firedShooter.summoningSickness,
      tapped: firedShooter.tapped,
      warded: firedShooter.warded,
    }, {
      damage: 0,
      location: 'C4',
      summoningSickness: false,
      tapped: true,
      warded: true,
    });
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.damage, 4);
    assert.equal(fired.receipt.events.some(({ type }) => type === 'strike-damage-allocated'), false);
    assert.equal(fired.receipt.randomDraws.length, 0);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a restricted attacker can target units but not sites', async () => {
  await withNorthAttacksAtC2(114, {
    attack: 4,
    cannotAttackSites: true,
    charge: true,
    defense: 4,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, undefined, 0, async ({ ctx, targetInstanceId }) => {
    const actions = await ctx.legalActions('north');
    const targets = actions.flatMap(({ descriptor }) =>
      descriptor.kind === 'declare-attack' ? [descriptor.target] : []);
    assert.equal(targets.some(({ instanceId, kind }) =>
      kind === 'minion' && instanceId === targetInstanceId), true);
    assert.equal(targets.some(({ kind }) => kind === 'site'), false);
    assert.equal(actions.some(({ descriptor }) => descriptor.kind === 'decline-attack'), true);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 attacking-only first strike resolves deaths before normal strikes and is inactive while defending', async () => {
  const vanilla = {
    attack: 3,
    defense: 3,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const firstStrike = { ...vanilla, strikesFirstWhileAttacking: true };
  await withNorthAttacksAtC2(117, firstStrike, undefined, false, vanilla, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.damage, 0);
    assert.equal(ctx.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  const deathrite = {
    attack: 2,
    deathriteDamageEachUnitHere: 1,
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  await withNorthAttacksAtC2(
    280,
    { ...firstStrike, attack: 6, defense: 10 },
    { attack: 2, defense: 1, drawSpell: false, life: 20 },
    false,
    deathrite,
    1,
    async ({ attackerInstanceId, ctx, targetInstanceId }) => {
      const defenderIds = ctx.state.realm.units
        .filter(({ controller, location }) => controller === 'south' && location === 'C1')
        .map(({ instanceId }) => instanceId)
        .sort();
      assert.equal(defenderIds.length, 2);
      const deathriteIds = [targetInstanceId, defenderIds[0]!].sort();
      const survivingDefenderId = defenderIds[1]!;
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === targetInstanceId);
      for (const unitInstanceId of defenderIds) {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'defend'
            && descriptor.unitInstanceId === unitInstanceId);
      }
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      assert.equal(ctx.state.phase, 'allocate');
      while (ctx.state.phase === 'allocate') {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'allocate-strike'
            && descriptor.amount === (deathriteIds.includes(descriptor.targetInstanceId) ? 3 : 0));
      }

      assert.equal(ctx.state.phase, 'deathrite-order');
      assert.equal(ctx.state.decisionSeat, 'south');
      assert.equal(deathriteIds.every((instanceId) => !ctx.state.players.south.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'minion-died'), false);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === attackerInstanceId)?.damage, 0);
      assert.equal(ctx.session.transcript.at(-1)?.events.some(({ payload, type }) =>
        type === 'damage-dealt'
          && payload !== null
          && typeof payload === 'object'
          && 'instanceId' in payload
          && payload.instanceId === attackerInstanceId), false);
      const orderActions = (await ctx.legalActions('south')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
        descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), deathriteIds);
      const ordered = await ctx.step(orderActions[0]!);
      assert.equal(ordered.accepted, true);
      if (!ordered.accepted) return;

      const orderedAllocations = ordered.receipt.events.filter(({ type }) =>
        type === 'deathrite-damage-allocated');
      const orderedSources = orderedAllocations
        .map(({ payload }) => payload !== null && typeof payload === 'object'
          && 'sourceInstanceId' in payload ? payload.sourceInstanceId : undefined)
        .filter((source, index, sources) => source !== undefined && sources.indexOf(source) === index);
      assert.deepEqual(orderedSources, [
        orderActions[0]!.descriptor.kind === 'order-deathrites'
          ? orderActions[0]!.descriptor.sourceInstanceId
          : '',
        deathriteIds.find((instanceId) => instanceId !== orderedSources[0])!,
      ]);
      const attackerDamage = ordered.receipt.events.filter(({ payload, type }) =>
        type === 'damage-dealt'
          && payload !== null
          && typeof payload === 'object'
          && 'instanceId' in payload
          && payload.instanceId === attackerInstanceId);
      assert.deepEqual(attackerDamage.at(-1)?.payload, {
        accumulated: 4,
        amount: 2,
        direct: true,
        instanceId: attackerInstanceId,
        seat: 'north',
      });
      const lastDeathriteIndex = ordered.receipt.events.findLastIndex(({ type }) =>
        type === 'deathrite-damage-allocated');
      const firstDeathIndex = ordered.receipt.events.findIndex(({ type }) => type === 'minion-died');
      const returnStrikeIndex = ordered.receipt.events.findLastIndex(({ payload, type }) =>
        type === 'damage-dealt'
          && payload !== null
          && typeof payload === 'object'
          && 'instanceId' in payload
          && payload.instanceId === attackerInstanceId);
      assert.ok(lastDeathriteIndex < firstDeathIndex && firstDeathIndex < returnStrikeIndex);
      assert.equal(ordered.receipt.events.filter(({ type }) => type === 'minion-died').length, 2);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === attackerInstanceId)?.damage, 4);
      assert.equal(ctx.state.players.south.avatar.life, 20);
      assert.equal(deathriteIds.every((instanceId) => ctx.state.players.south.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === survivingDefenderId)?.damage, 2);
      assert.equal(ctx.state.phase, 'main');
      assert.equal(ctx.state.pendingCombat, null);
      assert.equal(ctx.state.pendingDeathrites ?? null, null);
      assert.equal(await ctx.verifyReplay(), true);
    },
  );

  await withNorthAttacksAtC2(118, vanilla, undefined, false, firstStrike, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(ctx.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === attackerInstanceId), true);
    assert.equal(ctx.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/04 Genesis sleep ends on real damage without retroactive strikes', async () => {
  const attacker = {
    attack: 2,
    defense: 6,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const sleeper = {
    attack: 5,
    defense: 5,
    genesisDisableSelfUntilDamaged: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const fight = async (
    seed: number,
    northSpell: SpellFacts,
    southSpell: SpellFacts,
    run: (setup: Readonly<{
      attackerInstanceId: string;
      ctx: SetupCtx;
      targetInstanceId: string;
    }>) => Promise<void>,
  ): Promise<void> => {
    await withNorthAttacksAtC2(seed, northSpell, undefined, false, southSpell, 0, async ({
      attackerInstanceId,
      ctx,
      targetInstanceId,
    }) => {
      assert.equal(ctx.observe('north').realm.units
        .find(({ instanceId }) => instanceId === targetInstanceId)?.disabled, true);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === targetInstanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      await run({ attackerInstanceId, ctx, targetInstanceId });
    });
  };

  await fight(119, attacker, sleeper, async ({ attackerInstanceId, ctx, targetInstanceId }) => {
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.damage, 0);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.damage, 2);
    assert.equal(ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.disabled, false);
    assert.equal(ctx.session.transcript.at(-1)?.events
      .filter(({ type }) => type === 'minion-awakened').length, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await fight(120, { ...attacker, strikesFirstWhileAttacking: true }, sleeper, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.damage, 5);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.damage, 2);
    assert.equal(ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.disabled, false);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await fight(121, attacker, { ...sleeper, ward: true }, async ({ ctx, targetInstanceId }) => {
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.damage, 0);
    assert.equal(ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.disabled, true);
    assert.equal(ctx.session.transcript.at(-1)?.events
      .some(({ type }) => type === 'minion-awakened'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Lance buffs and breaks on the next unit strike but not a site strike', async () => {
  const lance = {
    attack: 1,
    defense: 1,
    lanceCount: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const twoPower = {
    attack: 2,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;

  await withNorthAttacksAtC2(124, lance, undefined, false, twoPower, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    assert.equal(ctx.observe('south').realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.carriedLanceCount, 1);
    assert.equal(ctx.session.transcript.some(({ events }) => events.some(({ payload, type }) =>
      type === 'lance-gained'
        && canonicalJson(payload) === canonicalJson({
          bearerInstanceId: attackerInstanceId,
          count: 1,
          sourceInstanceId: attackerInstanceId,
        }))), true);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    const attackEvents = ctx.session.transcript.at(-1)!.events;
    assert.deepEqual(attackEvents.find(({ type }) => type === 'lance-broken')?.payload, {
      bearerInstanceId: attackerInstanceId,
      count: 1,
      sourceInstanceId: attackerInstanceId,
    });
    assert.ok(attackEvents.findIndex(({ type }) => type === 'lance-broken')
      < attackEvents.findIndex(({ type }) => type === 'minion-died'));
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.damage, 0);
    assert.equal(ctx.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === targetInstanceId), true);
    assert.equal(ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.carriedLanceCount, undefined);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(125, lance, undefined, false, twoPower, 0, async ({
    attackerInstanceId,
    ctx,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-defend');
    const siteEvents = ctx.session.transcript.at(-1)!.events;
    assert.equal(siteEvents.some(({ type }) => type === 'lance-broken'), false);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.carriedLanceCount, 1);
    const siteStruck = siteEvents.find(({ type }) => type === 'undefended-site-struck');
    assert.ok(siteStruck);
    assert.equal(canonicalJson(siteStruck.payload).includes('"amount":1'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  const tripleLance = { ...lance, defense: 3, lanceCount: 3 as const };
  await withNorthAttacksAtC2(126, {
    ...lance,
    defense: 4,
  }, undefined, false, tripleLance, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    const defendEvents = ctx.session.transcript.at(-1)!.events;
    assert.equal(ctx.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === attackerInstanceId), true);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.damage, 2);
    assert.deepEqual(defendEvents.find(({ payload, type }) =>
      type === 'lance-broken' && canonicalJson(payload).includes(targetInstanceId))?.payload, {
      bearerInstanceId: targetInstanceId,
      count: 3,
      sourceInstanceId: targetInstanceId,
    });
    assert.deepEqual(defendEvents.find(({ payload, type }) =>
      type === 'damage-dealt' && canonicalJson(payload).includes(attackerInstanceId))?.payload, {
      accumulated: 4,
      amount: 4,
      direct: true,
      instanceId: attackerInstanceId,
      seat: 'north',
    });
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(127, { ...lance, ranged: true }, undefined, false, {
    ...twoPower,
    defense: 3,
    ward: true,
  }, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    if (ctx.state.phase === 'intercept') {
      await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === attackerInstanceId
        && descriptor.hit?.instanceId === targetInstanceId);
    const rangedEvents = ctx.session.transcript.at(-1)!.events;
    assert.deepEqual(rangedEvents.map(({ type }) => type), [
      'projectile-shot',
      'strike-damage-allocated',
      'damage-dealt',
      'ward-broken',
      'lance-broken',
    ]);
    assert.equal(canonicalJson(rangedEvents[1]!.payload).includes('"amount":2'), true);
    const rangedTarget = ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId);
    assert.deepEqual(
      rangedTarget && { damage: rangedTarget.damage, warded: rangedTarget.warded },
      { damage: 0, warded: false },
    );
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.carriedLanceCount, undefined);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Ranged strikes without return damage and Ward prevents the first positive damage event', async () => {
  const base = manifest(116, {
    spell: {
      attack: 3,
      defense: 3,
      manaCost: 1,
      ranged: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    site: { rangedUnitsHereRangeBonus: 1 },
  });
  const wardManifest = createGameManifest({
    ...base,
    cards: Object.fromEntries(Object.entries(base.cards).map(([cardId, definition]) => [
      cardId,
      definition.cardType === 'minion' && cardId.startsWith('south-')
        ? { ...definition, ranged: false, ward: true }
        : definition,
    ])),
  });
  await withSetup(wardManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const shooterInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'north')?.instanceId;
    assert.ok(shooterInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const targetInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(targetInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === shooterInstanceId
        && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await withFork(ctx, async (longRange) => {
      await longRange.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await longRange.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await longRange.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await longRange.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const longRangeShots = (await longRange.legalActions('north'))
        .filter(({ descriptor }) => descriptor.kind === 'shoot-projectile');
      const longRangeShot = longRangeShots.find(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.shooterInstanceId === shooterInstanceId
          && descriptor.hit?.instanceId === targetInstanceId);
      assert.ok(longRangeShot);
      assert.deepEqual(
        longRangeShot.descriptor.kind === 'shoot-projectile'
          ? longRangeShot.descriptor.path.map(({ cell }) => cell)
          : [],
        ['C3', 'C2', 'C1'],
      );
      assert.equal(longRangeShots.some(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile' && descriptor.path.length > 3), false);
      await longRange.accept(longRangeShot);
      assert.equal(await longRange.verifyReplay(), true);
    });

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C2');
    const secondTargetInstanceId = ctx.state.realm.units
      .find(({ controller, location }) => controller === 'south' && location === 'C2')?.instanceId;
    assert.ok(secondTargetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === targetInstanceId
        && descriptor.to.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const shots = (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'shoot-projectile');
    const shot = shots.find(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === shooterInstanceId
        && descriptor.direction === 'south'
        && descriptor.hit?.instanceId === targetInstanceId);
    assert.ok(shot);
    assert.equal(shots.filter(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.direction === 'south'
        && descriptor.hit?.seat === 'south').length, 2);
    assert.deepEqual(
      shot.descriptor.kind === 'shoot-projectile'
        ? shot.descriptor.path.map(({ cell }) => cell)
        : [],
      ['C3', 'C2'],
    );
    assert.equal(canonicalJson(shots).includes('"kind":"site"'), false);
    await ctx.accept(shot);

    const shooter = ctx.state.realm.units.find(({ instanceId }) => instanceId === shooterInstanceId);
    const wardedTarget = ctx.state.realm.units.find(({ instanceId }) => instanceId === targetInstanceId);
    assert.deepEqual({ damage: shooter?.damage, location: shooter?.location, tapped: shooter?.tapped }, {
      damage: 0,
      location: 'C3',
      tapped: true,
    });
    assert.deepEqual({ damage: wardedTarget?.damage, warded: wardedTarget?.warded }, { damage: 0, warded: false });
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === secondTargetInstanceId), true);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'projectile-shot',
      'strike-damage-allocated',
      'damage-dealt',
      'ward-broken',
    ]);
    assert.equal(ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.warded, false);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === shooterInstanceId
        && descriptor.hit?.instanceId === targetInstanceId);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) => instanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a surviving enabled minion may take one legal step after its Ranged strike', async () => {
  await withNorthAttacksAtC2(176, {
    attack: 1,
    defense: 3,
    manaCost: 1,
    mayStepAfterRangedStrike: true,
    ranged: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, {
    attack: 1,
    defense: 5,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, 0, async ({ attackerInstanceId, ctx, targetInstanceId }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    if (ctx.state.phase === 'intercept') {
      await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const shot = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === attackerInstanceId
        && descriptor.hit?.instanceId === targetInstanceId));
    assert.equal(shot.accepted, true);
    if (!shot.accepted) return;
    assert.deepEqual({
      decisionSeat: ctx.state.decisionSeat,
      phase: ctx.state.phase,
      sourceInstanceId: ctx.state.pendingRangedStep?.sourceInstanceId,
    }, {
      decisionSeat: 'north',
      phase: 'ranged-step',
      sourceInstanceId: attackerInstanceId,
    });
    const choices = await ctx.legalActions('north');
    assert.equal(choices.every(({ descriptor }) => descriptor.kind === 'resolve-ranged-step'), true);
    assert.deepEqual(choices.flatMap(({ descriptor }) =>
      descriptor.kind === 'resolve-ranged-step' && descriptor.choice === 'step'
        ? [descriptor.to.cell]
        : []).sort(), ['C1', 'C3']);
    const decline = choices.find(({ descriptor }) =>
      descriptor.kind === 'resolve-ranged-step' && descriptor.choice === 'decline');
    const step = choices.find(({ descriptor }) =>
      descriptor.kind === 'resolve-ranged-step'
        && descriptor.choice === 'step'
        && descriptor.to.cell === 'C3');
    assert.ok(decline);
    assert.ok(step);

    await withFork(ctx, async (declinedCtx) => {
      const declined = await declinedCtx.step(decline);
      assert.equal(declined.accepted, true);
      if (!declined.accepted) return;
      assert.equal(declinedCtx.state.phase, 'main');
      assert.equal(declinedCtx.state.pendingRangedStep, null);
      assert.deepEqual(declined.receipt.events, []);
      assert.equal(await declinedCtx.verifyReplay(), true);
    });

    const stepped = await ctx.step(step);
    assert.equal(stepped.accepted, true);
    if (!stepped.accepted) return;
    const steppedUnit = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === attackerInstanceId);
    assert.deepEqual(
      steppedUnit && { location: steppedUnit.location, tapped: steppedUnit.tapped },
      { location: 'C3', tapped: true },
    );
    assert.deepEqual(stepped.receipt.events.map(({ type }) => type), ['unit-stepped']);
    assert.deepEqual(stepped.receipt.events[0]?.payload, {
      from: { cell: 'C2', region: 'surface' },
      instanceId: attackerInstanceId,
      seat: 'north',
      sourceInstanceId: attackerInstanceId,
      steps: 1,
      to: { cell: 'C3', region: 'surface' },
    });
    assert.equal(await ctx.verifyReplay(), true);
    assert.equal((await ctx.step(decline)).accepted, false);
  });
});

test('RULE-04 an empty Ranged projectile does not offer the post-strike step', async () => {
  await withSetup(manifest(177, {
    northSpell: {
      attack: 1,
      defense: 3,
      manaCost: 1,
      mayStepAfterRangedStrike: true,
      ranged: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const shooterInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(shooterInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === shooterInstanceId
        && descriptor.hit === null);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.pendingRangedStep, undefined);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a Ranged striker that dies during the hit cannot leave a pending step', async () => {
  await withNorthAttacksAtC2(178, {
    attack: 1,
    defense: 1,
    manaCost: 1,
    mayStepAfterRangedStrike: true,
    ranged: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, {
    attack: 1,
    deathriteDamageEachUnitHere: 1,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, 0, async ({ attackerInstanceId, ctx, targetInstanceId }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    if (ctx.state.phase === 'intercept') {
      await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === attackerInstanceId
        && descriptor.hit?.instanceId === targetInstanceId);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.pendingRangedStep, undefined);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === attackerInstanceId), true);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a Ranged unit may strike once during Move and Attack or Defend', async () => {
  const base = manifest(179, {
    site: { rangedUnitsHereRangeBonus: 1 },
    spell: {
      attack: 1,
      defense: 5,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  });
  await withPreview(base, async (preview) => {
    const [shooterCard, defendedCard] = preview.state.players.north.hand.spellbook;
    const [attackerCard, visibleCard, stealthedCard] = preview.state.players.south.hand.spellbook;
    assert.ok(shooterCard);
    assert.ok(defendedCard);
    assert.ok(attackerCard);
    assert.ok(visibleCard);
    assert.ok(stealthedCard);
    const cards: Record<string, GameCardDefinition> = { ...base.cards };
    cards[shooterCard.cardId] = {
      ...cards[shooterCard.cardId]!,
      mayRangedStrikeOnceDuringBasicMovement: true,
      movementBonus: 1,
      ranged: true,
      stealth: true,
    } as GameCardDefinition;
    for (const card of [attackerCard, visibleCard, stealthedCard]) {
      cards[card.cardId] = {
        ...cards[card.cardId]!,
        summonToAnySite: true,
      } as GameCardDefinition;
    }
    cards[stealthedCard.cardId] = {
      ...cards[stealthedCard.cardId]!,
      stealth: true,
    } as GameCardDefinition;
    const gameManifest = createGameManifest({ ...base, cards });
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === shooterCard.cardId && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === attackerCard.cardId && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === visibleCard.cardId && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === stealthedCard.cardId && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === defendedCard.cardId && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ctx.state.realm.units
          .find(({ cardId }) => cardId === attackerCard.cardId)?.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      if (ctx.state.phase === 'intercept') {
        await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

      const shooter = ctx.state.realm.units.find(({ cardId }) =>
        cardId === shooterCard.cardId);
      const defended = ctx.state.realm.units.find(({ cardId }) =>
        cardId === defendedCard.cardId);
      const attacker = ctx.state.realm.units.find(({ cardId }) =>
        cardId === attackerCard.cardId);
      const visible = ctx.state.realm.units.find(({ cardId }) =>
        cardId === visibleCard.cardId);
      const stealthed = ctx.state.realm.units.find(({ cardId }) =>
        cardId === stealthedCard.cardId);
      assert.ok(shooter);
      assert.ok(defended);
      assert.ok(attacker);
      assert.ok(visible);
      assert.ok(stealthed);
      const shooterId = shooter.instanceId;
      const defendedId = defended.instanceId;
      const attackerId = attacker.instanceId;
      const visibleId = visible.instanceId;
      const stealthedId = stealthed.instanceId;
      const findUnit = (state: typeof ctx.state, instanceId: string) =>
        state.realm.units.find((unit) => unit.instanceId === instanceId);
      const shooterShots = async (live: SetupCtx) =>
        (await live.legalActions('north')).filter(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.shooterInstanceId === shooterId);

      await withFork(ctx, async (moving) => {
        const moveStarted = await moving.step(await moving.action(({ descriptor }) =>
          descriptor.kind === 'move-and-attack'
            && descriptor.unitInstanceId === shooterId
            && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'));
        assert.equal(moveStarted.accepted, true);
        if (!moveStarted.accepted) return;
        assert.equal(moving.state.phase, 'movement');
        assert.equal(findUnit(moving.state, shooterId)?.location, 'C4');
        const originShots = await shooterShots(moving);
        const originShot = originShots.find(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.direction === 'south'
            && descriptor.hit?.instanceId === visibleId);
        assert.ok(originShot);
        assert.deepEqual(originShot.descriptor.kind === 'shoot-projectile'
          ? originShot.descriptor.path.map(({ cell }) => cell)
          : [], ['C4', 'C3']);
        assert.equal(originShots.some(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.hit?.instanceId === stealthedId), false);
        assert.equal(originShots.some(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.hit?.instanceId === attackerId), false);

        const originFired = await moving.step(originShot);
        assert.equal(originFired.accepted, true);
        if (!originFired.accepted) return;
        assert.equal(moving.state.phase, 'movement');
        assert.equal(findUnit(moving.state, visibleId)?.damage, 1);
        assert.equal(findUnit(moving.state, shooterId)?.stealthed, false);
        assert.equal(originFired.receipt.events.some(({ payload, type }) =>
          type === 'stealth-lost' && canonicalJson(payload).includes(shooterId)), true);
        assert.equal((await shooterShots(moving)).length, 0);

        const beforeForgeHash = hashGameState(moving.state);
        const beforeForgeTranscript = moving.session.transcript.length;
        const forged = await moving.stepRequest({
          actionId: opaqueActionId(
            'sorcery-core-v1',
            'north',
            moving.state.stateVersion,
            originShot.descriptor,
          ),
          seat: 'north',
          stateVersion: moving.state.stateVersion,
        });
        assert.equal(forged.accepted, false);
        if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
        assert.equal(hashGameState(forged.session.state), beforeForgeHash);
        assert.equal(forged.session.transcript.length, beforeForgeTranscript);
        await moving.take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
        assert.equal(findUnit(moving.state, shooterId)?.location, 'C3');
        await moving.take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
        assert.equal(findUnit(moving.state, shooterId)?.location, 'C2');
        await moving.take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
        assert.equal(moving.state.phase, 'attack');
        const movedShooter = findUnit(moving.state, shooterId);
        assert.deepEqual(movedShooter && {
          location: movedShooter.location,
          tapped: movedShooter.tapped,
        }, { location: 'C2', tapped: true });
        assert.equal(moving.session.transcript.flatMap(({ events }) => events)
          .filter(({ type }) => type === 'projectile-shot').length, 1);
        assert.equal(await moving.verifyReplay(), true);
      });

      await withFork(ctx, async (defending) => {
        await defending.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await defending.take(({ descriptor }) =>
          descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await defending.take(({ descriptor }) =>
          descriptor.kind === 'move-and-attack'
            && descriptor.unitInstanceId === attackerId
            && descriptor.path.length === 1);
        await defending.take(({ descriptor }) =>
          descriptor.kind === 'declare-attack'
            && descriptor.target.kind === 'minion'
            && descriptor.target.instanceId === defendedId);
        const defendStarted = await defending.step(await defending.action(({ descriptor }) =>
          descriptor.kind === 'defend'
            && descriptor.unitInstanceId === shooterId
            && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'));
        assert.equal(defendStarted.accepted, true);
        if (!defendStarted.accepted) return;
        await defending.take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
        assert.equal(defending.state.phase, 'movement');
        assert.equal(findUnit(defending.state, shooterId)?.location, 'C3');
        const intermediateShots = await shooterShots(defending);
        const intermediateShot = intermediateShots.find(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.hit?.instanceId === visibleId);
        assert.ok(intermediateShot);
        assert.deepEqual(intermediateShot.descriptor.kind === 'shoot-projectile'
          ? intermediateShot.descriptor.path.map(({ cell }) => cell)
          : [], ['C3']);
        assert.equal(intermediateShots.some(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.hit?.instanceId === stealthedId), false);
        assert.equal(intermediateShots.some(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.hit?.instanceId === attackerId), false);
        const intermediateFired = await defending.step(intermediateShot);
        assert.equal(intermediateFired.accepted, true);
        if (!intermediateFired.accepted) return;
        assert.equal(findUnit(defending.state, visibleId)?.damage, 1);
        assert.equal((await shooterShots(defending)).length, 0);
        await defending.take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
        assert.equal(findUnit(defending.state, shooterId)?.location, 'C2');
        const defendFinished = await defending.step(await defending.action(({ descriptor }) =>
          descriptor.kind === 'continue-basic-movement'));
        assert.equal(defendFinished.accepted, true);
        if (!defendFinished.accepted) return;
        assert.equal(defendFinished.session.state.phase, 'defend');
        assert.equal(defendFinished.session.state.pendingCombat?.defenders.some(({ instanceId }) =>
          instanceId === shooterId), true);
        assert.equal(defendFinished.receipt.events.some(({ type }) => type === 'defender-joined'), true);
        assert.equal(defendFinished.session.transcript.flatMap(({ events }) => events)
          .filter(({ type }) => type === 'projectile-shot').length, 1);
        assert.equal(await defending.verifyReplay(), true);
      });
    });
  });
});

test('RULE-04 a drag projectile stops at the first visible unit and may fight after arrival', async () => {
  const pudge = {
    attack: 5,
    defense: 5,
    immobile: true,
    manaCost: 1,
    shootsDragProjectile: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const target = {
    attack: 3,
    defense: 6,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    ward: true,
  } as const;
  const base = manifest(142, { northSpell: pudge, southSpell: target });
  await withPreview(base, async (preview) => {
    const pudgeCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const blockerCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    assert.ok(pudgeCardId);
    assert.ok(blockerCardId);
    assert.notEqual(pudgeCardId, blockerCardId);
    const blockerDefinition = base.cards[blockerCardId];
    assert.equal(blockerDefinition?.cardType, 'minion');
    const custom = createGameManifest({
      authority: base.authority,
      cards: {
        ...base.cards,
        [blockerCardId]: {
          ...blockerDefinition,
          immobile: false,
          shootsDragProjectile: false,
          stealth: true,
        } as GameCardDefinition,
      },
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });
    await withSetup(custom, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const pudgeCard = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === pudgeCardId);
      const blockerCard = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === blockerCardId);
      assert.ok(pudgeCard);
      assert.ok(blockerCard);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === pudgeCard.instanceId
          && descriptor.cell === 'C4');
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile'
          && descriptor.shooterInstanceId === pudgeCard.instanceId), false);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === blockerCard.instanceId
          && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      const targetCard = ctx.state.players.south.hand.spellbook[0];
      assert.ok(targetCard);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === targetCard.instanceId
          && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const pudgeInstanceId = pudgeCard.instanceId;
      const blockerInstanceId = blockerCard.instanceId;
      const targetInstanceId = targetCard.instanceId;
      const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile'
          && descriptor.shooterInstanceId === pudgeInstanceId
          && descriptor.direction === 'south'
          && descriptor.hit?.instanceId === targetInstanceId);
      assert.deepEqual(choices.map(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival), [false, true]);
      assert.equal(choices.every(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile'
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'), true);
      assert.equal(choices.some(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile'
          && descriptor.hit?.instanceId === blockerInstanceId), false);

      await withFork(ctx, async (noFightCtx) => {
        await noFightCtx.accept(choices.find(({ descriptor }) =>
          descriptor.kind === 'shoot-drag-projectile' && !descriptor.fightOnArrival)!);
        const dragged = noFightCtx.state.realm.units
          .find(({ instanceId }) => instanceId === targetInstanceId);
        assert.deepEqual({ location: dragged?.location, tapped: dragged?.tapped, warded: dragged?.warded }, {
          location: 'C4',
          tapped: false,
          warded: true,
        });
        assert.equal(noFightCtx.state.realm.units
          .find(({ instanceId }) => instanceId === pudgeInstanceId)?.tapped, true);
        assert.deepEqual(noFightCtx.session.transcript.at(-1)?.events.map(({ type }) => type), [
          'projectile-shot',
          'unit-dragged',
        ]);
        const draggedPayload = noFightCtx.session.transcript.at(-1)?.events[1]?.payload;
        const draggedJson = canonicalJson(draggedPayload ?? null);
        assert.equal(draggedJson.includes('"steps":2'), true);
        assert.match(
          draggedJson,
          /"path":\[{"cell":"C2","region":"surface"},{"cell":"C3","region":"surface"},{"cell":"C4","region":"surface"}\]/,
        );
        assert.equal(await noFightCtx.verifyReplay(), true);
      });

      await ctx.accept(choices.find(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival)!);
      assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'projectile-shot',
        'unit-dragged',
        'fight-started',
        'strike-damage-allocated',
        'damage-dealt',
        'damage-dealt',
        'ward-broken',
      ]);
      const foughtPudge = ctx.state.realm.units
        .find(({ instanceId }) => instanceId === pudgeInstanceId);
      const foughtTarget = ctx.state.realm.units
        .find(({ instanceId }) => instanceId === targetInstanceId);
      assert.deepEqual({ damage: foughtPudge?.damage, location: foughtPudge?.location }, {
        damage: 3,
        location: 'C4',
      });
      assert.deepEqual({ damage: foughtTarget?.damage, location: foughtTarget?.location, warded: foughtTarget?.warded }, {
        damage: 0,
        location: 'C4',
        warded: false,
      });
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-03/04 a drag projectile resumes after ordered movement Deathrites before fighting', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const base = manifest(143, {
    northSpell: {
      attack: 5,
      defense: 5,
      immobile: true,
      manaCost: 1,
      shootsDragProjectile: true,
      thresholds: { ...thresholds, earth: 1 },
    },
    southSpell: {
      attack: 3,
      defense: 10,
      manaCost: 1,
      otherNearbyAlliesPowerBonus: 1,
      thresholds: { ...thresholds, earth: 1 },
    },
  });
  await withPreview(base, async (preview) => {
    const pudgeCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const rainCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    const fragileIds = preview.state.players.south.hand.spellbook.slice(0, 2).map(({ cardId }) => cardId);
    const targetCardId = preview.state.players.south.hand.spellbook[2]?.cardId;
    assert.ok(pudgeCardId && rainCardId && targetCardId);
    assert.equal(fragileIds.length, 2);
    const cards = { ...base.cards };
    cards[rainCardId] = {
      cardType: 'magic',
      damageEachAbovegroundMinion: 1,
      manaCost: 0,
      thresholds,
    };
    for (const fragileId of fragileIds) {
      cards[fragileId] = {
        attack: 1,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        stealth: true,
        thresholds,
      };
    }
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });
    const runSetup = async (
      fragileCell: 'C1' | 'C2',
      run: (setup: Readonly<{
        ctx: SetupCtx;
        fragiles: GameSession['state']['realm']['units'];
        pudge: GameSession['state']['realm']['units'][number];
        target: GameSession['state']['realm']['units'][number];
      }>) => Promise<void>,
    ): Promise<void> => {
      await withSetup(gameManifest, async (ctx) => {
        await ctx.keep();
        await ctx.keep();
        await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === pudgeCardId
          && descriptor.cell === 'C4');
        await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
        if (fragileCell === 'C1') {
          for (const fragileId of fragileIds) {
            await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
              && descriptor.cardId === fragileId
              && descriptor.cell === fragileCell);
          }
        }
        await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
        await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
        if (fragileCell === 'C2') {
          for (const fragileId of fragileIds) {
            await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
              && descriptor.cardId === fragileId
              && descriptor.cell === fragileCell);
          }
        }
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === targetCardId
          && descriptor.cell === 'C2');
        await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
        await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
        await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic' && descriptor.cardId === rainCardId);
        const pudge = ctx.state.realm.units.find(({ cardId }) => cardId === pudgeCardId);
        const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetCardId);
        const fragiles = ctx.state.realm.units.filter(({ cardId }) => fragileIds.includes(cardId));
        assert.ok(pudge && target);
        assert.equal(fragiles.length, 2);
        assert.equal(fragiles.every(({ damage }) => damage === 1), true);
        await run({ ctx, fragiles, pudge, target });
      });
    };

    await runSetup('C1', async ({ ctx, fragiles, pudge, target }) => {
      const atlasBefore = ctx.state.players.south.atlas.length;
      const atlasHandBefore = ctx.state.players.south.hand.atlas.length;
      const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile'
          && descriptor.shooterInstanceId === pudge.instanceId
          && descriptor.hit?.instanceId === target.instanceId
          && descriptor.direction === 'south');
      assert.deepEqual(choices.map(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival), [false, true]);

      const branchHashes: string[] = [];
      for (const [choiceIndex, choice] of choices.entries()) {
        assert.equal(choice.descriptor.kind, 'shoot-drag-projectile');
        if (choice.descriptor.kind !== 'shoot-drag-projectile') throw new Error('unreachable');
        const fightOnArrival = choice.descriptor.fightOnArrival;
        await withFork(ctx, async (fork) => {
          const interrupted = await fork.step(choice);
          assert.equal(interrupted.accepted, true);
          if (!interrupted.accepted) throw new Error('expected drag to reach Deathrite ordering');
          assert.equal(fork.state.stateVersion, ctx.state.stateVersion + 1);
          assert.deepEqual(interrupted.receipt.events.map(({ type }) => type), [
            'projectile-shot',
            'unit-dragged',
          ]);
          assert.equal(fork.state.phase, 'deathrite-order');
          assert.equal(fork.state.decisionSeat, 'south');
          assert.equal(fork.state.realm.units.find(({ instanceId }) =>
            instanceId === target.instanceId)?.location, 'C3');
          assert.equal(fragiles.every(({ instanceId }) => !fork.state.realm.units
            .some((unit) => unit.instanceId === instanceId)), true);
          assert.equal(fragiles.every(({ instanceId }) => !fork.state.players.south.cemetery
            .some((card) => card.instanceId === instanceId)), true);
          const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
            fork.checkpoint(),
          )));
          assert.equal(
            canonicalJson(restored as unknown as JsonValue),
            canonicalJson(fork.session as unknown as JsonValue),
          );
          const orders = (await fork.legalActions('south')).filter(({ descriptor }) =>
            descriptor.kind === 'order-deathrites');
          assert.equal(orders.length, 2);
          assert.equal((await fork.legalActions('north')).length, 0);
          const resolved = await fork.step(orders[choiceIndex]!);
          assert.equal(resolved.accepted, true);
          if (!resolved.accepted) throw new Error('expected drag to resume after Deathrites');
          assert.equal(fork.state.stateVersion, restored.state.stateVersion + 1);
          const types = resolved.receipt.events.map(({ type }) => type);
          assert.deepEqual(types.slice(0, 5), [
            'deathrite-order-committed',
            'site-drawn',
            'site-drawn',
            'minion-died',
            'minion-died',
          ]);
          assert.equal(types[5], 'unit-dragged');
          assert.equal(fork.state.phase, 'main');
          assert.equal(fork.state.pendingDeathrites, undefined);
          assert.equal(fork.state.realm.units.find(({ instanceId }) =>
            instanceId === target.instanceId)?.location, 'C4');
          assert.equal(fragiles.every(({ instanceId }) => fork.state.players.south.cemetery
            .some((card) => card.instanceId === instanceId)), true);
          assert.equal(fork.state.players.south.atlas.length, atlasBefore - 2);
          assert.equal(fork.state.players.south.hand.atlas.length, atlasHandBefore + 2);
          assert.equal(types.includes('fight-started'), fightOnArrival);
          assert.equal(types.indexOf('fight-started') > types.indexOf('unit-dragged'),
            fightOnArrival);
          assert.equal(await fork.verifyReplay(), true);
          branchHashes.push(hashGameState(fork.state));
        });
      }
      assert.equal(new Set(branchHashes).size, 2);
    });

    await runSetup('C2', async ({ ctx, pudge, target }) => {
      const finalChoices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'shoot-drag-projectile'
          && descriptor.shooterInstanceId === pudge.instanceId
          && descriptor.hit?.instanceId === target.instanceId
          && descriptor.direction === 'south');
      for (const [choiceIndex, choice] of finalChoices.entries()) {
        assert.equal(choice.descriptor.kind, 'shoot-drag-projectile');
        if (choice.descriptor.kind !== 'shoot-drag-projectile') throw new Error('unreachable');
        const fightOnArrival = choice.descriptor.fightOnArrival;
        await withFork(ctx, async (fork) => {
          const interrupted = await fork.step(choice);
          assert.equal(interrupted.accepted, true);
          if (!interrupted.accepted) throw new Error('expected final drag edge to reach Deathrites');
          assert.equal(fork.state.stateVersion, ctx.state.stateVersion + 1);
          assert.equal(fork.state.phase, 'deathrite-order');
          assert.equal(fork.state.realm.units.find(({ instanceId }) =>
            instanceId === target.instanceId)?.location, 'C4');
          assert.equal(interrupted.receipt.events.some(({ type }) => type === 'fight-started'), false);
          const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
            fork.checkpoint(),
          )));
          const orders = (await fork.legalActions('south')).filter(({ descriptor }) =>
            descriptor.kind === 'order-deathrites');
          assert.equal(orders.length, 2);
          const resolved = await fork.step(orders[choiceIndex]!);
          assert.equal(resolved.accepted, true);
          if (!resolved.accepted) throw new Error('expected final-edge Deathrites to finish');
          assert.equal(fork.state.stateVersion, restored.state.stateVersion + 1);
          const types = resolved.receipt.events.map(({ type }) => type);
          assert.equal(types.includes('unit-dragged'), false);
          assert.equal(types.includes('fight-started'), fightOnArrival);
          assert.ok(!fightOnArrival
            || types.indexOf('fight-started') > types.lastIndexOf('minion-died'));
          assert.equal(fork.state.phase, 'main');
          assert.equal(await fork.verifyReplay(), true);
        });
      }
    });
  });
});

test('RULE-03 Granary Rats suppresses its site threshold while enabled', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(6).fill('dual-site'),
    avatar: 'north-avatar',
    spellbook: ['rats', 'gated', 'filler'],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('dual-site'),
    avatar: 'south-avatar',
    spellbook: Array(3).fill('rats'),
  };
  const cards: Record<string, GameCardDefinition> = {
    'dual-site': { cardType: 'site', elements: ['earth', 'fire'] },
    filler: {
      attack: 1, cardType: 'minion', defense: 1, manaCost: 0, thresholds,
    },
    gated: {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds: { ...thresholds, earth: 1, fire: 1 },
    },
    'north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    rats: {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      siteProvidesNoThreshold: true,
      summonToAnySite: true,
      thresholds,
    },
    'south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-granary-rats-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      rats: { ...cards.rats, siteProvidesNoThreshold: false } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /siteProvidesNoThreshold must be true when defined/);

  const gameManifest = createGameManifest({ ...input, seed: 1 });
  assert.deepEqual(gameManifest.cards.rats, cards.rats);
  const canSummonGated = async (ctx: SetupCtx) =>
    (await ctx.legalActions('north')).some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'gated' && descriptor.cell === 'C4');

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    assert.deepEqual(ctx.observe('north').players.north.affinity,
      { air: 0, earth: 1, fire: 1, water: 0 });
    assert.equal(await canSummonGated(ctx), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'rats' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    assert.deepEqual(ctx.observe('north').players.north.affinity,
      { air: 0, earth: 0, fire: 0, water: 0 });
    assert.equal(await canSummonGated(ctx), false);
    assert.equal(await ctx.verifyReplay(), true);
  });

  const disabledManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      rats: { ...cards.rats, genesisDisableSelfUntilDamaged: true } as GameCardDefinition,
    },
    seed: 1,
  });
  await withSetup(disabledManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'rats' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    const rat = ctx.state.realm.units.find(({ cardId }) => cardId === 'rats');
    assert.equal(rat?.disabledUntilDamaged, true);
    assert.deepEqual(ctx.observe('north').players.north.affinity,
      { air: 0, earth: 1, fire: 1, water: 0 });
    assert.equal(await canSummonGated(ctx), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  const protectedManifest = createGameManifest({
    ...input,
    cards: {
      ...cards,
      'dual-site': {
        ...cards['dual-site'],
        cannotBeMovedDestroyedOrModified: true,
      } as GameCardDefinition,
    },
    seed: 1,
  });
  await withSetup(protectedManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'rats' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    assert.deepEqual(ctx.observe('north').players.north.affinity,
      { air: 0, earth: 1, fire: 1, water: 0 });
    assert.equal(await canSummonGated(ctx), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a provider adds affinity until that minion dies', async () => {
  await withNorthAttacksAtC2(46, {
    attack: 1,
    defense: 1,
    manaCost: 1,
    provides: 'earth',
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, undefined, 0, async ({ ctx, targetInstanceId }) => {
    assert.equal(ctx.observe('north').players.north.affinity.earth, 3);
    assert.equal(ctx.observe('south').players.south.affinity.earth, 4);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(ctx.observe('north').players.north.affinity.earth, 2);
    assert.equal(ctx.observe('south').players.south.affinity.earth, 3);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Lethal kills a tougher minion with positive damage but not zero damage', async () => {
  const spell = (attack: number): SpellFacts => ({
    attack,
    defense: 5,
    lethal: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  await withNorthAttacksAtC2(45, spell(1), undefined, false, undefined, 0, async ({ ctx, targetInstanceId }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(ctx.state.players.north.cemetery.length, 1);
    assert.equal(ctx.state.players.south.cemetery.length, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(44, spell(0), undefined, false, undefined, 0, async ({ ctx, targetInstanceId }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(ctx.state.players.north.cemetery.length, 0);
    assert.equal(ctx.state.players.south.cemetery.length, 0);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a prohibited minion cannot move to Defend but can still Intercept', async () => {
  const spell = {
    attack: 2,
    cannotDefend: true,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  await withNorthAttacksAtC2(50, spell, undefined, false, undefined, 0, async ({
    ctx,
    defenderInstanceId,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === defenderInstanceId), false);
  });

  await withNorthAttacksAtC2(51, spell, undefined, false, undefined, 0, async ({
    ctx,
    targetInstanceId,
  }) => {
    const site = ctx.state.realm.sites.C2;
    assert.ok(site);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === site.instanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === targetInstanceId), true);
  });

  await withNorthAttacksAtC2(52, spell, undefined, false, undefined, 0, async ({
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept' && descriptor.unitInstanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Immobile units attack and Defend in place but cannot move themselves', async () => {
  const vanilla = {
    airborne: true,
    attack: 2,
    defense: 5,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const immobile = {
    ...vanilla,
    connectsTopBottom: true,
    immobile: true,
    movementBonus: 2 as const,
  };

  await withNorthAttacksAtC2(145, vanilla, undefined, false, immobile, 0, async ({
    ctx,
    defenderInstanceId,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === defenderInstanceId), false);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(146, vanilla, undefined, false, immobile, 0, async ({
    ctx,
    targetInstanceId,
  }) => {
    const site = ctx.state.realm.sites.C2;
    assert.ok(site);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === site.instanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'defend'
      && descriptor.unitInstanceId === targetInstanceId
      && descriptor.path.length === 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(147, vanilla, undefined, false, immobile, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = (await ctx.legalActions('south')).filter(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === targetInstanceId);
    assert.equal(moves.length, 1);
    const zero = moves[0];
    assert.ok(zero);
    assert.equal(zero.descriptor.kind === 'move-and-attack'
      && zero.descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(','), 'C2/surface');
    assert.equal(zero.descriptor.kind, 'move-and-attack');
    const forged: GameLegalAction = {
      ...zero,
      descriptor: {
        ...zero.descriptor,
        path: [...zero.descriptor.path, { cell: 'C3', region: 'surface' }],
        to: { cell: 'C3', region: 'surface' },
      },
    };
    const ignored = await ctx.step(forged);
    assert.equal(ignored.accepted, true);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.location, 'C2');
    assert.doesNotMatch(canonicalJson(ignored.receipt.events), /"C3"/);
    await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === attackerInstanceId);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a fully prohibited minion cannot use Defend or Intercept', async () => {
  const spell = {
    attack: 4,
    cannotDefendOrIntercept: true,
    defense: 4,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  await withNorthAttacksAtC2(112, spell, undefined, false, undefined, 0, async ({
    ctx,
    defenderInstanceId,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === defenderInstanceId), false);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates), true);
  });

  await withNorthAttacksAtC2(113, spell, undefined, false, undefined, 0, async ({ ctx }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'main');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Airborne moves diagonally and restricts attacks and Intercept', async () => {
  const ground = {
    attack: 3,
    defense: 3,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const airborne = { ...ground, airborne: true };
  const ranged = { ...ground, ranged: true };
  const canTarget = async (
    ctx: SetupCtx,
    targetInstanceId: string,
  ): Promise<boolean> => (await ctx.legalActions('north')).some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === targetInstanceId);
  const addDiagonalSite = async (ctx: SetupCtx): Promise<void> => {
    if (ctx.state.phase === 'intercept') {
      await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B3');
  };

  await withNorthAttacksAtC2(118, ranged, undefined, false, airborne, 0, async ({
    ctx,
    targetInstanceId,
  }) => {
    assert.equal(await canTarget(ctx, targetInstanceId), false);
  });

  await withNorthAttacksAtC2(119, airborne, undefined, false, airborne, 0, async ({
    ctx,
    targetInstanceId,
  }) => {
    assert.equal(await canTarget(ctx, targetInstanceId), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'intercept');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept'
        && descriptor.unitInstanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(120, airborne, undefined, false, ground, 0, async ({
    attackerInstanceId,
    ctx,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'main');
    await addDiagonalSite(ctx);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,B3');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.location, 'B3');
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(121, airborne, undefined, false, ranged, 0, async ({
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'intercept');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept' && descriptor.unitInstanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(122, ground, undefined, false, ground, 0, async ({
    attackerInstanceId,
    ctx,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await addDiagonalSite(ctx);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,B3'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02 Cloud City flies once per turn at three Air affinity and carries normal occupants', async () => {
  await withSetup(manifest(136, {
    northSpell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    },
    site: {
      elements: ['air'],
      flyToNearbyVoidOncePerTurnAtAirThreshold: 3,
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const sourceSiteId = ctx.state.realm.sites.C4?.instanceId;
    const minionId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(sourceSiteId);
    assert.ok(minionId);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'fly-site'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');

    const manaBefore = ctx.state.players.north.mana;
    const flown = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'fly-site'
      && descriptor.sourceSiteInstanceId === sourceSiteId
      && descriptor.targetCell === 'D4'));
    assert.equal(flown.accepted, true);
    if (!flown.accepted) return;
    const receipt = flown.receipt;
    assert.equal(ctx.state.realm.sites.C4, undefined);
    assert.equal(ctx.state.realm.sites.D4?.instanceId, sourceSiteId);
    assert.equal(ctx.state.players.north.avatar.location, 'D4');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === minionId)?.location, 'D4');
    assert.equal(ctx.state.players.north.mana, manaBefore);
    assert.equal(receipt.events.some(({ type }) => type === 'site-flown'), true);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'fly-site' && descriptor.sourceSiteInstanceId === sourceSiteId), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Updraft Ridge gives only Airborne minions a free departure to move or Defend', async () => {
  const base = manifest(162, {
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  });
  await withPreview(base, async (preview) => {
    const updraftId = preview.state.players.north.hand.atlas[0]?.cardId;
    const airborneId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const groundId = preview.state.players.north.hand.spellbook[1]?.cardId;
    assert.ok(updraftId);
    assert.ok(airborneId);
    assert.ok(groundId);
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards: {
        ...base.cards,
        [airborneId]: { ...base.cards[airborneId], airborne: true } as GameCardDefinition,
        [updraftId]: {
          ...base.cards[updraftId],
          airborneMinionsAtopMoveFreelyAway: true,
        } as GameCardDefinition,
      },
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });
    assert.throws(() => createGameManifest({
      authority: gameManifest.authority,
      cards: {
        ...gameManifest.cards,
        [updraftId]: {
          ...gameManifest.cards[updraftId],
          airborneMinionsAtopMoveFreelyAway: false,
        } as unknown as GameCardDefinition,
      },
      decks: gameManifest.decks,
      firstSeat: gameManifest.firstSeat,
      seed: gameManifest.seed,
    }), /airborneMinionsAtopMoveFreelyAway must be true when defined/);

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cardId === updraftId && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cardId === airborneId);
      const airborneInstanceId = ctx.state.realm.units.at(-1)?.instanceId;
      assert.ok(airborneInstanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cardId === groundId);
      const groundInstanceId = ctx.state.realm.units.at(-1)?.instanceId;
      assert.ok(groundInstanceId);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
      const attackerInstanceId = ctx.state.realm.units.at(-1)?.instanceId;
      assert.ok(attackerInstanceId);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      const hasPath = async (unitInstanceId: string): Promise<boolean> =>
        (await ctx.legalActions('north'))
          .some(({ descriptor }) => descriptor.kind === 'move-and-attack'
            && descriptor.unitInstanceId === unitInstanceId
            && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
      assert.equal(await hasPath(airborneInstanceId), true);
      assert.equal(await hasPath(groundInstanceId), false);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === attackerInstanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'defend'
          && descriptor.unitInstanceId === groundInstanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'), false);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'defend'
          && descriptor.unitInstanceId === airborneInstanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
      assert.equal(ctx.state.realm.units
        .find(({ instanceId }) => instanceId === airborneInstanceId)?.location, 'C2');
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-04 Mountain Pass blocks only occupied ground-minion entry', async () => {
  const base = manifest(161, {
    site: { blocksGroundMinionEntryWhileMinionAtop: true },
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  });
  await withPreview(base, async (preview) => {
    const northGroundCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const northAirborneCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    const southGroundCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
    const southAirborneCardId = preview.state.players.south.hand.spellbook[1]?.cardId;
    assert.ok(northGroundCardId);
    assert.ok(northAirborneCardId);
    assert.ok(southGroundCardId);
    assert.ok(southAirborneCardId);
    const siteCardId = base.decks.north.atlas[0]!;
    assert.throws(() => createGameManifest({
      authority: base.authority,
      cards: {
        ...base.cards,
        [siteCardId]: {
          ...base.cards[siteCardId],
          blocksGroundMinionEntryWhileMinionAtop: false,
        } as unknown as GameCardDefinition,
      },
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    }), /blocksGroundMinionEntryWhileMinionAtop must be true when defined/);
    const airborne = (cardId: string): GameCardDefinition => ({
      ...base.cards[cardId],
      airborne: true,
    } as GameCardDefinition);
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards: {
        ...base.cards,
        [northAirborneCardId]: airborne(northAirborneCardId),
        [southAirborneCardId]: airborne(southAirborneCardId),
      },
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed: base.seed,
    });
    assert.equal(gameManifest.cards[siteCardId]?.cardType === 'site'
      && gameManifest.cards[siteCardId].blocksGroundMinionEntryWhileMinionAtop, true);

    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      const northGround = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === northGroundCardId);
      const northAirborne = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === northAirborneCardId);
      assert.ok(northGround);
      assert.ok(northAirborne);
      const northGroundInstanceId = northGround.instanceId;
      const northAirborneInstanceId = northAirborne.instanceId;
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === northGroundInstanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === northAirborneInstanceId);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const southGround = ctx.state.players.south.hand.spellbook
        .find(({ cardId }) => cardId === southGroundCardId);
      assert.ok(southGround);
      const southGroundInstanceId = southGround.instanceId;
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === southGroundInstanceId);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === northGroundInstanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === northAirborneInstanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const southAirborne = ctx.state.players.south.hand.spellbook
        .find(({ cardId }) => cardId === southAirborneCardId);
      assert.ok(southAirborne);
      const southAirborneInstanceId = southAirborne.instanceId;
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === southAirborneInstanceId
          && descriptor.cell === 'C2');
      assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cell === 'C2'), true);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const moves = await ctx.legalActions('north');
      const entersC2 = (instanceId: string): boolean => moves.some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === instanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2');
      assert.equal(entersC2(northGroundInstanceId), false);
      assert.equal(entersC2(northAirborneInstanceId), true);
      assert.equal(moves.some(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'), true);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === northAirborneInstanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === southAirborneInstanceId);
      assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
        descriptor.kind === 'defend' && descriptor.unitInstanceId === southGroundInstanceId), false);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-04 Stealth blocks attacks, Defend, Intercept, and projectiles until interaction', async () => {
  const ground = {
    attack: 1,
    defense: 5,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const stealth = { ...ground, attack: 3, stealth: true };
  await withNorthAttacksAtC2(123, stealth, undefined, false, ground, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.stealthed, true);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === targetInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === attackerInstanceId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'defend-window-closed'), false);
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attackerInstanceId)?.stealthed, false);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === targetInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === attackerInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(
    124,
    { ...ground, ranged: true, stealth: true },
    undefined,
    false,
    stealth,
    0,
    async ({ attackerInstanceId, ctx, targetInstanceId }) => {
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === targetInstanceId), false);
      await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const shots = (await ctx.legalActions('north'))
        .filter(({ descriptor }) => descriptor.kind === 'shoot-projectile');
      assert.equal(shots.some(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.hit?.instanceId === targetInstanceId), false);
      const miss = shots.find(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile' && descriptor.direction === 'north');
      assert.ok(miss);
      await ctx.accept(miss);
      assert.equal(ctx.state.realm.units
        .find(({ instanceId }) => instanceId === attackerInstanceId)?.stealthed, false);
      assert.equal(ctx.state.realm.units
        .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, true);
      assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
      assert.equal(await ctx.verifyReplay(), true);
    },
  );
});

test('RULE-04 Sly Fox gains Stealth once at the end of its controller turn', async () => {
  await withSetup(manifest(126, {
    spell: {
      attack: 1,
      defense: 1,
      gainsStealthAtEndOfTurn: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion');
    assert.equal(ctx.state.realm.units[0]?.stealthed, false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units[0]?.stealthed, true);
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['stealth-gained', 'turn-ended', 'turn-started'],
    );
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-gained'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 conditional end-turn Stealth requires no nearby enemy in the same region', async () => {
  await withSetup(manifest(241, {
    northSpell: {
      attack: 2,
      defense: 2,
      gainsStealthAtEndOfTurnIfNoEnemiesNearby: true,
      manaCost: 0,
      submerge: true,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    site: { elements: ['water'] },
    southSpell: {
      attack: 1,
      defense: 1,
      manaCost: 0,
      stealth: true,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B3' && (descriptor.region ?? 'surface') === 'surface');
    const enemyInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(enemyInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const sourceId = (session: GameSession): string => {
      const instanceId = session.state.realm.units
        .find(({ controller }) => controller === 'north')?.instanceId;
      assert.ok(instanceId);
      return instanceId;
    };

    await withFork(ctx, async (avatarBlocked) => {
      await avatarBlocked.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cell === 'C1' && (descriptor.region ?? 'surface') === 'surface');
      const avatarBlockedId = sourceId(avatarBlocked.session);
      await avatarBlocked.take(({ descriptor }) => descriptor.kind === 'end-turn');
      assert.equal(avatarBlocked.state.realm.units
        .find(({ instanceId }) => instanceId === avatarBlockedId)?.stealthed, false);
      assert.deepEqual(avatarBlocked.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'turn-ended',
        'turn-started',
      ]);
      assert.equal(await avatarBlocked.verifyReplay(), true);
    });

    await withFork(ctx, async (otherRegion) => {
      await otherRegion.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cell === 'C1' && (descriptor.region ?? 'surface') === 'underwater');
      const otherRegionId = sourceId(otherRegion.session);
      await otherRegion.take(({ descriptor }) => descriptor.kind === 'end-turn');
      assert.equal(otherRegion.state.realm.units
        .find(({ instanceId }) => instanceId === otherRegionId)?.stealthed, true);
      assert.deepEqual(otherRegion.session.transcript.at(-1)?.events.map(({ type }) => type), [
        'stealth-gained',
        'turn-ended',
        'turn-started',
      ]);
      assert.equal(await otherRegion.verifyReplay(), true);
    });

    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && (descriptor.region ?? 'surface') === 'surface');
    const minionBlockedId = sourceId(ctx.session);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === minionBlockedId)?.stealthed, false);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === enemyInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'B3,B2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === minionBlockedId)?.stealthed, true);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'stealth-gained',
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Scent Hounds permanently removes nearby enemy Stealth', async () => {
  const gameManifest = manifest(160, {
    northSpell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      nearbyEnemiesPermanentlyLoseStealth: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    southSpell: {
      attack: 1,
      defense: 1,
      gainsStealthAtEndOfTurn: true,
      manaCost: 1,
      stealth: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  });
  const canonicalHound = gameManifest.cards[gameManifest.decks.north.spellbook[0]!];
  assert.equal(canonicalHound?.cardType === 'minion'
    && canonicalHound.nearbyEnemiesPermanentlyLoseStealth, true);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const houndInstanceId = ctx.state.realm.units[0]!.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const targetInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')!.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === houndInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    const moved = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === targetInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
    assert.equal(moved.accepted, true);
    if (!moved.accepted) return;
    assert.deepEqual(moved.receipt.events.map(({ type }) => type), [
      'move-and-attack-activated',
      'stealth-lost',
    ]);
    assert.deepEqual(moved.receipt.events[1]?.payload, {
      instanceId: targetInstanceId,
      seat: 'south',
      sourceInstanceId: houndInstanceId,
    });
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');

    const regained = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(regained.accepted, true);
    if (!regained.accepted) return;
    assert.deepEqual(regained.receipt.events.map(({ type }) => type), [
      'stealth-gained',
      'stealth-lost',
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === houndInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Malakhim untaps at its controller End Phase unless Disabled', async () => {
  const gameManifest = manifest(159, {
    northSpell: {
      airborne: true,
      attack: 4,
      defense: 4,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      untapsAtEndOfControllerTurn: true,
      ward: true,
    },
    southSpell: {
      attack: 1,
      defense: 1,
      manaCost: 1,
      movementBonus: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  });
  const malakhimCardId = gameManifest.decks.north.spellbook[0]!;
  const canonicalMalakhim = gameManifest.cards[malakhimCardId];
  assert.equal(canonicalMalakhim?.cardType === 'minion'
    && canonicalMalakhim.airborne
    && canonicalMalakhim.untapsAtEndOfControllerTurn
    && canonicalMalakhim.ward, true);
  assert.throws(() => createGameManifest({
    authority: gameManifest.authority,
    cards: {
      ...gameManifest.cards,
      [malakhimCardId]: {
        ...canonicalMalakhim,
        untapsAtEndOfControllerTurn: false,
      } as unknown as GameCardDefinition,
    },
    decks: gameManifest.decks,
    firstSeat: gameManifest.firstSeat,
    seed: gameManifest.seed,
  }), /untapsAtEndOfControllerTurn must be true when defined/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const malakhim = ctx.state.realm.units[0];
    assert.ok(malakhim);
    const malakhimInstanceId = malakhim.instanceId;
    const readyEnd = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(readyEnd.accepted, true);
    if (!readyEnd.accepted) return;
    assert.equal(readyEnd.receipt.events.some(({ type }) => type === 'minion-untapped'), false);
    assert.deepEqual(readyEnd.receipt.randomDraws, []);

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const attackerInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(attackerInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === malakhimInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    const tappedMalakhim = ctx.state.realm.units
      .find(({ instanceId }) => instanceId === malakhimInstanceId);
    assert.deepEqual({
      controller: tappedMalakhim?.controller,
      location: tappedMalakhim?.location,
      owner: tappedMalakhim?.owner,
      region: tappedMalakhim?.region,
      tapped: tappedMalakhim?.tapped,
      warded: tappedMalakhim?.warded,
    }, {
      controller: 'north',
      location: 'C3',
      owner: 'north',
      region: 'surface',
      tapped: true,
      warded: true,
    });

    const ended = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(ended.accepted, true);
    if (!ended.accepted) return;
    assert.deepEqual(ended.receipt.events.map(({ type }) => type), [
      'minion-untapped',
      'turn-ended',
      'turn-started',
    ]);
    assert.deepEqual(ended.receipt.events[0]?.payload, {
      instanceId: malakhimInstanceId,
      seat: 'north',
      sourceInstanceId: malakhimInstanceId,
    });
    assert.equal(ended.receipt.events[0]?.type, 'minion-untapped');
    assert.deepEqual(ended.receipt.randomDraws, []);
    assert.deepEqual(ctx.state.realm.units
      .filter(({ instanceId }) => instanceId === malakhimInstanceId)
      .map(({ controller, damage, location, owner, region, tapped, warded }) => ({
        controller,
        damage,
        location,
        owner,
        region,
        tapped,
        warded,
      })), [{
      controller: 'north',
      damage: 0,
      location: 'C3',
      owner: 'north',
      region: 'surface',
      tapped: false,
      warded: true,
    }]);
    assert.equal(ctx.observe('south').realm.units
      .find(({ instanceId }) => instanceId === malakhimInstanceId)?.warded, true);

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2,C3');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === malakhimInstanceId);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Ignited dies before turn cleanup unless it is Disabled', async () => {
  const gameManifest = manifest(127, {
    spell: {
      attack: 3,
      charge: true,
      defense: 3,
      diesAtEndOfControllerTurn: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  });
  const ignitedCardId = gameManifest.decks.north.spellbook[0]!;
  assert.equal(gameManifest.cards[ignitedCardId]?.cardType === 'minion'
    && gameManifest.cards[ignitedCardId].diesAtEndOfControllerTurn, true);
  assert.throws(() => createGameManifest({
    authority: gameManifest.authority,
    cards: {
      ...gameManifest.cards,
      [ignitedCardId]: {
        ...gameManifest.cards[ignitedCardId],
        diesAtEndOfControllerTurn: false,
      } as unknown as GameCardDefinition,
    },
    decks: gameManifest.decks,
    firstSeat: gameManifest.firstSeat,
    seed: gameManifest.seed,
  }), /diesAtEndOfControllerTurn must be true when defined/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion');
    const ignited = ctx.state.realm.units[0];
    assert.ok(ignited);
    const ignitedInstanceId = ignited.instanceId;
    const versionBefore = ctx.state.stateVersion;

    const ended = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(ended.accepted, true);
    if (!ended.accepted) return;
    assert.deepEqual(ended.receipt.events.map(({ type }) => type), [
      'minion-died',
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === ignitedInstanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === ignitedInstanceId), true);
    assert.deepEqual({
      activeSeat: ctx.state.activeSeat,
      mana: ctx.state.players.south.mana,
      phase: ctx.state.phase,
      stateVersion: ctx.state.stateVersion,
    }, {
      activeSeat: 'south',
      mana: 0,
      phase: 'draw',
      stateVersion: versionBefore + 1,
    });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Sedge Crabs can move themselves only sideways', async () => {
  const crab = {
    attack: 3,
    defense: 3,
    manaCost: 1,
    movementBonus: 1 as const,
    movesOnlySideways: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  await withNorthAttacksAtC2(128, undefined, undefined, false, crab, 0, async ({
    ctx,
    defenderInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === defenderInstanceId), false);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(127, { spell: crab }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C3');
    const crabInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(crabInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === crabInstanceId);
    const paths = moves.map(({ descriptor }) => descriptor.kind === 'move-and-attack'
      ? descriptor.path.map(({ cell }) => cell).join(',')
      : '');
    assert.equal(paths.includes('C3'), true);
    assert.equal(paths.includes('C3,B3'), true);
    assert.equal(paths.includes('C3,B3,C3'), true);
    assert.equal(paths.includes('C3,C2'), false);
    assert.equal(paths.includes('C3,C4'), false);
    assert.equal(moves.every(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.path.every(({ cell }, index) =>
        index === 0 || cell[1] === descriptor.path[index - 1]!.cell[1])), true);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,B3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units[0]?.location, 'B3');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Dalcean Phalanx can move itself only forward for its seat', async () => {
  const phalanx = {
    attack: 5,
    connectsTopBottom: true,
    defense: 5,
    manaCost: 1,
    movementBonus: 1 as const,
    movesOnlyForward: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  await withNorthAttacksAtC2(141, undefined, undefined, false, phalanx, 0, async ({
    ctx,
    defenderInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === defenderInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(142, { spell: phalanx }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C3');
    const instanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(instanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const paths = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === instanceId
        ? [descriptor.path.map(({ cell }) => cell).join(',')]
        : []);
    assert.equal(paths.includes('C3'), true);
    assert.equal(paths.includes('C3,C2'), true);
    assert.equal(paths.includes('C3,C2,C1'), true);
    assert.equal(paths.includes('C3,C4'), false);
    assert.equal(paths.includes('C3,B3'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2,C1');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const edgePaths = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === instanceId
        ? [descriptor.path.map(({ cell }) => cell).join(',')]
        : []);
    assert.equal(edgePaths.includes('C1,C4'), true);
    assert.equal(edgePaths.includes('C1,C2'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units[0]?.location, 'C4');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02/04 a unit can move across connected top and bottom realm edges', async () => {
  await withSetup(manifest(128, {
    spell: {
      connectsTopBottom: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'));
    const unit = ctx.state.realm.units[0];
    assert.ok(unit);
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

    const actions = await ctx.legalActions('north');
    const wraps = ({ descriptor }: GameLegalAction): boolean => descriptor.kind === 'move-and-attack'
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C1';
    assert.equal(actions.some((candidate) =>
      wraps(candidate) && candidate.descriptor.kind === 'move-and-attack'
        && candidate.descriptor.unitInstanceId === unit.instanceId), true);
    assert.equal(actions.some((candidate) =>
      wraps(candidate) && candidate.descriptor.kind === 'move-and-attack'
        && candidate.descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId), false);
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unit.instanceId
      && descriptor.to.cell === 'C1'));
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), true);
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'decline-attack'));
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Submerge uses underwater summons, movement, and region-isolated combat', async () => {
  const submerge = {
    attack: 2,
    defense: 2,
    manaCost: 1,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
  } as const;
  const ordinary = { ...submerge, submerge: false };
  await withSetup(manifest(129, {
    northSpell: submerge,
    site: { elements: ['water'] },
    southSpell: ordinary,
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    const northSummons = await ctx.legalActions('north');
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === undefined), true);
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underwater');
    const northUnitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(northUnitId);
    assert.equal(ctx.observe('north').realm.units[0]?.region, 'underwater');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion');
    const southUnitId = ctx.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(southUnitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const underwaterMoves = await ctx.legalActions('north');
    const surfaces = underwaterMoves.find(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northUnitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C4/underwater,C4/surface');
    const swims = underwaterMoves.find(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northUnitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C4/underwater,C3/underwater');
    assert.ok(surfaces);
    assert.ok(swims);
    await withFork(ctx, async (surfaced) => {
      await surfaced.accept(surfaces);
      await surfaced.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      assert.equal(surfaced.state.realm.units.find(({ instanceId }) => instanceId === northUnitId)?.region, 'surface');
      assert.equal(await surfaced.verifyReplay(), true);
    });

    await ctx.accept(swims);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === southUnitId
        && descriptor.to.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northUnitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C3/underwater,C2/underwater');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === southUnitId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'main');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northUnitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C2/underwater,C2/surface');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === southUnitId), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(130, { northSpell: submerge }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Burrowing uses underground summons and movement only at land sites', async () => {
  const burrowing = {
    attack: 2,
    burrowing: true,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const ordinary = { ...burrowing, burrowing: false };
  await withSetup(manifest(131, { northSpell: burrowing, southSpell: ordinary }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    const northSummons = await ctx.legalActions('north');
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === undefined), true);
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underground');
    const northUnitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(northUnitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const moves = await ctx.legalActions('north');
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C3/underground'), true);
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C4/surface'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.to.cell === 'C3'
      && descriptor.to.region === 'underground');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(132, {
    northSpell: burrowing,
    site: { elements: ['water'] },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Secret Tunnel connects burrowed allies only to the controller\'s other sites', async () => {
  const decks = { north: deck('tunnel-north', 3), south: deck('tunnel-south', 3) };
  const cards = cardsFor(decks, {
    attack: 3,
    burrowing: true,
    defense: 3,
    manaCost: 1,
    movementBonus: 1,
    movesOnlyForward: true,
    submerge: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  const tunnelCardId = decks.north.atlas[0]!;
  const waterCardId = decks.north.atlas[2]!;
  cards[tunnelCardId] = {
    cardType: 'site',
    connectsBurrowedAllies: true,
    elements: ['earth'],
  };
  cards[waterCardId] = { cardType: 'site', elements: ['water'] };
  const tunnelManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-secret-tunnel-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 143,
  });
  await withSetup(tunnelManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const tunnel = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === tunnelCardId);
    const land = ctx.state.players.north.hand.atlas.find(({ cardId }) =>
      cardId !== tunnelCardId && cardId !== waterCardId);
    const water = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === waterCardId);
    assert.ok(tunnel);
    assert.ok(land);
    assert.ok(water);
    const tunnelInstanceId = tunnel.instanceId;
    const landInstanceId = land.instanceId;
    const waterInstanceId = water.instanceId;

    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === tunnelInstanceId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underground');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === landInstanceId && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === waterInstanceId && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const pathFor = (candidate: GameLegalAction): string => candidate.descriptor.kind === 'move-and-attack'
      ? candidate.descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      : '';
    const moves = await ctx.legalActions('north');
    const unitPaths = moves
      .filter(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unitId)
      .map(pathFor);
    assert.equal(unitPaths.filter((path) => path === 'C4/underground,C3/underground').length, 1);
    assert.equal(unitPaths.includes('C4/underground,C2/underwater'), true);
    assert.equal(unitPaths.includes('C4/underground,C1/underground'), false);
    assert.equal(unitPaths.includes('C4/underground,C2/underwater,C4/underground'), false);
    const avatarId = ctx.state.players.north.avatar.card.instanceId;
    const avatarPaths = moves
      .filter(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === avatarId)
      .map(pathFor);
    assert.equal(avatarPaths.includes('C4/surface,C3/surface'), true);
    assert.equal(avatarPaths.includes('C4/surface,C2/surface'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C2/underwater');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.deepEqual(ctx.state.realm.units[0]?.location, 'C2');
    assert.deepEqual(ctx.state.realm.units[0]?.region, 'underwater');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a must-be-burrowed cast restriction suppresses only non-underground casts', async () => {
  await withSetup(manifest(139, {
    northSpell: {
      attack: 3,
      burrowing: true,
      defense: 3,
      manaCost: 1,
      mustBeCastBurrowed: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await ctx.accept(await ctx.action(predicate));
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const summons = await ctx.legalActions('north');
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === undefined), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underground'), true);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underground');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C4/surface');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a must-be-submerged cast restriction suppresses only non-underwater casts', async () => {
  const decks = { north: deck('submerged-north'), south: deck('submerged-south') };
  const cards = cardsFor(decks, {
    attack: 3,
    burrowing: true,
    defense: 3,
    manaCost: 1,
    mustBeCastSubmerged: true,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
    voidwalk: true,
  });
  decks.north.atlas.forEach((cardId, index) => {
    cards[cardId] = { cardType: 'site', elements: [index % 2 === 0 ? 'water' : 'earth'] };
  });
  decks.south.atlas.forEach((cardId) => {
    cards[cardId] = { cardType: 'site', elements: ['water'] };
  });
  const submergedManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-submerged-only-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 0,
  });
  await withSetup(submergedManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const water = ctx.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const land = ctx.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && !definition.elements.includes('water');
    });
    assert.ok(water);
    assert.ok(land);
    const waterInstanceId = water.instanceId;
    const landInstanceId = land.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === waterInstanceId && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === landInstanceId && descriptor.cell === 'C3');
    const summons = await ctx.legalActions('north');
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.region === undefined), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C3' && descriptor.region === 'underground'), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.region === 'void'), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underwater'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underwater');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = await ctx.legalActions('north');
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.to.cell === 'C3' && descriptor.to.region === 'underground'), true);
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId && descriptor.to.region === 'void'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underwater,C4/surface');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 combined region abilities permit eligible cross-region adjacency steps', async () => {
  const decks = { north: deck('cross-north'), south: deck('cross-south') };
  const cards = cardsFor(decks, {
    burrowing: true,
    manaCost: 1,
    movementBonus: 1,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    voidwalk: true,
  });
  decks.north.atlas.forEach((cardId, index) => {
    cards[cardId] = { cardType: 'site', elements: [index % 2 === 0 ? 'earth' : 'water'] };
  });
  const crossManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-burrowing-crossover-v1',
    },
    cards,
    decks,
    firstSeat: 'north',
    seed: 133,
  });
  await withSetup(crossManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const north = ctx.state.players.north;
    const land = north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && !definition.elements.includes('water');
    });
    const water = north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    assert.ok(land);
    assert.ok(water);
    const landInstanceId = land.instanceId;
    const waterInstanceId = water.instanceId;

    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cardInstanceId === landInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underground');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === waterInstanceId
      && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C3/underwater');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C3/underwater,C4/underground,B4/void');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Voidwalk summons to any void and moves between adjacent void and surface locations', async () => {
  const voidwalk = {
    attack: 2,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    voidwalk: true,
  } as const;
  await withSetup(manifest(134, {
    northSpell: voidwalk,
    site: { elements: ['air'] },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    const summons = await ctx.legalActions('north');
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === undefined), true);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B4' && descriptor.region === 'void'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B4' && descriptor.region === 'void');
    const coveredUnitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(coveredUnitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'void'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === coveredUnitId)?.region, 'surface');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B3' && descriptor.region === 'void');
    const unitId = ctx.state.realm.units.find(({ region }) => region === 'void')?.instanceId;
    assert.ok(unitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = await ctx.legalActions('north');
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,A3/void'), true);
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,B4/surface'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.to.cell === 'B4' && descriptor.to.region === 'surface');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Planar Gate grants minions Voidwalk only until they leave the void', async () => {
  const baseNorth = deck('gate-north');
  const north = { ...baseNorth, atlas: baseNorth.atlas.map(() => 'planar-gate') };
  const withGateEverywhere = manifest(162, {
    north,
    northSpell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      movementBonus: 2,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    site: { minionsHereGainVoidwalkUntilLeavingVoid: true },
  });
  const southSiteIds = new Set(withGateEverywhere.decks.south.atlas);
  const cards = Object.fromEntries(Object.entries(withGateEverywhere.cards).map(([cardId, card]) => {
    if (card.cardType !== 'site' || !southSiteIds.has(cardId)) return [cardId, card];
    const ordinarySite = { ...card };
    delete ordinarySite.minionsHereGainVoidwalkUntilLeavingVoid;
    return [cardId, ordinarySite];
  })) as Record<string, GameCardDefinition>;
  const gameManifest = createGameManifest({ ...withGateEverywhere, cards });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/surface,B4/void');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === unitId)
      ?.planarGateVoidwalk, true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'B4/void,B3/void'), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B4/void,B3/void');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const exits = await ctx.legalActions('north');
    assert.equal(exits.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,C3/surface,D3/void'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,C3/surface');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === unitId)
      ?.planarGateVoidwalk, undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C3/surface,D3/void'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 an outer-column cast restriction filters surface and Voidwalk summons, not movement', async () => {
  await withSetup(manifest(135, {
    northSpell: {
      attack: 3,
      defense: 3,
      manaCost: 3,
      mustBeCastToOuterColumn: true,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
    },
    site: { elements: ['air'] },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A4');

    const summons = (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'summon-minion');
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'A4' && descriptor.region === undefined), true);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B4' && descriptor.region === undefined), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === undefined), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'E2' && descriptor.region === 'void'), true);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'D2' && descriptor.region === 'void'), false);
    assert.equal(summons.every(({ descriptor }) => descriptor.kind === 'summon-minion'
      && (descriptor.cell[0] === 'A' || descriptor.cell[0] === 'E')), true);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'E2' && descriptor.region === 'void');
    const unitId = ctx.state.realm.units.find(({ location }) => location === 'E2')?.instanceId;
    assert.ok(unitId);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'E2/void,D2/void');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Movement +1 issues exact two-step and returning Move and Attack paths', async () => {
  await withNorthAttacksAtC2(53, {
    attack: 2,
    defense: 2,
    manaCost: 1,
    movementBonus: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, undefined, 0, async ({ attackerInstanceId, ctx }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const actions = await ctx.legalActions('north');
    const twoStep = actions.find(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4');
    assert.ok(twoStep);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2'), true);
    const result = await ctx.step(twoStep);
    assert.equal(result.accepted, true);
    assert.equal(ctx.state.pendingCombat?.cell, 'C4');
    assert.match(canonicalJson(result.receipt.events[0]?.payload ?? null), /\"steps\":2/);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Movement +1 can take an exact two-step path to Defend', async () => {
  await withSetup(manifest(54, {
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      movementBonus: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const defenderInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(defenderInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const attackerInstanceId = ctx.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(attackerInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === defenderInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === defenderInstanceId)?.location, 'C2');
    assert.match(canonicalJson(ctx.session.transcript.at(-1)?.events[0]?.payload ?? null), /\"steps\":2/);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Movement +2 issues exact three-step paths and attacks after moving', async () => {
  await withNorthAttacksAtC2(125, {
    attack: 2,
    defense: 2,
    manaCost: 1,
    movementBonus: 2,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, undefined, 0, async ({
    attackerInstanceId,
    ctx,
    defenderInstanceId,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const actions = await ctx.legalActions('north');
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2'), true);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2,C3'), false);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4,C3'), true);
    const threeStep = actions.find(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2,C1');
    assert.ok(threeStep);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.length > 4), false);
    const result = await ctx.step(threeStep);
    assert.equal(result.accepted, true);
    assert.match(canonicalJson(result.receipt.events[0]?.payload ?? null), /\"steps\":3/);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === defenderInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-05 AP commits before NAP, then NAP Deathrites resolve before AP Deathrites', async () => {
  const decks = {
    north: deck('ap-nap-north', 5, 6),
    south: deck('ap-nap-south', 5, 6),
  };
  const baseCards = cardsFor(decks, {
    attack: 1,
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['earth'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-ap-nap-deathrites-v1',
  };
  const seed = 281;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const apCardIds = preview.state.players.north.hand.spellbook.slice(0, 2)
      .map(({ cardId }) => cardId);
    const genesisCardId = preview.state.players.north.hand.spellbook[2]?.cardId;
    const napCardIds = preview.state.players.south.hand.spellbook.slice(0, 2)
      .map(({ cardId }) => cardId);
    assert.equal(apCardIds.length, 2);
    assert.equal(napCardIds.length, 2);
    assert.ok(genesisCardId);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    for (const cardId of [...apCardIds, ...napCardIds]) {
      cards[cardId] = {
        ...baseCards[cardId]!,
        deathriteDrawSite: true,
        summonToAnySite: true,
      } as GameCardDefinition;
    }
    cards[genesisCardId] = {
      ...baseCards[genesisCardId]!,
      defense: 5,
      genesisDamageEachOtherUnitHere: 1,
      summonToAnySite: true,
    } as GameCardDefinition;
    await withSetup(createGameManifest({
      authority,
      cards,
      decks,
      firstSeat: 'north',
      seed,
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      for (const cardId of apCardIds) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === cardId && descriptor.cell === 'C4');
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      for (const cardId of napCardIds) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === cardId && descriptor.cell === 'C4');
      }
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

      const apInstanceIds = ctx.state.realm.units
        .filter(({ cardId }) => apCardIds.includes(cardId))
        .map(({ instanceId }) => instanceId)
        .sort();
      const napInstanceIds = ctx.state.realm.units
        .filter(({ cardId }) => napCardIds.includes(cardId))
        .map(({ instanceId }) => instanceId)
        .sort();
      const genesis = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === genesisCardId);
      assert.equal(apInstanceIds.length, 2);
      assert.equal(napInstanceIds.length, 2);
      assert.ok(genesis);
      const genesisInstanceId = genesis.instanceId;
      const triggered = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === genesisInstanceId
          && descriptor.cell === 'C4'));
      assert.equal(triggered.accepted, true);
      if (!triggered.accepted) return;
      assert.equal(ctx.state.phase, 'deathrite-order');
      assert.equal(ctx.state.decisionSeat, 'north');
      assert.deepEqual(await ctx.legalActions('south'), []);
      const apActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.deepEqual(apActions.flatMap(({ descriptor }) =>
        descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), apInstanceIds);
      assert.equal(triggered.receipt.events.some(({ type }) =>
        type === 'site-drawn' || type === 'minion-died'), false);

      const apFirst = apActions[0]!;
      assert.equal(apFirst.descriptor.kind, 'order-deathrites');
      if (apFirst.descriptor.kind !== 'order-deathrites') return;
      const apFirstInstanceId = apFirst.descriptor.sourceInstanceId;
      const apCommittedOrder = [
        apFirstInstanceId,
        apInstanceIds.find((instanceId) => instanceId !== apFirstInstanceId)!,
      ];
      const apCommitted = await ctx.step(apFirst);
      assert.equal(apCommitted.accepted, true);
      if (!apCommitted.accepted) return;
      assert.equal(ctx.state.phase, 'deathrite-order');
      assert.equal(ctx.state.decisionSeat, 'south');
      assert.deepEqual(apCommitted.receipt.events.map(({ type }) => type), ['deathrite-order-committed']);

      const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
        ctx.checkpoint(),
      )));
      assert.equal(
        canonicalJson(restored as unknown as JsonValue),
        canonicalJson(ctx.session as unknown as JsonValue),
      );
      const napActions = (await ctx.legalActions('south')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.deepEqual(napActions.flatMap(({ descriptor }) =>
        descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), napInstanceIds);
      const napFirst = napActions[0]!;
      assert.equal(napFirst.descriptor.kind, 'order-deathrites');
      if (napFirst.descriptor.kind !== 'order-deathrites') return;
      const napFirstInstanceId = napFirst.descriptor.sourceInstanceId;
      const napCommittedOrder = [
        napFirstInstanceId,
        napInstanceIds.find((instanceId) => instanceId !== napFirstInstanceId)!,
      ];
      const resolved = await ctx.step(napFirst);
      assert.equal(resolved.accepted, true);
      if (!resolved.accepted) return;
      const events = resolved.receipt.events;
      assert.deepEqual(events.filter(({ type }) => type === 'site-drawn').map(({ payload }) =>
        payload !== null && typeof payload === 'object' && 'sourceInstanceId' in payload
          ? payload.sourceInstanceId
          : undefined), [...napCommittedOrder, ...apCommittedOrder]);
      assert.ok(events.findIndex(({ type }) => type === 'minion-died')
        > events.findLastIndex(({ type }) => type === 'site-drawn'));
      assert.equal(events.filter(({ type }) => type === 'minion-died').length, 4);
      assert.equal(await ctx.verifyReplay(), true);
    });
  });
});

test('RULE-05 NAP then AP Deathrites resolve before simultaneous deaths enter their cemeteries', async () => {
  const spell = {
    attack: 1,
    deathriteDrawSite: true,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  await withNorthAttacksAtC2(48, spell, undefined, false, undefined, 0, async ({
    attackerInstanceId,
    ctx,
    targetInstanceId,
  }) => {
    const before = ctx.state.players;
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);

    for (const seat of ['north', 'south'] as const) {
      assert.equal(ctx.state.players[seat].atlas.length, before[seat].atlas.length - 1);
      assert.equal(ctx.state.players[seat].hand.atlas.length, before[seat].hand.atlas.length + 1);
    }
    const events = ctx.session.transcript.at(-1)?.events ?? [];
    const firstCemeteryEvent = events.findIndex(({ type }) => type === 'minion-died');
    const draws = events.filter(({ type }) => type === 'site-drawn');
    assert.ok(firstCemeteryEvent > 0);
    assert.equal(events.slice(0, firstCemeteryEvent).filter(({ type }) => type === 'site-drawn').length, 2);
    assert.deepEqual(draws.map(({ payload }) => payload), [
      { seat: 'south', sourceInstanceId: targetInstanceId },
      { seat: 'north', sourceInstanceId: attackerInstanceId },
    ]);
    assert.equal(ctx.state.players.north.cemetery.length, 1);
    assert.equal(ctx.state.players.south.cemetery.length, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2(49, spell, undefined, true, undefined, 0, async ({
    ctx,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.deepEqual(ctx.state.terminal, {
      reason: 'simultaneous_defeat',
      result: 'draw',
      status: 'finished',
    });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-05 Bladderblimp Deathrite makes each player lose life for their nearby sites', async () => {
  const blimp = {
    airborne: true,
    attack: 1,
    deathriteLoseLifePerNearbySiteControlled: 1,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const ordinary = {
    attack: 1,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const validation = manifest(146, { northSpell: blimp, southSpell: ordinary });
  const blimpCardId = validation.decks.north.spellbook[0]!;
  assert.equal(validation.cards[blimpCardId]?.cardType === 'minion'
    && validation.cards[blimpCardId].deathriteLoseLifePerNearbySiteControlled, 1);
  assert.throws(() => createGameManifest({
    authority: validation.authority,
    cards: {
      ...validation.cards,
      [blimpCardId]: {
        ...validation.cards[blimpCardId]!,
        deathriteLoseLifePerNearbySiteControlled: 2,
      } as unknown as GameCardDefinition,
    },
    decks: validation.decks,
    firstSeat: validation.firstSeat,
    seed: validation.seed,
  }), /deathriteLoseLifePerNearbySiteControlled must be 1/);

  await withNorthAttacksAtC2(
    147,
    blimp,
    { attack: 1, defense: 1, drawSpell: false, life: 2 },
    false,
    ordinary,
    0,
    async ({ attackerInstanceId, ctx, targetInstanceId }) => {
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === targetInstanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      assert.equal(ctx.state.players.north.avatar.life, 1);
      assert.equal(ctx.state.players.south.avatar.life, 0);
      assert.deepEqual(ctx.state.terminal, { status: 'active' });
      const events = ctx.session.transcript.at(-1)?.events ?? [];
      const lifeEvents = events.filter(({ type }) => type === 'avatar-life-lost');
      assert.deepEqual(lifeEvents.map(({ payload }) => payload), [
        {
          amount: 1,
          life: 1,
          seat: 'north',
          sourceInstanceId: attackerInstanceId,
        },
        {
          amount: 2,
          life: 0,
          seat: 'south',
          sourceInstanceId: attackerInstanceId,
        },
      ]);
      const deathsDoor = events.find(({ type }) => type === 'avatar-reached-deaths-door');
      assert.deepEqual(deathsDoor?.payload, {
        seat: 'south',
        sourceInstanceId: attackerInstanceId,
        turnNumber: ctx.state.turnNumber,
      });
      const firstDeath = events.findIndex(({ type }) => type === 'minion-died');
      assert.ok(firstDeath > events.findIndex(({ type }) => type === 'avatar-reached-deaths-door'));
      assert.equal(events.some(({ type }) => type === 'game-ended'), false);
      assert.equal(await ctx.verifyReplay(), true);
    },
  );
});

test('RULE-05 Deathrite damages each other remaining unit here in simultaneous chained batches', async () => {
  const decks = {
    north: deck('deathrite-damage-north', 5, 6),
    south: deck('deathrite-damage-south', 5, 6),
  };
  const baseCards = cardsFor(decks, {
    attack: 1,
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  }, undefined, { elements: ['earth'] });
  const authority = {
    contentHash: SYNTHETIC_AUTHORITY_HASH,
    mode: 'synthetic' as const,
    revisionId: 'synthetic-deathrite-area-damage-v1',
  };
  const seed = 263;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (preview) => {
    const sourceCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    const scarabCardIds = preview.state.players.south.hand.spellbook
      .slice(0, 2)
      .map(({ cardId }) => cardId);
    const chainedCardId = preview.state.players.south.hand.spellbook[2]?.cardId;
    assert.ok(sourceCardId);
    assert.equal(scarabCardIds.length, 2);
    assert.ok(chainedCardId);

    const cards: Record<string, GameCardDefinition> = { ...baseCards };
    cards[sourceCardId] = {
      ...baseCards[sourceCardId]!,
      attack: 1,
      defense: 4,
      genesisDamageEachOtherUnitHere: 1,
      summonToAnySite: true,
    } as GameCardDefinition;
    for (const cardId of [...scarabCardIds, chainedCardId]) {
      cards[cardId] = {
        ...baseCards[cardId]!,
        deathriteDamageEachUnitHere: 1,
        defense: cardId === chainedCardId ? 2 : 1,
        summonToAnySite: true,
      } as unknown as GameCardDefinition;
    }
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [scarabCardIds[0]!]: {
          ...cards[scarabCardIds[0]!]!,
          deathriteDamageEachUnitHere: 0,
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /deathriteDamageEachUnitHere must be a safe integer between 1 and/);
    assert.throws(() => createGameManifest({
      authority,
      cards: {
        ...cards,
        [scarabCardIds[0]!]: {
          ...cards[scarabCardIds[0]!]!,
          occupiesSquareArea: 2,
        } as unknown as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    }), /occupiesSquareArea has an unsupported ability combination/);
    const threeDamageManifest = createGameManifest({
      authority,
      cards: {
        ...cards,
        [scarabCardIds[0]!]: {
          ...cards[scarabCardIds[0]!]!,
          deathriteDamageEachUnitHere: 3,
        } as GameCardDefinition,
      },
      decks,
      firstSeat: 'north',
      seed,
    });
    const threeDamageDefinition = threeDamageManifest.cards[scarabCardIds[0]!];
    assert.equal(
      threeDamageDefinition?.cardType === 'minion'
        && threeDamageDefinition.deathriteDamageEachUnitHere,
      3,
    );

    await withSetup(createGameManifest({
      authority,
      cards,
      decks,
      firstSeat: 'north',
      seed,
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      for (const cardId of [...scarabCardIds, chainedCardId]) {
        await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === cardId
          && descriptor.cell === 'C1');
      }
      const southUnits = [...ctx.state.realm.units];
      const scarabInstanceIds = southUnits
        .filter(({ cardId }) => scarabCardIds.includes(cardId))
        .map(({ instanceId }) => instanceId)
        .sort();
      const chainedInstanceId = southUnits.find(({ cardId }) => cardId === chainedCardId)?.instanceId;
      assert.equal(scarabInstanceIds.length, 2);
      assert.ok(chainedInstanceId);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

      const sourceInstanceId = ctx.state.players.north.hand.spellbook
        .find(({ cardId }) => cardId === sourceCardId)?.instanceId;
      assert.ok(sourceInstanceId);
      const result = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === sourceInstanceId
          && descriptor.cell === 'C1'));
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      assert.equal(ctx.state.phase, 'deathrite-order');
      assert.equal(ctx.state.decisionSeat, 'south');
      assert.deepEqual(await ctx.legalActions('north'), []);
      const orderActions = (await ctx.legalActions('south')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
        descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), scarabInstanceIds);
      assert.equal(scarabInstanceIds.every((instanceId) => !ctx.state.players.south.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === chainedInstanceId), true);
      assert.equal(result.receipt.events.some(({ type }) => type === 'minion-died'), false);
      assert.equal(result.receipt.events.some(({ type }) => type === 'deathrite-damage-allocated'), false);

      const beforeOrder = canonicalJson(ctx.state);
      const forged = await ctx.stepRequest({
        actionId: opaqueActionId('sorcery-core-v1', 'south', ctx.state.stateVersion, {
          kind: 'order-deathrites',
          sourceInstanceId,
        }),
        seat: 'south',
        stateVersion: ctx.state.stateVersion,
      });
      assert.equal(forged.accepted, false);
      if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
      assert.equal(canonicalJson(forged.session.state), beforeOrder);

      const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
        ctx.checkpoint(),
      )));
      assert.equal(
        canonicalJson(restored as unknown as JsonValue),
        canonicalJson(ctx.session as unknown as JsonValue),
      );
      await withFork(ctx, async (restoredFork) => {
        assert.deepEqual(
          (await restoredFork.legalActions('south')).map(({ actionId }) => actionId),
          orderActions.map(({ actionId }) => actionId),
        );
      });

      const southAvatarId = ctx.state.players.south.avatar.card.instanceId;
      const northAvatarId = ctx.state.players.north.avatar.card.instanceId;
      const branchHashes: string[] = [];
      for (const orderedFirst of orderActions) {
        assert.equal(orderedFirst.descriptor.kind, 'order-deathrites');
        if (orderedFirst.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
        const chosenInstanceId = orderedFirst.descriptor.sourceInstanceId;
        const otherInstanceId = scarabInstanceIds.find((instanceId) => instanceId !== chosenInstanceId);
        assert.ok(otherInstanceId);
        await withFork(ctx, async (fork) => {
          const ordered = await fork.step(orderedFirst);
          assert.equal(ordered.accepted, true);
          if (!ordered.accepted) throw new Error('unreachable');

          const events = ordered.receipt.events;
          const allocations = events.filter(({ type }) => type === 'deathrite-damage-allocated');
          assert.equal(allocations.length, 7);
          const sourceOrder = allocations
            .map(({ payload }) => payload !== null && typeof payload === 'object'
              && 'sourceInstanceId' in payload ? payload.sourceInstanceId : undefined)
            .filter((source, index, sources) => source !== undefined && sources.indexOf(source) === index);
          assert.deepEqual(sourceOrder, [chosenInstanceId, chainedInstanceId, otherInstanceId]);
          for (const [source, targets] of [
            [chosenInstanceId, [sourceInstanceId, southAvatarId, chainedInstanceId].sort()],
            [chainedInstanceId, [sourceInstanceId, southAvatarId].sort()],
            [otherInstanceId, [sourceInstanceId, southAvatarId].sort()],
          ] as const) {
            const expectedTargets = [...targets].sort();
            assert.deepEqual(allocations.flatMap(({ payload }) =>
              payload !== null
                && typeof payload === 'object'
                && 'sourceInstanceId' in payload
                && payload.sourceInstanceId === source
                && 'targetInstanceId' in payload
                && typeof payload.targetInstanceId === 'string'
                ? [payload.targetInstanceId]
                : []), expectedTargets);
            const allocationIndexes = events.flatMap((event, index) =>
              event.type === 'deathrite-damage-allocated'
                && event.payload !== null
                && typeof event.payload === 'object'
                && 'sourceInstanceId' in event.payload
                && event.payload.sourceInstanceId === source
                ? [index]
                : []);
            assert.deepEqual(
              allocationIndexes,
              Array.from(
                { length: expectedTargets.length },
                (_, index) => allocationIndexes[0]! + index,
              ),
            );
            assert.equal(events[allocationIndexes.at(-1)! + 1]?.type, 'damage-dealt');
          }
          assert.ok(events.findIndex(({ type }) => type === 'minion-died')
            > events.findLastIndex(({ type }) => type === 'deathrite-damage-allocated'));
          assert.equal(events.filter(({ type }) => type === 'minion-died').length, 3);
          assert.equal(allocations.some(({ payload }) =>
            payload !== null
              && typeof payload === 'object'
              && 'targetInstanceId' in payload
              && payload.targetInstanceId === northAvatarId), false);
          assert.equal(events.some(({ type }) =>
            type === 'fight-started' || type === 'strike-damage-allocated'), false);
          assert.equal(fork.state.players.north.avatar.life, 20);
          assert.equal(fork.state.players.south.avatar.life, 16);
          assert.equal(fork.state.realm.units.length, 1);
          assert.deepEqual(fork.state.realm.units[0] && {
            damage: fork.state.realm.units[0].damage,
            instanceId: fork.state.realm.units[0].instanceId,
            location: fork.state.realm.units[0].location,
          }, { damage: 3, instanceId: sourceInstanceId, location: 'C1' });
          assert.deepEqual(
            fork.state.players.south.cemetery.map(({ instanceId }) => instanceId).sort(),
            [...scarabInstanceIds, chainedInstanceId].sort(),
          );
          assert.equal(ordered.receipt.randomDraws.length, 0);
          assert.equal(await fork.verifyReplay(), true);

          const beforeStale = canonicalJson(fork.state);
          const stale = await fork.step(orderedFirst);
          assert.equal(stale.accepted, false);
          if (!stale.accepted) assert.equal(stale.reason.code, 'stale_version');
          assert.equal(canonicalJson(stale.session.state), beforeStale);
          branchHashes.push(hashGameState(fork.state));
        });
      }
      assert.equal(branchHashes[0], branchHashes[1]);
    });
  });
});

test('RULE-05 Deathrite uses its moved last location with Ward, reduction, and Lethal', async () => {
  const deathrite = {
    attack: 0,
    deathriteDamageEachUnitHere: 2,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const resolve = async (
    seed: number,
    source: SpellFacts,
    target: SpellFacts,
    run: (setup: Readonly<{
      attackerInstanceId: string;
      ctx: SetupCtx;
      defenderInstanceId: string;
      targetInstanceId: string;
    }>) => Promise<void>,
  ): Promise<void> => {
    await withNorthAttacksAtC2(seed, source, undefined, false, target, 0, async ({
      attackerInstanceId,
      ctx,
      defenderInstanceId,
      targetInstanceId,
    }) => {
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === targetInstanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      await run({ attackerInstanceId, ctx, defenderInstanceId, targetInstanceId });
    });
  };

  await resolve(266, deathrite, {
    attack: 2,
    defense: 10,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    ward: true,
  }, async ({ attackerInstanceId, ctx, defenderInstanceId, targetInstanceId }) => {
    const wardedEvents = ctx.session.transcript.at(-1)?.events ?? [];
    const wardedAllocationIndex = wardedEvents.findIndex(({ type }) =>
      type === 'deathrite-damage-allocated');
    assert.deepEqual(wardedEvents[wardedAllocationIndex]?.payload, {
      amount: 2,
      sourceInstanceId: attackerInstanceId,
      targetInstanceId,
    });
    assert.deepEqual(wardedEvents.slice(wardedAllocationIndex + 1).find(({ payload, type }) =>
      type === 'damage-dealt'
        && canonicalJson(payload).includes(targetInstanceId))?.payload, {
      amount: 0,
      attemptedAmount: 2,
      direct: true,
      instanceId: targetInstanceId,
      prevented: true,
      seat: 'south',
    });
    assert.ok(wardedEvents.findIndex(({ type }) => type === 'ward-broken') > wardedAllocationIndex);
    const sourceMoves = ctx.session.transcript.flatMap(({ events }) =>
      events.filter(({ payload, type }) =>
        type === 'move-and-attack-activated'
          && canonicalJson(payload).includes(attackerInstanceId)));
    assert.equal(sourceMoves.length, 2);
    assert.equal(canonicalJson(sourceMoves.at(-1)!.payload).includes('"to":{"cell":"C2"'), true);
    const wardedTarget = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === targetInstanceId);
    const distantWarded = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === defenderInstanceId);
    assert.deepEqual(wardedTarget && {
      damage: wardedTarget.damage,
      location: wardedTarget.location,
      warded: wardedTarget.warded,
    }, { damage: 0, location: 'C2', warded: false });
    assert.deepEqual(distantWarded && {
      damage: distantWarded.damage,
      location: distantWarded.location,
      warded: distantWarded.warded,
    }, { damage: 0, location: 'C1', warded: true });
    assert.equal(await ctx.verifyReplay(), true);
  });

  await resolve(267, { ...deathrite, lethal: true }, {
    attack: 2,
    defense: 10,
    manaCost: 1,
    takesLessDamage: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, async ({ attackerInstanceId, ctx, defenderInstanceId, targetInstanceId }) => {
    const lethalEvents = ctx.session.transcript.at(-1)?.events ?? [];
    const lethalAllocationIndex = lethalEvents.findIndex(({ type }) =>
      type === 'deathrite-damage-allocated');
    assert.deepEqual(lethalEvents[lethalAllocationIndex]?.payload, {
      amount: 2,
      sourceInstanceId: attackerInstanceId,
      targetInstanceId,
    });
    assert.deepEqual(lethalEvents.slice(lethalAllocationIndex + 1).find(({ payload, type }) =>
      type === 'damage-dealt'
        && canonicalJson(payload).includes(targetInstanceId))?.payload, {
      accumulated: 1,
      amount: 1,
      attemptedAmount: 2,
      direct: true,
      instanceId: targetInstanceId,
      prevented: true,
      seat: 'south',
    });
    assert.ok(lethalEvents.findIndex(({ payload, type }) =>
      type === 'minion-died' && canonicalJson(payload).includes(targetInstanceId))
        > lethalAllocationIndex);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === targetInstanceId), true);
    const distantReduced = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === defenderInstanceId);
    assert.deepEqual(distantReduced && {
      damage: distantReduced.damage,
      location: distantReduced.location,
    }, { damage: 0, location: 'C1' });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-05 Deathrite preserves its unit-source power snapshot before cemetery entry', async () => {
  await withNorthAttacksAtC2(171, {
    attack: 4,
    deathriteDamageEachUnitHere: 1,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, {
    attack: 1,
    defense: 10,
    manaCost: 1,
    preventsDamageFromUnitsWithPowerAtLeast: 4,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, 0, async ({ attackerInstanceId, ctx, targetInstanceId }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    if (!fought.accepted) return;
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === attackerInstanceId), true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === targetInstanceId)?.damage, 0);
    assert.deepEqual(fought.receipt.events.filter(({ payload, type }) =>
      type === 'damage-dealt'
        && canonicalJson(payload).includes(targetInstanceId)).map(({ payload }) => payload), [{
      accumulated: 0,
      amount: 0,
      attemptedAmount: 4,
      direct: true,
      instanceId: targetInstanceId,
      prevented: true,
      seat: 'south',
    }, {
      accumulated: 0,
      amount: 0,
      attemptedAmount: 1,
      direct: true,
      instanceId: targetInstanceId,
      prevented: true,
      seat: 'south',
    }]);
    assert.equal(fought.receipt.randomDraws.length, 0);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-05 Deathrite healing caps at maximum, fails at Death\'s Door, and precedes cemetery entry', async () => {
  const resolveHealingFight = async (
    life: number,
    seed: number,
    run: (ctx: SetupCtx) => Promise<void>,
  ): Promise<void> => {
    await withNorthAttacksAtC2(seed, {
      attack: 1,
      deathriteHeal: 3,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    }, { attack: 1, defense: 1, drawSpell: false, life }, false, undefined, 0, async ({
      attackerInstanceId,
      ctx,
      defenderInstanceId,
    }) => {
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
      assert.equal(ctx.state.players.south.avatar.life, life - 1);
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === defenderInstanceId
          && descriptor.to.cell === 'C2');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === attackerInstanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      await run(ctx);
    });
  };

  await resolveHealingFight(5, 56, async (ctx) => {
    assert.equal(ctx.state.players.south.avatar.life, 5);
    const cappedEvents = ctx.session.transcript.at(-1)?.events ?? [];
    assert.equal(cappedEvents.filter(({ type }) => type === 'avatar-healed').some(({ payload }) =>
      canonicalJson(payload).includes('"amount":1')
        && canonicalJson(payload).includes('"seat":"south"')), true);
    assert.ok(Math.max(...cappedEvents.map(({ type }, index) => type === 'avatar-healed' ? index : -1))
      < cappedEvents.findIndex(({ type }) => type === 'minion-died'));
    assert.equal(await ctx.verifyReplay(), true);
  });

  await resolveHealingFight(1, 57, async (ctx) => {
    assert.equal(ctx.state.players.south.avatar.life, 0);
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) =>
      type === 'avatar-healed'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Move and Attack stages movement before an undefended enemy-site strike', async () => {
  await withNorthAttacksAtC2(47, undefined, undefined, false, undefined, 0, async ({
    attackerInstanceId,
    ctx,
  }) => {
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal(ctx.state.decisionSeat, 'north');
    assert.equal(ctx.state.phase, 'attack');
    const attacker = ctx.state.realm.units.find(({ instanceId }) => instanceId === attackerInstanceId);
    assert.deepEqual({
      location: attacker?.location,
      region: attacker?.region,
      tapped: attacker?.tapped,
    }, { location: 'C2', region: 'surface', tapped: true });

    const site = ctx.state.realm.sites.C2;
    assert.ok(site);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === site.instanceId);
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal(ctx.state.decisionSeat, 'south');
    assert.equal(ctx.state.phase, 'defend');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.decisionSeat, 'north');
    assert.equal(ctx.state.players.south.avatar.life, 19);
    assert.equal(ctx.state.players.south.avatar.deathDoorTurn, null);
    assert.equal(ctx.state.realm.units.length, 3);
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['defend-window-closed', 'undefended-site-struck', 'avatar-life-lost'],
    );
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Defend moves a unit into a simultaneous fight and stages exact split damage', async () => {
  await withNorthAttacksAtC2(53, undefined, undefined, false, undefined, 0, async ({
    attackerInstanceId,
    ctx,
    defenderInstanceId,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === defenderInstanceId);
    const movedDefender = ctx.state.realm.units.find(({ instanceId }) => instanceId === defenderInstanceId);
    assert.deepEqual({ location: movedDefender?.location, tapped: movedDefender?.tapped }, {
      location: 'C2',
      tapped: true,
    });

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(ctx.state.phase, 'allocate');
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal(ctx.state.decisionSeat, 'north');
    const firstAllocation = (await ctx.legalActions('north'))
      .find(({ descriptor }) => descriptor.kind === 'allocate-strike' && descriptor.amount === 0);
    assert.ok(firstAllocation);
    await ctx.accept(firstAllocation);
    const finalAllocation = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'allocate-strike' && descriptor.amount === 1);
    const damagedInstanceId = finalAllocation.descriptor.kind === 'allocate-strike'
      ? finalAllocation.descriptor.targetInstanceId
      : '';
    await ctx.accept(finalAllocation);

    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === attackerInstanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === damagedInstanceId), false);
    assert.equal(ctx.state.realm.units.filter(({ controller }) => controller === 'south').length, 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) => instanceId === attackerInstanceId), true);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) => instanceId === damagedInstanceId), true);
    assert.equal(ctx.session.transcript.flatMap(({ events }) => events).filter(({ type }) => type === 'minion-died').length, 2);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 takes-less prevention applies to each simultaneous damage source before Lethal', async () => {
  for (const scenario of [
    { attack: 2, defense: 2, enemyPower: 1, expectedDamage: 0, lethal: true, seed: 142 },
    { attack: 4, defense: 3, enemyPower: 2, expectedDamage: 2, lethal: false, seed: 143 },
  ] as const) {
    await withNorthAttacksAtC2(
      scenario.seed,
      {
        attack: scenario.attack,
        defense: scenario.defense,
        manaCost: 1,
        takesLessDamage: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
      undefined,
      false,
      {
        attack: scenario.enemyPower,
        defense: scenario.enemyPower,
        lethal: scenario.lethal,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
      0,
      async ({ attackerInstanceId, ctx, defenderInstanceId, targetInstanceId }) => {
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'declare-attack'
            && descriptor.target.kind === 'minion'
            && descriptor.target.instanceId === targetInstanceId);
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'defend'
            && descriptor.unitInstanceId === defenderInstanceId);
        await ctx.take(({ descriptor }) =>
          descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
        const allocated = await ctx.step(await ctx.action(({ descriptor }) =>
          descriptor.kind === 'allocate-strike' && descriptor.amount === scenario.enemyPower));
        assert.equal(allocated.accepted, true);
        const fought = await ctx.step(await ctx.action(({ descriptor }) =>
          descriptor.kind === 'allocate-strike' && descriptor.amount === scenario.enemyPower));
        assert.equal(fought.accepted, true);

        assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
          instanceId === attackerInstanceId)?.damage, scenario.expectedDamage);
        assert.equal(ctx.state.realm.units.filter(({ controller }) => controller === 'south').length, 0);
        assert.deepEqual(fought.receipt.events.find(({ payload, type }) =>
          type === 'damage-dealt'
            && canonicalJson(payload).includes(attackerInstanceId))?.payload, {
          accumulated: scenario.expectedDamage,
          amount: scenario.expectedDamage,
          attemptedAmount: scenario.enemyPower * 2,
          direct: true,
          instanceId: attackerInstanceId,
          prevented: true,
          seat: 'north',
        });
        assert.equal(fought.receipt.randomDraws.length, 0);
        assert.equal(await ctx.verifyReplay(), true);
      },
    );
  }
});

test('RULE-04 declining an attack gives only co-located ready enemies an Intercept window', async () => {
  await withNorthAttacksAtC2(59, undefined, undefined, false, undefined, 0, async ({
    attackerInstanceId,
    ctx,
    defenderInstanceId,
    targetInstanceId,
  }) => {
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'intercept');
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal(ctx.state.decisionSeat, 'south');
    const interceptors = (await ctx.legalActions('south'))
      .filter(({ descriptor }) => descriptor.kind === 'intercept');
    assert.deepEqual(
      interceptors.map(({ descriptor }) => descriptor.kind === 'intercept' && descriptor.unitInstanceId),
      [targetInstanceId],
    );
    assert.equal(interceptors.some(({ descriptor }) =>
      descriptor.kind === 'intercept' && descriptor.unitInstanceId === defenderInstanceId), false);

    await ctx.accept(interceptors[0]!);
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-intercept');
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === attackerInstanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === targetInstanceId), false);
    const eventTypes = ctx.session.transcript.flatMap(({ events }) => events.map(({ type }) => type));
    assert.equal(eventTypes.includes('attack-declared'), false);
    assert.equal(eventTypes.includes('interceptor-joined'), true);
    assert.equal(eventTypes.includes('fight-started'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 surviving minion damage persists through the turn and clears in End Phase', async () => {
  await withNorthAttacksAtC2(61, {
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, undefined, false, undefined, 0, async ({ ctx, targetInstanceId }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.deepEqual(ctx.state.realm.units
      .filter(({ location }) => location === 'C2')
      .map(({ damage }) => damage), [1, 1]);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units.every(({ damage }) => damage === 0), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

async function withNorthAvatarAttacksSouthAtC2(
  seed: number,
  run: (setup: Readonly<{
    ctx: SetupCtx;
    northAvatarInstanceId: string;
    northMinionInstanceId: string;
    southAvatarInstanceId: string;
  }>) => Promise<void>,
): Promise<void> {
  await withSetup(manifest(seed, {
    avatar: { attack: 2, defense: 1, drawSpell: false, life: 1 },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const northAvatarInstanceId = ctx.state.players.north.avatar.card.instanceId;
    const southAvatarInstanceId = ctx.state.players.south.avatar.card.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const northMinionInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(northMinionInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northMinionInstanceId
        && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northMinionInstanceId
        && descriptor.to.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northAvatarInstanceId
        && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === southAvatarInstanceId
        && descriptor.to.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northAvatarInstanceId
        && descriptor.to.cell === 'C2');
    await run({
      ctx,
      northAvatarInstanceId,
      northMinionInstanceId,
      southAvatarInstanceId,
    });
  });
}

test("RULE-04 Death's Door prevents same-turn direct damage and later simultaneous death blows draw", async () => {
  await withNorthAvatarAttacksSouthAtC2(67, async ({
    ctx,
    northAvatarInstanceId,
    northMinionInstanceId,
    southAvatarInstanceId,
  }) => {
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'avatar'
        && descriptor.target.instanceId === southAvatarInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(ctx.state.players.north.avatar.life, 0);
    assert.equal(ctx.state.players.south.avatar.life, 0);
    assert.equal(ctx.state.players.north.avatar.deathDoorTurn, 7);
    assert.equal(ctx.state.players.south.avatar.deathDoorTurn, 7);
    assert.deepEqual(ctx.state.terminal, { status: 'active' });
    const firstFightEvents = ctx.session.transcript.at(-1)?.events ?? [];
    assert.deepEqual(
      firstFightEvents.filter(({ type }) => type === 'damage-dealt').map(({ payload }) => payload),
      [
        { amount: 2, direct: true, instanceId: northAvatarInstanceId, seat: 'north' },
        { amount: 2, direct: true, instanceId: southAvatarInstanceId, seat: 'south' },
      ],
    );
    assert.deepEqual(
      firstFightEvents.filter(({ type }) => type === 'avatar-life-lost').map(({ payload }) => payload),
      [
        { amount: 1, life: 0, seat: 'north' },
        { amount: 1, life: 0, seat: 'south' },
      ],
    );

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northMinionInstanceId
        && descriptor.to.cell === 'C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'avatar'
        && descriptor.target.instanceId === southAvatarInstanceId);
    const immunityResult = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(immunityResult.accepted, true);
    assert.deepEqual(ctx.state.terminal, { status: 'active' });
    assert.equal(ctx.state.players.south.avatar.life, 0);
    assert.equal(immunityResult.receipt.events.some(({ payload, type }) =>
      type === 'damage-dealt'
        && typeof payload === 'object'
        && payload !== null
        && !Array.isArray(payload)
        && 'prevented' in payload
        && payload.prevented === true), true);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === southAvatarInstanceId
        && descriptor.to.cell === 'C2');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'avatar'
        && descriptor.target.instanceId === northAvatarInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.deepEqual(ctx.state.terminal, {
      reason: 'simultaneous_avatar_defeat',
      result: 'draw',
      status: 'finished',
    });
    assert.equal(ctx.session.transcript.at(-1)?.events.filter(({ type }) => type === 'death-blow').length, 2);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test("RULE-04 later undefended site strikes cannot deliver Death's Door death blows", async () => {
  await withNorthAttacksAtC2(
    71,
    undefined,
    { attack: 1, defense: 1, drawSpell: false, life: 1 },
    false,
    undefined,
    0,
    async ({ attackerInstanceId, ctx }) => {
      const site = ctx.state.realm.sites.C2;
      assert.ok(site);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'site'
          && descriptor.target.instanceId === site.instanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
      assert.equal(ctx.state.players.south.avatar.life, 0);
      assert.equal(ctx.state.players.south.avatar.deathDoorTurn, 5);
      assert.deepEqual(ctx.state.terminal, { status: 'active' });

      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === attackerInstanceId
          && descriptor.to.cell === 'C2');
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'site'
          && descriptor.target.instanceId === site.instanceId);
      await ctx.take(({ descriptor }) =>
        descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
      assert.equal(ctx.state.players.south.avatar.life, 0);
      assert.deepEqual(ctx.state.terminal, { status: 'active' });
      assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'death-blow'), false);
      assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'avatar-life-lost'), false);
      assert.equal(await ctx.verifyReplay(), true);
    },
  );
});

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

test('RULE-01 attempting to draw from an empty deck immediately loses', async () => {
  const short = deck('short', 3, 3);
  await withSetup(manifest(19, { north: short, south: short }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'play-site'));
    await ctx.accept(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    await ctx.accept(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas'));

    assert.deepEqual(ctx.state.terminal, {
      loser: 'south',
      reason: 'deck_empty',
      status: 'finished',
      winner: 'north',
    });
    assert.equal(ctx.state.phase, 'terminal');
    assert.deepEqual(await ctx.legalActions('south'), []);
    assert.equal(ctx.session.transcript.at(-1)?.events[0]?.type, 'game-ended');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Pick Up and Drop manage local carried Artifacts once per unit turn', async () => {
  const north: GameDeckSpec = {
    atlas: Array(6).fill('artifact-north-site'),
    avatar: 'artifact-north-avatar',
    spellbook: ['sword-and-shield', 'sword-and-shield', 'sword-and-shield', 'artifact-bearer'],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('artifact-south-site'),
    avatar: 'artifact-south-avatar',
    spellbook: ['artifact-enemy', 'artifact-enemy', 'artifact-enemy', 'artifact-dummy'],
  };
  const cards: Record<string, GameCardDefinition> = {
    'artifact-bearer': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 1,
      spellcaster: true,
      stealth: true,
      tapForMana: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'artifact-dummy': {
      attack: 0,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    'artifact-enemy': {
      attack: 7,
      cardType: 'minion',
      charge: true,
      defense: 2,
      manaCost: 1,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'artifact-north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'artifact-north-site': { cardType: 'site', elements: ['earth'], genesisGainMana: 6 },
    'artifact-south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'artifact-south-site': { cardType: 'site', elements: ['earth'], genesisGainMana: 6 },
    'sword-and-shield': {
      cardType: 'artifact',
      grantsBearerPower: 2,
      manaCost: 2,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-artifact-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const opening = session.state.players.north.hand.spellbook;
      return opening.filter(({ cardId }) => cardId === 'sword-and-shield').length >= 2
        && opening.some(({ cardId }) => cardId === 'artifact-bearer');
    },
    { from: 83, to: 4_096 },
  );
  assert.deepEqual(gameManifest.cards['sword-and-shield'], cards['sword-and-shield']);
  assert.throws(() => createGameManifest({
    ...input,
    seed: 83,
    cards: {
      ...cards,
      'sword-and-shield': {
        cardType: 'artifact',
        grantsBearerPower: 1 as unknown as 2,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  }), /grantsBearerPower must be 2/);

  await withSetup(gameManifest, async (ctx) => {
    const ofKind = async (
      setup: SetupCtx,
      kind: 'drop-artifacts' | 'pick-up-artifacts',
    ) => (await setup.legalActions()).flatMap(({ descriptor }) =>
      descriptor.kind === kind ? [descriptor] : []);

    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'artifact-bearer' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'artifact-bearer');
    assert.ok(bearer);
    const bearerInstanceId = bearer.instanceId;
    assert.equal(bearer.summoningSickness, true);
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    const avatarInstanceId = ctx.state.players.north.avatar.card.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword-and-shield'
      && descriptor.casterInstanceId === avatarInstanceId
      && descriptor.cell === 'C4'
      && descriptor.bearer === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword-and-shield'
      && descriptor.casterInstanceId === avatarInstanceId
      && descriptor.cell === 'C4'
      && descriptor.bearer === undefined);

    const surfaceArtifacts = (ctx.state.realm.artifacts ?? [])
      .flatMap((artifact) => 'bearer' in artifact ? [] : [artifact]);
    assert.equal(surfaceArtifacts.length, 2);
    const artifactInstanceIds = surfaceArtifacts.map(({ instanceId }) => instanceId).sort();
    const expectedSubsets = [
      artifactInstanceIds[0],
      artifactInstanceIds[1],
      artifactInstanceIds.join(','),
    ].sort();
    const initialPickups = await ofKind(ctx, 'pick-up-artifacts');
    assert.equal(initialPickups.length, 6);
    assert.deepEqual(initialPickups
      .filter(({ unit }) => unit.kind === 'avatar')
      .map(({ artifactInstanceIds: ids }) => ids.join(','))
      .sort(), expectedSubsets);
    assert.deepEqual(initialPickups
      .filter(({ unit }) => unit.kind === 'minion')
      .map(({ artifactInstanceIds: ids }) => ids.join(','))
      .sort(), expectedSubsets);
    assert.equal(initialPickups.every(({ artifactInstanceIds: ids }) =>
      ids.length > 0 && canonicalJson(ids) === canonicalJson([...ids].sort())), true);

    const beforeForge = ctx.stateHash();
    const forged = await ctx.stepRequest({
      actionId: 'sha256:9999999999999999999999999999999999999999999999999999999999999999',
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(ctx.stateHash(), beforeForge);

    const manaBeforePickUp = ctx.state.players.north.mana;
    const picked = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'pick-up-artifacts'
        && descriptor.unit.kind === 'minion'
        && descriptor.artifactInstanceIds.length === 1
        && descriptor.artifactInstanceIds[0] === artifactInstanceIds[0]));
    assert.equal(picked.accepted, true);
    if (!picked.accepted) return;
    assert.deepEqual(picked.receipt.events.map(({ payload, type }) => ({ payload, type })), [{
      payload: {
        artifactInstanceIds: [artifactInstanceIds[0]],
        seat: 'north',
        unitInstanceId: bearerInstanceId,
        unitKind: 'minion',
      },
      type: 'artifacts-picked-up',
    }]);
    assert.deepEqual(picked.receipt.randomDraws, []);
    assert.equal(ctx.state.players.north.mana, manaBeforePickUp);
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.deepEqual(ctx.state.realm.units
      .filter(({ instanceId }) => instanceId === bearerInstanceId)
      .map(({ stealthed, summoningSickness, tapped }) => ({ stealthed, summoningSickness, tapped })), [{
      stealthed: true,
      summoningSickness: true,
      tapped: false,
    }]);
    assert.equal(ctx.state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === artifactInstanceIds[0])?.owner, 'north');

    await withFork(ctx, async (dropped) => {
      const voluntarilyDropped = await dropped.step(await dropped.action(({ descriptor }) =>
        descriptor.kind === 'drop-artifacts'
          && descriptor.unit.instanceId === bearerInstanceId
          && descriptor.artifactInstanceIds[0] === artifactInstanceIds[0]));
      assert.equal(voluntarilyDropped.accepted, true);
      if (!voluntarilyDropped.accepted) return;
      assert.deepEqual(voluntarilyDropped.receipt.events.map(({ payload, type }) => ({ payload, type })), [{
        payload: {
          artifactInstanceIds: [artifactInstanceIds[0]],
          seat: 'north',
          unitInstanceId: bearerInstanceId,
          unitKind: 'minion',
        },
        type: 'artifacts-dropped',
      }]);
      assert.deepEqual(voluntarilyDropped.receipt.randomDraws, []);
      assert.deepEqual(dropped.observe('north').realm.artifacts
        ?.filter(({ instanceId }) => instanceId === artifactInstanceIds[0])
        .map(({ bearer: droppedBearer, controller, location, owner, region }) => ({
          bearer: droppedBearer,
          controller,
          location,
          owner,
          region,
        })), [{
        bearer: undefined,
        controller: null,
        location: 'C4',
        owner: 'north',
        region: 'surface',
      }]);
      assert.deepEqual(dropped.state.realm.units
        .filter(({ instanceId }) => instanceId === bearerInstanceId)
        .map(({ damage, stealthed, summoningSickness, tapped }) => ({
          damage, stealthed, summoningSickness, tapped,
        })), [{ damage: 0, stealthed: true, summoningSickness: true, tapped: false }]);
      assert.equal(dropped.state.players.north.mana, manaBeforePickUp);
      assert.equal(await dropped.verifyReplay(), true);
    });

    assert.equal((await ofKind(ctx, 'pick-up-artifacts')).some(({ unit }) => unit.kind === 'minion'), false);
    assert.equal((await ofKind(ctx, 'pick-up-artifacts')).some(({ unit }) => unit.kind === 'avatar'), true);
    assert.equal(ctx.observe('north').realm.units.find(({ instanceId }) =>
      instanceId === bearerInstanceId)?.attack, 3);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    assert.equal((await ofKind(ctx, 'pick-up-artifacts')).some(({ unit }) => unit.kind === 'minion'), true);
    const pickedAgain = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'pick-up-artifacts'
        && descriptor.unit.instanceId === bearerInstanceId
        && descriptor.artifactInstanceIds.length === 1
        && descriptor.artifactInstanceIds[0] === artifactInstanceIds[1]));
    assert.equal(pickedAgain.accepted, true);
    if (!pickedAgain.accepted) return;
    assert.deepEqual(pickedAgain.receipt.randomDraws, []);

    await withFork(ctx, async (movedOnly) => {
      await movedOnly.take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === bearerInstanceId
          && descriptor.to.cell === 'C3');
      await movedOnly.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      assert.equal((await ofKind(movedOnly, 'drop-artifacts'))
        .some(({ unit }) => unit.instanceId === bearerInstanceId), true);
      assert.equal(movedOnly.state.realm.units.find(({ instanceId }) =>
        instanceId === bearerInstanceId)?.stealthed, true);
    });

    await withFork(ctx, async (activated) => {
      await activated.take(({ descriptor }) =>
        descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === bearerInstanceId);
      assert.equal((await ofKind(activated, 'drop-artifacts'))
        .some(({ unit }) => unit.instanceId === bearerInstanceId), false);
    });

    const observedBearer = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === bearerInstanceId);
    assert.equal(observedBearer?.attack, 5);
    assert.equal(observedBearer?.defense, 5);

    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword-and-shield'
      && descriptor.casterInstanceId === bearerInstanceId
      && descriptor.cell === 'C3'
      && descriptor.bearer === undefined);
    assert.equal((await ofKind(ctx, 'drop-artifacts'))
      .some(({ unit }) => unit.instanceId === bearerInstanceId), false);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearerInstanceId)?.stealthed, false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === bearerInstanceId && descriptor.to.cell === 'C3');
    assert.deepEqual(ctx.observe('north').realm.artifacts?.map(({ bearer: artifactBearer, controller, location, region }) => ({
      bearer: artifactBearer?.instanceId,
      controller,
      location,
      region,
    })), [
      { bearer: bearerInstanceId, controller: 'north', location: 'C3', region: 'surface' },
      { bearer: bearerInstanceId, controller: 'north', location: 'C3', region: 'surface' },
      { bearer: undefined, controller: null, location: 'C3', region: 'surface' },
    ]);
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'artifact-enemy' && descriptor.cell === 'C3');
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === 'artifact-enemy');
    assert.ok(enemy);
    const enemyInstanceId = enemy.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === enemyInstanceId && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion' && descriptor.target.instanceId === bearerInstanceId);
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    if (!fought.accepted) return;
    const events = fought.receipt.events;
    const dropIndexes = events.flatMap(({ type }, index) => type === 'artifact-dropped' ? [index] : []);
    const deathIndex = events.findIndex(({ payload, type }) => type === 'minion-died'
      && canonicalJson(payload).includes(bearerInstanceId));
    assert.equal(dropIndexes.length, 2);
    assert.equal(dropIndexes.every((index) => index < deathIndex), true);
    assert.deepEqual(ctx.observe('north').realm.artifacts?.map((artifact) => ({
      bearer: artifact.bearer,
      controller: artifact.controller,
      location: artifact.location,
      owner: artifact.owner,
      region: artifact.region,
    })), [
      { bearer: undefined, controller: null, location: 'C3', owner: 'north', region: 'surface' },
      { bearer: undefined, controller: null, location: 'C3', owner: 'north', region: 'surface' },
      { bearer: undefined, controller: null, location: 'C3', owner: 'north', region: 'surface' },
    ]);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) => instanceId === bearerInstanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ cardId }) => cardId === 'sword-and-shield'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 dropping a power Artifact immediately kills a lethally wounded bearer', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(3).fill('drop-north-site'),
    avatar: 'drop-north-avatar',
    spellbook: ['drop-bearer', 'drop-sword', 'drop-static-servant'],
  };
  const south: GameDeckSpec = {
    atlas: Array(3).fill('drop-south-site'),
    avatar: 'drop-south-avatar',
    spellbook: Array(3).fill('drop-south-minion'),
  };
  const cards: Record<string, GameCardDefinition> = {
    'drop-bearer': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds,
    },
    'drop-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'drop-north-site': { cardType: 'site', elements: [] },
    'drop-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'drop-south-minion': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 0,
      thresholds,
    },
    'drop-south-site': { cardType: 'site', elements: [] },
    'drop-static-servant': {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      genesisDamageEachOtherUnitHere: 1,
      manaCost: 0,
      thresholds,
    },
    'drop-sword': {
      cardType: 'artifact',
      grantsBearerPower: 2,
      manaCost: 0,
      thresholds,
    },
  };
  await withSetup(createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-drop-state-based-death-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 214,
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === 'drop-bearer');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'drop-bearer');
    assert.ok(bearer);
    const bearerInstanceId = bearer.instanceId;
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'cast-artifact'
        && descriptor.cardId === 'drop-sword'
        && descriptor.bearer?.instanceId === bearerInstanceId);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === 'drop-static-servant');
    assert.deepEqual(ctx.observe('north').realm.units
      .filter(({ instanceId }) => instanceId === bearerInstanceId)
      .map(({ damage, defense }) => ({ damage, defense })), [{ damage: 1, defense: 3 }]);

    const beforeDropVersion = ctx.state.stateVersion;
    const dropped = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'drop-artifacts'
        && descriptor.unit.instanceId === bearerInstanceId
        && descriptor.artifactInstanceIds.length === 1));
    assert.equal(dropped.accepted, true);
    if (!dropped.accepted) return;

    assert.equal(ctx.state.stateVersion, beforeDropVersion + 1);
    assert.deepEqual(dropped.receipt.events.map(({ type }) => type), [
      'artifacts-dropped',
      'minion-died',
    ]);
    assert.deepEqual(dropped.receipt.randomDraws, []);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) => instanceId === bearerInstanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === bearerInstanceId), true);
    assert.deepEqual(ctx.observe('north').realm.artifacts?.map((artifact) => ({
      bearer: artifact.bearer,
      controller: artifact.controller,
      location: artifact.location,
      owner: artifact.owner,
      region: artifact.region,
    })), [{
      bearer: undefined,
      controller: null,
      location: 'C4',
      owner: 'north',
      region: 'surface',
    }]);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a carried Lethal Artifact kills on positive strike damage and drops with its bearer', async () => {
  const north: GameDeckSpec = {
    atlas: Array(4).fill('dagger-north-site'),
    avatar: 'dagger-north-avatar',
    spellbook: ['poisonous-dagger', 'poisonous-dagger', 'dagger-bearer', 'dagger-bearer'],
  };
  const south: GameDeckSpec = {
    atlas: Array(5).fill('dagger-south-site'),
    avatar: 'dagger-south-avatar',
    spellbook: Array(4).fill('dagger-enemy'),
  };
  const cards: Record<string, GameCardDefinition> = {
    'dagger-bearer': {
      attack: 2,
      cardType: 'minion',
      defense: 2,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'dagger-enemy': {
      attack: 2,
      cardType: 'minion',
      charge: true,
      defense: 3,
      manaCost: 1,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    'dagger-north-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'dagger-north-site': { cardType: 'site', elements: ['earth'], genesisGainMana: 6 },
    'dagger-south-avatar': { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    'dagger-south-site': { cardType: 'site', elements: ['earth'], genesisGainMana: 6 },
    'poisonous-dagger': {
      cardType: 'artifact',
      grantsBearerLethal: true,
      manaCost: 2,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-lethal-artifact-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
    seed: 89,
  };
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards['poisonous-dagger'], cards['poisonous-dagger']);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'poisonous-dagger': {
        cardType: 'artifact',
        grantsBearerLethal: false,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /grantsBearerLethal must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'poisonous-dagger': {
        cardType: 'artifact',
        grantsBearerLethal: true,
        grantsBearerPower: 2,
        manaCost: 2,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /exactly one supported Artifact effect/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'dagger-bearer' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'dagger-bearer')!;
    const bearerInstanceId = bearer.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'poisonous-dagger'
      && descriptor.bearer?.instanceId === bearerInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === bearerInstanceId && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'dagger-enemy' && descriptor.cell === 'C3');
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === 'dagger-enemy')!;
    const enemyInstanceId = enemy.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === enemyInstanceId && descriptor.to.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion' && descriptor.target.instanceId === bearerInstanceId);
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);

    const events = fought.receipt.events;
    const lethalDamage = events.find(({ payload, type }) => type === 'damage-dealt'
      && canonicalJson(payload).includes(enemyInstanceId));
    assert.ok(lethalDamage);
    assert.match(canonicalJson(lethalDamage.payload), /"amount":2/);
    assert.equal(events.filter(({ type }) => type === 'minion-died').length, 2);
    const dropIndex = events.findIndex(({ type }) => type === 'artifact-dropped');
    const bearerDeathIndex = events.findIndex(({ payload, type }) => type === 'minion-died'
      && canonicalJson(payload).includes(bearerInstanceId));
    assert.ok(dropIndex >= 0 && dropIndex < bearerDeathIndex);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === bearerInstanceId || instanceId === enemyInstanceId), false);
    assert.deepEqual(ctx.observe('north').realm.artifacts?.map((artifact) => ({
      bearer: artifact.bearer,
      controller: artifact.controller,
      location: artifact.location,
      region: artifact.region,
    })), [{ bearer: undefined, controller: null, location: 'C3', region: 'surface' }]);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Siege Ballista taps its bearer and another ally for measured artifact damage', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(6).fill('ballista-north-site'),
    avatar: 'ballista-north-avatar',
    spellbook: [
      'siege-ballista',
      'ballista-bearer',
      'ballista-helper',
      'siege-ballista',
      'ballista-bearer',
      'ballista-helper',
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('ballista-south-site'),
    avatar: 'ballista-south-avatar',
    spellbook: [
      'ballista-far-target',
      'ballista-near-target',
      'ballista-burrowed-target',
      'ballista-far-target',
      'ballista-near-target',
      'ballista-burrowed-target',
    ],
  };
  const cards: Record<string, GameCardDefinition> = {
    'ballista-bearer': {
      attack: 4,
      burrowing: true,
      cardType: 'minion',
      defense: 2,
      lethal: true,
      manaCost: 0,
      stealth: true,
      thresholds,
    },
    'ballista-burrowed-target': {
      attack: 1,
      burrowing: true,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      thresholds,
    },
    'ballista-far-target': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      thresholds,
    },
    'ballista-helper': {
      attack: 1,
      burrowing: true,
      cardType: 'minion',
      defense: 2,
      manaCost: 0,
      thresholds,
    },
    'ballista-near-target': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      preventsDamageFromUnitsWithPowerAtLeast: 4,
      thresholds,
    },
    'ballista-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'ballista-north-site': { cardType: 'site', elements: ['earth'] },
    'ballista-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'ballista-south-site': { cardType: 'site', elements: ['earth'] },
    'siege-ballista': {
      cardType: 'artifact',
      manaCost: 0,
      tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3,
      thresholds,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-siege-ballista-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'siege-ballista': {
        ...cards['siege-ballista'],
        tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 4,
      } as unknown as GameCardDefinition,
    },
    seed: 2,
  }), /tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps must be 3/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'siege-ballista': {
        ...cards['siege-ballista'],
        grantsBearerLethal: true,
      } as unknown as GameCardDefinition,
    },
    seed: 2,
  }), /exactly one supported Artifact effect/);
  const manifest = createGameManifest({ ...input, seed: 2 });
  assert.deepEqual(manifest.cards['siege-ballista'], cards['siege-ballista']);

  await withSetup(manifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-bearer'
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-helper'
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-bearer');
    const helper = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-helper');
    assert.ok(bearer && helper);
    const bearerInstanceId = bearer.instanceId;
    const helperInstanceId = helper.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'siege-ballista'
      && descriptor.bearer?.kind === 'minion'
      && descriptor.bearer.instanceId === bearerInstanceId);
    const ballista = ctx.state.realm.artifacts?.find(({ cardId }) => cardId === 'siege-ballista');
    assert.ok(ballista);
    const ballistaInstanceId = ballista.instanceId;
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.artifactInstanceId === ballistaInstanceId
        && descriptor.helper.instanceId === helperInstanceId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-far-target' && descriptor.cell === 'C1');
    const farTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-far-target');
    assert.ok(farTarget);
    const farTargetId = farTarget.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.artifactInstanceId === ballistaInstanceId
        && descriptor.target.instanceId === farTargetId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-near-target'
      && descriptor.cell === 'C2'
      && descriptor.region === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-burrowed-target'
      && descriptor.cell === 'C2'
      && descriptor.region === 'underground');
    const nearTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-near-target');
    const burrowedTarget = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'ballista-burrowed-target');
    assert.ok(nearTarget && burrowedTarget);
    const nearTargetId = nearTarget.instanceId;
    const burrowedTargetId = burrowedTarget.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const abilities = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.artifactInstanceId === ballistaInstanceId);
    assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
      && descriptor.helper.instanceId === helperInstanceId
      && descriptor.target.instanceId === nearTargetId), true, JSON.stringify({
      abilities: abilities.map(({ descriptor }) => descriptor),
      bearer: bearerInstanceId,
      burrowedTarget: burrowedTargetId,
      farTarget: farTargetId,
      helper: helperInstanceId,
      nearTarget: nearTargetId,
    }));
    assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
      && descriptor.target.instanceId === farTargetId), false);
    assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
      && descriptor.target.instanceId === burrowedTargetId), false);
    assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
      && descriptor.target.instanceId === bearerInstanceId), true);

    await withFork(ctx, async (noHelper) => {
      await noHelper.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === helperInstanceId
        && descriptor.to.cell === 'C3');
      if ((await noHelper.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'decline-attack')) {
        await noHelper.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      }
      assert.equal((await noHelper.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-damage'
          && descriptor.helper.instanceId === helperInstanceId), false);
      assert.equal(await noHelper.verifyReplay(), true);
    });

    await withFork(ctx, async (dropped) => {
      await dropped.take(({ descriptor }) => descriptor.kind === 'drop-artifacts'
        && descriptor.unit.instanceId === bearerInstanceId);
      assert.equal((await dropped.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-damage'
          && descriptor.artifactInstanceId === ballistaInstanceId), false);
      assert.equal(await dropped.verifyReplay(), true);
    });

    await withFork(ctx, async (underground) => {
      await underground.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === bearerInstanceId
        && descriptor.to.cell === 'C4'
        && descriptor.to.region === 'underground');
      if ((await underground.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'decline-attack')) {
        await underground.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      }
      await underground.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === helperInstanceId
        && descriptor.to.cell === 'C4'
        && descriptor.to.region === 'underground');
      if ((await underground.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'decline-attack')) {
        await underground.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      }
      await underground.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await underground.take(({ descriptor }) => descriptor.kind === 'draw');
      await underground.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await underground.take(({ descriptor }) => descriptor.kind === 'draw');
      assert.equal((await underground.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-damage'
          && descriptor.artifactInstanceId === ballistaInstanceId
          && descriptor.helper.instanceId === helperInstanceId
          && descriptor.target.instanceId === burrowedTargetId), true);
      assert.equal(await underground.verifyReplay(), true);
    });

    await withFork(ctx, async (overlapFork) => {
      const overlap = abilities.find(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-damage'
          && descriptor.helper.instanceId === helperInstanceId
          && descriptor.target.instanceId === helperInstanceId);
      assert.ok(overlap);
      const overlapResult = await overlapFork.step(overlap);
      assert.equal(overlapResult.accepted, true);
      if (!overlapResult.accepted) return;
      assert.equal(overlapFork.state.realm.units.some(({ instanceId }) =>
        instanceId === helperInstanceId), false);
      assert.equal(overlapFork.state.realm.units.find(({ instanceId }) =>
        instanceId === bearerInstanceId)?.tapped, true);
      assert.equal(await overlapFork.verifyReplay(), true);
    });

    const activation = abilities.find(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.helper.instanceId === helperInstanceId
        && descriptor.target.instanceId === nearTargetId);
    assert.ok(activation);
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const survivingBearer = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearerInstanceId);
    const survivingHelper = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === helperInstanceId);
    const damagedTarget = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === nearTargetId);
    assert.deepEqual({
      bearerStealthed: survivingBearer?.stealthed,
      bearerTapped: survivingBearer?.tapped,
      helperTapped: survivingHelper?.tapped,
      targetDamage: damagedTarget?.damage,
    }, {
      bearerStealthed: true,
      bearerTapped: true,
      helperTapped: true,
      targetDamage: 3,
    });
    const activated = result.receipt.events.find(({ type }) => type === 'artifact-damage-activated');
    const allocated = result.receipt.events.find(({ type }) => type === 'artifact-damage-allocated');
    assert.ok(activated);
    assert.ok(allocated);
    assert.equal(canonicalJson(activated.payload).includes(ballistaInstanceId), true);
    assert.deepEqual(allocated.payload, {
      amount: 3,
      sourceInstanceId: ballistaInstanceId,
      targetInstanceId: nearTargetId,
    });
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'artifact-damage-activated',
      'artifact-damage-allocated',
      'damage-dealt',
    ]);
    assert.equal(result.receipt.randomDraws.length, 0);
    assert.equal(ctx.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Payload Trebuchet discards a card for measured location damage', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(6).fill('payload-north-site'),
    avatar: 'payload-north-avatar',
    spellbook: [
      'payload-trebuchet',
      'payload-bearer',
      'payload-helper',
      'payload-discard',
      'payload-trebuchet',
      'payload-bearer',
      'payload-helper',
      'payload-discard',
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('payload-south-site'),
    avatar: 'payload-south-avatar',
    spellbook: [
      'payload-target',
      'payload-warded-target',
      'payload-burrowed-target',
      'payload-target',
      'payload-warded-target',
      'payload-burrowed-target',
    ],
  };
  const cards: Record<string, GameCardDefinition> = {
    'payload-bearer': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      lanceCount: 1,
      lethal: true,
      manaCost: 0,
      stealth: true,
      thresholds,
    },
    'payload-burrowed-target': {
      attack: 1,
      burrowing: true,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      thresholds,
    },
    'payload-discard': {
      attack: 4,
      cardType: 'minion',
      defense: 4,
      manaCost: 4,
      thresholds,
    },
    'payload-helper': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      thresholds,
    },
    'payload-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'payload-north-site': { cardType: 'site', elements: ['earth'] },
    'payload-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'payload-south-site': { cardType: 'site', elements: ['earth'] },
    'payload-target': {
      attack: 1,
      cardType: 'minion',
      defense: 4,
      manaCost: 0,
      thresholds,
    },
    'payload-trebuchet': {
      cardType: 'artifact',
      manaCost: 0,
      tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps: true,
      thresholds,
    },
    'payload-warded-target': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      stealth: true,
      thresholds,
      ward: true,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-payload-trebuchet-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'payload-trebuchet': {
        ...cards['payload-trebuchet'],
        tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps: false,
      } as unknown as GameCardDefinition,
    },
    seed: 5,
  }), /tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps must be true/);
  const manifest = createGameManifest({ ...input, seed: 5 });
  assert.deepEqual(manifest.cards['payload-trebuchet'], cards['payload-trebuchet']);

  await withSetup(manifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-bearer' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-helper' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-bearer');
    const helper = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-helper');
    assert.ok(bearer && helper);
    const bearerInstanceId = bearer.instanceId;
    const helperInstanceId = helper.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'payload-trebuchet'
      && descriptor.bearer?.instanceId === bearerInstanceId);
    const payload = ctx.state.realm.artifacts?.find(({ cardId }) => cardId === 'payload-trebuchet');
    assert.ok(payload);
    const payloadInstanceId = payload.instanceId;
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-target'
      && descriptor.cell === 'C1'
      && descriptor.region === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-warded-target'
      && descriptor.cell === 'C1'
      && descriptor.region === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-burrowed-target'
      && descriptor.cell === 'C1'
      && descriptor.region === 'underground');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const discard = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === 'payload-discard');
    const siteDiscard = ctx.state.players.north.hand.atlas[0];
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-target');
    const warded = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-warded-target');
    const burrowed = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'payload-burrowed-target');
    assert.ok(discard && siteDiscard && target && warded && burrowed);
    const discardInstanceId = discard.instanceId;
    const siteDiscardInstanceId = siteDiscard.instanceId;
    const targetInstanceId = target.instanceId;
    const wardedInstanceId = warded.instanceId;
    const burrowedInstanceId = burrowed.instanceId;
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'
        && descriptor.artifactInstanceId === payloadInstanceId
        && descriptor.helper.instanceId === helperInstanceId);
    assert.equal(choices.some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'
        && descriptor.discardCardInstanceId === discardInstanceId
        && descriptor.discardZone === 'spellbook'
        && descriptor.targetLocation.cell === 'C1'
        && descriptor.targetLocation.region === 'surface'), true);
    assert.equal(choices.some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'
        && descriptor.targetLocation.cell === 'C1'
        && descriptor.targetLocation.region === 'underground'), false);

    await withFork(ctx, async (friendlyFork) => {
      const friendly = choices.find(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-discard-area-damage'
          && descriptor.discardCardInstanceId === discardInstanceId
          && descriptor.targetLocation.cell === 'C4');
      assert.ok(friendly);
      const friendlyResult = await friendlyFork.step(friendly);
      assert.equal(friendlyResult.accepted, true);
      if (!friendlyResult.accepted) return;
      const friendlyBearer = friendlyFork.state.realm.units.find(({ instanceId }) =>
        instanceId === bearerInstanceId);
      assert.deepEqual({
        avatarLife: friendlyFork.state.players.north.avatar.life,
        bearerDamage: friendlyBearer?.damage,
        bearerLance: friendlyBearer?.carriedLanceCount,
        bearerStealth: friendlyBearer?.stealthed,
        helperDamage: friendlyFork.state.realm.units.find(({ instanceId }) =>
          instanceId === helperInstanceId)?.damage,
      }, {
        avatarLife: 16,
        bearerDamage: 4,
        bearerLance: 1,
        bearerStealth: true,
        helperDamage: 4,
      });
      assert.equal(await friendlyFork.verifyReplay(), true);
    });

    await withFork(ctx, async (zeroFork) => {
      const zero = choices.find(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-discard-area-damage'
          && descriptor.discardCardInstanceId === siteDiscardInstanceId
          && descriptor.discardZone === 'atlas'
          && descriptor.targetLocation.cell === 'C1');
      assert.ok(zero);
      const zeroResult = await zeroFork.step(zero);
      assert.equal(zeroResult.accepted, true);
      if (!zeroResult.accepted) return;
      assert.equal(zeroFork.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === siteDiscardInstanceId), true);
      assert.equal(zeroResult.receipt.events.filter(({ type }) =>
        type === 'artifact-discard-area-damage-allocated').every(({ payload }) =>
        (payload as { amount: number }).amount === 0), true);
      assert.equal(zeroFork.state.realm.units.find(({ instanceId }) =>
        instanceId === wardedInstanceId)?.warded, true);
      assert.equal(zeroFork.state.realm.units.find(({ instanceId }) =>
        instanceId === targetInstanceId)?.damage, 0);
      assert.equal(await zeroFork.verifyReplay(), true);
    });

    const activation = choices.find(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'
        && descriptor.discardCardInstanceId === discardInstanceId
        && descriptor.discardZone === 'spellbook'
        && descriptor.targetLocation.cell === 'C1');
    assert.ok(activation);
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const survivingBearer = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearerInstanceId);
    const survivingHelper = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === helperInstanceId);
    const survivingWard = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === wardedInstanceId);
    assert.deepEqual({
      bearerDamage: survivingBearer?.damage,
      bearerLance: survivingBearer?.carriedLanceCount,
      bearerStealth: survivingBearer?.stealthed,
      bearerTapped: survivingBearer?.tapped,
      burrowedDamage: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === burrowedInstanceId)?.damage,
      helperDamage: survivingHelper?.damage,
      helperTapped: survivingHelper?.tapped,
      southLife: ctx.state.players.south.avatar.life,
      targetPresent: ctx.state.realm.units.some(({ instanceId }) =>
        instanceId === targetInstanceId),
      wardDamage: survivingWard?.damage,
      wardStealth: survivingWard?.stealthed,
      warded: survivingWard?.warded,
    }, {
      bearerDamage: 0,
      bearerLance: 1,
      bearerStealth: true,
      bearerTapped: true,
      burrowedDamage: 0,
      helperDamage: 0,
      helperTapped: true,
      southLife: 16,
      targetPresent: false,
      wardDamage: 0,
      wardStealth: true,
      warded: false,
    });
    assert.equal(ctx.state.players.north.hand.spellbook.some(({ instanceId }) =>
      instanceId === discardInstanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === discardInstanceId), true);
    const allocations = result.receipt.events.filter(({ type }) =>
      type === 'artifact-discard-area-damage-allocated');
    assert.equal(allocations.length, 3);
    assert.equal(allocations.every(({ payload: allocationPayload }) => {
      const allocation = allocationPayload as {
        amount?: number;
        sourceInstanceId?: string;
      };
      return allocation.amount === 4 && allocation.sourceInstanceId === payloadInstanceId;
    }), true);
    assert.deepEqual(result.receipt.events.slice(0, 2).map(({ type }) => type), [
      'card-discarded',
      'artifact-discard-area-damage-activated',
    ]);
    assert.equal(result.receipt.events.some(({ type }) =>
      type === 'fight-started' || type === 'strike-damage-allocated' || type === 'lance-broken'), false);
    assert.equal(result.receipt.randomDraws.length, 0);
    assert.equal(ctx.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Rolling Boulder rolls maximally and damages other units along its path', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(6).fill('boulder-north-site'),
    avatar: 'boulder-north-avatar',
    spellbook: ['rolling-boulder', 'boulder-pusher', 'boulder-origin-target'],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('boulder-south-site'),
    avatar: 'boulder-south-avatar',
    spellbook: ['boulder-warded-target', 'boulder-underground-target', 'boulder-off-path-target'],
  };
  const cards: Record<string, GameCardDefinition> = {
    'boulder-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'boulder-north-site': { cardType: 'site', elements: ['earth'] },
    'boulder-off-path-target': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      thresholds,
    },
    'boulder-origin-target': {
      attack: 1,
      cardType: 'minion',
      defense: 4,
      manaCost: 0,
      thresholds,
    },
    'boulder-pusher': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      lanceCount: 1,
      lethal: true,
      manaCost: 0,
      stealth: true,
      thresholds,
    },
    'boulder-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'boulder-south-site': { cardType: 'site', elements: ['earth'] },
    'boulder-underground-target': {
      attack: 1,
      burrowing: true,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      thresholds,
    },
    'boulder-warded-target': {
      attack: 1,
      cardType: 'minion',
      defense: 5,
      manaCost: 0,
      stealth: true,
      thresholds,
      ward: true,
    },
    'rolling-boulder': {
      cardType: 'artifact',
      manaCost: 0,
      tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4,
      thresholds,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-rolling-boulder-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'rolling-boulder': {
        ...cards['rolling-boulder'],
        tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 5,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath must be 4/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      'rolling-boulder': {
        ...cards['rolling-boulder'],
        grantsBearerLethal: true,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /exactly one supported Artifact effect/);
  const rollingManifest = createGameManifest({ ...input, seed: 1 });
  assert.deepEqual(rollingManifest.cards['rolling-boulder'], cards['rolling-boulder']);

  await withSetup(rollingManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-pusher' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-origin-target' && descriptor.cell === 'C4');
    const pusher = ctx.state.realm.units.find(({ cardId }) => cardId === 'boulder-pusher');
    const originTarget = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'boulder-origin-target');
    assert.ok(pusher && originTarget);
    const pusherInstanceId = pusher.instanceId;
    const originTargetInstanceId = originTarget.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'rolling-boulder'
      && descriptor.bearer === undefined
      && descriptor.cell === 'C4');
    const boulder = ctx.state.realm.artifacts?.find(({ cardId }) => cardId === 'rolling-boulder');
    assert.ok(boulder);
    const boulderInstanceId = boulder.instanceId;
    const freshRolls = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage');
    assert.equal(freshRolls.some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage'
        && descriptor.pusher.instanceId === pusherInstanceId), false);
    assert.deepEqual({
      avatarLocation: ctx.state.players.north.avatar.location,
      avatarTapped: ctx.state.players.north.avatar.tapped,
      pushers: freshRolls.flatMap(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-roll-damage'
          ? [descriptor.pusher.kind]
          : []),
    }, {
      avatarLocation: 'C4',
      avatarTapped: true,
      pushers: [],
    });
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-warded-target'
      && descriptor.cell === 'C2'
      && descriptor.region === undefined);
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-underground-target'
      && descriptor.cell === 'C2'
      && descriptor.region === 'underground');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-off-path-target'
      && descriptor.cell === 'B1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const warded = ctx.state.realm.units.find(({ cardId }) => cardId === 'boulder-warded-target');
    const underground = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'boulder-underground-target');
    const offPath = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'boulder-off-path-target');
    assert.ok(warded && underground && offPath);
    const wardedInstanceId = warded.instanceId;
    const undergroundInstanceId = underground.instanceId;
    const offPathInstanceId = offPath.instanceId;
    const rolls = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage'
        && descriptor.artifactInstanceId === boulderInstanceId
        && descriptor.pusher.instanceId === pusherInstanceId);
    assert.deepEqual(rolls.flatMap(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage'
        ? [[descriptor.direction, descriptor.path.map(({ cell }) => cell)]]
        : []), [
      ['east', ['C4']],
      ['north', ['C4']],
      ['south', ['C4', 'C3', 'C2', 'C1']],
      ['west', ['C4']],
    ]);
    const southRoll = rolls.find(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage' && descriptor.direction === 'south');
    const zeroRoll = rolls.find(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage' && descriptor.direction === 'north');
    assert.ok(southRoll && zeroRoll);
    if (southRoll.descriptor.kind !== 'activate-artifact-roll-damage') return;
    const shortenedDescriptor = {
      ...southRoll.descriptor,
      path: southRoll.descriptor.path.slice(0, 2),
    };
    const beforeForge = ctx.stateHash();
    const forged = await ctx.stepRequest({
      actionId: opaqueActionId(
        'sorcery-core-v1',
        'north',
        ctx.state.stateVersion,
        shortenedDescriptor,
      ),
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(hashGameState(forged.session.state), beforeForge);

    await withFork(ctx, async (zeroFork) => {
      const zeroResult = await zeroFork.step(zeroRoll);
      assert.equal(zeroResult.accepted, true);
      if (!zeroResult.accepted) return;
      assert.equal(zeroFork.state.realm.units.find(({ instanceId }) =>
        instanceId === pusherInstanceId)?.tapped, true);
      assert.equal(zeroFork.state.realm.units.find(({ instanceId }) =>
        instanceId === originTargetInstanceId)?.damage, 0);
      assert.deepEqual(zeroResult.receipt.events.map(({ type }) => type), [
        'artifact-roll-damage-activated',
      ]);
      assert.equal(zeroFork.observe('north').realm.artifacts
        ?.find(({ instanceId }) => instanceId === boulderInstanceId)?.location, 'C4');
      assert.equal(await zeroFork.verifyReplay(), true);
    });

    await withFork(ctx, async (carried) => {
      await carried.take(({ descriptor }) => descriptor.kind === 'pick-up-artifacts'
        && descriptor.unit.instanceId === pusherInstanceId
        && descriptor.artifactInstanceIds.includes(boulderInstanceId));
      const carriedResult = await carried.step(await carried.action(({ descriptor }) =>
        descriptor.kind === 'activate-artifact-roll-damage'
          && descriptor.artifactInstanceId === boulderInstanceId
          && descriptor.pusher.instanceId === pusherInstanceId
          && descriptor.direction === 'south'));
      assert.equal(carriedResult.accepted, true);
      if (!carriedResult.accepted) return;
      assert.deepEqual(carried.observe('north').realm.artifacts
        ?.filter(({ instanceId }) => instanceId === boulderInstanceId)
        .map(({ bearer, controller, location, region }) => ({
          bearer, controller, location, region,
        })),
      [{ bearer: undefined, controller: null, location: 'C1', region: 'surface' }]);
      assert.equal(await carried.verifyReplay(), true);
    });

    const result = await ctx.step(southRoll);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const survivingPusher = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === pusherInstanceId);
    const survivingWard = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === wardedInstanceId);
    assert.deepEqual({
      northLife: ctx.state.players.north.avatar.life,
      offPathDamage: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === offPathInstanceId)?.damage,
      originTargetPresent: ctx.state.realm.units.some(({ instanceId }) =>
        instanceId === originTargetInstanceId),
      pusherDamage: survivingPusher?.damage,
      pusherLance: survivingPusher?.carriedLanceCount,
      pusherStealth: survivingPusher?.stealthed,
      pusherTapped: survivingPusher?.tapped,
      southLife: ctx.state.players.south.avatar.life,
      undergroundDamage: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === undergroundInstanceId)?.damage,
      wardDamage: survivingWard?.damage,
      wardStealth: survivingWard?.stealthed,
      warded: survivingWard?.warded,
    }, {
      northLife: 16,
      offPathDamage: 0,
      originTargetPresent: false,
      pusherDamage: 0,
      pusherLance: 1,
      pusherStealth: true,
      pusherTapped: true,
      southLife: 16,
      undergroundDamage: 0,
      wardDamage: 0,
      wardStealth: true,
      warded: false,
    });
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === originTargetInstanceId), true);
    assert.deepEqual(ctx.observe('north').realm.artifacts
      ?.filter(({ instanceId }) => instanceId === boulderInstanceId)
      .map(({ bearer, controller, location, region }) => ({ bearer, controller, location, region })),
    [{ bearer: undefined, controller: null, location: 'C1', region: 'surface' }]);
    const allocations = result.receipt.events.filter(({ type }) =>
      type === 'artifact-roll-damage-allocated');
    const expectedTargetIds = [
      ctx.state.players.north.avatar.card.instanceId,
      originTargetInstanceId,
      ctx.state.players.south.avatar.card.instanceId,
      wardedInstanceId,
    ].sort((left, right) => left.localeCompare(right));
    assert.deepEqual(allocations.map(({ payload }) =>
      (payload as { targetInstanceId: string }).targetInstanceId), expectedTargetIds);
    assert.equal(allocations.every(({ payload }) => {
      const allocation = payload as { amount?: number; sourceInstanceId?: string };
      return allocation.amount === 4 && allocation.sourceInstanceId === boulderInstanceId;
    }), true);
    assert.equal(result.receipt.events[0]?.type, 'artifact-roll-damage-activated');
    assert.equal(canonicalJson(result.receipt.events[0]!.payload)
      .includes('C4'), true);
    assert.equal(canonicalJson(result.receipt.events[0]!.payload)
      .includes('C1'), true);
    assert.equal(result.receipt.events.some(({ type }) =>
      type === 'fight-started'
        || type === 'strike-damage-allocated'
        || type === 'lance-broken'
        || type === 'lethal-damage'), false);
    assert.equal(result.receipt.randomDraws.length, 0);
    assert.equal(ctx.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03/05 Mesmerism transfers a minion and its Deathrite to the new controller', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(4).fill('mesmerism-north-site'),
    avatar: 'mesmerism-north-avatar',
    spellbook: Array(4).fill('mesmerism'),
  };
  const south: GameDeckSpec = {
    atlas: Array(4).fill('mesmerism-south-site'),
    avatar: 'mesmerism-south-avatar',
    spellbook: ['mesmerism-target', 'mesmerism-warded', 'mesmerism-artifact', 'mesmerism-attacker'],
  };
  const cards: Record<string, GameCardDefinition> = {
    mesmerism: {
      cardType: 'magic',
      gainControlOfTargetNearbyMinion: true,
      manaCost: 1,
      thresholds: { ...thresholds, air: 1 },
    },
    'mesmerism-artifact': {
      cardType: 'artifact',
      grantsBearerPower: 2,
      manaCost: 0,
      thresholds,
    },
    'mesmerism-attacker': {
      attack: 7,
      cardType: 'minion',
      defense: 10,
      manaCost: 0,
      movementBonus: 2,
      summonToAnySite: true,
      thresholds,
    },
    'mesmerism-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'mesmerism-north-site': {
      cardType: 'site',
      elements: ['air'],
      genesisGainMana: 3,
    },
    'mesmerism-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'mesmerism-south-site': { cardType: 'site', elements: ['air'] },
    'mesmerism-target': {
      attack: 1,
      cardType: 'minion',
      deathriteDrawSite: true,
      defense: 3,
      lanceCount: 2,
      manaCost: 0,
      provides: 'fire',
      spellcaster: true,
      stealth: true,
      summonToAnySite: true,
      thresholds,
    },
    'mesmerism-warded': {
      attack: 1,
      cardType: 'minion',
      defense: 3,
      manaCost: 0,
      summonToAnySite: true,
      thresholds,
      ward: true,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-mesmerism-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
    seed: 211,
  };
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards.mesmerism, cards.mesmerism);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      mesmerism: {
        cardType: 'magic',
        gainControlOfTargetNearbyMinion: false,
        manaCost: 1,
        thresholds: { ...thresholds, air: 1 },
      } as unknown as GameCardDefinition,
    },
  }), /gainControlOfTargetNearbyMinion must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      mesmerism: {
        cardType: 'magic',
        gainControlOfTargetNearbyMinion: true,
        healController: 1,
        manaCost: 1,
        thresholds: { ...thresholds, air: 1 },
      },
    },
  }), /exactly one supported Magic effect/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'mesmerism-target'
      && descriptor.casterInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'mesmerism-warded'
      && descriptor.casterInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'mesmerism-attacker'
      && descriptor.casterInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.cell === 'C1');
    const hiddenTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-target')!;
    const wardedTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-warded')!;
    const attacker = ctx.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-attacker')!;
    const hiddenTargetId = hiddenTarget.instanceId;
    const wardedTargetId = wardedTarget.instanceId;
    const attackerId = attacker.instanceId;

    await withFork(ctx, async (hidden) => {
      await hidden.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await hidden.take(({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const hiddenTargetIds = (await hidden.legalActions('north')).flatMap(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.target?.kind === 'minion'
          ? [descriptor.target.instanceId]
          : []);
      assert.deepEqual([...new Set(hiddenTargetIds)], [wardedTargetId]);
      assert.equal(await hidden.verifyReplay(), true);
    });

    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'mesmerism-artifact'
      && descriptor.casterInstanceId === hiddenTargetId
      && descriptor.bearer?.instanceId === hiddenTargetId);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTargetId)?.stealthed, false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const targetBefore = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTargetId)!;
    const carriedBefore = ctx.state.realm.artifacts?.find((artifact) =>
      'bearer' in artifact && artifact.bearer.instanceId === hiddenTargetId);
    assert.ok(carriedBefore);
    assert.ok('bearer' in carriedBefore);
    const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic');
    assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.kind === 'minion'
        ? [descriptor.target.instanceId]
        : []))].sort(), [hiddenTargetId, wardedTargetId].sort());

    await withFork(ctx, async (warded) => {
      const wardedResult = await warded.step(await warded.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === wardedTargetId));
      assert.equal(wardedResult.accepted, true);
      if (!wardedResult.accepted) return;
      assert.deepEqual(wardedResult.receipt.events.map(({ type }) => type), [
        'magic-cast',
        'ward-broken',
        'magic-resolved',
      ]);
      assert.deepEqual({
        controller: warded.state.realm.units.find(({ instanceId }) =>
          instanceId === wardedTargetId)?.controller,
        warded: warded.state.realm.units.find(({ instanceId }) =>
          instanceId === wardedTargetId)?.warded,
      }, { controller: 'south', warded: false });
      assert.equal(await warded.verifyReplay(), true);
    });

    const manaAtCheckpoint = ctx.state.players.north.mana;
    const gainedAction = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === hiddenTargetId);
    const sourceInstanceId = gainedAction.descriptor.kind === 'cast-magic'
      ? gainedAction.descriptor.cardInstanceId
      : '';
    const gained = await ctx.step(gainedAction);
    assert.equal(gained.accepted, true);
    if (!gained.accepted) return;
    const controlled = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTargetId)!;
    assert.deepEqual(controlled, { ...targetBefore, controller: 'north' });
    assert.deepEqual(gained.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-control-changed',
      'magic-resolved',
    ]);
    assert.deepEqual(gained.receipt.events[1]?.payload, {
      fromSeat: 'south',
      instanceId: hiddenTargetId,
      seat: 'north',
      sourceInstanceId,
    });
    const carried = ctx.state.realm.artifacts?.find((artifact) =>
      'bearer' in artifact && artifact.bearer.instanceId === hiddenTargetId);
    assert.ok(carried);
    assert.ok('bearer' in carried);
    assert.deepEqual(carried, {
      ...carriedBefore,
      bearer: { ...carriedBefore.bearer, seat: 'north' },
    });
    const northView = ctx.observe('north');
    assert.deepEqual({
      artifactController: northView.realm.artifacts?.[0]?.controller,
      artifactSeat: northView.realm.artifacts?.[0]?.bearer?.seat,
      attack: northView.realm.units.find(({ instanceId }) =>
        instanceId === hiddenTargetId)?.attack,
      defense: northView.realm.units.find(({ instanceId }) =>
        instanceId === hiddenTargetId)?.defense,
      northFire: northView.players.north.affinity.fire,
      southFire: northView.players.south.affinity.fire,
    }, {
      artifactController: 'north',
      artifactSeat: 'north',
      attack: 3,
      defense: 5,
      northFire: 1,
      southFire: 0,
    });
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === hiddenTargetId
        && descriptor.to.cell === 'C3'), true);

    const ownNoOp = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.casterInstanceId === ctx.state.players.north.avatar.card.instanceId
        && descriptor.target?.instanceId === hiddenTargetId));
    assert.equal(ownNoOp.accepted, true);
    if (!ownNoOp.accepted) return;
    assert.deepEqual(ownNoOp.receipt.events.map(({ type }) => type), ['magic-cast', 'magic-resolved']);
    assert.deepEqual(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTargetId), controlled);
    assert.deepEqual(ctx.state.realm.artifacts?.find((artifact) =>
      'bearer' in artifact && artifact.bearer.instanceId === hiddenTargetId), carried);
    assert.equal(gained.receipt.randomDraws.length + ownNoOp.receipt.randomDraws.length, 0);
    assert.equal(ctx.state.players.north.mana, manaAtCheckpoint - 2);

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attackerId && descriptor.to.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === hiddenTargetId);
    const playersBeforeDeathrite = ctx.state.players;
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    if (!fought.accepted) return;
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === hiddenTargetId), false);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === hiddenTargetId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === hiddenTargetId), false);
    const deathEvents = fought.receipt.events;
    assert.equal(ctx.state.players.north.atlas.length, playersBeforeDeathrite.north.atlas.length - 1);
    assert.equal(ctx.state.players.north.hand.atlas.length,
      playersBeforeDeathrite.north.hand.atlas.length + 1);
    assert.equal(ctx.state.players.south.atlas.length, playersBeforeDeathrite.south.atlas.length);
    assert.equal(ctx.state.players.south.hand.atlas.length,
      playersBeforeDeathrite.south.hand.atlas.length);
    assert.deepEqual(deathEvents.find(({ type }) => type === 'site-drawn')?.payload, {
      seat: 'north',
      sourceInstanceId: hiddenTargetId,
    });
    const drawIndex = deathEvents.findIndex(({ type }) => type === 'site-drawn');
    const dropIndex = deathEvents.findIndex(({ type }) => type === 'artifact-dropped');
    const deathIndex = deathEvents.findIndex(({ payload, type }) => type === 'minion-died'
      && canonicalJson(payload).includes(hiddenTargetId));
    assert.ok(drawIndex >= 0 && drawIndex < dropIndex && dropIndex < deathIndex);
    assert.equal(ctx.observe('north').realm.artifacts?.[0]?.controller, null);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Fatality kills only a wounded minion in the caster region', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(4).fill('fatality-north-site'),
    avatar: 'fatality-north-avatar',
    spellbook: Array(4).fill('fatality'),
  };
  const south: GameDeckSpec = {
    atlas: Array(4).fill('fatality-south-site'),
    avatar: 'fatality-south-avatar',
    spellbook: Array(4).fill('fatality-target'),
  };
  const cards: Record<string, GameCardDefinition> = {
    fatality: {
      cardType: 'magic',
      killTargetWoundedMinion: true,
      manaCost: 1,
      thresholds,
    },
    'fatality-north-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'fatality-north-site': { cardType: 'site', elements: ['earth'] },
    'fatality-south-avatar': {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      drawSpell: false,
      life: 20,
    },
    'fatality-south-site': { cardType: 'site', elements: ['earth'] },
    'fatality-target': {
      attack: 0,
      cardType: 'minion',
      defense: 3,
      manaCost: 0,
      summonToAnySite: true,
      thresholds,
    },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-fatality-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
    seed: 212,
  };
  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards.fatality, cards.fatality);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      fatality: {
        cardType: 'magic',
        killTargetWoundedMinion: false,
        manaCost: 1,
        thresholds,
      } as unknown as GameCardDefinition,
    },
  }), /killTargetWoundedMinion must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      fatality: {
        cardType: 'magic',
        healController: 1,
        killTargetWoundedMinion: true,
        manaCost: 1,
        thresholds,
      },
    },
  }), /exactly one supported Magic effect/);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'fatality-target' && descriptor.cell === 'C4');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === 'fatality-target');
    assert.ok(target);
    const targetInstanceId = target.instanceId;
    const targetCardId = target.cardId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.path.length === 1);
    await ctx.take(({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === targetInstanceId);
    await ctx.take(({ descriptor }) => descriptor.kind === 'close-defend'
      && descriptor.originalTargetParticipates);
    const wounded = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === targetInstanceId);
    const fatality = ctx.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === 'fatality');
    assert.equal(wounded?.damage, 1);
    assert.ok(fatality);
    const fatalityInstanceId = fatality.instanceId;

    const targetRefs = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === fatalityInstanceId
        && descriptor.target
        ? [descriptor.target]
        : []);
    assert.equal(targetRefs.some(({ kind }) => kind === 'avatar'), false);
    assert.deepEqual(targetRefs.map(({ instanceId }) => instanceId), [targetInstanceId]);

    const cast = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === targetInstanceId);
    const sourceInstanceId = cast.descriptor.kind === 'cast-magic'
      ? cast.descriptor.cardInstanceId
      : '';
    const killed = await ctx.step(cast);
    assert.equal(killed.accepted, true);
    if (!killed.accepted) return;
    assert.deepEqual(killed.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-killed',
      'minion-died',
      'magic-resolved',
    ]);
    assert.deepEqual(killed.receipt.events[1]?.payload, {
      cardId: targetCardId,
      instanceId: targetInstanceId,
      owner: 'south',
      seat: 'south',
      sourceInstanceId,
    });
    assert.equal(killed.receipt.events.some(({ type }) => type === 'damage-dealt'), false);
    assert.deepEqual(killed.receipt.randomDraws, []);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === targetInstanceId), false);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === targetInstanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === sourceInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Sparkmage may tap for zero damage at a nearby location with no other unit', async () => {
  await withSetup(manifest(417, {
    avatar: {
      attack: 1,
      defense: 1,
      drawSpell: false,
      life: 20,
      tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true,
    },
    site: { elements: ['air'] },
    spell: {
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const activation = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-sparkmage'
        && descriptor.targetLocation.cell === 'C4'
        && descriptor.targetLocation.region === 'surface');
    assert.match(activation.label, /deal 0 to a random other unit at C4/);
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;

    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 0);
    assert.deepEqual(result.receipt.randomDraws, []);
    assert.deepEqual(result.receipt.events.map(({ type }) => type), ['sparkmage-activated']);
    assert.doesNotMatch(canonicalJson(result.receipt.events[0]!.payload), /targetInstanceId/);
    assert.equal(ctx.observe('south').players.north.airThresholdsCastThisTurn, 0);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Sparkmage counts every player-cast spell source, resets, and damages one other unit', async () => {
  const north: GameDeckSpec = {
    atlas: Array(6).fill('sparkmage-air-site'),
    avatar: 'sparkmage-avatar',
    spellbook: [
      'sparkmage-caster', 'sparkmage-artifact', 'sparkmage-magic',
      'sparkmage-caster', 'sparkmage-artifact', 'sparkmage-magic',
      'sparkmage-caster', 'sparkmage-artifact', 'sparkmage-magic',
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('sparkmage-south-site'),
    avatar: 'sparkmage-south-avatar',
    spellbook: Array(6).fill('sparkmage-south-dummy'),
  };
  const cards = cardsFor(
    { north, south },
    {
      defense: 4,
      manaCost: 0,
      preventsDamageFromUnitsWithPowerAtLeast: 4,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    },
    {
      attack: 4,
      defense: 4,
      drawSpell: false,
      life: 20,
      tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true,
    },
    { elements: ['air'], genesisGainMana: 6 },
    {
      south: {
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  );
  cards['sparkmage-caster'] = {
    ...cards['sparkmage-caster']!,
    defense: 4,
    spellcaster: true,
  } as GameCardDefinition;
  cards['sparkmage-artifact'] = {
    cardType: 'artifact',
    grantsBearerPower: 2,
    manaCost: 0,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  };
  cards['sparkmage-magic'] = {
    cardType: 'magic',
    healController: 1,
    manaCost: 0,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-sparkmage-cast-counter-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  const gameManifest = createGameManifest({ ...input, seed: 20 });
  await withSetup(gameManifest, async (ctx) => {
    const opening = ctx.state.players.north.hand.spellbook.map(({ cardId }) => cardId);
    assert.equal(['sparkmage-caster', 'sparkmage-artifact', 'sparkmage-magic']
      .every((cardId) => opening.includes(cardId)), true);
    assert.equal(ctx.state.players.north.spellbook[0]?.cardId, 'sparkmage-magic');
    assert.deepEqual(gameManifest.cards['sparkmage-avatar'], {
      attack: 4,
      cardType: 'avatar',
      defense: 4,
      drawSpell: false,
      life: 20,
      tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true,
    });
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'sparkmage-caster' && descriptor.cell === 'C4');
    const caster = ctx.state.realm.units.find(({ cardId }) => cardId === 'sparkmage-caster');
    assert.ok(caster);
    const casterInstanceId = caster.instanceId;
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 1);
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sparkmage-artifact'
      && descriptor.casterInstanceId === casterInstanceId
      && descriptor.bearer?.instanceId === casterInstanceId);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 2);
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'sparkmage-magic'
      && descriptor.casterInstanceId === casterInstanceId);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 3);
    const opponentView = ctx.observe('south');
    assert.equal(opponentView.players.north.airThresholdsCastThisTurn, 3);
    assert.equal(typeof opponentView.players.north.hand.spellbook, 'number');

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 0);
    assert.equal(ctx.state.players.south.airThresholdsCastThisTurn, 0);
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'sparkmage-magic'
      && descriptor.casterInstanceId === casterInstanceId);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 1);

    const activation = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-sparkmage'
        && descriptor.targetLocation.cell === 'C4'
        && descriptor.targetLocation.region === 'surface');
    assert.doesNotMatch(canonicalJson(activation.descriptor), new RegExp(casterInstanceId));
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;

    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === casterInstanceId)?.damage, 0);
    assert.equal(result.receipt.randomDraws.length, 1);
    assert.equal(
      result.receipt.randomDraws[0]?.purpose,
      'sparkmage_random_other_unit_at_nearby_location',
    );
    assert.deepEqual(result.receipt.events.map(({ type }) => type), [
      'sparkmage-activated',
      'damage-dealt',
    ]);
    assert.deepEqual(result.receipt.events[1]?.payload, {
      accumulated: 0,
      amount: 0,
      attemptedAmount: 1,
      direct: true,
      instanceId: casterInstanceId,
      prevented: true,
      seat: 'north',
    });
    assert.match(canonicalJson(result.receipt.events[0]!.payload), new RegExp(casterInstanceId));
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Sparkmage chooses among multiple other units with deterministic private RNG', async () => {
  const north: GameDeckSpec = {
    atlas: Array(6).fill('sparkmage-many-site'),
    avatar: 'sparkmage-many-avatar',
    spellbook: [
      'sparkmage-many-target', 'sparkmage-many-target', 'sparkmage-many-target',
      'sparkmage-many-magic', 'sparkmage-many-magic', 'sparkmage-many-magic',
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('sparkmage-many-south-site'),
    avatar: 'sparkmage-many-south-avatar',
    spellbook: Array(6).fill('sparkmage-many-south-dummy'),
  };
  const cards = cardsFor(
    { north, south },
    { defense: 3, manaCost: 0, thresholds: { air: 1, earth: 0, fire: 0, water: 0 } },
    {
      attack: 1,
      defense: 1,
      drawSpell: false,
      life: 20,
      tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true,
    },
    { elements: ['air'], genesisGainMana: 6 },
    {
      south: {
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
  );
  cards['sparkmage-many-magic'] = {
    cardType: 'magic',
    healController: 1,
    manaCost: 0,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-sparkmage-many-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
  };
  const gameManifest = createGameManifest({ ...input, seed: 7 });
  await withSetup(gameManifest, async (ctx) => {
    assert.ok(ctx.state.players.north.hand.spellbook.filter(({ cardId }) =>
      cardId === 'sparkmage-many-target').length >= 2);
    assert.equal(ctx.state.players.north.spellbook[0]?.cardId, 'sparkmage-many-magic');
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const targets = ctx.state.players.north.hand.spellbook
      .filter(({ cardId }) => cardId === 'sparkmage-many-target')
      .slice(0, 2);
    assert.equal(targets.length, 2);
    const targetIds = targets.map(({ instanceId }) => instanceId);
    for (const target of targets) {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === target.instanceId
        && descriptor.cell === 'C4');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'sparkmage-many-magic');

    const activation = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-sparkmage'
        && descriptor.targetLocation.cell === 'C4'
        && descriptor.targetLocation.region === 'surface');
    for (const instanceId of targetIds) {
      assert.doesNotMatch(canonicalJson(activation.descriptor), new RegExp(instanceId));
    }
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;

    assert.ok(result.receipt.randomDraws.length >= 1);
    assert.ok(result.receipt.randomDraws.every(({ purpose }) =>
      purpose === 'sparkmage_random_other_unit_at_nearby_location'));
    const activated = result.receipt.events.find(({ type }) => type === 'sparkmage-activated');
    const damaged = result.receipt.events.filter(({ type }) => type === 'damage-dealt');
    assert.ok(activated);
    assert.equal(damaged.length, 1);
    const activatedJson = canonicalJson(activated.payload);
    const damagedJson = canonicalJson(damaged[0]!.payload);
    const selected = targetIds.find((instanceId) => activatedJson.includes(instanceId));
    assert.ok(selected);
    assert.match(damagedJson, new RegExp(selected));
    assert.equal(ctx.state.realm.units.filter(({ damage }) => damage === 1).length, 1);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 an active surface minion derives power, Ranged, and Spellcaster atop a Tower', async () => {
  const decks = {
    north: deck('tower-minion-north', 6, 8),
    south: deck('tower-minion-south', 6, 8),
  };
  const cards = cardsFor(decks, {
    attack: 1,
    defense: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  });
  const towerId = decks.north.atlas[0]!;
  const nonTowerId = decks.north.atlas[1]!;
  const conditionalId = decks.north.spellbook[0]!;
  const magicId = decks.north.spellbook[1]!;
  const targetId = decks.south.spellbook[0]!;
  const disableMagicId = decks.south.spellbook[1]!;
  cards[towerId] = {
    ...cards[towerId]!,
    isTower: true,
  } as unknown as GameCardDefinition;
  cards[conditionalId] = {
    ...cards[conditionalId]!,
    burrowing: true,
    gainsPowerRangedAndSpellcasterAtopTower: 2,
  } as unknown as GameCardDefinition;
  cards[nonTowerId] = {
    ...cards[nonTowerId]!,
    sacrificeToDestroyNearbySite: true,
  } as GameCardDefinition;
  cards[magicId] = {
    cardType: 'magic',
    damageTargetUnit: 1,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  cards[targetId] = {
    ...cards[targetId]!,
    defense: 5,
    spellcaster: true,
    summonToAnySite: true,
  } as GameCardDefinition;
  cards[disableMagicId] = {
    cardType: 'magic',
    disableTargetNearbyMinionUntilNextTurn: true,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-tower-minion-v1',
    },
    cards,
    decks,
    firstSeat: 'north' as const,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [towerId]: { ...cards[towerId], isTower: false } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /isTower must be true/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [conditionalId]: {
        ...cards[conditionalId],
        gainsPowerRangedAndSpellcasterAtopTower: 1,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /gainsPowerRangedAndSpellcasterAtopTower must be 2/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [conditionalId]: {
        ...cards[conditionalId],
        occupiesSquareArea: 2,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /occupiesSquareArea has an unsupported ability combination/);

  const gameManifest = await findOpeningManifest(
    (seed) => createGameManifest({ ...input, seed }),
    (session) => {
      const preview = session.state.players;
      const northAtlas = preview.north.hand.atlas.map(({ cardId }) => cardId);
      const northSpells = preview.north.hand.spellbook.map(({ cardId }) => cardId);
      const southSpells = preview.south.hand.spellbook.map(({ cardId }) => cardId);
      return northAtlas.includes(towerId)
        && northAtlas.includes(nonTowerId)
        && northSpells.includes(conditionalId)
        && northSpells.includes(magicId)
        && [targetId, disableMagicId].every((cardId) => southSpells.includes(cardId));
    },
    { from: 1, to: 16_384 },
  );
  assert.deepEqual(gameManifest.cards[towerId], {
    cardType: 'site',
    elements: ['earth'],
    isTower: true,
  });
  assert.equal(gameManifest.cards[conditionalId]?.cardType === 'minion'
    && gameManifest.cards[conditionalId].gainsPowerRangedAndSpellcasterAtopTower, 2);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === towerId && descriptor.cell === 'C4');
    await withFork(ctx, async (underground) => {
      await underground.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === conditionalId
        && descriptor.cell === 'C4'
        && descriptor.region === 'underground');
      const buried = underground.state.realm.units.find(({ cardId }) => cardId === conditionalId);
      assert.ok(buried);
      const buriedObserved = underground.observe('north').realm.units
        .find(({ instanceId }) => instanceId === buried.instanceId);
      assert.deepEqual({ attack: buriedObserved?.attack, defense: buriedObserved?.defense }, {
        attack: 1,
        defense: 1,
      });
      assert.equal((await underground.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.casterInstanceId === buried.instanceId), false);
      await underground.take(({ descriptor }) => descriptor.kind === 'end-turn');
      assert.equal(await underground.verifyReplay(), true);
    });

    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === conditionalId && descriptor.cell === 'C4' && !descriptor.region);
    const conditional = ctx.state.realm.units.find(({ cardId }) => cardId === conditionalId);
    assert.ok(conditional);
    const conditionalInstanceId = conditional.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetId && descriptor.cell === 'C4');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetId);
    assert.ok(target);
    const targetInstanceId = target.instanceId;
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === nonTowerId && descriptor.cell === 'C3');

    const atopTower = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === conditionalInstanceId);
    assert.deepEqual({ attack: atopTower?.attack, defense: atopTower?.defense }, {
      attack: 3,
      defense: 3,
    });
    const tower = ctx.state.realm.sites.C4;
    assert.ok(tower && !('rubble' in tower));
    const towerActions = await ctx.legalActions('north');
    assert.equal(towerActions.some(({ descriptor }) => descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === conditionalInstanceId
      && descriptor.hit?.instanceId === targetInstanceId), true);
    assert.equal(towerActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === magicId
      && descriptor.casterInstanceId === conditionalInstanceId), true);

    await withFork(ctx, async (offTower) => {
      await offTower.take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === conditionalInstanceId
        && descriptor.from.cell === 'C4'
        && descriptor.to.cell === 'C3');
      await offTower.take(({ descriptor }) => descriptor.kind === 'decline-attack');
      const moved = offTower.observe('north').realm.units
        .find(({ instanceId }) => instanceId === conditionalInstanceId);
      assert.deepEqual({ attack: moved?.attack, defense: moved?.defense }, {
        attack: 1,
        defense: 1,
      });
      const offTowerActions = await offTower.legalActions('north');
      assert.equal(offTowerActions.some(({ descriptor }) => descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === conditionalInstanceId), false);
      assert.equal(offTowerActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.casterInstanceId === conditionalInstanceId), false);
      assert.equal(offTower.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
      assert.equal(await offTower.verifyReplay(), true);
    });

    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === disableMagicId
      && descriptor.casterInstanceId === targetInstanceId
      && descriptor.target?.instanceId === conditionalInstanceId);
    const disabled = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === conditionalInstanceId);
    assert.deepEqual({
      attack: disabled?.attack,
      defense: disabled?.defense,
      disabled: disabled?.disabled,
    }, { attack: 1, defense: 1, disabled: true });
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw');
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.casterInstanceId === conditionalInstanceId), false);
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw');
    const awakened = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === conditionalInstanceId);
    assert.deepEqual({
      attack: awakened?.attack,
      defense: awakened?.defense,
      disabled: awakened?.disabled,
    }, { attack: 3, defense: 3, disabled: false });
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === magicId
      && descriptor.casterInstanceId === conditionalInstanceId
      && descriptor.target?.instanceId === conditionalInstanceId);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === conditionalInstanceId)?.damage, 1);
    const destroyed = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-site-destruction'
        && descriptor.sourceSiteInstanceId === ctx.state.realm.sites.C3?.instanceId
        && descriptor.targetCell === 'C4'));
    assert.equal(destroyed.accepted, true);
    if (!destroyed.accepted) return;
    assert.equal(destroyed.receipt.events.some(({ payload, type }) =>
      type === 'minion-died'
        && canonicalJson(payload).includes(conditionalInstanceId)), true);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === conditionalInstanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === conditionalInstanceId), true);
    assert.equal(destroyed.receipt.randomDraws.length, 0);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a minion discards a chosen Spellbook card to damage a random other unit here', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(6).fill('nimbus-site'),
    avatar: 'nimbus-avatar',
    spellbook: [
      'nimbus-source', 'nimbus-discard-a', 'nimbus-discard-b',
      'nimbus-source', 'nimbus-discard-a', 'nimbus-discard-b',
      'nimbus-source', 'nimbus-discard-a', 'nimbus-discard-b',
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('nimbus-south-site'),
    avatar: 'nimbus-south-avatar',
    spellbook: Array(6).fill('nimbus-south-dummy'),
  };
  const cards = cardsFor(
    { north, south },
    {
      attack: 4,
      defense: 4,
      discardSpellToDamageRandomOtherUnitHere: 3,
      manaCost: 0,
      thresholds,
    },
    { attack: 1, defense: 1, drawSpell: false, life: 20 },
    { elements: ['air'], genesisGainMana: 6 },
  );
  for (const cardId of ['nimbus-discard-a', 'nimbus-discard-b']) {
    cards[cardId] = {
      cardType: 'magic',
      healController: 1,
      manaCost: 0,
      thresholds,
    };
  }
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-nimbus-discard-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 3,
  });
  assert.equal(gameManifest.cards['nimbus-source']?.cardType === 'minion'
    && gameManifest.cards['nimbus-source'].discardSpellToDamageRandomOtherUnitHere, 3);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const sourceCard = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === 'nimbus-source');
    assert.ok(sourceCard);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === sourceCard.instanceId
        && descriptor.cell === 'C4');
    const source = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === sourceCard.instanceId);
    assert.ok(source);
    const discardCards = ctx.state.players.north.hand.spellbook
      .filter(({ cardId }) => cardId.startsWith('nimbus-discard-'))
      .sort((left, right) => left.instanceId.localeCompare(right.instanceId));
    assert.equal(discardCards.length, 2);
    const activations = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-discard-random-damage'
        && descriptor.sourceInstanceId === source.instanceId);
    assert.deepEqual(activations.map(({ descriptor }) =>
      descriptor.kind === 'activate-discard-random-damage'
        ? descriptor.discardCardInstanceId
        : ''), discardCards.map(({ instanceId }) => instanceId));
    assert.equal(new Set(activations.map(({ actionId }) => actionId)).size, 2);
    assert.equal(activations.every(({ descriptor }) =>
      !canonicalJson(descriptor as unknown as JsonValue).includes('target')), true);
    const labelsByCardId = Object.fromEntries(activations.map(({ descriptor, label }) => {
      if (descriptor.kind !== 'activate-discard-random-damage') return ['', label];
      const discard = discardCards.find(({ instanceId }) =>
        instanceId === descriptor.discardCardInstanceId);
      return [discard?.cardId ?? '', label];
    }));
    assert.deepEqual(labelsByCardId, {
      'nimbus-discard-a': `Discard nimbus-discard-a to activate ${source.instanceId.slice(0, 15)}…`,
      'nimbus-discard-b': `Discard nimbus-discard-b to activate ${source.instanceId.slice(0, 15)}…`,
    });
    assert.equal(activations.every(({ descriptor, label }) =>
      descriptor.kind === 'activate-discard-random-damage'
        && !label.includes(descriptor.discardCardInstanceId.slice(0, 15))), true);

    const chosen = activations[0]!;
    const discardedCard = discardCards[0]!;
    const avatarInstanceId = ctx.state.players.north.avatar.card.instanceId;
    const first = await ctx.step(chosen);
    assert.equal(first.accepted, true);
    if (!first.accepted) return;
    assert.deepEqual(first.receipt.events.map(({ type }) => type), [
      'card-discarded',
      'discard-random-damage-activated',
      'discard-random-damage-allocated',
      'damage-dealt',
      'avatar-life-lost',
    ]);
    assert.deepEqual(first.receipt.events[0]?.payload, {
      cardId: discardedCard.cardId,
      instanceId: discardedCard.instanceId,
      owner: 'north',
      seat: 'north',
      sourceInstanceId: source.instanceId,
      zone: 'spellbook',
    });
    assert.deepEqual(first.receipt.events[1]?.payload, {
      amount: 3,
      discardCardInstanceId: discardedCard.instanceId,
      seat: 'north',
      sourceInstanceId: source.instanceId,
      sourceLocation: { cell: 'C4', region: 'surface' },
      targetInstanceId: avatarInstanceId,
      targetKind: 'avatar',
      targetSeat: 'north',
    });
    assert.deepEqual(first.receipt.events[2]?.payload, {
      amount: 3,
      sourceInstanceId: source.instanceId,
      targetInstanceId: avatarInstanceId,
    });
    assert.equal(first.receipt.randomDraws.length, 1);
    assert.equal(first.receipt.randomDraws[0]?.purpose,
      'discard_spell_random_other_unit_here');
    assert.deepEqual(first.receipt.randomDraws[0]?.domain, {
      accepted: true,
      exclusiveMaximum: 1,
      kind: 'unit_index_candidate',
    });
    assert.equal(ctx.state.players.north.avatar.life, 17);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === discardedCard.instanceId), true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === source.instanceId)?.tapped, false);
    const southView = ctx.observe('south');
    assert.equal(southView.players.north.cemetery.some(({ instanceId }) =>
      instanceId === discardedCard.instanceId), true);
    assert.equal(typeof southView.players.north.hand.spellbook, 'number');

    const stale = await ctx.step(chosen);
    assert.equal(stale.accepted, false);
    if (!stale.accepted) assert.equal(stale.reason.code, 'stale_version');
    const repeat = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-discard-random-damage'
        && descriptor.sourceInstanceId === source.instanceId));
    assert.equal(repeat.accepted, true);
    if (!repeat.accepted) return;
    assert.equal(ctx.state.players.north.avatar.life, 14);
    assert.equal(repeat.receipt.randomDraws.length, 1);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.reset(gameManifest);
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'nimbus-source'
      && descriptor.cell === 'C3');
    const emptySource = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'nimbus-source');
    assert.ok(emptySource);
    const emptyResult = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-discard-random-damage'
        && descriptor.sourceInstanceId === emptySource.instanceId));
    assert.equal(emptyResult.accepted, true);
    if (!emptyResult.accepted) return;
    assert.deepEqual(emptyResult.receipt.events.map(({ type }) => type), [
      'card-discarded',
      'discard-random-damage-activated',
    ]);
    assert.equal(emptyResult.receipt.randomDraws.length, 0);
    assert.equal(
      canonicalJson(emptyResult.receipt.events[1]!.payload).includes('targetInstanceId'),
      false,
    );
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 random other-unit damage includes allied, enemy, Avatar, and Stealth candidates', async () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(6).fill('nimbus-many-site'),
    avatar: 'nimbus-many-avatar',
    spellbook: [
      'nimbus-many-source', 'nimbus-many-ally', 'nimbus-many-discard',
      'nimbus-many-source', 'nimbus-many-ally', 'nimbus-many-discard',
      'nimbus-many-source', 'nimbus-many-ally', 'nimbus-many-discard',
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill('nimbus-many-south-site'),
    avatar: 'nimbus-many-south-avatar',
    spellbook: [
      'nimbus-many-enemy', 'nimbus-many-enemy', 'nimbus-many-enemy',
      'nimbus-many-dummy', 'nimbus-many-dummy', 'nimbus-many-dummy',
    ],
  };
  const cards = cardsFor({ north, south }, { manaCost: 0, thresholds });
  cards['nimbus-many-source'] = {
    attack: 4,
    cardType: 'minion',
    defense: 4,
    discardSpellToDamageRandomOtherUnitHere: 3,
    manaCost: 0,
    thresholds,
  };
  cards['nimbus-many-ally'] = {
    attack: 1,
    cardType: 'minion',
    defense: 5,
    manaCost: 0,
    stealth: true,
    thresholds,
  };
  cards['nimbus-many-discard'] = {
    cardType: 'magic',
    healController: 1,
    manaCost: 0,
    thresholds,
  };
  cards['nimbus-many-enemy'] = {
    attack: 1,
    cardType: 'minion',
    defense: 5,
    manaCost: 0,
    summonToAnySite: true,
    thresholds,
  };
  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-nimbus-many-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 3,
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'nimbus-many-ally' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'nimbus-many-enemy' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'nimbus-many-source' && descriptor.cell === 'C4');
    const source = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'nimbus-many-source');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === 'nimbus-many-ally');
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === 'nimbus-many-enemy');
    assert.ok(source && ally && enemy);
    const activation = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-discard-random-damage'
        && descriptor.sourceInstanceId === source.instanceId);
    if (activation.descriptor.kind !== 'activate-discard-random-damage') return;
    assert.deepEqual(await ctx.legalActions('south'), []);

    const beforeForge = ctx.stateHash();
    const forgedDescriptor = { ...activation.descriptor, targetInstanceId: enemy.instanceId };
    const forged = await ctx.stepRequest({
      actionId: opaqueActionId(
        'sorcery-core-v1',
        'north',
        ctx.state.stateVersion,
        forgedDescriptor,
      ),
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(ctx.stateHash(), beforeForge);

    const candidates = [
      ctx.state.players.north.avatar.card.instanceId,
      ally.instanceId,
      enemy.instanceId,
    ].sort((left, right) => left.localeCompare(right));
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.equal(result.receipt.randomDraws.length, 1);
    assert.deepEqual(result.receipt.randomDraws[0]?.domain, {
      accepted: true,
      exclusiveMaximum: 3,
      kind: 'unit_index_candidate',
    });
    const randomResult = result.receipt.randomDraws[0]!.result;
    assert.equal(typeof randomResult, 'number');
    if (typeof randomResult !== 'number') return;
    const selectedIndex = randomResult % candidates.length;
    const selectedInstanceId = candidates[selectedIndex]!;
    const activated = result.receipt.events.find(({ type }) =>
      type === 'discard-random-damage-activated');
    assert.ok(activated);
    assert.match(canonicalJson(activated.payload), new RegExp(selectedInstanceId));
    assert.equal(result.receipt.events.filter(({ type }) =>
      type === 'discard-random-damage-allocated').length, 1);
    assert.equal(result.receipt.events.filter(({ type }) => type === 'damage-dealt').length, 1);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === source.instanceId)?.damage, 0);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === ally.instanceId)?.stealthed, true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 discard damage snapshots derived unit power and uses Ward and prevention', async () => {
  const run = async (
    auraBonus: boolean,
    targetFacts: Readonly<{ prevents?: number; ward?: boolean }>,
  ): Promise<Readonly<{
    result: Extract<GameStepResult, { accepted: true }>;
    targetId: string;
  }>> => {
    const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
    const north: GameDeckSpec = {
      atlas: Array(6).fill('nimbus-prevention-site'),
      avatar: 'nimbus-prevention-avatar',
      spellbook: [
        'nimbus-prevention-source', 'nimbus-prevention-aura',
        'nimbus-prevention-target', 'nimbus-prevention-discard',
        'nimbus-prevention-source', 'nimbus-prevention-aura',
        'nimbus-prevention-target', 'nimbus-prevention-discard',
        'nimbus-prevention-source', 'nimbus-prevention-aura',
        'nimbus-prevention-target', 'nimbus-prevention-discard',
      ],
    };
    const south: GameDeckSpec = {
      atlas: Array(6).fill('nimbus-prevention-south-site'),
      avatar: 'nimbus-prevention-south-avatar',
      spellbook: Array(6).fill('nimbus-prevention-dummy'),
    };
    const cards = cardsFor({ north, south }, { manaCost: 0, thresholds });
    cards['nimbus-prevention-source'] = {
      attack: 3,
      cardType: 'minion',
      defense: 4,
      discardSpellToDamageRandomOtherUnitHere: 3,
      manaCost: 0,
      thresholds,
    };
    cards['nimbus-prevention-aura'] = {
      attack: 1,
      cardType: 'minion',
      defense: 4,
      manaCost: 0,
      ...(auraBonus ? { otherNearbyAlliesPowerBonus: 1 as const } : {}),
      thresholds,
    };
    cards['nimbus-prevention-target'] = {
      attack: 1,
      cardType: 'minion',
      defense: 3,
      manaCost: 0,
      ...(targetFacts.prevents !== undefined
        ? { preventsDamageFromUnitsWithPowerAtLeast: targetFacts.prevents }
        : {}),
      thresholds,
      ...(targetFacts.ward ? { ward: true } : {}),
    };
    cards['nimbus-prevention-discard'] = {
      cardType: 'magic',
      healController: 1,
      manaCost: 0,
      thresholds,
    };
    const gameManifest = createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-nimbus-prevention-${auraBonus}-${targetFacts.ward ?? false}`,
      },
      cards,
      decks: { north, south },
      firstSeat: 'north',
      seed: 31,
    });
    return withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'nimbus-prevention-aura' && descriptor.cell === 'C4');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
      await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'nimbus-prevention-target' && descriptor.cell === 'C3');
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'nimbus-prevention-source' && descriptor.cell === 'C3');
      const source = ctx.state.realm.units.find(({ cardId }) =>
        cardId === 'nimbus-prevention-source');
      const target = ctx.state.realm.units.find(({ cardId }) =>
        cardId === 'nimbus-prevention-target');
      assert.ok(source && target);
      assert.equal(ctx.observe('north').realm.units.find(({ instanceId }) =>
        instanceId === source.instanceId)?.attack, auraBonus ? 4 : 3);
      const result = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'activate-discard-random-damage'
          && descriptor.sourceInstanceId === source.instanceId));
      assert.equal(result.accepted, true);
      if (!result.accepted) throw new Error('expected Nimbus prevention scenario activation');
      assert.equal(result.receipt.randomDraws.length, 1);
      assert.equal(await ctx.verifyReplay(), true);
      return { result, targetId: target.instanceId };
    });
  };

  const protectedResult = await run(true, { prevents: 4 });
  assert.deepEqual(protectedResult.result.receipt.events.find(({ type }) =>
    type === 'damage-dealt')?.payload, {
    accumulated: 0,
    amount: 0,
    attemptedAmount: 3,
    direct: true,
    instanceId: protectedResult.targetId,
    prevented: true,
    seat: 'north',
  });
  assert.equal(protectedResult.result.session.state.realm.units.some(({ instanceId }) =>
    instanceId === protectedResult.targetId), true);

  const belowThreshold = await run(false, { prevents: 4 });
  assert.equal(belowThreshold.result.receipt.events.some(({ payload, type }) =>
    type === 'minion-died' && canonicalJson(payload).includes(belowThreshold.targetId)), true);
  assert.equal(belowThreshold.result.session.state.realm.units.some(({ instanceId }) =>
    instanceId === belowThreshold.targetId), false);

  const warded = await run(true, { ward: true });
  assert.deepEqual(warded.result.receipt.events.slice(-2).map(({ type }) => type), [
    'damage-dealt',
    'ward-broken',
  ]);
  assert.equal(warded.result.session.state.realm.units.find(({ instanceId }) =>
    instanceId === warded.targetId)?.warded, false);
});

async function withDevilsEgg(
  kind: 'both' | 'carried',
  seed: number,
  run: (
    ctx: SetupCtx,
    ids: Readonly<{ carrier: string; northEgg: string }>,
    gameManifest: GameManifest,
  ) => Promise<void>,
): Promise<void> {
  const prefix = 'egg-' + kind;
  const ids = { carrier: prefix + '-carrier', northEgg: prefix + '-north-egg' } as const;
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const artifact: GameCardDefinition = {
    atEndOfEachTurnSiteControllerLosesLife: 1, cardType: 'artifact', manaCost: 0, thresholds,
  };
  const minion: GameCardDefinition = {
    attack: 1, cardType: 'minion', defense: 1, manaCost: 0, thresholds,
  };
  const avatar: GameCardDefinition = {
    attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20,
  };
  const northSite = prefix + '-north-site';
  const southSite = prefix + '-south-site';
  const southEgg = prefix + '-south-egg';
  const dummy = prefix + '-dummy';
  const northSpellbook = kind === 'carried'
    ? Array.from({ length: 6 }, (_, index) => index % 2 === 0 ? ids.carrier : ids.northEgg)
    : Array(6).fill(ids.northEgg);
  const decks = {
    north: { atlas: Array(6).fill(northSite), avatar: prefix + '-north-avatar', spellbook: northSpellbook },
    south: {
      atlas: Array(6).fill(southSite), avatar: prefix + '-south-avatar',
      spellbook: Array(6).fill(kind === 'both' ? southEgg : dummy),
    },
  } satisfies Record<'north' | 'south', GameDeckSpec>;
  const gameManifest = createGameManifest({
    authority: { contentHash: SYNTHETIC_AUTHORITY_HASH, mode: 'synthetic',
      revisionId: 'synthetic-devils-egg-' + kind + '-v1' },
    cards: {
      [decks.north.avatar]: avatar,
      [ids.northEgg]: artifact,
      [northSite]: { cardType: 'site', elements: ['air'] },
      [decks.south.avatar]: avatar,
      [southSite]: { cardType: 'site', elements: ['air'] },
      ...(kind === 'carried'
        ? { [ids.carrier]: { ...minion, diesAtEndOfControllerTurn: true } }
        : {}),
      ...(kind === 'both' ? { [southEgg]: artifact } : { [dummy]: minion }),
    },
    decks,
    firstSeat: 'north',
    seed,
  });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    if (kind === 'carried') {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === ids.carrier && descriptor.cell === 'C4');
      const carrier = ctx.state.realm.units.find(({ cardId }) => cardId === ids.carrier);
      assert.ok(carrier);
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
        && descriptor.cardId === ids.northEgg
        && descriptor.bearer?.instanceId === carrier.instanceId);
    } else {
      await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
        && descriptor.cardId === ids.northEgg && descriptor.cell === 'C4');
    }
    await run(ctx, ids, gameManifest);
  });
}

test('RULE-03 Artifacts make their current site controller lose life at each turn end', async () => {
  await withDevilsEgg('both', 73, async (ctx, ids, gameManifest) => {
    const northEggDefinition = gameManifest.cards[ids.northEgg];
    assert.equal(northEggDefinition?.cardType === 'artifact'
      && northEggDefinition.atEndOfEachTurnSiteControllerLosesLife, 1);
    assert.throws(() => createGameManifest({
      ...gameManifest,
      cards: {
        ...gameManifest.cards,
        [ids.northEgg]: {
          ...northEggDefinition,
          atEndOfEachTurnSiteControllerLosesLife: 0,
        } as unknown as GameCardDefinition,
      },
    }), /atEndOfEachTurnSiteControllerLosesLife must be a safe integer between 1 and/);

    const northEggCard = ctx.state.realm.artifacts?.find(({ cardId }) => cardId === ids.northEgg);
    assert.ok(northEggCard);
    const northSite = ctx.state.realm.sites.C4;
    assert.ok(northSite);
    const northEnded = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(northEnded.accepted, true);
    if (!northEnded.accepted) return;
    assert.deepEqual(northEnded.receipt.events.map(({ type }) => type), [
      'end-turn-site-life-loss-triggered',
      'avatar-life-lost',
      'turn-ended',
      'turn-started',
    ]);
    assert.deepEqual(northEnded.receipt.events.slice(0, 2).map(({ payload }) => payload), [
      { amount: 1, seat: 'north', siteInstanceId: northSite.instanceId,
        sourceInstanceId: northEggCard.instanceId },
      { amount: 1, life: 19, seat: 'north', sourceInstanceId: northEggCard.instanceId },
    ]);

    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    const southEggCard = ctx.state.players.south.hand.spellbook[0];
    assert.ok(southEggCard);
    await ctx.take(({ descriptor }) =>
      descriptor.kind === 'cast-artifact'
        && descriptor.cardInstanceId === southEggCard.instanceId
        && descriptor.cell === 'C1');
    const southSite = ctx.state.realm.sites.C1;
    assert.ok(southSite);
    const southEnded = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(southEnded.accepted, true);
    if (!southEnded.accepted) return;
    assert.deepEqual(southEnded.receipt.events.slice(0, 4).map(({ payload, type }) => ({ payload, type })), [
      {
        payload: { amount: 1, seat: 'north', siteInstanceId: northSite.instanceId,
          sourceInstanceId: northEggCard.instanceId },
        type: 'end-turn-site-life-loss-triggered',
      },
      {
        payload: { amount: 1, life: 18, seat: 'north', sourceInstanceId: northEggCard.instanceId },
        type: 'avatar-life-lost',
      },
      {
        payload: { amount: 1, seat: 'south', siteInstanceId: southSite.instanceId,
          sourceInstanceId: southEggCard.instanceId },
        type: 'end-turn-site-life-loss-triggered',
      },
      {
        payload: { amount: 1, life: 19, seat: 'south', sourceInstanceId: southEggCard.instanceId },
        type: 'avatar-life-lost',
      },
    ]);
    assert.deepEqual({
      north: ctx.state.players.north.avatar.life,
      south: ctx.state.players.south.avatar.life,
    }, { north: 18, south: 19 });
    assert.equal(southEnded.receipt.randomDraws.length, 0);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 end-turn Artifact life loss uses its carried cell and survives bearer Disable', async () => {
  await withDevilsEgg('carried', 4, async (ctx, ids) => {
    const carrier = ctx.state.realm.units.find(({ cardId }) => cardId === ids.carrier);
    assert.ok(carrier);
    const carried = ctx.state.realm.artifacts?.find(({ cardId }) => cardId === ids.northEgg);
    assert.ok(carried && 'bearer' in carried);

    const ordinaryEnd = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'end-turn'));
    assert.equal(ordinaryEnd.accepted, true);
    if (!ordinaryEnd.accepted) return;
    assert.deepEqual(ordinaryEnd.receipt.events.map(({ type }) => type), [
      'end-turn-site-life-loss-triggered',
      'avatar-life-lost',
      'artifact-dropped',
      'minion-died',
      'turn-ended',
      'turn-started',
    ]);
    assert.deepEqual(ctx.observe('north').realm.artifacts?.map((artifact) => ({
      bearer: artifact.bearer,
      controller: artifact.controller,
      location: artifact.location,
      region: artifact.region,
    })), [{ bearer: undefined, controller: null, location: 'C4', region: 'surface' }]);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 start-turn random teleports resolve in controller-chosen order through Lucky Charm', async () => {
  const sourceCardId = 'headless-source';
  const luckyCharmCardId = 'headless-lucky-charm';
  const blockedSiteCardId = 'headless-blocked-site';
  const north: GameDeckSpec = {
    atlas: Array(6).fill('headless-open-site'),
    avatar: 'headless-north-avatar',
    spellbook: [
      ...Array(4).fill(sourceCardId),
      ...Array(2).fill(luckyCharmCardId),
    ],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill(blockedSiteCardId),
    avatar: 'headless-south-avatar',
    spellbook: Array(6).fill('headless-blocker'),
  };
  const cards = cardsFor(
    { north, south },
    {
      attack: 1,
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  );
  cards[sourceCardId] = {
    ...cards[sourceCardId]!,
    atStartOfControllerTurnTeleportToRandomSiteOrVoid: true,
    attack: 3,
    voidwalk: true,
  } as GameCardDefinition;
  cards[luckyCharmCardId] = {
    bearerControllerChoosesExtraRandomOutcome: true,
    cardType: 'artifact',
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  cards['headless-blocker'] = {
    ...cards['headless-blocker']!,
    attack: 2,
    defense: 5,
  } as GameCardDefinition;
  cards[blockedSiteCardId] = {
    ...cards[blockedSiteCardId]!,
    preventsUnitsWithPowerAtLeastFromEntering: 3,
  } as GameCardDefinition;

  await withSetup(createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-headless-start-turn-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 10,
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === luckyCharmCardId
      && descriptor.bearer?.kind === 'avatar');
    for (let count = 0; count < 2; count += 1) {
      await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === sourceCardId
        && descriptor.cell === 'C4');
    }
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === blockedSiteCardId
      && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'headless-blocker'
      && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');

    const sourceIds = ctx.state.realm.units
      .filter(({ cardId }) => cardId === sourceCardId)
      .map(({ instanceId }) => instanceId)
      .sort();
    const triggers = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'resolve-start-turn-trigger');
    const firstTrigger = triggers.find(({ descriptor }) =>
      descriptor.kind === 'resolve-start-turn-trigger'
        && descriptor.sourceInstanceId === sourceIds[1]);
    assert.ok(firstTrigger);

    let blockedReceipt: GameStepResult | undefined;
    let secondTrigger: GameLegalAction | undefined;
    let secondCommittedReceipt: GameStepResult | undefined;
    let movedChoice: GameLegalAction | undefined;
    await withFork(ctx, async (discover) => {
      const committed = await discover.step(firstTrigger);
      assert.equal(committed.accepted, true);
      if (!committed.accepted) return;
      const choices = (await discover.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'resolve-random-outcome');
      const blockedChoice = choices.find(({ label }) => label === 'Lucky Charm chooses C1 surface');
      assert.ok(blockedChoice);
      const blocked = await discover.step(blockedChoice);
      assert.equal(blocked.accepted, true);
      if (!blocked.accepted) return;
      blockedReceipt = blocked;
      const foundSecond = (await discover.legalActions('north'))
        .find(({ descriptor }) => descriptor.kind === 'resolve-start-turn-trigger');
      assert.ok(foundSecond);
      secondTrigger = foundSecond;
      const secondCommitted = await discover.step(foundSecond);
      assert.equal(secondCommitted.accepted, true);
      if (!secondCommitted.accepted) return;
      secondCommittedReceipt = secondCommitted;
      for (const choice of (await discover.legalActions('north'))
        .filter(({ descriptor }) => descriptor.kind === 'resolve-random-outcome')) {
        await withFork(discover, async (tryChoice) => {
          const result = await tryChoice.step(choice);
          if (result.accepted && result.receipt.events.some(({ type }) =>
            type === 'unit-teleported')) {
            movedChoice = choice;
          }
        });
      }
    });
    assert.ok(blockedReceipt);
    assert.ok(secondTrigger);
    assert.ok(secondCommittedReceipt);
    assert.ok(movedChoice);
    assert.match(movedChoice.label, /^Lucky Charm chooses [A-E][1-4] (surface|void)$/);

    assert.equal(ctx.state.phase, 'start-turn');
    assert.deepEqual(triggers.flatMap(({ descriptor }) =>
      descriptor.kind === 'resolve-start-turn-trigger'
        ? [descriptor.sourceInstanceId]
        : []).sort(), sourceIds);
    assert.equal(firstTrigger.descriptor.kind, 'resolve-start-turn-trigger');
    if (firstTrigger.descriptor.kind !== 'resolve-start-turn-trigger') return;
    assert.equal(firstTrigger.descriptor.sourceInstanceId, sourceIds[1]);
    assert.equal(ctx.state.realm.units.some(({ cardId, location }) =>
      cardId === 'headless-blocker' && location === 'C1'), true);
    assert.equal(ctx.state.cards[blockedSiteCardId]?.cardType === 'site'
      && ctx.state.cards[blockedSiteCardId].preventsUnitsWithPowerAtLeastFromEntering, 3);

    await withFork(ctx, async (first) => {
      const committed = await first.step(firstTrigger);
      assert.equal(committed.accepted, true);
      if (!committed.accepted) return;
      await withFork(ctx, async (second) => {
        const repeated = await second.step(firstTrigger);
        assert.equal(repeated.accepted, true);
        if (!repeated.accepted) return;
        assert.deepEqual(repeated.receipt, committed.receipt);
      });
    });

    const committed = await ctx.step(firstTrigger);
    assert.equal(committed.accepted, true);
    if (!committed.accepted) return;
    assert.equal(committed.receipt.events.length, 0);
    assert.equal(committed.receipt.randomDraws.length, 2);
    assert.equal(committed.receipt.randomDraws.every(({ purpose }) =>
      purpose === 'start_turn_random_teleport'), true);
    assert.equal(ctx.state.phase, 'random-choice');
    const blockedChoice = (await ctx.legalActions('north')).find(({ label }) =>
      label === 'Lucky Charm chooses C1 surface');
    assert.ok(blockedChoice);
    assert.equal(blockedChoice.descriptor.kind, 'resolve-random-outcome');
    if (blockedChoice.descriptor.kind !== 'resolve-random-outcome') return;
    const blocked = await ctx.step(blockedChoice);
    assert.equal(blocked.accepted, true);
    if (!blocked.accepted) return;
    assert.deepEqual(blocked.receipt.events.map(({ payload, type }) => ({ payload, type })), [{
      payload: {
        from: { cell: 'C4', region: 'surface' },
        outcomeInstanceId: blockedChoice.descriptor.outcomeInstanceId,
        reason: 'illegal-entry',
        seat: 'north',
        sourceInstanceId: sourceIds[1],
        to: { cell: 'C1', region: 'surface' },
      },
      type: 'unit-teleport-failed',
    }]);
    assert.deepEqual(blocked.receipt.randomDraws, []);
    assert.equal(ctx.state.phase, 'start-turn');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === sourceIds[1])?.location, 'C4');
    assert.equal(secondTrigger.descriptor.kind, 'resolve-start-turn-trigger');
    if (secondTrigger.descriptor.kind !== 'resolve-start-turn-trigger') return;
    assert.equal(secondTrigger.descriptor.sourceInstanceId, sourceIds[0]);

    const beforeForgeHash = ctx.stateHash();
    const beforeForgeTranscript = ctx.session.transcript.length;
    const forgedDescriptor = {
      kind: 'resolve-start-turn-trigger' as const,
      sourceInstanceId: sourceIds[1]!,
    };
    const forged = await ctx.stepRequest({
      actionId: opaqueActionId(
        'sorcery-core-v1',
        'north',
        ctx.state.stateVersion,
        forgedDescriptor,
      ),
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(hashGameState(forged.session.state), beforeForgeHash);
    assert.equal(ctx.session.transcript.length, beforeForgeTranscript);

    const committedAfterForge = await ctx.step(secondTrigger);
    assert.equal(committedAfterForge.accepted, true);
    if (!committedAfterForge.accepted) return;
    assert.equal(secondCommittedReceipt.accepted, true);
    if (!secondCommittedReceipt.accepted) return;
    assert.deepEqual(committedAfterForge.receipt, secondCommittedReceipt.receipt);
    const moved = await ctx.step(movedChoice);
    assert.equal(moved.accepted, true);
    if (!moved.accepted) return;
    assert.equal(moved.receipt.events.some(({ payload, type }) =>
      type === 'unit-teleported'
        && canonicalJson(payload).includes(sourceIds[0]!)
        && canonicalJson(payload).includes('"region":"void"')), true);
    assert.deepEqual(moved.receipt.randomDraws, []);
    assert.equal(ctx.state.phase, 'draw');
    assert.equal(ctx.state.pendingStartTurn, undefined);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === sourceIds[0])?.region, 'void');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 Raise Dead selects a public random cemetery minion before free placement', async () => {
  const raiseDeadId = 'raise-dead';
  const luckyCharmId = 'raise-dead-lucky-charm';
  const northCorpseId = 'raise-dead-north-corpse';
  const southCorpseId = 'raise-dead-south-corpse';
  const discardSiteId = 'raise-dead-discard-site';
  const fillerIds = Array.from({ length: 8 }, (_, index) => 'raise-dead-filler-' + (index + 1));
  const north: GameDeckSpec = {
    atlas: Array(6).fill(discardSiteId),
    avatar: 'raise-dead-north-avatar',
    spellbook: [raiseDeadId, luckyCharmId, northCorpseId, ...fillerIds.slice(0, 3)],
  };
  const south: GameDeckSpec = {
    atlas: Array(6).fill(discardSiteId),
    avatar: 'raise-dead-south-avatar',
    spellbook: [southCorpseId, ...fillerIds.slice(3)],
  };
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const cards = cardsFor({ north, south }, { manaCost: 0, thresholds }, undefined, {
    genesisDiscardTopSpells: 2,
  });
  cards[raiseDeadId] = {
    cardType: 'magic',
    manaCost: 0,
    summonRandomMinionFromAnyCemetery: true,
    thresholds,
  };
  cards[luckyCharmId] = {
    bearerControllerChoosesExtraRandomOutcome: true,
    cardType: 'artifact',
    manaCost: 0,
    thresholds,
  };
  for (const cardId of fillerIds) {
    cards[cardId] = { cardType: 'magic', healController: 1, manaCost: 0, thresholds };
  }
  for (const cardId of [northCorpseId, southCorpseId]) {
    cards[cardId] = {
      attack: 2,
      cardType: 'minion',
      defense: 3,
      ...(cardId === northCorpseId
        ? { genesisMayDamageTargetAdjacentUnit: 2 as const }
        : { genesisLoseControllerLife: 2 as const }),
      manaCost: 9,
      thresholds: { air: 0, earth: 0, fire: 0, water: 4 },
    };
  }
  const input = {
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic' as const,
      revisionId: 'synthetic-raise-dead-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north' as const,
    seed: 196,
  };
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [raiseDeadId]: {
        ...cards[raiseDeadId]!,
        summonRandomMinionFromAnyCemetery: false,
      } as unknown as GameCardDefinition,
    },
  }), /summonRandomMinionFromAnyCemetery must be true/);

  const gameManifest = createGameManifest(input);
  assert.deepEqual(gameManifest.cards[raiseDeadId], {
    cardType: 'magic',
    manaCost: 0,
    summonRandomMinionFromAnyCemetery: true,
    thresholds,
  });

  const emptyManifest = createGameManifest({
    ...input,
    cards: {
      ...gameManifest.cards,
      [discardSiteId]: { cardType: 'site', elements: ['earth'] },
    },
    seed: gameManifest.seed,
  });
  const blockedManifest = createGameManifest({
    ...input,
    cards: {
      ...gameManifest.cards,
      [discardSiteId]: {
        ...gameManifest.cards[discardSiteId],
        preventsUnitsWithPowerAtLeastFromEntering: 2,
      } as GameCardDefinition,
    },
  });

  await withSetup(emptyManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    assert.equal(ctx.state.players.north.hand.spellbook.some(({ cardId }) =>
      cardId === raiseDeadId), true);
    const emptyCast = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardId === raiseDeadId));
    assert.equal(emptyCast.accepted, true);
    if (!emptyCast.accepted) return;
    assert.deepEqual(emptyCast.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'magic-resolved',
    ]);
    assert.deepEqual(emptyCast.receipt.randomDraws, []);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.reset(blockedManifest);
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === luckyCharmId && descriptor.bearer?.kind === 'avatar');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    const blockedCorpse = ctx.state.players.south.cemetery.find(({ cardId }) =>
      cardId === southCorpseId);
    assert.ok(blockedCorpse);
    const blockedCorpseId = blockedCorpse.instanceId;
    const blockedCast = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardId === raiseDeadId));
    assert.equal(blockedCast.accepted, true);
    if (!blockedCast.accepted) return;
    const blockedChoice = (await ctx.legalActions('north'))
      .find(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
        && descriptor.outcomeInstanceId === blockedCorpseId);
    assert.ok(blockedChoice);
    const failedPlacement = await ctx.step(blockedChoice);
    assert.equal(failedPlacement.accepted, true);
    if (!failedPlacement.accepted) return;
    assert.equal(ctx.state.phase, 'main');
    assert.deepEqual(failedPlacement.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'dead-minion-selected',
      'minion-summon-failed',
      'magic-resolved',
    ]);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === blockedCorpseId), true);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.reset(gameManifest);
    await ctx.keep();
    await ctx.keep();
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await ctx.take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === luckyCharmId
      && descriptor.bearer?.kind === 'avatar');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await ctx.take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await ctx.take(({ descriptor }) => descriptor.kind === 'end-turn');
    await ctx.take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const northCorpse = ctx.state.players.north.cemetery.find(({ cardId }) =>
      cardId === northCorpseId);
    const southCorpse = ctx.state.players.south.cemetery.find(({ cardId }) =>
      cardId === southCorpseId);
    const raiseDead = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === raiseDeadId);
    assert.ok(northCorpse && southCorpse && raiseDead);
    const northCorpseInstanceId = northCorpse.instanceId;
    const southCorpseInstanceId = southCorpse.instanceId;
    const corpseIds = [northCorpseInstanceId, southCorpseInstanceId].sort();
    const raiseCasts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === raiseDead.instanceId);
    assert.equal(raiseCasts.length, 1);
    const raiseCast = raiseCasts[0]!;
    assert.deepEqual(raiseCast.descriptor, {
      cardId: raiseDeadId,
      cardInstanceId: raiseDead.instanceId,
      casterInstanceId: ctx.state.players.north.avatar.card.instanceId,
      kind: 'cast-magic',
    });

    await withFork(ctx, async (first) => {
      const committed = await first.step(raiseCast);
      assert.equal(committed.accepted, true);
      if (!committed.accepted) return;
      await withFork(ctx, async (second) => {
        const repeated = await second.step(raiseCast);
        assert.equal(repeated.accepted, true);
        if (!repeated.accepted) return;
        assert.deepEqual(repeated.receipt, committed.receipt);
      });
    });

    const committed = await ctx.step(raiseCast);
    assert.equal(committed.accepted, true);
    if (!committed.accepted) return;
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'resolve-random-outcome');
    assert.equal(ctx.state.phase, 'random-choice');
    assert.deepEqual(choices.flatMap(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
      ? [descriptor.outcomeInstanceId]
      : []).sort(), corpseIds);
    assert.equal(choices.every(({ label }) => label.startsWith('Lucky Charm chooses raise-dead-')), true);
    assert.deepEqual(committed.receipt.events, []);
    assert.equal(committed.receipt.randomDraws.length, 2);
    assert.equal(committed.receipt.randomDraws.every(({ purpose }) =>
      purpose === 'magic_random_dead_minion'), true);

    const northChoice = choices.find(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
      && descriptor.outcomeInstanceId === northCorpseInstanceId);
    assert.ok(northChoice);
    await withFork(ctx, async (northFork) => {
      const northSelected = await northFork.step(northChoice);
      assert.equal(northSelected.accepted, true);
      if (!northSelected.accepted) return;
      const genesisLabels = (await northFork.legalActions('north'))
        .filter(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4')
        .map(({ label }) => label);
      assert.equal(genesisLabels.some((label) => label.endsWith('; decline Genesis')), true);
      assert.equal(genesisLabels.some((label) => label.includes('; Genesis targets avatar ')), true);
      assert.equal(await northFork.verifyReplay(), true);
    });

    const forcedChoice = choices.find(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
      && descriptor.outcomeInstanceId === southCorpseInstanceId);
    assert.ok(forcedChoice);
    const selected = await ctx.step(forcedChoice);
    assert.equal(selected.accepted, true);
    if (!selected.accepted) return;
    assert.equal(ctx.state.phase, 'cemetery-summon');
    assert.deepEqual(selected.receipt.randomDraws, []);
    assert.deepEqual(selected.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'dead-minion-selected',
    ]);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === southCorpseInstanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === northCorpseInstanceId), true);

    const placements = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'summon-minion');
    assert.deepEqual([...new Set(placements.flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion' ? [descriptor.cell] : []))].sort(), ['C1', 'C4']);
    assert.equal(placements.every(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === southCorpseInstanceId
      && descriptor.manaCost === 0), true);
    const placement = placements.find(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C1' && !descriptor.region);
    assert.ok(placement);
    assert.equal(placement.descriptor.kind, 'summon-minion');
    if (placement.descriptor.kind !== 'summon-minion') return;
    const beforePlacementMana = ctx.state.players.north.mana;
    const lifeBeforePlacement = ctx.state.players.north.avatar.life;
    const beforeForgeHash = ctx.stateHash();
    const beforeForgeTranscript = ctx.session.transcript.length;
    const forgedDescriptor = { ...placement.descriptor, cell: 'A1' as const };
    const forged = await ctx.stepRequest({
      actionId: opaqueActionId(
        'sorcery-core-v1',
        'north',
        ctx.state.stateVersion,
        forgedDescriptor,
      ),
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(ctx.stateHash(), beforeForgeHash);
    assert.equal(ctx.session.transcript.length, beforeForgeTranscript);

    const placed = await ctx.step(placement);
    assert.equal(placed.accepted, true);
    if (!placed.accepted) return;
    const raised = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === southCorpseInstanceId);
    assert.deepEqual(raised && {
      controller: raised.controller,
      location: raised.location,
      owner: raised.owner,
      region: raised.region,
      summoningSickness: raised.summoningSickness,
    }, {
      controller: 'north',
      location: 'C1',
      owner: 'south',
      region: 'surface',
      summoningSickness: true,
    });
    assert.equal(ctx.state.players.north.mana, beforePlacementMana);
    assert.equal(ctx.state.players.north.avatar.life, lifeBeforePlacement - 2);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === southCorpseInstanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === northCorpseInstanceId), true);
    assert.deepEqual(placed.receipt.randomDraws, []);
    assert.deepEqual(placed.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'avatar-life-lost',
      'magic-resolved',
    ]);

    const reusedHash = ctx.stateHash();
    const reusedTranscript = ctx.session.transcript.length;
    const reused = await ctx.step(placement);
    assert.equal(reused.accepted, false);
    if (!reused.accepted) assert.equal(reused.reason.code, 'stale_version');
    assert.equal(ctx.stateHash(), reusedHash);
    assert.equal(ctx.session.transcript.length, reusedTranscript);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

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

