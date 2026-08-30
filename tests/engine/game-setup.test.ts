import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
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
  discardRandomCardInsteadOfMana?: true;
  diesAtEndOfControllerTurn?: true;
  gainsStealthAtEndOfTurn?: boolean;
  genesisDrawSpell?: boolean;
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
  ranged?: boolean;
  sacrificeMinionAtSummoningLocationForManaDiscount?: 2;
  shootsDragProjectile?: boolean;
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
  genesisDiscardTopSpells?: 2;
  genesisDrawSpellPerAdjacentSameCard?: boolean;
  genesisGainMana?: number;
  genesisGainManaIfOnlyControlledCopy?: 1;
  genesisHealNearbyAvatars?: 3;
  genesisImmobilizeNearbyUntilNextTurn?: true;
  genesisMayBottomNextSpell?: true;
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
        ...(facts.discardRandomCardInsteadOfMana === true
          ? { discardRandomCardInsteadOfMana: true as const }
          : {}),
        ...(facts.diesAtEndOfControllerTurn === true
          ? { diesAtEndOfControllerTurn: true as const }
          : {}),
        gainsStealthAtEndOfTurn: facts.gainsStealthAtEndOfTurn ?? false,
        genesisDrawSpell: facts.genesisDrawSpell ?? false,
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

function action(
  session: GameSession,
  predicate: (candidate: GameLegalAction) => boolean,
): GameLegalAction {
  const found = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  assert.ok(found, 'expected legal action');
  return found;
}

function accept(session: GameSession, candidate: GameLegalAction): GameSession {
  const result = stepGame(session, candidate);
  assert.equal(result.accepted, true);
  return result.session;
}

function keep(session: GameSession): GameSession {
  return accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
}

test('RULE-01 setup shuffles two decks, deals split hidden hands, and places Avatars', () => {
  const session = createGameSession(manifest(7));
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
  const northActions = legalGameActions(session.state, 'north');
  assert.equal(northActions.length, 76);
  assert.equal(legalGameActions(session.state, 'north'), northActions);
  assert.deepEqual(legalGameActions(session.state, 'south'), []);
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
  assert.doesNotThrow(() => createGameManifest({ ...input, cards }));
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      unused: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    },
  }), /exactly the deck-referenced definitions/);
  const firstSpell = decks.north.spellbook[0];
  assert.ok(firstSpell);
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
      [firstSpell]: { ...cards[firstSpell]!, genesisDrawSpell: 'yes' } as unknown as GameCardDefinition,
    },
  }), /genesisDrawSpell/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSite: true,
        genesisDrawSpell: true,
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
  }), /Waterbound with Ward or Stealth/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        ...cards[firstSpell]!,
        genesisDrawSpell: true,
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

test('TEST-03 different opponent hidden cards cannot change an observation or legal actions', () => {
  const first = createGameSession(manifest(9));
  const second = createGameSession(manifest(9, {
    north: {
      ...deck('north'),
      atlas: deck('north').atlas.map((_, index) => `alternate-site-${index + 1}`),
      spellbook: deck('north').spellbook.map((_, index) => `alternate-spell-${index + 1}`),
    },
  }));

  assert.notEqual(first.state.players.north.hand.atlas[0]?.cardId, second.state.players.north.hand.atlas[0]?.cardId);
  assert.equal(canonicalJson(observeGame(first.state, 'south')), canonicalJson(observeGame(second.state, 'south')));
  assert.equal(
    canonicalJson(legalGameActions(first.state, 'south')),
    canonicalJson(legalGameActions(second.state, 'south')),
  );
});

test('RULE-01 one mulligan returns at most three chosen cards to their deck bottoms and redraws', () => {
  const initial = createGameSession(manifest(11));
  const returned = initial.state.players.north.hand.atlas[0];
  assert.ok(returned);
  const mulligan = action(initial, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 1
      && descriptor.atlasOrder[0] === returned.instanceId
      && descriptor.spellbookOrder.length === 0);
  const result = stepGame(initial, mulligan);
  assert.equal(result.accepted, true);
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

test('RULE-01 first player skips its draw, establishes a domain, then second player chooses a deck', () => {
  let session = keep(createGameSession(manifest(13)));
  session = keep(session);

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

  const site = action(session, ({ descriptor }) => descriptor.kind === 'play-site');
  const siteId = site.descriptor.kind === 'play-site' ? site.descriptor.cardInstanceId : '';
  session = accept(session, site);
  assert.equal(session.state.realm.sites.C4?.instanceId, siteId);
  assert.equal(session.state.realm.sites.C4?.controller, 'north');
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.players.north.domainEstablished, true);
  assert.equal(session.state.players.north.mana, 1);
  const afterSiteKinds = legalGameActions(session.state, 'north').map(({ descriptor }) => descriptor.kind);
  assert.equal(afterSiteKinds.includes('play-site'), false);
  assert.equal(afterSiteKinds.includes('draw-site'), false);
  assert.equal(afterSiteKinds.includes('end-turn'), true);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.state.turnNumber, 2);
  assert.equal(session.state.activeSeat, 'south');
  assert.equal(session.state.phase, 'draw');
  assert.deepEqual(
    legalGameActions(session.state, 'south').map(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone),
    ['atlas', 'spellbook'],
  );

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.players.south.hand.atlas.length, 4);
  assert.deepEqual(session.transcript.at(-1)?.events[0]?.payload, { seat: 'south', zone: 'atlas' });
  assert.doesNotMatch(canonicalJson(session.transcript.at(-1)?.events[0]?.payload ?? null), /south-site-/);
  assert.equal(verifyGameReplay(session), true);
});

function northSecondMain(seed = 23, shortDecks = false, spell?: SpellFacts): GameSession {
  const options = shortDecks
    ? { north: deck('north', 3, 4), south: deck('south', 3, 4), ...(spell ? { spell } : {}) }
    : spell ? { spell } : {};
  let session = keep(createGameSession(manifest(seed, options)));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  return accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
}

test('RULE-02 sites expand through unoccupied orthogonal cells controlled by their player', () => {
  let session = northSecondMain();
  const northCard = session.state.players.north.hand.atlas[0];
  assert.ok(northCard);
  const northCells = legalGameActions(session.state, 'north')
    .filter(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cardInstanceId === northCard.instanceId)
    .map(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell);
  assert.deepEqual(northCells, ['B4', 'C3', 'D4']);

  const playC3 = action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northCard.instanceId
      && descriptor.cell === 'C3');
  const beforeHand = session.state.players.north.hand.atlas.length;
  const result = stepGame(session, playC3);
  assert.equal(result.accepted, true);
  session = result.session;
  assert.equal(session.state.realm.sites.C3?.instanceId, northCard.instanceId);
  assert.equal(session.state.realm.sites.C3?.controller, 'north');
  assert.equal(session.state.players.north.hand.atlas.length, beforeHand - 1);
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.players.north.mana, 2);
  assert.equal(result.receipt.events[0]?.type, 'site-played');
  const afterPlayKinds = legalGameActions(session.state, 'north').map(({ descriptor }) => descriptor.kind);
  assert.equal(afterPlayKinds.includes('play-site'), false);
  assert.equal(afterPlayKinds.includes('draw-site'), false);
  assert.equal(afterPlayKinds.includes('end-turn'), true);
  assert.equal(verifyGameReplay(session), true);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const expansion = [...new Set(legalGameActions(session.state, 'north')
    .filter(({ descriptor }) => descriptor.kind === 'play-site')
    .map(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell))];
  assert.deepEqual(expansion, ['B3', 'B4', 'D3', 'D4']);
  assert.equal(session.state.players.north.avatar.tapped, false);
  assert.equal(session.state.players.north.mana, 2);
});

test('RULE-02 a player with no sites recovers at the closest available cell', () => {
  const base = manifest(267);
  const preview = createGameSession(base);
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
  let session = keep(keep(createGameSession(gameManifest)));
  const sourceCard = session.state.players.north.hand.atlas.find(({ cardId }) => cardId === sourceCardId);
  assert.ok(sourceCard);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === sourceCard.instanceId);
  take(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
    && descriptor.sourceSiteInstanceId === sourceCard.instanceId
    && descriptor.targetSiteInstanceId === sourceCard.instanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const zeroDomain = session;
  const recoveryCard = zeroDomain.state.players.north.hand.atlas[0];
  const rubble = zeroDomain.state.realm.sites.C4;
  assert.ok(recoveryCard);
  assert.ok(rubble && 'rubble' in rubble);
  assert.equal(zeroDomain.state.players.north.domainEstablished, true);
  assert.equal(zeroDomain.state.players.north.avatar.location, 'C4');
  assert.equal(Object.values(zeroDomain.state.realm.sites)
    .some(({ controller }) => controller === 'north'), false);
  assert.deepEqual(legalGameActions(zeroDomain.state, 'north')
    .flatMap(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === recoveryCard.instanceId ? [descriptor.cell] : []), ['C4']);

  const southSite = zeroDomain.state.realm.sites.C1;
  assert.ok(southSite && !('rubble' in southSite));
  const tiedCheckpoint: GameSession = {
    ...zeroDomain,
    state: {
      ...zeroDomain.state,
      players: {
        ...zeroDomain.state.players,
        north: {
          ...zeroDomain.state.players.north,
          avatar: { ...zeroDomain.state.players.north.avatar, location: 'C3' },
        },
      },
      realm: {
        ...zeroDomain.state.realm,
        sites: { C3: southSite, C4: rubble },
      },
    },
  };
  assert.deepEqual(legalGameActions(tiedCheckpoint.state, 'north')
    .flatMap(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === recoveryCard.instanceId ? [descriptor.cell] : []),
  ['B3', 'C2', 'C4', 'D3']);
  const beforeForge = canonicalJson(tiedCheckpoint.state);
  const forged = stepGame(tiedCheckpoint, {
    actionId: opaqueActionId('sorcery-core-v1', 'north', tiedCheckpoint.state.stateVersion, {
      cardId: recoveryCard.cardId,
      cardInstanceId: recoveryCard.instanceId,
      cell: 'A1',
      kind: 'play-site',
    }),
    seat: 'north',
    stateVersion: tiedCheckpoint.state.stateVersion,
  });
  assert.equal(forged.accepted, false);
  assert.equal(forged.reason.code, 'unknown_action');
  assert.equal(canonicalJson(forged.session.state), beforeForge);

  const handBefore = zeroDomain.state.players.north.hand.atlas.length;
  const recovered = stepGame(zeroDomain, action(zeroDomain, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === recoveryCard.instanceId
      && descriptor.cell === 'C4'));
  assert.equal(recovered.accepted, true);
  if (!recovered.accepted) return;
  assert.deepEqual(recovered.receipt.events.map(({ type }) => type), ['rubble-replaced', 'site-played']);
  assert.deepEqual(recovered.receipt.randomDraws, []);
  assert.equal(recovered.session.state.realm.sites.C4?.controller, 'north');
  assert.equal('rubble' in recovered.session.state.realm.sites.C4!, false);
  assert.equal(observeGame(recovered.session.state, 'north').players.north.affinity.earth, 1);
  assert.equal(recovered.session.state.players.north.avatar.tapped, true);
  assert.equal(recovered.session.state.players.north.hand.atlas.length, handBefore - 1);
  assert.equal(recovered.session.state.players.north.mana, 1);
  assert.equal(verifyGameReplay(recovered.session), true);
});

test('RULE-02 the Avatar may draw a private site instead of playing one', () => {
  let session = northSecondMain(29);
  const before = session.state.players.north;
  const drawn = before.atlas[0];
  assert.ok(drawn);
  const draw = action(session, ({ descriptor }) => descriptor.kind === 'draw-site');
  const result = stepGame(session, draw);
  assert.equal(result.accepted, true);
  session = result.session;

  assert.equal(session.state.players.north.atlas.length, before.atlas.length - 1);
  assert.equal(session.state.players.north.hand.atlas.length, before.hand.atlas.length + 1);
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.deepEqual(result.receipt.events[0]?.payload, { seat: 'north' });
  assert.equal(result.receipt.events[0]?.type, 'site-drawn');
  assert.equal(canonicalJson(result.receipt.events[0]?.payload ?? null).includes(drawn.cardId), false);
  assert.equal(canonicalJson(observeGame(session.state, 'south')).includes(drawn.cardId), false);
  const afterDrawKinds = legalGameActions(session.state, 'north').map(({ descriptor }) => descriptor.kind);
  assert.equal(afterDrawKinds.includes('play-site'), false);
  assert.equal(afterDrawKinds.includes('draw-site'), false);
  assert.equal(afterDrawKinds.includes('end-turn'), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 a draw-spell Avatar pays its tap cost and keeps the drawn identity private', () => {
  let session = keep(createGameSession(manifest(30, {
    avatar: { attack: 1, defense: 1, drawSpell: true, life: 20 },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const before = session.state.players.north;
  const drawn = before.spellbook[0];
  assert.ok(drawn);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'draw-spell'));
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.players.north.spellbook.length, before.spellbook.length - 1);
  assert.equal(session.state.players.north.hand.spellbook.length, before.hand.spellbook.length + 1);
  assert.equal(session.transcript.at(-1)?.events[0]?.type, 'spell-drawn');
  assert.doesNotMatch(canonicalJson(session.transcript.at(-1)?.events[0]?.payload ?? null), /north-spell-/);
  assert.doesNotMatch(canonicalJson(observeGame(session.state, 'south')), new RegExp(drawn.cardId));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-02 drawing a site from an empty Atlas pays the tap cost and loses', () => {
  let session = northSecondMain(31, true);
  assert.equal(session.state.players.north.atlas.length, 0);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'draw-site'));

  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.deepEqual(session.state.terminal, {
    loser: 'north',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'south',
  });
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-02 forged spatial actions cannot mutate the game', () => {
  const session = northSecondMain(37);
  const beforeState = canonicalJson(session.state);
  const result = stepGame(session, {
    actionId: 'sha256:2222222222222222222222222222222222222222222222222222222222222222',
    seat: 'north',
    stateVersion: session.state.stateVersion,
  });

  assert.equal(result.accepted, false);
  assert.equal(result.reason.code, 'unknown_action');
  assert.equal(canonicalJson(result.session.state), beforeState);
  assert.equal(result.session.transcript.length, session.transcript.length);
});

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
  const survivorCardId = southMinions[1]?.cardId;
  const artifactCardId = southMinions[2]?.cardId;
  assert.ok(sourceCardId);
  assert.ok(protectedCardId);
  assert.ok(targetCardId);
  assert.ok(replacementCardId);
  assert.ok(drownedCardId);
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
  cards[drownedCardId] = {
    attack: 1,
    cardType: 'minion',
    defense: 1,
    manaCost: 0,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
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
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === drownedCard.instanceId
      && descriptor.cell === 'C2'
      && descriptor.region === 'underwater'));
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
    'minion-died',
    'rubble-created',
    'rubble-created',
  ]);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === sourceCard.instanceId), true);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === targetCard.instanceId), true);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === drownedCard.instanceId), true);
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

test('RULE-03/04 a Spellcaster pays mana and summons a minion atop a controlled site', () => {
  let session = keep(createGameSession(manifest(41)));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  const before = session.state.players.north;
  const summons = legalGameActions(session.state, 'north')
    .filter(({ descriptor }) => descriptor.kind === 'summon-minion');
  assert.equal(observeGame(session.state, 'north').players.north.affinity.earth, 1);
  assert.equal(summons.length, 3);
  assert.ok(summons.every(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4'
      && descriptor.casterInstanceId === before.avatar.card.instanceId
      && descriptor.manaCost === 1));

  const summon = summons[0];
  assert.ok(summon);
  const result = stepGame(session, summon);
  assert.equal(result.accepted, true);
  session = result.session;
  const unit = session.state.realm.units[0];
  assert.ok(unit);
  assert.equal(session.state.players.north.hand.spellbook.length, before.hand.spellbook.length - 1);
  assert.equal(session.state.players.north.mana, 0);
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
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion'), false);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === unit.instanceId), false);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.state.realm.units[0]?.summoningSickness, false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 token Magic summons deterministically and tokens banish instead of entering a cemetery', () => {
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  const emptyCast = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target === undefined));
  assert.equal(emptyCast.accepted, true);
  if (!emptyCast.accepted) return;
  session = emptyCast.session;
  assert.deepEqual(emptyCast.receipt.events.map(({ type }) => type), ['magic-cast', 'magic-resolved']);
  assert.equal(session.state.realm.units.some(({ source }) => source === 'token'), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'B2');
  const attacker = session.state.realm.units.find(({ controller, location }) =>
    controller === 'south' && location === 'B2');
  assert.ok(attacker);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

  const tokenCast = action(session, ({ descriptor }) => descriptor.kind === 'cast-magic');
  assert.equal(tokenCast.descriptor.kind === 'cast-magic' && tokenCast.descriptor.target, undefined);
  const sourceInstanceId = tokenCast.descriptor.kind === 'cast-magic'
    ? tokenCast.descriptor.cardInstanceId
    : '';
  const summoned = stepGame(session, tokenCast);
  assert.equal(summoned.accepted, true);
  if (!summoned.accepted) return;
  session = summoned.session;
  const summonEvents = summoned.receipt.events.filter(({ payload, type }) =>
    type === 'minion-summoned'
      && (payload as { sourceInstanceId?: string }).sourceInstanceId === sourceInstanceId);
  assert.deepEqual(summonEvents.map(({ payload }) =>
    (payload as { cell: string }).cell), ['B3', 'C3']);
  const tokens = session.state.realm.units
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
  assert.deepEqual(observeGame(session.state, 'south').realm.units
    .filter(({ token }) => token)
    .map(({ attack, defense, instanceId }) => ({ attack, defense, instanceId })),
  tokens.map(({ instanceId }) => ({ attack: 1, defense: 1, instanceId })));
  assert.deepEqual(summonEvents.map(({ payload }) =>
    (payload as { instanceId: string }).instanceId), tokens.map(({ instanceId }) => instanceId));
  assert.deepEqual(summoned.receipt.randomDraws, []);
  const killed = tokens[0]!;
  const survivor = tokens[1]!;

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === attacker.instanceId
    && descriptor.to.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === killed.instanceId);
  const fight = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(fight.accepted, true);
  if (!fight.accepted) return;
  session = fight.session;
  const tokenExitEvents = fight.receipt.events.filter(({ payload, type }) =>
    (type === 'minion-died' || type === 'minion-banished')
      && (payload as { instanceId?: string }).instanceId === killed.instanceId);
  assert.deepEqual(tokenExitEvents.map(({ type }) => type), ['minion-died', 'minion-banished']);
  const diedIndex = fight.receipt.events.indexOf(tokenExitEvents[0]!);
  assert.equal(fight.receipt.events[diedIndex + 1], tokenExitEvents[1]);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === killed.instanceId), false);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === killed.instanceId), false);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === killed.instanceId), false);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === survivor.instanceId)?.instanceId, survivor.instanceId);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Aramos Mercenaries may discard a deterministic random hand card instead of paying mana', () => {
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

  let session = keep(keep(createGameSession(gameManifest)));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  assert.equal(session.state.players.north.mana, 1);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion'), false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  assert.equal(session.state.players.north.mana, 2);

  const alternateActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.paymentMode === 'random-card-discard');
  assert.ok(alternateActions.length > 0);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
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
    ...session.state.players.north.hand.atlas.map((candidate) => ({ ...candidate, zone: 'atlas' as const })),
    ...session.state.players.north.hand.spellbook
      .filter(({ instanceId }) => instanceId !== castInstanceId)
      .map((candidate) => ({ ...candidate, zone: 'spellbook' as const })),
  ];
  assert.ok(eligible.length > 0);
  assert.deepEqual(new Set(eligible.map(({ zone }) => zone)), new Set(['atlas', 'spellbook']));
  const handBefore = session.state.players.north.hand;
  const southBefore = canonicalJson(observeGame(session.state, 'south'));
  assert.ok(eligible.every(({ cardId, instanceId }) =>
    !southBefore.includes(cardId) && !southBefore.includes(instanceId)));
  const checkpoint = session;
  const first = stepGame(checkpoint, cast);
  const second = stepGame(checkpoint, cast);
  assert.equal(first.accepted, true);
  assert.equal(second.accepted, true);
  assert.deepEqual(second.receipt.randomDraws, first.receipt.randomDraws);
  assert.deepEqual(second.receipt.events, first.receipt.events);
  assert.equal(hashGameState(second.session.state), hashGameState(first.session.state));
  session = first.session;

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
  assert.equal(session.state.players.north.mana, 2);
  assert.equal(session.state.players.north.hand.atlas.length,
    handBefore.atlas.length - (discardedZone === 'atlas' ? 1 : 0));
  assert.equal(session.state.players.north.hand.spellbook.length,
    handBefore.spellbook.length - 1 - (discardedZone === 'spellbook' ? 1 : 0));
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === discardedInstanceId), true);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === castInstanceId), false);
  assert.ok(first.receipt.randomDraws.length > 0);
  assert.ok(first.receipt.randomDraws.every(({ domain, purpose }) => {
    if (typeof domain !== 'object' || domain === null || Array.isArray(domain)) return false;
    const drawDomain = domain as Readonly<Record<string, unknown>>;
    return purpose === 'summon_random_card_discard_cost'
      && drawDomain.kind === 'card_index_candidate'
      && drawDomain.exclusiveMaximum === eligible.length;
  }));
  const southAfter = canonicalJson(observeGame(session.state, 'south'));
  assert.ok(southAfter.includes(String(discardedCardId)));
  assert.ok(southAfter.includes(String(discardedInstanceId)));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Gnarled Wendigo sacrifices local minions before paying its discounted summon', () => {
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === localMinionId
    && descriptor.cell === 'C4'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === submergedMinionId
    && descriptor.cell === 'C4'
    && descriptor.region === 'underwater');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === enemyMinionId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === secondLocalMinionId
    && descriptor.cell === 'C4'
    && descriptor.region === undefined);
  assert.equal(session.state.players.north.mana, 4);

  const local = session.state.realm.units.find(({ cardId }) => cardId === localMinionId);
  const secondLocal = session.state.realm.units.find(({ cardId }) => cardId === secondLocalMinionId);
  const submerged = session.state.realm.units.find(({ cardId }) => cardId === submergedMinionId);
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === enemyMinionId);
  assert.ok(local && secondLocal && submerged && enemy);
  const wendigoActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
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

  const checkpoint = session;
  const checkpointHash = hashGameState(checkpoint.state);
  const forged = stepGame(checkpoint, {
    actionId: `${cast.actionId}:forged`,
    seat: 'north',
    stateVersion: checkpoint.state.stateVersion,
  });
  assert.equal(forged.accepted, false);
  assert.equal(forged.reason?.code, 'unknown_action');
  assert.equal(hashGameState(forged.session.state), checkpointHash);
  assert.equal(forged.session.transcript.length, checkpoint.transcript.length);

  const result = stepGame(checkpoint, cast);
  assert.equal(result.accepted, true);
  if (!result.accepted) throw new Error('expected Gnarled Wendigo summon to be accepted');
  session = result.session;
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
  assert.equal(session.state.players.north.mana, 0);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === local.instanceId), true);
  assert.equal(session.state.realm.units.some(({ instanceId }) =>
    instanceId === local.instanceId), false);
  const wendigo = session.state.realm.units.find(({ cardId }) => cardId === wendigoId);
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
  assert.equal(session.state.stateVersion, checkpoint.state.stateVersion + 1);
  assert.equal(verifyGameReplay(session), true);
});

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

test('RULE-03 printed Spellcasters cast while tapped or summoning sick from their own location', () => {
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === tappedCasterId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === targetId && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === stealthedTargetId && descriptor.cell === 'C1');
  const target = session.state.realm.units.find(({ cardId }) => cardId === targetId);
  const stealthedTarget = session.state.realm.units.find(({ cardId }) =>
    cardId === stealthedTargetId);
  assert.ok(target);
  assert.ok(stealthedTarget);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  const tappedCaster = session.state.realm.units.find(({ cardId }) => cardId === tappedCasterId);
  assert.ok(tappedCaster);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === tappedCaster.instanceId
    && descriptor.from.cell === 'C4'
    && descriptor.to.cell === 'C3'
    && descriptor.path.length === 2);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === activeCasterId
    && descriptor.casterInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === disabledCasterId
    && descriptor.casterInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.cell === 'C3');
  const activeCaster = session.state.realm.units.find(({ cardId }) => cardId === activeCasterId);
  const disabledCaster = session.state.realm.units.find(({ cardId }) => cardId === disabledCasterId);
  const freeze = session.state.players.north.hand.spellbook.find(({ cardId }) => cardId === freezeId);
  const summonedCard = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === summonId);
  assert.ok(activeCaster);
  assert.ok(disabledCaster);
  assert.ok(freeze);
  assert.ok(summonedCard);
  take(({ descriptor }) => descriptor.kind === 'activate-mana'
    && descriptor.unitInstanceId === tappedCaster.instanceId);
  const checkpoint = session;
  assert.deepEqual({
    stealthed: activeCaster.stealthed,
    summoningSickness: activeCaster.summoningSickness,
    tapped: checkpoint.state.realm.units.find(({ instanceId }) =>
      instanceId === activeCaster.instanceId)?.tapped,
  }, { stealthed: true, summoningSickness: true, tapped: false });
  assert.equal(checkpoint.state.realm.units.find(({ instanceId }) =>
    instanceId === tappedCaster.instanceId)?.tapped, true);
  assert.equal(observeGame(checkpoint.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === disabledCaster.instanceId)?.disabled, true);

  const actions = legalGameActions(checkpoint.state, 'north');
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
    checkpoint.state.players.north.avatar.card.instanceId,
    tappedCaster.instanceId,
  ].sort());
  assert.equal(summonCasters.has(disabledCaster.instanceId), false);

  const frozen = accept(checkpoint, targetFreezes[0]!);
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
  assert.equal(verifyGameReplay(frozen), true);

  const summoned = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(summoned), true);
});

test('RULE-03/05 targeted Magic pays mana, damages any unit, resolves Deathrite, and enters the cemetery', () => {
  const decks = { north: deck('magic-north', 4, 6), south: deck('magic-south', 4, 6) };
  const cards = cardsFor(decks, {
    deathriteDrawSite: true,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, { attack: 1, defense: 1, drawSpell: false, life: 1 }, { elements: ['air'] });
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

test('RULE-03 Magic targets stay in the caster region and exclude enemy Stealth', () => {
  const targetIsLegal = (
    spell: SpellFacts,
    region: 'surface' | 'underground',
    targetNearby = false,
  ): boolean => {
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
    let session = keep(createGameSession(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-magic-target-${region}-v1`,
      },
      cards,
      decks,
      firstSeat: 'north',
      seed: region === 'surface' ? 149 : 150,
    })));
    session = keep(session);
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && (descriptor.region ?? 'surface') === region));
    const target = session.state.realm.units.find(({ controller }) => controller === 'south');
    assert.ok(target);
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    assert.equal(verifyGameReplay(session), true);
    if (targetNearby) {
      assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.target !== undefined
          && descriptor.target.kind === 'avatar'
          && descriptor.target.seat === 'north'), true);
    }
    return legalGameActions(session.state, 'north').some(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.target !== undefined
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === target.instanceId);
  };

  assert.equal(targetIsLegal({
    manaCost: 1,
    stealth: true,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, 'surface'), false);
  assert.equal(targetIsLegal({
    burrowing: true,
    manaCost: 1,
    mustBeCastBurrowed: true,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, 'underground'), false);
  assert.equal(targetIsLegal({
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  }, 'surface', true), false);
});

test('RULE-03 Lash damages then untaps only a surviving nearby minion target', () => {
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
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
  const nearbyTarget = session.state.realm.units.find(({ controller, location }) =>
    controller === 'south' && location === 'C4');
  const distantTarget = session.state.realm.units.find(({ controller, location }) =>
    controller === 'south' && location === 'C1');
  assert.ok(nearbyTarget);
  assert.ok(distantTarget);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'activate-mana'
    && descriptor.unitInstanceId === nearbyTarget.instanceId);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === nearbyTarget.instanceId)?.tapped, true);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const spell = session.state.players.north.hand.spellbook.find(({ cardId }) => {
    const definition = gameManifest.cards[cardId];
    return definition?.cardType === 'magic' && definition.untapTargetMinionAfterDamage === true;
  });
  assert.ok(spell);
  const lashActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
  assert.deepEqual(lashActions.map(({ descriptor }) =>
    descriptor.kind === 'cast-magic' ? descriptor.target?.instanceId : undefined), [nearbyTarget.instanceId]);
  assert.equal(lashActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'avatar'), false);
  assert.equal(lashActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.instanceId === distantTarget.instanceId), false);

  const before = session.state;
  const result = stepGame(session, lashActions[0]!);
  assert.equal(result.accepted, true);
  const targetAfter = result.session.state.realm.units.find(({ instanceId }) =>
    instanceId === nearbyTarget.instanceId);
  assert.equal(targetAfter?.damage, 1);
  assert.equal(targetAfter?.tapped, false);
  assert.equal(result.session.state.players.north.mana, before.players.north.mana - 1);
  assert.equal(result.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === spell.instanceId), true);
  assert.equal(result.session.state.stateVersion, before.stateVersion + 1);
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
  assert.equal(verifyGameReplay(result.session), true);
});

test('RULE-03 Freeze disables a nearby minion until the caster next Start Phase', () => {
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
  let gameManifest: GameManifest | undefined;
  for (let seed = 156; seed < 556; seed += 1) {
    const candidate = createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: 'synthetic-freeze-fixture-v1',
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    });
    const preview = createGameSession(candidate).state.players;
    if (preview.north.hand.spellbook.some(({ cardId }) => cardId === allyCardId)
      && preview.south.hand.spellbook.some(({ cardId }) => cardId === targetCardId)
      && preview.south.hand.spellbook.some(({ cardId }) => cardId === stealthCardId)
      && preview.south.hand.spellbook.some(({ cardId }) => cardId === wardCardId)) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === allyCardId && descriptor.cell === 'C4');
  const ally = session.state.realm.units.find(({ cardId }) => cardId === allyCardId);
  assert.ok(ally);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === targetCardId && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === stealthCardId && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === wardCardId && descriptor.cell === 'C2');
  const target = session.state.realm.units.find(({ cardId }) => cardId === targetCardId);
  const stealthed = session.state.realm.units.find(({ cardId }) => cardId === stealthCardId);
  const wardedTarget = session.state.realm.units.find(({ cardId }) => cardId === wardCardId);
  assert.ok(target);
  assert.ok(stealthed);
  assert.ok(wardedTarget);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.from.cell === 'C4'
    && descriptor.to.cell === 'C3'
    && descriptor.path.length === 2);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const checkpoint = session;
  const freezeCards = checkpoint.state.players.north.hand.spellbook.filter(({ cardId }) =>
    gameManifest.cards[cardId]?.cardType === 'magic');
  assert.equal(freezeCards.length, 2);
  const firstTargets = legalGameActions(checkpoint.state, 'north').flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === freezeCards[0]?.instanceId
      && descriptor.target
      ? [descriptor.target.instanceId]
      : []);
  assert.equal(firstTargets.includes(ally.instanceId), true);
  assert.equal(firstTargets.includes(target.instanceId), true);
  assert.equal(firstTargets.includes(wardedTarget.instanceId), true);
  assert.equal(firstTargets.includes(stealthed.instanceId), false);
  assert.equal(firstTargets.includes(checkpoint.state.players.north.avatar.card.instanceId), false);

  let unfrozen = accept(checkpoint, action(checkpoint, ({ descriptor }) => descriptor.kind === 'end-turn'));
  unfrozen = accept(unfrozen, action(unfrozen, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  const unfrozenKinds = legalGameActions(unfrozen.state, 'south').flatMap(({ descriptor }) =>
    'unitInstanceId' in descriptor && descriptor.unitInstanceId === target.instanceId
      ? [descriptor.kind]
      : 'shooterInstanceId' in descriptor && descriptor.shooterInstanceId === target.instanceId
        ? [descriptor.kind]
        : []);
  assert.equal(unfrozenKinds.includes('move-and-attack'), true);
  assert.equal(unfrozenKinds.includes('shoot-projectile'), true);
  assert.equal(unfrozenKinds.includes('activate-mana'), true);

  const allied = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === freezeCards[0]?.instanceId
      && descriptor.target?.instanceId === ally.instanceId));
  const disabledAlly = allied.state.realm.units.find(({ instanceId }) => instanceId === ally.instanceId);
  assert.equal(observeGame(allied.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === ally.instanceId)?.disabled, true);
  assert.deepEqual({ stealthed: disabledAlly?.stealthed, warded: disabledAlly?.warded }, {
    stealthed: false,
    warded: false,
  });
  assert.deepEqual(allied.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'minion-disabled',
    'magic-resolved',
  ]);
  assert.deepEqual(allied.transcript.at(-1)?.events[1]?.payload, {
    expiresAtSeat: 'north',
    instanceId: ally.instanceId,
    seat: 'north',
    sourceInstanceId: freezeCards[0]!.instanceId,
    stealthRemoved: true,
    wardRemoved: true,
  });
  assert.equal(verifyGameReplay(allied), true);

  const warded = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
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
  assert.equal(warded.session.state.realm.units.find(({ instanceId }) =>
    instanceId === wardedTarget.instanceId)?.disableEffects, undefined);
  assert.equal(verifyGameReplay(warded.session), true);
  const firstFreeze = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === freezeCards[0]?.instanceId
      && descriptor.target?.instanceId === target.instanceId));
  assert.equal(firstFreeze.accepted, true);
  if (!firstFreeze.accepted) return;
  session = firstFreeze.session;
  const freeze = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === freezeCards[1]?.instanceId
      && descriptor.target?.instanceId === target.instanceId));
  assert.equal(freeze.accepted, true);
  if (!freeze.accepted) return;
  session = freeze.session;
  const disabledTarget = session.state.realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId);
  assert.deepEqual(disabledTarget?.disableEffects, [{
    expiresAtSeat: 'north',
    sourceInstanceId: freezeCards[0]!.instanceId,
  }, {
    expiresAtSeat: 'north',
    sourceInstanceId: freezeCards[1]!.instanceId,
  }]);
  assert.equal(observeGame(session.state, 'south').realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId)?.disabled, true);
  assert.equal(observeGame(session.state, 'south').players.south.affinity.air, 0);
  assert.deepEqual(freeze.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'minion-disabled',
    'magic-resolved',
  ]);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) =>
    type === 'minion-disable-expired'), false);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  assert.equal(observeGame(session.state, 'south').realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId)?.disabled, true);
  const disabledKinds = legalGameActions(session.state, 'south').flatMap(({ descriptor }) =>
    'unitInstanceId' in descriptor && descriptor.unitInstanceId === target.instanceId
      ? [descriptor.kind]
      : 'shooterInstanceId' in descriptor && descriptor.shooterInstanceId === target.instanceId
        ? [descriptor.kind]
        : []);
  assert.deepEqual(disabledKinds, []);
  const expiration = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(expiration.accepted, true);
  if (!expiration.accepted) return;
  session = expiration.session;
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
  const expiredTarget = session.state.realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId);
  assert.equal(expiredTarget?.disableEffects, undefined);
  assert.deepEqual({ warded: expiredTarget?.warded }, { warded: false });
  assert.equal(observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId)?.disabled, false);
  assert.equal(observeGame(session.state, 'north').players.south.affinity.air, 1);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 disabling a subsurface minion immediately settles its region', () => {
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
  let checkpoint = keep(keep(createGameSession(gameManifest)));
  checkpoint = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));

  const freezeAt = (region: 'underwater' | 'void', cell: 'C4' | 'B4') => {
    let session = checkpoint;
    const take = (predicate: Parameters<typeof action>[1]): void => {
      session = accept(session, action(session, predicate));
    };
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === casterCardId && descriptor.cell === cell
      && descriptor.region === region);
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetCardId && descriptor.cell === cell
      && descriptor.region === region);
    const caster = session.state.realm.units.find(({ cardId }) => cardId === casterCardId);
    const target = session.state.realm.units.find(({ cardId }) => cardId === targetCardId);
    const freeze = session.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === freezeCardId);
    assert.ok(caster && target && freeze);
    const result = stepGame(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === freeze.instanceId
        && descriptor.casterInstanceId === caster.instanceId
        && descriptor.target?.instanceId === target.instanceId));
    assert.equal(result.accepted, true);
    if (!result.accepted) throw new Error('expected Freeze to resolve');
    return { caster, freeze, result, target };
  };

  const underwater = freezeAt('underwater', 'C4');
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
  assert.equal(verifyGameReplay(underwater.result.session), true);

  const voided = freezeAt('void', 'B4');
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
  assert.equal(verifyGameReplay(voided.result.session), true);
});

test('RULE-03 Lightning Bolt targets a location and deterministically damages one random unit there', () => {
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
  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  for (let count = 0; count < 2; count += 1) {
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  }
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const bolt = session.state.players.north.hand.spellbook[0];
  assert.ok(bolt);
  const occupants = [
    session.state.players.south.avatar.card.instanceId,
    ...session.state.realm.units
      .filter(({ location, region }) => location === 'C1' && region === 'surface')
      .map(({ instanceId }) => instanceId),
  ].sort();
  assert.equal(occupants.length, 3);
  assert.equal(session.state.realm.units.every(({ stealthed }) => stealthed), true);
  const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === bolt.instanceId);
  assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.targetLocation
    ? [`${descriptor.targetLocation.cell}:${descriptor.targetLocation.region}`]
    : []), ['C1:surface', 'C4:surface']);
  assert.equal(casts.some(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target !== undefined), false);

  const before = session.state;
  const result = stepGame(session, casts.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.targetLocation?.cell === 'C1')!);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
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
  const selectedDamage = selectedId === session.state.players.south.avatar.card.instanceId
    ? 20 - session.state.players.south.avatar.life
    : session.state.realm.units.find(({ instanceId }) => instanceId === selectedId)?.damage;
  assert.equal(selectedDamage, 3);
  assert.equal(session.state.stateVersion, before.stateVersion + 1);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) => instanceId === bolt.instanceId), true);
  assert.equal(result.receipt.events.at(-1)?.type, 'magic-resolved');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Minor Explosion damages every unit at a location up to two cardinal steps away', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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
  cards[stealthCardId] = { ...baseCards[stealthCardId]!, stealth: true } as GameCardDefinition;
  const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  for (const cardId of [deathriteCardId, wardedCardId, stealthCardId]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === cardId && descriptor.cell === 'C2');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.players.south.avatar.card.instanceId
    && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === allyCardId && descriptor.cell === 'C2');

  const checkpoint = session;
  const explosion = checkpoint.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === explosionCardId);
  const ally = checkpoint.state.realm.units.find(({ cardId }) => cardId === allyCardId);
  const deathrite = checkpoint.state.realm.units.find(({ cardId }) => cardId === deathriteCardId);
  const warded = checkpoint.state.realm.units.find(({ cardId }) => cardId === wardedCardId);
  const stealthed = checkpoint.state.realm.units.find(({ cardId }) => cardId === stealthCardId);
  assert.ok(explosion);
  assert.ok(ally);
  assert.ok(deathrite);
  assert.ok(warded);
  assert.ok(stealthed);
  const casts = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === explosion.instanceId);
  assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.targetLocation ? [descriptor.targetLocation.cell] : []), ['C2', 'C3', 'C4']);
  assert.equal(casts.some(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target !== undefined), false);

  const beforeMana = checkpoint.state.players.north.mana;
  const result = stepGame(checkpoint, casts.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.targetLocation?.cell === 'C2')!);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
  const affectedIds = [
    checkpoint.state.players.south.avatar.card.instanceId,
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
  assert.equal(session.state.stateVersion, checkpoint.state.stateVersion + 1);
  assert.equal(session.state.players.north.mana, beforeMana - 1);
  assert.equal(session.state.players.south.avatar.life, 20);
  assert.equal(result.receipt.events.some(({ payload, type }) => type === 'avatar-life-lost'
    && (payload as { amount: number }).amount === 3), true);
  assert.equal(result.receipt.events.some(({ payload, type }) => type === 'avatar-healed'
    && (payload as { amount: number }).amount === 3
    && (payload as { sourceInstanceId: string }).sourceInstanceId === deathrite.instanceId), true);
  const survivingWard = session.state.realm.units.find(({ instanceId }) => instanceId === warded.instanceId);
  assert.deepEqual({ damage: survivingWard?.damage, warded: survivingWard?.warded }, {
    damage: 0,
    warded: false,
  });
  const deadIds = [ally.instanceId, deathrite.instanceId, stealthed.instanceId];
  assert.equal(deadIds.every((instanceId) => !session.state.realm.units.some((unit) =>
    unit.instanceId === instanceId)), true);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === explosion.instanceId), true);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === ally.instanceId), true);
  assert.equal([deathrite.instanceId, stealthed.instanceId].every((instanceId) =>
    session.state.players.south.cemetery.some((card) => card.instanceId === instanceId)), true);
  const firstDeath = result.receipt.events.findIndex(({ type }) => type === 'minion-died');
  assert.equal(result.receipt.events.filter(({ type }) => type === 'damage-dealt').every((event) =>
    result.receipt.events.indexOf(event) < firstDeath), true);
  assert.equal(result.receipt.events.findIndex(({ type }) => type === 'avatar-healed') < firstDeath, true);
  assert.equal(result.receipt.events.at(-1)?.type, 'magic-resolved');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Rain of Arrows simultaneously damages every aboveground minion', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === deathriteCardId && descriptor.cell === 'C4' && !descriptor.region);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === burrowedCardId && descriptor.cell === 'C4'
    && descriptor.region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === stealthedCardId && descriptor.cell === 'C1' && !descriptor.region);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === wardedCardId && descriptor.cell === 'C1' && !descriptor.region);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === submergedCardId && descriptor.cell === 'C1'
    && descriptor.region === 'underwater');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === voidCardId && descriptor.cell === 'B4' && descriptor.region === 'void');

  const checkpoint = session;
  const rain = checkpoint.state.players.north.hand.spellbook.find(({ cardId }) => cardId === rainCardId);
  const deathrite = checkpoint.state.realm.units.find(({ cardId }) => cardId === deathriteCardId);
  const burrowed = checkpoint.state.realm.units.find(({ cardId }) => cardId === burrowedCardId);
  const voidwalker = checkpoint.state.realm.units.find(({ cardId }) => cardId === voidCardId);
  const stealthed = checkpoint.state.realm.units.find(({ cardId }) => cardId === stealthedCardId);
  const warded = checkpoint.state.realm.units.find(({ cardId }) => cardId === wardedCardId);
  const submerged = checkpoint.state.realm.units.find(({ cardId }) => cardId === submergedCardId);
  assert.ok(rain);
  assert.ok(deathrite);
  assert.ok(burrowed);
  assert.ok(voidwalker);
  assert.ok(stealthed);
  assert.ok(warded);
  assert.ok(submerged);
  const casts = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === rain.instanceId);
  assert.equal(casts.length, 1);
  assert.equal(casts[0]?.descriptor.kind === 'cast-magic'
    && casts[0].descriptor.target === undefined
    && casts[0].descriptor.targetLocation === undefined, true);

  const beforeMana = checkpoint.state.players.north.mana;
  const result = stepGame(checkpoint, casts[0]!);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
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
  assert.equal(session.state.stateVersion, checkpoint.state.stateVersion + 1);
  assert.equal(session.state.players.north.mana, beforeMana - 1);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === rain.instanceId), true);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === deathrite.instanceId), true);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === stealthed.instanceId), true);
  const survivingWard = session.state.realm.units.find(({ instanceId }) => instanceId === warded.instanceId);
  assert.deepEqual({ damage: survivingWard?.damage, warded: survivingWard?.warded }, {
    damage: 0,
    warded: false,
  });
  for (const excluded of [burrowed, voidwalker, submerged]) {
    assert.deepEqual(session.state.realm.units.find(({ instanceId }) =>
      instanceId === excluded.instanceId)?.damage, 0);
  }
  assert.equal(session.state.players.north.avatar.life, 20);
  assert.equal(session.state.players.south.avatar.life, 20);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Charge Magic grants an untargeted ally Charge only for the current turn', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === printedChargeCardId
    && descriptor.cell === 'C4'
    && descriptor.region === 'underground');
  const printedCharge = session.state.realm.units.find(({ cardId }) => cardId === printedChargeCardId);
  assert.ok(printedCharge);
  assert.equal(printedCharge.summoningSickness, true);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === printedCharge.instanceId), true);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === enemyCardId && descriptor.cell === 'C1');
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === enemyCardId);
  assert.ok(enemy);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === summonedCardId && descriptor.cell === 'C4');

  const checkpoint = session;
  const summoned = checkpoint.state.realm.units.find(({ cardId }) => cardId === summonedCardId);
  const chargeCards = checkpoint.state.players.north.hand.spellbook.filter(({ cardId }) =>
    chargeCardIds.includes(cardId));
  assert.ok(summoned);
  assert.equal(chargeCards.length, 2);
  assert.deepEqual({
    region: checkpoint.state.realm.units.find(({ instanceId }) =>
      instanceId === printedCharge.instanceId)?.region,
    stealthed: checkpoint.state.realm.units.find(({ instanceId }) =>
      instanceId === printedCharge.instanceId)?.stealthed,
    warded: checkpoint.state.realm.units.find(({ instanceId }) =>
      instanceId === printedCharge.instanceId)?.warded,
  }, { region: 'underground', stealthed: true, warded: true });
  const casts = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === chargeCards[0]?.instanceId);
  const allyIds = casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.ally ? [descriptor.ally.instanceId] : []).sort();
  assert.deepEqual(allyIds, [
    checkpoint.state.players.north.avatar.card.instanceId,
    printedCharge.instanceId,
    summoned.instanceId,
  ].sort());
  assert.equal(allyIds.includes(enemy.instanceId), false);
  assert.equal(casts.every(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target === undefined), true);
  assert.equal(casts.find(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.ally?.instanceId === summoned.instanceId)?.label.includes('grant Charge'), true);
  assert.equal(legalGameActions(checkpoint.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === summoned.instanceId), false);

  const avatarGrant = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === chargeCards[0]?.instanceId
      && descriptor.ally?.kind === 'avatar'));
  assert.deepEqual(avatarGrant.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'charge-granted',
    'magic-resolved',
  ]);
  assert.equal(avatarGrant.state.realm.units.every(({ temporaryChargeSources }) =>
    temporaryChargeSources === undefined), true);
  assert.equal(verifyGameReplay(avatarGrant), true);

  const first = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === chargeCards[0]?.instanceId
      && descriptor.ally?.instanceId === summoned.instanceId));
  assert.equal(first.accepted, true);
  if (!first.accepted) return;
  session = first.session;
  assert.deepEqual(first.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'charge-granted',
    'magic-resolved',
  ]);
  assert.deepEqual(session.state.realm.units.find(({ instanceId }) =>
    instanceId === summoned.instanceId)?.temporaryChargeSources, [chargeCards[0]!.instanceId]);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === summoned.instanceId), true);

  const second = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === chargeCards[1]?.instanceId
      && descriptor.ally?.instanceId === summoned.instanceId));
  assert.equal(second.accepted, true);
  if (!second.accepted) return;
  session = second.session;
  assert.deepEqual(session.state.realm.units.find(({ instanceId }) =>
    instanceId === summoned.instanceId)?.temporaryChargeSources, chargeCards.map(({ instanceId }) => instanceId));
  assert.equal(session.state.players.north.mana, 0);
  assert.equal(session.state.players.north.cemetery.filter(({ instanceId }) =>
    chargeCards.some((card) => card.instanceId === instanceId)).length, 2);
  assert.equal(session.state.stateVersion, checkpoint.state.stateVersion + 2);
  assert.equal(first.receipt.randomDraws.length + second.receipt.randomDraws.length, 0);

  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === summoned.instanceId
    && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const ended = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(ended.accepted, true);
  if (!ended.accepted) return;
  session = ended.session;
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
  const expired = session.state.realm.units.find(({ instanceId }) => instanceId === summoned.instanceId);
  assert.equal(expired?.temporaryChargeSources, undefined);
  assert.equal(expired?.tapped, true);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === printedCharge.instanceId)?.temporaryChargeSources, undefined);
  assert.equal(gameManifest.cards[printedChargeCardId]?.cardType === 'minion'
    && gameManifest.cards[printedChargeCardId].charge, true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Overpower gives a chosen ally +2 power until the current End Phase', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === fighterCardId && descriptor.cell === 'C4' && !descriptor.region);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === hiddenCardId && descriptor.cell === 'C4'
    && descriptor.region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === enemyCardId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === disabledCardId && descriptor.cell === 'C4');

  const checkpoint = session;
  const overpower = checkpoint.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === overpowerCardId);
  const fighter = checkpoint.state.realm.units.find(({ cardId }) => cardId === fighterCardId);
  const hidden = checkpoint.state.realm.units.find(({ cardId }) => cardId === hiddenCardId);
  const disabled = checkpoint.state.realm.units.find(({ cardId }) => cardId === disabledCardId);
  const enemy = checkpoint.state.realm.units.find(({ cardId }) => cardId === enemyCardId);
  assert.ok(overpower);
  assert.ok(fighter);
  assert.ok(hidden);
  assert.ok(disabled);
  assert.ok(enemy);
  const casts = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === overpower.instanceId);
  assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.ally ? [descriptor.ally.instanceId] : []).sort(), [
    checkpoint.state.players.north.avatar.card.instanceId,
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

  const avatarGrant = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === overpower.instanceId
      && descriptor.ally?.kind === 'avatar'));
  assert.deepEqual({
    attack: observeGame(avatarGrant.state, 'north').players.north.avatar.attack,
    defense: observeGame(avatarGrant.state, 'north').players.north.avatar.defense,
  }, { attack: 3, defense: 3 });
  assert.deepEqual(avatarGrant.transcript.at(-1)?.events.slice(1, 2).map(({ payload, type }) => ({
    payload,
    type,
  })), [{
    payload: {
      amount: 2,
      instanceId: checkpoint.state.players.north.avatar.card.instanceId,
      seat: 'north',
      sourceInstanceId: overpower.instanceId,
    },
    type: 'power-granted',
  }]);
  assert.equal(verifyGameReplay(avatarGrant), true);

  const disabledGrant = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === overpower.instanceId
      && descriptor.ally?.instanceId === disabled.instanceId));
  const disabledView = observeGame(disabledGrant.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === disabled.instanceId);
  assert.deepEqual({
    attack: disabledView?.attack,
    defense: disabledView?.defense,
    disabled: disabledView?.disabled,
  }, { attack: 3, defense: 3, disabled: true });
  assert.equal(verifyGameReplay(disabledGrant), true);

  const powered = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === overpower.instanceId
      && descriptor.ally?.instanceId === fighter.instanceId));
  assert.equal(powered.accepted, true);
  if (!powered.accepted) return;
  session = powered.session;
  assert.deepEqual(powered.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'power-granted',
    'magic-resolved',
  ]);
  const poweredView = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === fighter.instanceId);
  assert.deepEqual({ attack: poweredView?.attack, defense: poweredView?.defense }, {
    attack: 3,
    defense: 3,
  });
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === fighter.instanceId && descriptor.path.length === 1);
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion' && descriptor.target.instanceId === enemy.instanceId);
  const fought = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(fought.accepted, true);
  if (!fought.accepted) return;
  session = fought.session;
  assert.equal(fought.receipt.events.some(({ payload, type }) => type === 'damage-dealt'
    && (payload as { amount?: number; instanceId?: string }).amount === 3
    && (payload as { instanceId?: string }).instanceId === enemy.instanceId), true);
  const survivor = session.state.realm.units.find(({ instanceId }) => instanceId === fighter.instanceId);
  assert.equal(survivor?.damage, 2);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === enemy.instanceId), false);

  const ended = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(ended.accepted, true);
  if (!ended.accepted) return;
  session = ended.session;
  const expiryIndex = ended.receipt.events.findIndex(({ type }) => type === 'power-expired');
  const turnEndedIndex = ended.receipt.events.findIndex(({ type }) => type === 'turn-ended');
  assert.equal(expiryIndex >= 0 && expiryIndex < turnEndedIndex, true);
  assert.deepEqual(ended.receipt.events[expiryIndex]?.payload, {
    amount: 2,
    instanceId: fighter.instanceId,
    seat: 'north',
    sourceInstanceId: overpower.instanceId,
  });
  const expired = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === fighter.instanceId);
  assert.deepEqual({
    attack: expired?.attack,
    damage: expired?.damage,
    defense: expired?.defense,
  }, { attack: 1, damage: 0, defense: 1 });
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === fighter.instanceId), true);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === overpower.instanceId), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 nearby-allies power is derived and settles deaths when its source dies', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  for (const card of [allyCard, sourceCard, disabledSourceCard]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === card.cardId
      && descriptor.cell === 'C4');
  }
  const ally = session.state.realm.units.find(({ cardId }) => cardId === allyCard.cardId);
  const source = session.state.realm.units.find(({ cardId }) => cardId === sourceCard.cardId);
  const disabledSource = session.state.realm.units.find(({ cardId }) =>
    cardId === disabledSourceCard.cardId);
  assert.ok(ally);
  assert.ok(source);
  assert.ok(disabledSource);
  const view = observeGame(session.state, 'north');
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

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  const damaged = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardId === rainCard.cardId));
  assert.equal(damaged.accepted, true);
  if (!damaged.accepted) return;
  session = damaged.session;
  assert.equal(session.state.realm.units.some(({ owner }) => owner === 'north'), false);
  assert.deepEqual(session.state.players.north.cemetery.map(({ instanceId }) => instanceId), [
    source.instanceId,
    ally.instanceId,
    disabledSource.instanceId,
  ]);
  assert.equal(damaged.receipt.events.filter(({ type }) => type === 'minion-died').length, 3);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 controlled Mortal power follows current control and settles deaths', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  for (const card of [northMortalCard, northKingACard, northKingBCard]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === card.cardId
      && descriptor.cell === 'C4');
  }
  const northMortal = session.state.realm.units.find(({ cardId }) =>
    cardId === northMortalCard.cardId);
  const northKingA = session.state.realm.units.find(({ cardId }) =>
    cardId === northKingACard.cardId);
  const northKingB = session.state.realm.units.find(({ cardId }) =>
    cardId === northKingBCard.cardId);
  assert.ok(northMortal);
  assert.ok(northKingA);
  assert.ok(northKingB);
  const northOpening = observeGame(session.state, 'north');
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

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  for (const card of [southMortalCard, southNonMortalCard]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === card.cardId
      && descriptor.cell === 'C1');
  }
  const southMortal = session.state.realm.units.find(({ cardId }) =>
    cardId === southMortalCard.cardId);
  const southNonMortal = session.state.realm.units.find(({ cardId }) =>
    cardId === southNonMortalCard.cardId);
  assert.ok(southMortal);
  assert.ok(southNonMortal);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === northKingA.instanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  const separated = observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === northMortal.instanceId);
  assert.deepEqual([separated?.attack, separated?.defense], [3, 3]);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === southMortal.instanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardId === damageCard.cardId
    && descriptor.target?.instanceId === northMortal.instanceId);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === northMortal.instanceId)?.damage, 2);
  const controlled = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardId === mesmerismCard.cardId
      && descriptor.target?.instanceId === northKingA.instanceId));
  assert.equal(controlled.accepted, true);
  if (!controlled.accepted) return;
  session = controlled.session;
  assert.deepEqual(controlled.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'minion-control-changed',
    'minion-died',
    'magic-resolved',
  ]);
  assert.equal(controlled.receipt.randomDraws.length, 0);
  assert.equal(session.state.realm.units.some(({ instanceId }) =>
    instanceId === northMortal.instanceId), false);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === northMortal.instanceId), true);

  const finalView = observeGame(session.state, 'north');
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
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 aura-loss Deathrite ends the game after its triggering Magic resolves', () => {
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
  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const opening = createGameSession(candidate).state.players.north;
    const openingNames = new Set(opening.hand.spellbook.map(({ cardId }) => cardId));
    const availableNames = new Set([
      ...opening.hand.spellbook,
      ...opening.spellbook.slice(0, 2),
    ].map(({ cardId }) => cardId));
    if (openingNames.has('banner-ally')
      && openingNames.has('banner-source')
      && ['banner-rain', 'banner-teleport']
      .every((cardId) => availableNames.has(cardId))) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'banner-source');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'banner-ally');
  const source = session.state.realm.units.find(({ cardId }) => cardId === 'banner-source');
  const ally = session.state.realm.units.find(({ cardId }) => cardId === 'banner-ally');
  assert.ok(source);
  assert.ok(ally);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A4');
  assert.equal(session.state.players.north.atlas.length, 0);
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardId === 'banner-rain');
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === ally.instanceId)?.damage, 1);

  const result = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'banner-teleport'
      && descriptor.ally?.instanceId === source.instanceId
      && descriptor.targetLocation?.cell === 'A4'));
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
  assert.deepEqual(result.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'unit-teleported',
    'minion-died',
    'magic-resolved',
    'game-ended',
  ]);
  assert.deepEqual(session.state.terminal, {
    loser: 'north',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'south',
  });
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 aura-loss deaths cannot restore stale combat during a defender path', () => {
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
  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const opening = createGameSession(candidate).state.players;
    const northOpening = new Set(opening.north.hand.spellbook.map(({ cardId }) => cardId));
    const southBySecondTurn = new Set([
      ...opening.south.hand.spellbook,
      ...opening.south.spellbook.slice(0, 2),
    ].map(({ cardId }) => cardId));
    if (['defend-fragile', 'defend-source', 'defend-target']
      .every((cardId) => northOpening.has(cardId))
      && southBySecondTurn.has('defend-attacker')
      && new Set([
        ...opening.south.hand.spellbook,
        ...opening.south.spellbook.slice(0, 3),
      ].map(({ cardId }) => cardId)).has('defend-rain')) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  for (const cardId of ['defend-target', 'defend-fragile', 'defend-source']) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === cardId
      && descriptor.cell === 'C4');
  }
  const target = session.state.realm.units.find(({ cardId }) => cardId === 'defend-target');
  const fragile = session.state.realm.units.find(({ cardId }) => cardId === 'defend-fragile');
  const source = session.state.realm.units.find(({ cardId }) => cardId === 'defend-source');
  assert.ok(target);
  assert.ok(fragile);
  assert.ok(source);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === source.instanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,B4');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'defend-attacker'
    && descriptor.cell === 'C4');
  const attacker = session.state.realm.units.find(({ cardId }) => cardId === 'defend-attacker');
  assert.ok(attacker);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A1');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardId === 'defend-rain');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === attacker.instanceId
    && descriptor.path.length === 1);
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === target.instanceId);
  take(({ descriptor }) => descriptor.kind === 'defend'
    && descriptor.unitInstanceId === fragile.instanceId
    && descriptor.path.length === 1);

  const defended = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === source.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'B4,A4,B4,C4'));
  assert.equal(defended.accepted, true);
  if (!defended.accepted) return;
  session = defended.session;
  assert.deepEqual(session.state.pendingCombat?.defenders.map(({ instanceId }) => instanceId), [
    source.instanceId,
  ]);
  assert.equal(session.state.realm.units.some(({ instanceId }) =>
    instanceId === fragile.instanceId), false);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === fragile.instanceId), true);
  assert.deepEqual(defended.receipt.events.map(({ type }) => type), [
    'defender-joined',
    'minion-died',
  ]);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Lure makes a chosen enemy minion take its own closer step', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === immobileCardId && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'D4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'D2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'D3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === mobileCardId && descriptor.cell === 'D3');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.realm.units.find(({ cardId }) =>
      cardId === mobileCardId)?.instanceId
    && descriptor.path.length === 1);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const checkpoint = session;
  const allyInstanceId = checkpoint.state.players.north.avatar.card.instanceId;
  const mobile = checkpoint.state.realm.units.find(({ cardId }) => cardId === mobileCardId);
  const immobile = checkpoint.state.realm.units.find(({ cardId }) => cardId === immobileCardId);
  const lureCards = checkpoint.state.players.north.hand.spellbook.slice(0, 3);
  assert.ok(mobile);
  assert.ok(immobile);
  assert.equal(lureCards.length, 3);
  assert.deepEqual({
    location: mobile.location,
    stealthed: mobile.stealthed,
    tapped: mobile.tapped,
    warded: mobile.warded,
  }, { location: 'D3', stealthed: true, tapped: true, warded: true });

  const choices = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
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

  const first = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === lureCards[0]?.instanceId
      && descriptor.temptedDestination?.cell === 'C3'));
  assert.equal(first.accepted, true);
  if (!first.accepted) return;
  session = first.session;
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
  assert.deepEqual(session.state.realm.units.find(({ instanceId }) =>
    instanceId === mobile.instanceId), { ...mobile, location: 'C3' });

  const second = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === lureCards[1]?.instanceId
      && descriptor.temptedEnemy?.instanceId === mobile.instanceId
      && descriptor.temptedDestination?.cell === 'C4'));
  assert.equal(second.accepted, true);
  if (!second.accepted) return;
  session = second.session;
  const noChoiceActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === lureCards[2]?.instanceId);
  assert.equal(noChoiceActions.length, 1);
  assert.equal(noChoiceActions[0]?.descriptor.kind === 'cast-magic'
    && noChoiceActions[0].descriptor.ally === undefined
    && noChoiceActions[0].descriptor.temptedEnemy === undefined
    && noChoiceActions[0].descriptor.temptedDestination === undefined, true);

  const noOp = stepGame(session, noChoiceActions[0]!);
  assert.equal(noOp.accepted, true);
  if (!noOp.accepted) return;
  session = noOp.session;
  assert.deepEqual(noOp.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
  assert.deepEqual(session.state.realm.units.find(({ instanceId }) =>
    instanceId === mobile.instanceId), { ...mobile, location: 'C4' });
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === immobile.instanceId)?.location, 'C3');
  assert.equal(session.state.players.north.mana, checkpoint.state.players.north.mana - 3);
  assert.equal(session.state.players.north.cemetery.filter(({ instanceId }) =>
    lureCards.some((card) => card.instanceId === instanceId)).length, 3);
  assert.equal(session.state.stateVersion, checkpoint.state.stateVersion + 3);
  assert.equal([
    ...first.receipt.randomDraws,
    ...second.receipt.randomDraws,
    ...noOp.receipt.randomDraws,
  ].length, 0);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Teleport forcefully moves a chosen ally to a target site surface', () => {
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
  const preview = createGameSession(base);
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
  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  const allyCard = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === allyCardId);
  const teleportCard = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === teleportCardId);
  assert.ok(allyCard);
  assert.ok(teleportCard);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === allyCard.instanceId
      && descriptor.cell === 'C4'
      && descriptor.region === 'underwater'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));

  const checkpoint = session;
  const casts = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === teleportCard.instanceId);
  assert.equal(casts.length, 6);
  assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.ally
    ? [descriptor.ally.instanceId]
    : []))].sort(), [
    allyCard.instanceId,
    checkpoint.state.players.north.avatar.card.instanceId,
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

  const noMove = accept(checkpoint, casts.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.ally?.kind === 'avatar'
      && descriptor.targetLocation?.cell === 'C4')!);
  assert.deepEqual(noMove.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
  assert.equal(noMove.state.players.north.avatar.location, 'C4');
  assert.equal(verifyGameReplay(noMove), true);

  const destinationSite = checkpoint.state.realm.sites.C1;
  assert.ok(destinationSite);
  const teleported = accept(checkpoint, casts.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.ally?.instanceId === allyCard.instanceId
      && descriptor.targetLocation?.cell === 'C1')!);
  const moved = teleported.state.realm.units.find(({ instanceId }) => instanceId === allyCard.instanceId);
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
  assert.deepEqual(teleported.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'unit-teleported',
    'magic-resolved',
  ]);
  const teleportEvent = teleported.transcript.at(-1)?.events[1];
  assert.equal(canonicalJson(teleportEvent?.payload ?? null).includes(
    `\"sourceInstanceId\":\"${teleportCard.instanceId}\"`), true);
  assert.equal(canonicalJson(teleportEvent?.payload ?? null).includes(
    `\"targetSiteInstanceId\":\"${destinationSite.instanceId}\"`), true);
  assert.equal(teleported.state.stateVersion, checkpoint.state.stateVersion + 1);
  assert.equal(teleported.state.players.north.mana, 0);
  assert.equal(teleported.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === teleportCard.instanceId), true);
  assert.equal(verifyGameReplay(teleported), true);
});

test('RULE-03 Blink teleports a nearby ally before deaths and a private chosen-deck draw', () => {
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
  const preview = createGameSession(createGameManifest({ ...input, cards: baseCards }));
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

  let session = keep(keep(createGameSession(gameManifest)));
  const sourceInstanceId = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === sourceCard.cardId)?.instanceId;
  const beneficiaryInstanceId = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === beneficiaryCard.cardId)?.instanceId;
  const rainInstanceId = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === rainCard.cardId)?.instanceId;
  assert.ok(sourceInstanceId);
  assert.ok(beneficiaryInstanceId);
  assert.ok(rainInstanceId);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === sourceInstanceId
      && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'B4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === beneficiaryInstanceId
      && descriptor.cell === 'B4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'D4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === rainInstanceId));

  const beneficiary = session.state.realm.units.find(({ instanceId }) =>
    instanceId === beneficiaryInstanceId);
  assert.ok(beneficiary);
  assert.equal(observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === beneficiaryInstanceId)?.disabled, true);
  assert.equal(beneficiary.damage, 1);
  const blinkInstanceId = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === blinkCard.cardId)?.instanceId;
  assert.ok(blinkInstanceId);
  const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
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

  const beforeBlink = session;
  const drawnSpell = beforeBlink.state.players.north.spellbook[0];
  const drawnSite = beforeBlink.state.players.north.atlas[0];
  assert.ok(drawnSpell);
  assert.ok(drawnSite);
  const beforeHandCount = beforeBlink.state.players.north.hand.spellbook.length;
  const spellResult = accept(beforeBlink, casts.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.ally?.instanceId === sourceInstanceId
      && descriptor.drawZone === 'spellbook'
      && descriptor.targetLocation?.cell === 'D4')!);
  assert.deepEqual(spellResult.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'unit-teleported',
    'minion-died',
    'spell-drawn',
    'magic-resolved',
  ]);
  assert.equal(spellResult.state.realm.units.find(({ instanceId }) =>
    instanceId === sourceInstanceId)?.location, 'D4');
  assert.equal(spellResult.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === beneficiaryInstanceId), true);
  assert.equal(spellResult.state.players.north.hand.spellbook.length, beforeHandCount);
  assert.equal(spellResult.state.players.north.hand.spellbook.some(({ instanceId }) =>
    instanceId === drawnSpell.instanceId), true);
  assert.equal(typeof observeGame(spellResult.state, 'south').players.north.hand.spellbook, 'number');
  assert.equal(verifyGameReplay(spellResult), true);

  const siteResult = accept(beforeBlink, casts.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.ally?.instanceId === sourceInstanceId
      && descriptor.drawZone === 'atlas'
      && descriptor.targetLocation?.cell === 'D4')!);
  assert.deepEqual(siteResult.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'unit-teleported',
    'minion-died',
    'site-drawn',
    'magic-resolved',
  ]);
  assert.equal(siteResult.state.players.north.hand.atlas.some(({ instanceId }) =>
    instanceId === drawnSite.instanceId), true);
  assert.equal(typeof observeGame(siteResult.state, 'south').players.north.hand.atlas, 'number');
  assert.equal(verifyGameReplay(siteResult), true);

  const emptyAtlas: GameSession = {
    ...beforeBlink,
    state: {
      ...beforeBlink.state,
      players: {
        ...beforeBlink.state.players,
        north: { ...beforeBlink.state.players.north, atlas: [] },
      },
    },
  };
  const atlasLoss = accept(emptyAtlas, action(emptyAtlas, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === blinkInstanceId
      && descriptor.ally?.instanceId === sourceInstanceId
      && descriptor.drawZone === 'atlas'
      && descriptor.targetLocation?.cell === 'D4'));
  assert.deepEqual(atlasLoss.state.terminal, {
    loser: 'north',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'south',
  });
  const spellWithEmptyAtlas = accept(emptyAtlas, action(emptyAtlas, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === blinkInstanceId
      && descriptor.ally?.instanceId === sourceInstanceId
      && descriptor.drawZone === 'spellbook'
      && descriptor.targetLocation?.cell === 'D4'));
  assert.deepEqual(spellWithEmptyAtlas.state.terminal, { status: 'active' });
});

test('RULE-03 Rescue returns a chosen own cemetery minion to hidden hand or resolves with none', () => {
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
  let gameManifest: GameManifest | undefined;
  for (let seed = 155; seed < 175; seed += 1) {
    const candidate = createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: 'synthetic-rescue-fixture-v1',
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    });
    const preview = createGameSession(candidate).state.players.north;
    const topTypes = preview.spellbook.slice(0, 2).map(({ cardId }) =>
      candidate.cards[cardId]?.cardType);
    if (topTypes.includes('minion')
      && topTypes.includes('magic')
      && preview.hand.spellbook.some(({ cardId }) => candidate.cards[cardId]?.cardType === 'magic')) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  const checkpoint = keep(keep(createGameSession(gameManifest)));

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
  let noChoiceSetup = keep(keep(createGameSession(noChoiceManifest)));
  noChoiceSetup = accept(noChoiceSetup, action(noChoiceSetup, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  const noChoiceCard = noChoiceSetup.state.players.north.hand.spellbook.find(({ cardId }) =>
    noChoiceManifest.cards[cardId]?.cardType === 'magic');
  assert.ok(noChoiceCard);
  const noChoiceActions = legalGameActions(noChoiceSetup.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === noChoiceCard.instanceId);
  assert.equal(noChoiceActions.length, 1);
  assert.equal(noChoiceActions[0]?.descriptor.kind === 'cast-magic'
    && noChoiceActions[0].descriptor.cemeteryMinionInstanceId, undefined);
  assert.equal(noChoiceActions[0]?.descriptor.kind === 'cast-magic'
    && noChoiceActions[0].descriptor.target, undefined);
  const noChoice = accept(noChoiceSetup, noChoiceActions[0]!);
  assert.deepEqual(noChoice.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
  assert.equal(noChoice.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === noChoiceCard.instanceId), true);
  assert.equal(verifyGameReplay(noChoice), true);

  let session = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  const ownMinion = session.state.players.north.cemetery.find(({ cardId }) =>
    gameManifest.cards[cardId]?.cardType === 'minion');
  const ownMagic = session.state.players.north.cemetery.find(({ cardId }) =>
    gameManifest.cards[cardId]?.cardType === 'magic');
  assert.ok(ownMinion);
  assert.ok(ownMagic);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  const opposingMinion = session.state.players.south.cemetery.find(({ cardId }) =>
    gameManifest.cards[cardId]?.cardType === 'minion');
  assert.ok(opposingMinion);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));

  const rescueCard = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    gameManifest.cards[cardId]?.cardType === 'magic');
  assert.ok(rescueCard);
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === rescueCard.instanceId);
  assert.deepEqual(choices.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cemeteryMinionInstanceId
    ? [descriptor.cemeteryMinionInstanceId]
    : []), [ownMinion.instanceId]);
  assert.equal(choices.some(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cemeteryMinionInstanceId === ownMagic.instanceId), false);
  assert.equal(choices.some(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cemeteryMinionInstanceId === opposingMinion.instanceId), false);

  const before = session.state;
  const result = stepGame(session, choices[0]!);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
  assert.equal(session.state.stateVersion, before.stateVersion + 1);
  assert.equal(session.state.players.north.hand.spellbook.length, before.players.north.hand.spellbook.length);
  assert.equal(session.state.players.north.hand.spellbook.some(({ instanceId }) =>
    instanceId === ownMinion.instanceId), true);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === ownMinion.instanceId), false);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
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
  const northView = observeGame(session.state, 'north');
  const southView = observeGame(session.state, 'south');
  assert.equal(Array.isArray(northView.players.north.hand.spellbook)
    && northView.players.north.hand.spellbook.some(({ instanceId }) =>
      instanceId === ownMinion.instanceId), true);
  assert.equal(southView.players.north.hand.spellbook, session.state.players.north.hand.spellbook.length);
  assert.doesNotMatch(canonicalJson(southView), new RegExp(ownMinion.instanceId));
  assert.doesNotMatch(canonicalJson(southView), new RegExp(ownMinion.cardId));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Bury forcefully burrows minions if able and immediately resolves survival', () => {
  const castBury = (
    targetFacts: Pick<SpellFacts, 'burrowing' | 'ward'>,
    waterTarget: boolean,
    seed: number,
  ): Readonly<{
    beforeCast: GameSession['state'];
    session: GameSession;
    targetInstanceId: string;
  }> => {
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
    let session = keep(createGameSession(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-bury-${seed}-v1`,
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    })));
    session = keep(session);
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cell === 'C1'
        && descriptor.region === undefined));
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
      ? [`${descriptor.target.kind}:${descriptor.target.instanceId}`]
      : []), [`minion:${target.instanceId}`]);
    const beforeCast = session.state;
    session = accept(session, casts[0]!);
    assert.equal(session.state.stateVersion, beforeCast.stateVersion + 1);
    assert.equal(session.state.players.north.mana, beforeCast.players.north.mana - 1);
    assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === spell.instanceId), true);
    assert.equal(verifyGameReplay(session), true);
    return { beforeCast, session, targetInstanceId: target.instanceId };
  };

  const survivor = castBury({ burrowing: true }, false, 152);
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

  const dead = castBury({}, false, 153);
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

  const warded = castBury({ ward: true }, false, 154);
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

  const water = castBury({}, true, 155);
  assert.deepEqual(water.session.state.realm.units.find(({ instanceId }) =>
    instanceId === water.targetInstanceId), water.beforeCast.realm.units.find(({ instanceId }) =>
    instanceId === water.targetInstanceId));
  assert.deepEqual(water.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
});

test('RULE-03/04 Bury detaches and burrows Artifacts if able', () => {
  const castBury = (
    carried: boolean,
    waterTarget: boolean,
  ): Readonly<{
    artifactInstanceId: string;
    beforeCast: GameSession['state'];
    session: GameSession;
  }> => {
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
    let session = keep(keep(createGameSession(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-bury-artifact-${carried ? 'carried' : 'uncarried'}-${waterTarget ? 'water' : 'land'}-v1`,
      },
      cards,
      decks: { north, south },
      firstSeat: 'north',
      seed: 156,
    }))));
    const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
      session = accept(session, action(session, predicate));
    };
    take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'bury-artifact-target'
      && (carried
        ? descriptor.bearer?.kind === 'avatar'
        : descriptor.bearer === undefined && descriptor.cell === 'C1'));
    const artifact = session.state.realm.artifacts?.[0];
    assert.ok(artifact);
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const bury = session.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === 'bury-artifact-magic');
    assert.ok(bury);
    const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === bury.instanceId
        && descriptor.targetArtifactInstanceId === artifact.instanceId);
    assert.equal(choices.length, 1);
    assert.match(choices[0]!.label, /artifact/);
    const beforeCast = session.state;
    session = accept(session, choices[0]!);
    assert.equal(verifyGameReplay(session), true);
    return { artifactInstanceId: artifact.instanceId, beforeCast, session };
  };

  const uncarried = castBury(false, false);
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

  const carried = castBury(true, false);
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

  const water = castBury(false, true);
  assert.deepEqual(water.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === water.artifactInstanceId), water.beforeCast.realm.artifacts?.find(({ instanceId }) =>
    instanceId === water.artifactInstanceId));
  assert.deepEqual(water.session.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
});

test('RULE-03/04 Cave-In burrows every surface minion and Artifact at one Land Site together', () => {
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
  let checkpoint = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    checkpoint = accept(checkpoint, action(checkpoint, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'cave-in-burrower'
    && descriptor.cell === 'C1'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'cave-in-victim' && descriptor.cell === 'C1');
  const burrower = checkpoint.state.realm.units.find(({ cardId }) => cardId === 'cave-in-burrower');
  const victim = checkpoint.state.realm.units.find(({ cardId }) => cardId === 'cave-in-victim');
  const artifactCard = checkpoint.state.players.south.hand.spellbook
    .find(({ cardId }) => cardId === 'cave-in-artifact');
  assert.ok(burrower && victim && artifactCard);

  const castCaveIn = (bearer: 'avatar' | 'minion'): GameSession => {
    let session = accept(checkpoint, action(checkpoint, ({ descriptor }) =>
      descriptor.kind === 'cast-artifact'
        && descriptor.cardInstanceId === artifactCard.instanceId
        && descriptor.bearer?.kind === bearer
        && (bearer === 'avatar' || descriptor.bearer.instanceId === burrower.instanceId)));
    const artifact = session.state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === artifactCard.instanceId);
    assert.ok(artifact);
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    const spell = session.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === 'cave-in-magic');
    assert.ok(spell);
    const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
    assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.targetLocation
      ? [`${descriptor.targetLocation.cell}:${descriptor.targetLocation.region}`]
      : []), ['C1:surface']);
    const rubbleId = 'sha256:6666666666666666666666666666666666666666666666666666666666666666' as const;
    const rubbleState = {
      ...session.state,
      realm: {
        ...session.state.realm,
        sites: {
          ...session.state.realm.sites,
          A1: { controller: null, instanceId: rubbleId, rubble: true as const },
        },
      },
    };
    assert.equal(legalGameActions(rubbleState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === spell.instanceId
        && descriptor.targetSiteInstanceId === rubbleId
        && descriptor.targetLocation?.cell === 'A1'), true);
    assert.equal(legalGameActions({
      ...session.state,
      players: {
        ...session.state.players,
        north: {
          ...session.state.players.north,
          avatar: { ...session.state.players.north.avatar, region: 'underground' as const },
        },
      },
    }, 'north').some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === spell.instanceId), false);
    const result = stepGame(session, casts[0]!);
    assert.equal(result.accepted, true);
    if (!result.accepted) return session;
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
    const survivingBurrower = result.session.state.realm.units.find(({ instanceId }) =>
      instanceId === burrower.instanceId);
    assert.deepEqual({
      region: survivingBurrower?.region,
      stealthed: survivingBurrower?.stealthed,
      warded: survivingBurrower?.warded,
    }, { region: 'underground', stealthed: true, warded: true });
    assert.equal(result.session.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === victim.instanceId), true);
    assert.equal(result.session.state.players.south.avatar.region, 'surface');
    const movedArtifact = result.session.state.realm.artifacts?.find(({ instanceId }) =>
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
    assert.equal(result.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
    assert.equal(verifyGameReplay(result.session), true);
    return result.session;
  };

  castCaveIn('minion');
  castCaveIn('avatar');
});

test('RULE-03/04 Drown forcefully submerges a target minion if able', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed: 246,
  }));
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
  let session = keep(keep(createGameSession(gameManifest)));
  const instance = (cardId: string) => [
    ...session.state.players.south.hand.atlas,
    ...session.state.players.south.hand.spellbook,
    ...session.state.players.south.atlas,
    ...session.state.players.south.spellbook,
  ].find((card) => card.cardId === cardId)!;
  const waterSite = instance(waterSiteId);
  const landSite = instance(landSiteId);
  const ordinary = instance(ordinaryId);
  const survivor = instance(survivorId);
  const warded = instance(wardedId);
  const landTarget = instance(landTargetId);
  const stealth = instance(stealthId);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === waterSite.instanceId
      && descriptor.cell === 'C1'));
  for (const target of [ordinary, survivor, warded]) {
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === target.instanceId
        && descriptor.cell === 'C1'
        && descriptor.region === undefined));
  }
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === landSite.instanceId
      && descriptor.cell === 'C2'));
  for (const target of [landTarget, stealth]) {
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === target.instanceId
        && descriptor.cell === 'C2'
        && descriptor.region === undefined));
  }
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const checkpoint = session;
  const drown = checkpoint.state.players.north.hand.spellbook.find(({ cardId }) => cardId === drownCardId);
  assert.ok(drown);
  const casts = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === drown.instanceId);
  const targetIds = casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'minion' ? [descriptor.target.instanceId] : []);
  assert.deepEqual([...targetIds].sort(), [ordinary, survivor, warded, landTarget]
    .map(({ instanceId }) => instanceId).sort());
  assert.equal(targetIds.includes(stealth.instanceId), false);
  assert.equal(checkpoint.state.realm.units.find(({ instanceId }) =>
    instanceId === ordinary.instanceId)?.location, 'C1');
  assert.equal(checkpoint.state.players.north.avatar.location, 'C4');
  const castAt = (targetInstanceId: string): GameSession => accept(checkpoint, casts.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === targetInstanceId)!);
  const verifyCast = (cast: GameSession): void => {
    assert.equal(cast.state.stateVersion, checkpoint.state.stateVersion + 1);
    assert.equal(cast.state.players.north.mana, checkpoint.state.players.north.mana - 1);
    assert.equal(cast.state.players.north.cemetery.some(({ instanceId }) => instanceId === drown.instanceId), true);
    assert.equal(verifyGameReplay(cast), true);
  };

  const dead = castAt(ordinary.instanceId);
  verifyCast(dead);
  assert.equal(dead.state.realm.units.some(({ instanceId }) => instanceId === ordinary.instanceId), false);
  assert.equal(dead.state.players.south.cemetery.some(({ instanceId }) => instanceId === ordinary.instanceId), true);
  assert.deepEqual(dead.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'minion-submerged',
    'minion-died',
    'magic-resolved',
  ]);

  const submerged = castAt(survivor.instanceId);
  verifyCast(submerged);
  assert.equal(submerged.state.realm.units.find(({ instanceId }) => instanceId === survivor.instanceId)?.region,
    'underwater');
  assert.deepEqual(submerged.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'minion-submerged',
    'magic-resolved',
  ]);

  const protectedByWard = castAt(warded.instanceId);
  verifyCast(protectedByWard);
  const wardedUnit = protectedByWard.state.realm.units.find(({ instanceId }) => instanceId === warded.instanceId);
  assert.equal(wardedUnit?.region, 'surface');
  assert.equal(wardedUnit?.warded, false);
  assert.deepEqual(protectedByWard.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'ward-broken',
    'magic-resolved',
  ]);

  const unable = castAt(landTarget.instanceId);
  verifyCast(unable);
  assert.equal(unable.state.realm.units.find(({ instanceId }) => instanceId === landTarget.instanceId)?.region,
    'surface');
  assert.deepEqual(unable.transcript.at(-1)?.events.map(({ type }) => type), [
    'magic-cast',
    'magic-resolved',
  ]);
});

test("RULE-03/04 healing Magic is targetless, capped, and cannot leave Death's Door", () => {
  const healAfterDamage = (maximumLife: number, seed: number): Readonly<{
    beforeCast: GameSession['state'];
    session: GameSession;
    spellInstanceId: string;
  }> => {
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
    let session = keep(createGameSession(createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: `synthetic-healing-magic-${maximumLife}-v1`,
      },
      cards,
      decks,
      firstSeat: 'north',
      seed,
    })));
    session = keep(session);
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.target !== undefined
        && descriptor.target.kind === 'avatar'
        && descriptor.target.seat === 'north'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    const spell = session.state.players.north.hand.spellbook[0];
    assert.ok(spell);
    const casts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
    assert.equal(casts.length, 1);
    assert.equal(casts[0]?.descriptor.kind === 'cast-magic'
      && casts[0].descriptor.target === undefined, true);
    const beforeCast = session.state;
    session = accept(session, casts[0]!);
    assert.equal(verifyGameReplay(session), true);
    return { beforeCast, session, spellInstanceId: spell.instanceId };
  };

  const capped = healAfterDamage(20, 151);
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

  const deathDoor = healAfterDamage(4, 152);
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

test('RULE-03 explicit permission allows a minion to be summoned to any site', () => {
  const summonCells = (summonToAnySite: boolean): readonly string[] => {
    let session = keep(createGameSession(manifest(111, {
      spell: {
        manaCost: 1,
        summonToAnySite,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    })));
    session = keep(session);
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    const cardInstanceId = session.state.players.north.hand.spellbook[0]?.instanceId;
    assert.ok(cardInstanceId);
    const cells = legalGameActions(session.state, 'north').flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === cardInstanceId
        ? [descriptor.cell]
        : []);
    if (summonToAnySite) {
      session = accept(session, action(session, ({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === cardInstanceId
          && descriptor.cell === 'C1'));
      assert.equal(session.state.realm.units[0]?.location, 'C1');
      assert.equal(verifyGameReplay(session), true);
    }
    return cells;
  };

  assert.deepEqual(summonCells(false), ['C4']);
  assert.deepEqual(summonCells(true), ['C1', 'C4']);
});

test('RULE-03 a Water-site cast restriction filters unrestricted summons by terrain', () => {
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
  let session = keep(createGameSession(restrictedManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const northWater = session.state.players.north.hand.atlas.find(({ cardId }) => cardId === northWaterId);
  const northLand = session.state.players.north.hand.atlas.find(({ cardId }) => cardId === northLandId);
  const southLand = session.state.players.south.hand.atlas.find(({ cardId }) => cardId === southLandId);
  const southWater = session.state.players.south.hand.atlas.find(({ cardId }) => cardId === southWaterId);
  const featuredId = session.state.players.north.hand.spellbook[0]?.cardId;
  const ordinaryId = session.state.players.south.hand.spellbook[0]?.cardId;
  assert.ok(northWater);
  assert.ok(northLand);
  assert.ok(southLand);
  assert.ok(southWater);
  assert.ok(featuredId);
  assert.ok(ordinaryId);

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === northWater.instanceId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === southLand.instanceId && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === northLand.instanceId && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === southWater.instanceId && descriptor.cell === 'B1');
  const southSummons = legalGameActions(session.state, 'south');
  assert.equal(southSummons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === ordinaryId && descriptor.cell === 'C4'), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const summons = legalGameActions(session.state, 'north');
  const cellsFor = (cardId: string): readonly string[] => summons.flatMap(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === cardId ? [descriptor.cell] : []);
  assert.deepEqual(cellsFor(featuredId), ['B1', 'C4']);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === featuredId && descriptor.cell === 'B1');
  assert.equal(session.state.realm.units[0]?.location, 'B1');
  assert.equal(session.state.realm.units[0]?.controller, 'north');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Waterbound derives Disabled from terrain and survives only with active abilities', () => {
  const base = manifest(228);
  const preview = createGameSession(base);
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
  cards[waterboundId] = {
    attack: 2,
    cardType: 'minion',
    deathriteDrawSite: true,
    defense: 2,
    manaCost: 0,
    provides: 'water',
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
  let session = keep(keep(createGameSession(gameManifest)));
  const waterSite = session.state.players.north.hand.atlas.find(({ cardId }) =>
    cardId === waterSiteId);
  const sinkhole = session.state.players.north.hand.atlas.find(({ cardId }) =>
    cardId === sinkholeId);
  const waterbound = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === waterboundId);
  const teleport = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === teleportId);
  assert.ok(waterSite);
  assert.ok(sinkhole);
  assert.ok(waterbound);
  assert.ok(teleport);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === waterSite.instanceId
      && descriptor.cell === 'C4'));
  const summonCheckpoint = session;
  const summonRegions = legalGameActions(summonCheckpoint.state, 'north').flatMap(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === waterbound.instanceId
      ? [descriptor.region ?? 'surface']
      : []);
  assert.deepEqual(summonRegions, ['underwater', 'surface']);
  const summoned = (region: 'surface' | 'underwater'): GameSession =>
    accept(summonCheckpoint, action(summonCheckpoint, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === waterbound.instanceId
        && (descriptor.region ?? 'surface') === region));
  const underwater = summoned('underwater');
  assert.equal(observeGame(underwater.state, 'north').realm.units[0]?.disabled, false);
  assert.equal(observeGame(underwater.state, 'north').players.north.affinity.water, 2);
  assert.equal(verifyGameReplay(underwater), true);

  const northSecondMain = (start: GameSession): GameSession => {
    let branch = accept(start, action(start, ({ descriptor }) => descriptor.kind === 'end-turn'));
    branch = accept(branch, action(branch, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    branch = accept(branch, action(branch, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    branch = accept(branch, action(branch, ({ descriptor }) => descriptor.kind === 'end-turn'));
    branch = accept(branch, action(branch, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    return accept(branch, action(branch, ({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === sinkhole.instanceId
        && descriptor.cell === 'C3'));
  };

  const underwaterCheckpoint = northSecondMain(underwater);
  const atlasBefore = underwaterCheckpoint.state.players.north.atlas.length;
  const atlasHandBefore = underwaterCheckpoint.state.players.north.hand.atlas.length;
  const destroyed = stepGame(underwaterCheckpoint, action(underwaterCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'activate-site-destruction'
      && descriptor.sourceSiteInstanceId === sinkhole.instanceId
      && descriptor.targetCell === 'C4'));
  assert.equal(destroyed.accepted, true);
  assert.deepEqual(destroyed.receipt.events.map(({ type }) => type), [
    'site-sacrificed',
    'site-destroyed',
    'minion-died',
    'rubble-created',
    'rubble-created',
  ]);
  assert.equal(destroyed.session.state.realm.units.some(({ instanceId }) =>
    instanceId === waterbound.instanceId), false);
  assert.equal(destroyed.session.state.players.north.atlas.length, atlasBefore);
  assert.equal(destroyed.session.state.players.north.hand.atlas.length, atlasHandBefore);
  assert.equal(destroyed.receipt.events.some(({ type }) => type === 'site-drawn'), false);
  assert.equal(verifyGameReplay(destroyed.session), true);

  session = northSecondMain(summoned('surface'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === waterbound.instanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(observeGame(session.state, 'north').realm.units[0]?.disabled, true);
  assert.equal(observeGame(session.state, 'north').players.north.affinity.water, 1);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const disabledActions = legalGameActions(session.state, 'north');
  assert.equal(disabledActions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === waterbound.instanceId), false);
  assert.equal(disabledActions.some(({ descriptor }) =>
    descriptor.kind === 'activate-mana'
      && descriptor.unitInstanceId === waterbound.instanceId), false);
  const beforeTeleport = session.state.realm.units.find(({ instanceId }) =>
    instanceId === waterbound.instanceId);
  assert.ok(beforeTeleport);
  assert.deepEqual({
    damage: beforeTeleport.damage,
    summoningSickness: beforeTeleport.summoningSickness,
    tapped: beforeTeleport.tapped,
  }, { damage: 0, summoningSickness: false, tapped: false });
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === teleport.instanceId
      && descriptor.ally?.instanceId === waterbound.instanceId
      && descriptor.targetLocation?.cell === 'C4'));
  const returned = session.state.realm.units.find(({ instanceId }) =>
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
  assert.equal(observeGame(session.state, 'north').realm.units[0]?.disabled, false);
  assert.equal(observeGame(session.state, 'north').players.north.affinity.water, 2);
  const enabledActions = legalGameActions(session.state, 'north');
  assert.equal(enabledActions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === waterbound.instanceId), true);
  assert.equal(enabledActions.some(({ descriptor }) =>
    descriptor.kind === 'activate-mana'
      && descriptor.unitInstanceId === waterbound.instanceId), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-02/04 region settlement kills inhospitable minions and banishes them from the void', () => {
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
  const preview = createGameSession(base);
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
  const checkpoint = keep(keep(createGameSession(gameManifest)));
  const waterSite = checkpoint.state.players.north.hand.atlas.find(({ cardId }) =>
    cardId === waterSiteId);
  const landSite = checkpoint.state.players.north.hand.atlas.find(({ cardId }) =>
    cardId === landSiteId);
  const featured = checkpoint.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === featuredId);
  const target = checkpoint.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === targetId);
  assert.ok(waterSite);
  assert.ok(landSite);
  assert.ok(featured);
  assert.ok(target);
  const play = (start: GameSession, cardInstanceId: string): GameSession =>
    accept(start, action(start, ({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === cardInstanceId
        && descriptor.cell === 'C4'));

  const land = play(checkpoint, landSite.instanceId);
  const atlasBeforeDeath = land.state.players.north.atlas.length;
  const died = stepGame(land, action(land, ({ descriptor }) =>
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
  assert.equal(died.session.state.stateVersion, land.state.stateVersion + 1);
  assert.equal(verifyGameReplay(died.session), true);

  const banished = stepGame(land, action(land, ({ descriptor }) =>
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
  assert.equal(banished.session.state.stateVersion, land.state.stateVersion + 1);
  assert.equal(verifyGameReplay(banished.session), true);

  let movement = play(checkpoint, waterSite.instanceId);
  movement = accept(movement, action(movement, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === featured.instanceId
      && descriptor.cell === 'C4'
      && descriptor.region === undefined));
  movement = accept(movement, action(movement, ({ descriptor }) => descriptor.kind === 'end-turn'));
  movement = accept(movement, action(movement, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  movement = accept(movement, action(movement, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  movement = accept(movement, action(movement, ({ descriptor }) => descriptor.kind === 'end-turn'));
  movement = accept(movement, action(movement, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const beforeMove = movement;
  const moved = stepGame(beforeMove, action(beforeMove, ({ descriptor }) =>
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
  assert.equal(moved.session.state.stateVersion, beforeMove.state.stateVersion + 1);
  assert.equal(verifyGameReplay(moved.session), true);

  let defense = play(checkpoint, waterSite.instanceId);
  defense = accept(defense, action(defense, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === featured.instanceId
      && descriptor.cell === 'C4'
      && descriptor.region === undefined));
  defense = accept(defense, action(defense, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === target.instanceId
      && descriptor.cell === 'A4'
      && descriptor.region === 'void'));
  defense = accept(defense, action(defense, ({ descriptor }) => descriptor.kind === 'end-turn'));
  defense = accept(defense, action(defense, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  defense = accept(defense, action(defense, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  const southAttacker = defense.state.players.south.hand.spellbook[0];
  assert.ok(southAttacker);
  defense = accept(defense, action(defense, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === southAttacker.instanceId
      && descriptor.cell === 'A4'
      && descriptor.region === 'void'));
  defense = accept(defense, action(defense, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === southAttacker.instanceId
      && descriptor.path.length === 1));
  defense = accept(defense, action(defense, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === target.instanceId));
  const beforeDefend = defense;
  const defended = stepGame(beforeDefend, action(beforeDefend, ({ descriptor }) =>
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
  assert.equal(defended.session.state.stateVersion, beforeDefend.state.stateVersion + 1);
  assert.equal(verifyGameReplay(defended.session), true);
});

test('RULE-02/04 playing a site surfaces uncarried Artifacts from the covered void', () => {
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
  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed < 64; seed += 1) {
    const candidate = createGameManifest({
      authority: {
        contentHash: SYNTHETIC_AUTHORITY_HASH,
        mode: 'synthetic',
        revisionId: 'synthetic-artifact-site-entry-v1',
      },
      cards,
      decks: { north, south },
      firstSeat: 'north',
      seed,
    });
    const hand = createGameSession(candidate).state.players.north.hand.spellbook;
    if (hand.some(({ cardId }) => cardId === 'voidwalker')
      && hand.filter(({ cardId }) => cardId === 'sword').length >= 2) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'voidwalker' && descriptor.cell === 'C4');
  const bearer = session.state.realm.units.find(({ cardId }) => cardId === 'voidwalker');
  assert.ok(bearer);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'sword'
    && descriptor.bearer?.instanceId === bearer.instanceId);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'sword'
    && descriptor.bearer?.instanceId === bearer.instanceId);
  const [carried, stillCarried] = session.state.realm.artifacts ?? [];
  assert.ok(carried);
  assert.ok(stillCarried && 'bearer' in stillCarried);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === bearer.instanceId
    && descriptor.to.cell === 'B4'
    && descriptor.to.region === 'void');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'drop-artifacts'
    && descriptor.unit.instanceId === bearer.instanceId
    && descriptor.artifactInstanceIds.length === 1
    && descriptor.artifactInstanceIds[0] === carried.instanceId);
  const dropped = session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === carried.instanceId);
  assert.deepEqual(dropped, {
    cardId: carried.cardId,
    instanceId: carried.instanceId,
    location: 'B4',
    owner: 'north',
    region: 'void',
    source: carried.source,
  });

  const result = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'B4'));
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  assert.deepEqual(result.receipt.events.map(({ type }) => type), ['site-played', 'mana-gained']);
  assert.deepEqual(result.receipt.randomDraws, []);
  assert.deepEqual(result.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === carried.instanceId), { ...dropped, region: 'surface' });
  assert.deepEqual(result.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === stillCarried.instanceId), stillCarried);
  assert.deepEqual(observeGame(result.session.state, 'north').realm.artifacts
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
  assert.equal(verifyGameReplay(result.session), true);
});

test('RULE-04 Charge allows a summoned minion to Move and Attack immediately', () => {
  let session = keep(createGameSession(manifest(42, {
    spell: {
      charge: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  const unit = session.state.realm.units[0];
  assert.ok(unit);
  assert.equal(unit.summoningSickness, true);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unit.instanceId
      && descriptor.to.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(session.state.realm.units[0]?.tapped, true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Genesis draws a hidden site and an empty Atlas loses after summoning', () => {
  const spell: SpellFacts = {
    genesisDrawSite: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  let session = keep(createGameSession(manifest(40, { spell })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  const before = session.state.players.north;
  const drawn = before.atlas[0];
  assert.ok(drawn);
  const result = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  assert.equal(result.accepted, true);
  session = result.session;
  assert.equal(session.state.players.north.atlas.length, before.atlas.length - 1);
  assert.equal(session.state.players.north.hand.atlas.length, before.hand.atlas.length + 1);
  assert.deepEqual(result.receipt.events.map(({ type }) => type), ['minion-summoned', 'site-drawn']);
  assert.doesNotMatch(canonicalJson(result.receipt.events[1]?.payload ?? null), new RegExp(drawn.cardId));
  assert.doesNotMatch(canonicalJson(observeGame(session.state, 'south')), new RegExp(drawn.cardId));
  assert.equal(verifyGameReplay(session), true);

  const short = deck('genesis-short', 3, 3);
  session = keep(createGameSession(manifest(40, { north: short, south: short, spell })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  assert.equal(session.state.realm.units.length, 1);
  assert.deepEqual(session.state.terminal, {
    loser: 'north',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'south',
  });
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['minion-summoned', 'game-ended'],
  );
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Genesis draws a hidden spell and an empty Spellbook loses after summoning', () => {
  const spell: SpellFacts = {
    genesisDrawSpell: true,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  let session = keep(createGameSession(manifest(127, { spell })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  const before = session.state.players.north;
  const drawn = before.spellbook[0];
  assert.ok(drawn);
  const result = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  assert.equal(result.accepted, true);
  session = result.session;
  assert.equal(session.state.players.north.spellbook.length, before.spellbook.length - 1);
  assert.equal(session.state.players.north.hand.spellbook.length, before.hand.spellbook.length);
  assert.equal(session.state.players.north.hand.spellbook.some(({ instanceId }) => instanceId === drawn.instanceId), true);
  assert.deepEqual(result.receipt.events.map(({ type }) => type), ['minion-summoned', 'spell-drawn']);
  assert.doesNotMatch(canonicalJson(result.receipt.events[1]?.payload ?? null), new RegExp(drawn.cardId));
  assert.doesNotMatch(canonicalJson(observeGame(session.state, 'south')), new RegExp(drawn.cardId));
  assert.equal(verifyGameReplay(session), true);

  const short = deck('genesis-spell-short', 3, 3);
  session = keep(createGameSession(manifest(127, { north: short, south: short, spell })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  assert.equal(session.state.realm.units.length, 1);
  assert.deepEqual(session.state.terminal, {
    loser: 'north',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'south',
  });
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['minion-summoned', 'game-ended'],
  );
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Genesis resolves simultaneous area damage and enemy strikes', () => {
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
  for (const incompatible of [{ token: true }, { waterbound: true }]) {
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
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [staticId]: {
        ...cards[staticId]!,
        genesisDrawSpell: true,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /simultaneous Genesis/);
  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const opening = createGameSession(candidate).state.players;
    const northReady = [staticId, alliedMinionId, titanId].every((cardId) =>
      opening.north.hand.spellbook.some((card) => card.cardId === cardId));
    const southAvailable = [
      ...opening.south.hand.spellbook,
      opening.south.spellbook[0],
    ].flatMap((card) => card ? [card.cardId] : []);
    if (northReady && [teleportId, enemyMinionId, wardedMinionId, undergroundMinionId]
      .every((cardId) => southAvailable.includes(cardId))) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  assert.equal((gameManifest.cards[staticId] as unknown as
    Readonly<Record<string, unknown>>).genesisDamageEachOtherUnitHere, 1);
  assert.equal((gameManifest.cards[titanId] as unknown as
    Readonly<Record<string, unknown>>).genesisStrikeEachEnemyHere, true);
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === alliedMinionId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardId === teleportId
    && descriptor.ally?.kind === 'avatar'
    && descriptor.allyDestination === undefined
    && descriptor.targetLocation?.cell === 'C4');
  for (const minionId of [enemyMinionId, wardedMinionId]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === minionId
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
  }
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === undergroundMinionId
    && descriptor.cell === 'C4'
    && descriptor.region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');

  const beforeVersion = session.state.stateVersion;
  const beforeNorthAtlas = session.state.players.north.atlas.length;
  const alliedMinion = session.state.realm.units.find(({ cardId }) => cardId === alliedMinionId);
  const enemyMinion = session.state.realm.units.find(({ cardId }) => cardId === enemyMinionId);
  const wardedMinion = session.state.realm.units.find(({ cardId }) => cardId === wardedMinionId);
  const undergroundMinion = session.state.realm.units.find(({ cardId }) =>
    cardId === undergroundMinionId);
  assert.ok(alliedMinion);
  assert.ok(enemyMinion);
  assert.ok(wardedMinion);
  assert.ok(undergroundMinion);
  const result = stepGame(session, action(session, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(result.session), true);

  const titanResult = stepGame(session, action(session, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(titanResult.session), true);
});

test('RULE-03/04 Vile Imp may deal 2 damage to one adjacent unit or decline', () => {
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
  let checkpoint = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    checkpoint = accept(checkpoint, action(checkpoint, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const card = checkpoint.state.players.north.hand.spellbook[0];
  assert.ok(card);
  assert.equal((gameManifest.cards[card.cardId] as unknown as
    Readonly<Record<string, unknown>>).genesisMayDamageTargetAdjacentUnit, 2);
  const avatar = checkpoint.state.players.north.avatar.card;
  const wardedTarget = checkpoint.state.realm.units[0];
  assert.ok(wardedTarget);
  assert.equal(wardedTarget.warded, true);
  const summons = legalGameActions(checkpoint.state, 'north')
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

  const decline = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === card.instanceId
      && descriptor.cell === 'C4'
      && descriptor.genesisDamageChoice === 'decline'));
  assert.equal(decline.accepted, true);
  assert.equal(decline.session.state.players.north.avatar.life, 20);
  assert.deepEqual(decline.receipt.events.map(({ type }) => type), ['minion-summoned']);
  assert.equal(verifyGameReplay(decline.session), true);

  const targeted = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
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
  assert.equal(targeted.session.state.stateVersion, checkpoint.state.stateVersion + 1);
  assert.equal(verifyGameReplay(targeted.session), true);

  const warded = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(warded.session), true);
});

test("RULE-03 Genesis life loss reaches but cannot cross Death's Door", () => {
  const readyToSummon = (life: number, seed: number): GameSession => {
    let session = keep(createGameSession(manifest(seed, {
      avatar: { attack: 1, defense: 1, drawSpell: false, life },
      spell: {
        genesisLoseControllerLife: 2,
        manaCost: 0,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    })));
    session = keep(session);
    return accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  };
  const summon = (checkpoint: GameSession) => {
    const result = stepGame(
      checkpoint,
      action(checkpoint, ({ descriptor }) => descriptor.kind === 'summon-minion'),
    );
    assert.equal(result.accepted, true);
    assert.equal(result.session.state.stateVersion, checkpoint.state.stateVersion + 1);
    return result;
  };

  const lifeThree = summon(readyToSummon(3, 226));
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
  assert.equal(verifyGameReplay(lifeThree.session), true);

  const lifeTwo = summon(readyToSummon(2, 227));
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
  assert.equal(verifyGameReplay(lifeTwo.session), true);

  const atDeathsDoor = lifeTwo.session;
  const deathDoorTurn = atDeathsDoor.state.players.north.avatar.deathDoorTurn;
  const lifeZero = summon(atDeathsDoor);
  assert.equal(lifeZero.session.state.players.north.avatar.life, 0);
  assert.equal(lifeZero.session.state.players.north.avatar.deathDoorTurn, deathDoorTurn);
  assert.deepEqual(lifeZero.receipt.events.map(({ type }) => type), ['minion-summoned']);
  assert.deepEqual(lifeZero.session.state.terminal, { status: 'active' });
  assert.equal(verifyGameReplay(lifeZero.session), true);
});

test('RULE-03 Grain Sparrow Genesis gains controller life through shared healing semantics', () => {
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
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'avatar'
    && descriptor.target.seat === 'north');
  assert.equal(session.state.players.north.avatar.life, 19);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const beforeVersion = session.state.stateVersion;
  const result = stepGame(session, action(session, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(result.session), true);
});

test('RULE-03 site Genesis grants temporary mana once, pays a summon, and expires', () => {
  let session = keep(createGameSession(manifest(55, {
    site: { genesisGainMana: 1 },
    spell: {
      manaCost: 2,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  assert.equal(session.state.players.north.mana, 2);
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['site-played', 'mana-gained'],
  );
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  assert.equal(session.state.players.north.mana, 0);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.state.players.north.mana, 0);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  assert.equal(session.state.players.north.mana, 1);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 site Genesis heals every nearby Avatar through shared life caps', () => {
  const base = manifest(247, {
    avatar: { attack: 1, defense: 1, drawSpell: false, life: 10 },
  });
  const preview = createGameSession(base);
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === plainC4 && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === plainC1 && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'avatar' && descriptor.target.seat === 'south');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === plainC3 && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.target?.kind === 'avatar' && descriptor.target.seat === 'north');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === plainC2 && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.from.cell === 'C4' && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  assert.deepEqual({
    north: session.state.players.north.avatar.life,
    south: session.state.players.south.avatar.life,
  }, { north: 7, south: 9 });
  const southTurn = session;

  let farSession = accept(southTurn, action(southTurn, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  farSession = accept(farSession, action(farSession, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const farResult = stepGame(farSession, action(farSession, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(farResult.session), true);

  let nearSession = accept(southTurn, action(southTurn, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === southTurn.state.players.south.avatar.card.instanceId
      && descriptor.from.cell === 'C1'
      && descriptor.to.cell === 'C2'));
  nearSession = accept(nearSession, action(nearSession, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  nearSession = accept(nearSession, action(nearSession, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  nearSession = accept(nearSession, action(nearSession, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const nearResult = stepGame(nearSession, action(nearSession, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(nearResult.session), true);
});

test('RULE-03 site Genesis makes units at nearby sites Immobile until its controller next turn', () => {
  const base = manifest(246);
  const preview = createGameSession(base);
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === plainC4 && descriptor.cell === 'C4');
  for (const cardId of [casterA, casterB]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === cardId && descriptor.cell === 'C4');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === sinkhole && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === disabledEnemy && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === plainC3 && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === quagmire && descriptor.cell === 'C2');

  const quagmireSite = session.state.realm.sites.C2;
  const firstCaster = session.state.realm.units.find(({ cardId }) => cardId === casterA);
  const secondCaster = session.state.realm.units.find(({ cardId }) => cardId === casterB);
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === disabledEnemy);
  assert.ok(quagmireSite);
  assert.ok(firstCaster);
  assert.ok(secondCaster);
  assert.ok(enemy);
  let view = observeGame(session.state, 'north');
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
  const outsidePaths = legalGameActions(session.state, 'north').flatMap(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === firstCaster.instanceId
      ? [descriptor.path]
      : []);
  assert.equal(outsidePaths.some((path) => path.at(-1)?.cell === 'C3'), true);
  assert.equal(outsidePaths.every((path) => {
    const entryIndex = path.findIndex(({ cell }, index) => index > 0 && areaCells.has(cell));
    return entryIndex < 0 || entryIndex === path.length - 1;
  }), true);

  const teleports = [...session.state.players.north.hand.spellbook];
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardInstanceId === teleports[0]?.instanceId
    && descriptor.casterInstanceId === firstCaster.instanceId
    && descriptor.ally?.instanceId === secondCaster.instanceId
    && descriptor.targetLocation?.cell === 'C3');
  view = observeGame(session.state, 'north');
  assert.equal(view.realm.units.find(({ instanceId }) =>
    instanceId === secondCaster.instanceId)?.immobile, true);
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardInstanceId === teleports[1]?.instanceId
    && descriptor.casterInstanceId === secondCaster.instanceId
    && descriptor.ally?.instanceId === secondCaster.instanceId
    && descriptor.targetLocation?.cell === 'C4');
  view = observeGame(session.state, 'north');
  assert.equal(view.realm.units.find(({ instanceId }) =>
    instanceId === secondCaster.instanceId)?.immobile, false);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const southAvatarId = session.state.players.south.avatar.card.instanceId;
  const avatarPaths = legalGameActions(session.state, 'south').flatMap(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === southAvatarId
      ? [descriptor.path]
      : []);
  assert.equal(avatarPaths.length > 0 && avatarPaths.every((path) => path.length === 1), true);
  take(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
    && descriptor.sourceSiteInstanceId === session.state.realm.sites.C1?.instanceId
    && descriptor.targetCell === 'C2');
  view = observeGame(session.state, 'south');
  assert.deepEqual(view.realm.immobileAreas?.[0]?.cells, ['C1', 'C2', 'C3']);
  assert.equal(view.players.south.avatar.immobile, true);
  assert.equal(view.realm.units.find(({ instanceId }) => instanceId === enemy.instanceId)?.immobile, true);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  view = observeGame(session.state, 'north');
  assert.equal(view.realm.immobileAreas, undefined);
  assert.equal(view.players.south.avatar.immobile, false);
  assert.deepEqual({
    disabled: view.realm.units.find(({ instanceId }) => instanceId === enemy.instanceId)?.disabled,
    immobile: view.realm.units.find(({ instanceId }) => instanceId === enemy.instanceId)?.immobile,
  }, { disabled: true, immobile: false });
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === southAvatarId
      && descriptor.path.length > 1), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Aura occupies any canonical 2x2 area and grounds site minions for three controller turns', () => {
  const base = manifest(247);
  const preview = createGameSession(base);
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === northSiteId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === airborneCardId && descriptor.cell === 'C4'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === burrowingCardId && descriptor.cell === 'C4'
    && descriptor.region === 'underground');

  const auraCasts = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
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
  take(({ descriptor }) => descriptor.kind === 'cast-aura'
    && descriptor.cells.every((cell, index) => cell === affectedCells[index]));

  const auraInstance = session.state.realm.auras?.[0];
  const airborne = session.state.realm.units.find(({ cardId }) => cardId === airborneCardId);
  const burrowing = session.state.realm.units.find(({ cardId }) => cardId === burrowingCardId);
  assert.ok(auraInstance);
  assert.ok(airborne);
  assert.ok(burrowing);
  let view = observeGame(session.state, 'north');
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
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'aura-conjured'), true);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  assert.equal(observeGame(session.state, 'north').realm.auras?.[0]?.turnCounters, 1);
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === southSiteId && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === voidwalkCardId && descriptor.cell === 'B3'
    && descriptor.region === 'void');
  const voidwalk = session.state.realm.units.find(({ cardId }) => cardId === voidwalkCardId);
  assert.ok(voidwalk);
  view = observeGame(session.state, 'south');
  assert.equal(view.realm.units.find(({ instanceId }) =>
    instanceId === voidwalk.instanceId)?.immobile, false);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  assert.equal(observeGame(session.state, 'north').realm.auras?.[0]?.turnCounters, 2);
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  view = observeGame(session.state, 'north');
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
  assert.deepEqual(session.transcript.at(-1)?.events
    .filter(({ type }) => type.startsWith('aura-'))
    .map(({ type }) => type), ['aura-turn-counted', 'aura-dispelled']);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 oversized minions occupy one canonical 2x2 footprint for movement, combat, Auras, and terrain', () => {
  const base = manifest(248);
  const preview = createGameSession(base);
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  const northSites = [
    ...session.state.players.north.hand.atlas,
    session.state.players.north.atlas[0]!,
  ];
  const southSites = [
    ...session.state.players.south.hand.atlas,
    session.state.players.south.atlas[0]!,
  ];
  const play = (
    seat: 'north' | 'south',
    index: number,
    cell: 'B1' | 'B2' | 'B3' | 'B4' | 'C1' | 'C2' | 'C3' | 'C4',
  ): void => {
    const card = (seat === 'north' ? northSites : southSites)[index];
    assert.ok(card);
    take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === card.instanceId && descriptor.cell === cell);
  };
  const endAndDraw = (zone: 'atlas' | 'spellbook' = 'spellbook'): void => {
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === zone);
  };

  play('north', 0, 'C4');
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === giantCardId), false);
  endAndDraw();
  play('south', 0, 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === enemyCardId && descriptor.cell === 'C1');
  endAndDraw();
  play('north', 1, 'B4');
  endAndDraw('atlas');
  play('south', 1, 'B1');
  endAndDraw();
  play('north', 2, 'C3');
  endAndDraw('atlas');
  play('south', 2, 'C2');
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === enemyCardId);
  assert.ok(enemy);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === enemy.instanceId
    && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  endAndDraw('atlas');

  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === giantCardId), false);
  play('north', 3, 'B3');
  const giantSummons = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === giantCardId);
  assert.deepEqual(giantSummons.flatMap(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cells ? [descriptor.cells] : []), [
    ['B3', 'B4', 'C3', 'C4'],
  ]);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === giantCardId
    && descriptor.cells?.every((cell, index) =>
      cell === ['B3', 'B4', 'C3', 'C4'][index]) === true);
  const giant = session.state.realm.units.find(({ cardId }) => cardId === giantCardId);
  assert.ok(giant);
  assert.deepEqual(giant.occupiedCells, ['B3', 'B4', 'C3', 'C4']);
  assert.deepEqual(observeGame(session.state, 'south').realm.units
    .find(({ instanceId }) => instanceId === giant.instanceId)?.occupiedCells,
  ['B3', 'B4', 'C3', 'C4']);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === artifactCardId
    && descriptor.bearer?.instanceId === giant.instanceId
    && descriptor.bearerCell === 'C4');
  const artifact = session.state.realm.artifacts?.find(({ cardId }) =>
    cardId === artifactCardId);
  assert.ok(artifact);
  assert.deepEqual(observeGame(session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === artifact.instanceId)?.location, 'C4');

  take(({ descriptor }) => descriptor.kind === 'cast-aura'
    && descriptor.cardId === auraCardId
    && descriptor.cells.every((cell, index) =>
      cell === ['A2', 'A3', 'B2', 'B3'][index]));
  assert.equal(observeGame(session.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === giant.instanceId)?.immobile, true);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === giant.instanceId
      && descriptor.path.length > 1), false);

  for (let round = 0; round < 3; round += 1) {
    endAndDraw(round === 2 ? 'atlas' : 'spellbook');
    if (round === 0) {
      play('south', 3, 'B2');
      take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === remoteEnemyCardId && descriptor.cell === 'B2');
    }
    endAndDraw('spellbook');
  }
  const remoteInterceptor = session.state.realm.units.find(({ cardId, location }) =>
    cardId === remoteEnemyCardId && location === 'B2');
  assert.ok(remoteInterceptor);
  const teleportTargetSite = session.state.realm.sites.C3;
  assert.ok(teleportTargetSite);
  const destinationAreas = [
    ['B2', 'B3', 'C2', 'C3'],
    ['B3', 'B4', 'C3', 'C4'],
  ];
  const teleportActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
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
  const teleported = stepGame(session, shiftedTeleport);
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
  assert.equal(verifyGameReplay(teleported.session), true);
  const leapActions = legalGameActions(teleported.session.state, 'north')
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
  const leaped = stepGame(teleported.session, focusedLeap);
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
  assert.equal(verifyGameReplay(leaped.session), true);

  const blinkActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
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
  const blinked = stepGame(session, shiftedBlink);
  assert.equal(blinked.accepted, true);
  if (!blinked.accepted) return;
  assert.deepEqual(blinked.session.state.realm.units
    .find(({ instanceId }) => instanceId === giant.instanceId)?.occupiedCells,
  destinationAreas[0]);
  assert.equal(verifyGameReplay(blinked.session), true);

  const translated = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === giant.instanceId
      && descriptor.from.cell === 'B3' && descriptor.to.cell === 'B2');
  assert.deepEqual(translated.descriptor.kind === 'move-and-attack'
    ? translated.descriptor.path.map(({ cell }) => cell)
    : [], ['B3', 'B2']);
  take(({ actionId }) => actionId === translated.actionId);
  assert.deepEqual(session.state.realm.units
    .find(({ instanceId }) => instanceId === giant.instanceId)?.occupiedCells,
  ['B2', 'B3', 'C2', 'C3']);
  assert.equal(observeGame(session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === artifact.instanceId)?.location, 'C3');
  const attackCheckpoint = session;
  const declined = stepGame(attackCheckpoint, action(attackCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(declined.accepted, true);
  if (!declined.accepted) return;
  assert.equal(declined.session.state.pendingCombat?.cell, 'B2');
  assert.equal(legalGameActions(declined.session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'intercept'
      && descriptor.unitInstanceId === enemy.instanceId), false);
  const intercepted = stepGame(declined.session, action(declined.session, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(intercepted.session), true);

  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === enemy.instanceId);
  assert.equal(session.state.pendingCombat?.cell, 'C2');
  take(({ descriptor }) => descriptor.kind === 'close-defend'
    && descriptor.originalTargetParticipates);
  assert.equal(session.state.realm.units.some(({ cardId }) => cardId === enemyCardId), false);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === giant.instanceId)?.damage, 2);

  endAndDraw();
  const targetSite = session.state.realm.sites.C2;
  assert.ok(targetSite);
  const caveIn = stepGame(session, action(session, ({ descriptor }) =>
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
  assert.equal(verifyGameReplay(caveIn.session), true);
});

test('RULE-03 Tower Genesis grants mana only for the first controlled copy', () => {
  const north = deck('tower-north');
  let session = keep(keep(createGameSession(manifest(56, {
    north: { ...north, atlas: ['dark-tower', 'gothic-tower', 'dark-tower'] },
    site: { genesisGainManaIfOnlyControlledCopy: 1 },
  }))));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  const playNorthSite = (cardId: string, gainsBonus: boolean): void => {
    const instanceId = session.state.players.north.hand.atlas
      .find((card) => card.cardId === cardId)?.instanceId;
    assert.ok(instanceId);
    const manaBefore = session.state.players.north.mana;
    take(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cardInstanceId === instanceId);
    assert.equal(session.state.players.north.mana - manaBefore, gainsBonus ? 2 : 1);
    assert.deepEqual(session.transcript.at(-1)?.events.map(({ type }) => type), [
      'site-played',
      ...(gainsBonus ? ['mana-gained'] : []),
    ]);
  };
  playNorthSite('dark-tower', true);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  playNorthSite('gothic-tower', true);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  playNorthSite('dark-tower', false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Humble Village Genesis may spend its mana to summon one Foot Soldier', () => {
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
  const checkpoint = keep(keep(createGameSession(gameManifest)));
  const villageInstanceId = checkpoint.state.players.north.hand.atlas[0]!.instanceId;
  const choices = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
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

  const declinedResult = stepGame(checkpoint, declined);
  const paidResult = stepGame(checkpoint, paid);
  assert.equal(declinedResult.accepted, true);
  assert.equal(paidResult.accepted, true);
  if (!declinedResult.accepted || !paidResult.accepted) return;
  assert.equal(declinedResult.session.state.stateVersion, checkpoint.state.stateVersion + 1);
  assert.equal(paidResult.session.state.stateVersion, checkpoint.state.stateVersion + 1);
  assert.equal(declinedResult.session.state.players.north.mana, 1);
  assert.equal(declinedResult.session.state.realm.units.length, 0);
  assert.deepEqual(declinedResult.receipt.events.map(({ type }) => type), ['site-played']);

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
  assert.equal(declinedResult.receipt.randomDraws.length, 0);
  assert.equal(paidResult.receipt.randomDraws.length, 0);
  assert.equal(verifyGameReplay(declinedResult.session), true);
  assert.equal(verifyGameReplay(paidResult.session), true);
});

test('RULE-02/03 Geomancer creates Rubble and privately replaces it with the top Atlas site', () => {
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
  let session = keep(keep(createGameSession(gameManifest)));

  const firstPlay = action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cell === 'C4'
      && descriptor.createRubbleAt === 'C3'
      && descriptor.genesisTokenChoice === 'decline');
  const firstResult = stepGame(session, firstPlay);
  assert.equal(firstResult.accepted, true);
  if (!firstResult.accepted) return;
  session = firstResult.session;
  assert.deepEqual(firstResult.receipt.events.map(({ type }) => type), [
    'site-played',
    'rubble-created',
  ]);
  assert.equal('rubble' in session.state.realm.sites.C3!, true);

  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const top = session.state.players.north.atlas[0];
  const swap = session.state.players.north.hand.atlas[0];
  assert.ok(top);
  assert.ok(swap);
  const before = canonicalJson(observeGame(session.state, 'north') as unknown as JsonValue);
  assert.equal(before.includes(top.cardId), false);
  assert.equal(before.includes(top.instanceId), false);
  const replacement = action(session, ({ descriptor }) =>
    descriptor.kind === 'replace-rubble-with-top-atlas-site'
      && descriptor.targetCell === 'C3');
  assert.equal(replacement.label, 'Replace Rubble at C3 with the top site of your Atlas');
  assert.equal(canonicalJson(replacement as unknown as JsonValue).includes(top.cardId), false);
  assert.equal(canonicalJson(replacement as unknown as JsonValue).includes(top.instanceId), false);

  const swapped: GameSession = {
    ...session,
    state: {
      ...session.state,
      players: {
        ...session.state.players,
        north: {
          ...session.state.players.north,
          atlas: [swap, ...session.state.players.north.atlas.slice(1)],
          hand: {
            ...session.state.players.north.hand,
            atlas: [top, ...session.state.players.north.hand.atlas.slice(1)],
          },
        },
      },
    },
  };
  const swappedReplacement = action(swapped, ({ descriptor }) =>
    descriptor.kind === 'replace-rubble-with-top-atlas-site'
      && descriptor.targetCell === 'C3');
  assert.equal(swappedReplacement.actionId, replacement.actionId);
  assert.deepEqual(swappedReplacement.descriptor, replacement.descriptor);

  const replaced = stepGame(session, replacement);
  assert.equal(replaced.accepted, true);
  if (!replaced.accepted) return;
  session = replaced.session;
  assert.deepEqual(replaced.receipt.events.map(({ type }) => type), [
    'rubble-replaced',
    'site-played',
  ]);
  assert.equal(session.state.phase, 'genesis');
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.players.north.atlas.length, 0);
  assert.equal(session.state.players.north.hand.atlas.length, 2);
  assert.equal(session.state.realm.sites.C3?.instanceId, top.instanceId);
  assert.equal(Object.values(session.state.realm.sites)
    .filter((site) => 'rubble' in site).length, 0);
  const revealed = canonicalJson(observeGame(session.state, 'south') as unknown as JsonValue);
  assert.equal(revealed.includes(top.cardId), true);
  assert.equal(revealed.includes(top.instanceId), true);

  const choices = legalGameActions(session.state, 'north');
  assert.equal(choices.length, 2);
  assert.equal(choices.every(({ descriptor }) => descriptor.kind === 'resolve-genesis-token'), true);
  const paid = action(session, ({ descriptor }) =>
    descriptor.kind === 'resolve-genesis-token' && descriptor.choice === 'pay-one-mana');
  const paidResult = stepGame(session, paid);
  assert.equal(paidResult.accepted, true);
  if (!paidResult.accepted) return;
  session = paidResult.session;
  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.pendingGenesisToken, null);
  assert.equal(session.state.players.north.mana, 1);
  assert.equal(session.state.realm.units.at(-1)?.cardId, 'foot-soldier');
  assert.deepEqual(paidResult.receipt.events.map(({ type }) => type), ['minion-summoned']);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Hunter\'s Lodge Genesis removes only enemy Stealth', () => {
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

  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  assert.deepEqual(session.transcript.at(-1)?.events.map(({ type }) => type), ['site-played']);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'lodge-ally' && descriptor.cell === 'C4');
  const ally = session.state.realm.units.find(({ cardId }) => cardId === 'lodge-ally')!;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'lodge-enemy' && descriptor.cell === 'C4');
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === 'lodge-enemy')!;
  assert.equal(ally.stealthed, true);
  assert.equal(enemy.stealthed, true);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

  const play = action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardId === 'hunters-lodge' && descriptor.cell === 'C3');
  if (play.descriptor.kind !== 'play-site') throw new Error('expected Hunter\'s Lodge play');
  const result = stepGame(session, play);
  assert.equal(result.accepted, true);
  session = result.session;
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
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === ally.instanceId)?.stealthed, true);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === enemy.instanceId)?.stealthed, false);
  assert.deepEqual(result.receipt.randomDraws, []);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 seasonal River Genesis privately keeps or bottoms the next spell', () => {
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

  const checkpoint = keep(keep(createGameSession(gameManifest)));
  const before = checkpoint.state.players.north.spellbook;
  const [top, next] = before;
  const riverInstanceId = checkpoint.state.players.north.hand.atlas[0]?.instanceId;
  assert.ok(top);
  assert.ok(next);
  assert.ok(riverInstanceId);
  const southBefore = canonicalJson(observeGame(checkpoint.state, 'south') as unknown as JsonValue);
  assert.equal(southBefore.includes(top.cardId), false);
  assert.equal(southBefore.includes(top.instanceId), false);

  const plays = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === riverInstanceId
      && descriptor.cell === 'C4');
  const play = plays[0];
  assert.equal(plays.length, 1);
  assert.ok(play);
  assert.equal(play.label.includes(top.cardId), false);
  const played = stepGame(checkpoint, play);
  assert.equal(played.accepted, true);
  if (!played.accepted) return;
  assert.equal(played.session.state.phase, 'genesis');
  assert.deepEqual(played.session.state.players.north.spellbook, before);
  assert.deepEqual(played.receipt.events.map(({ type }) => type), ['site-played']);
  const pendingSouth = canonicalJson(observeGame(played.session.state, 'south') as unknown as JsonValue);
  assert.equal(pendingSouth.includes(top.cardId), false);
  assert.equal(pendingSouth.includes(top.instanceId), false);

  const choices = legalGameActions(played.session.state, 'north');
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

  const kept = stepGame(played.session, keepNext);
  const bottomed = stepGame(played.session, bottomNext);
  assert.equal(kept.accepted, true);
  assert.equal(bottomed.accepted, true);
  if (!kept.accepted || !bottomed.accepted) return;
  assert.equal(kept.session.state.stateVersion, checkpoint.state.stateVersion + 2);
  assert.equal(bottomed.session.state.stateVersion, checkpoint.state.stateVersion + 2);
  assert.equal(kept.session.state.phase, 'main');
  assert.equal(bottomed.session.state.phase, 'main');
  assert.equal(kept.session.state.pendingGenesisSpell, null);
  assert.equal(bottomed.session.state.pendingGenesisSpell, null);
  assert.deepEqual(kept.session.state.players.north.spellbook, before);
  assert.deepEqual(bottomed.session.state.players.north.spellbook, [...before.slice(1), top]);
  assert.equal(bottomed.session.state.players.north.spellbook[0]?.instanceId, next.instanceId);
  assert.deepEqual(kept.receipt.events.map(({ type }) => type), ['spell-kept']);
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
    canonicalJson(observeGame(kept.session.state, 'south') as unknown as JsonValue),
    canonicalJson(observeGame(bottomed.session.state, 'south') as unknown as JsonValue),
  );
  assert.equal(verifyGameReplay(kept.session), true);
  assert.equal(verifyGameReplay(bottomed.session), true);
});

test('RULE-03 adjacent matching sites trigger one spell draw apiece and a short deck loses', () => {
  const base = deck('leyline-north');
  const north = {
    ...base,
    atlas: base.atlas.map(() => 'leyline-site'),
  };
  let session = keep(createGameSession(manifest(137, {
    north,
    site: { genesisDrawSpellPerAdjacentSameCard: true },
  })));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const firstHandSize = session.state.players.north.hand.spellbook.length;
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  assert.equal(session.state.players.north.hand.spellbook.length, firstHandSize);
  assert.deepEqual(session.transcript.at(-1)?.events.map(({ type }) => type), ['site-played']);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const beforeOne = session.state.players.north.hand.spellbook.length;
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
  assert.equal(session.state.players.north.hand.spellbook.length, beforeOne + 1);
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['site-played', 'spell-drawn'],
  );
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  const beforeTwo = session.state.players.north.hand.spellbook.length;
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
  assert.equal(session.state.players.north.hand.spellbook.length, beforeTwo + 2);
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['site-played', 'spell-drawn', 'spell-drawn'],
  );
  assert.equal(verifyGameReplay(session), true);

  const shortBase = deck('leyline-short', 30, 6);
  const short = {
    ...shortBase,
    atlas: shortBase.atlas.map(() => 'leyline-short-site'),
  };
  session = keep(createGameSession(manifest(138, {
    north: short,
    site: { genesisDrawSpellPerAdjacentSameCard: true },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'B4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  const beforePartial = session.state.players.north;
  assert.equal(beforePartial.spellbook.length, 1);
  const fourth = action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'B3');
  const partial = stepGame(session, fourth);
  assert.equal(partial.accepted, true);
  session = partial.session;
  const fourthInstanceId = fourth.descriptor.kind === 'play-site'
    ? fourth.descriptor.cardInstanceId
    : '';
  assert.equal(session.state.players.north.spellbook.length, 0);
  assert.equal(
    session.state.players.north.hand.spellbook.length,
    beforePartial.hand.spellbook.length + 1,
  );
  assert.equal(session.state.realm.sites.B3?.instanceId, fourthInstanceId);
  assert.deepEqual(session.state.terminal, {
    loser: 'north',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'south',
  });
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['site-played', 'spell-drawn', 'game-ended'],
  );
  assert.equal(
    canonicalJson(session.transcript.at(-1)?.events[1]?.payload ?? null)
      .includes(fourthInstanceId),
    true,
  );
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 site Genesis discards up to two top spells publicly without deck-out', () => {
  let session = keep(createGameSession(manifest(139, {
    site: { genesisDiscardTopSpells: 2 },
  })));
  session = keep(session);
  const beforeVersion = session.state.stateVersion;
  const [first, second, next] = session.state.players.north.spellbook;
  assert.ok(first);
  assert.ok(second);
  assert.ok(next);
  const southBefore = canonicalJson(observeGame(session.state, 'south'));
  for (const hidden of [first, second, next]) {
    assert.equal(southBefore.includes(hidden.cardId), false);
    assert.equal(southBefore.includes(hidden.instanceId), false);
  }
  const play = action(session, ({ descriptor }) => descriptor.kind === 'play-site');
  if (play.descriptor.kind !== 'play-site') throw new Error('expected site play');
  const sourceInstanceId = play.descriptor.cardInstanceId;
  const result = stepGame(session, play);
  assert.equal(result.accepted, true);
  session = result.session;

  assert.equal(session.state.stateVersion, beforeVersion + 1);
  assert.deepEqual(session.state.players.north.cemetery, [first, second]);
  assert.equal(session.state.players.north.spellbook[0]?.instanceId, next.instanceId);
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
  const southAfter = canonicalJson(observeGame(session.state, 'south'));
  assert.equal(southAfter.includes(first.instanceId), true);
  assert.equal(southAfter.includes(second.instanceId), true);
  assert.equal(southAfter.includes(next.instanceId), false);
  assert.deepEqual(session.state.terminal, { status: 'active' });
  assert.equal(verifyGameReplay(session), true);

  session = keep(createGameSession(manifest(140, {
    north: deck('shallow-short', 30, 4),
    site: { genesisDiscardTopSpells: 2 },
  })));
  session = keep(session);
  const only = session.state.players.north.spellbook[0];
  assert.ok(only);
  const partial = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  assert.equal(partial.accepted, true);
  session = partial.session;
  assert.deepEqual(session.state.players.north.cemetery, [only]);
  assert.equal(session.state.players.north.spellbook.length, 0);
  assert.deepEqual(partial.receipt.events.map(({ type }) => type), ['site-played', 'spell-discarded']);
  assert.deepEqual(session.state.terminal, { status: 'active' });
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Vikings area damage uses bearer Lethal without becoming a strike', () => {
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

  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'vikings' && descriptor.cell === 'C3');
  const vikings = session.state.realm.units.find(({ cardId }) => cardId === 'vikings')!;
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === vikings.instanceId), false);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'poisonous-dagger'
    && descriptor.bearer?.instanceId === vikings.instanceId);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'disabled-vikings' && descriptor.cell === 'C3');
  const disabled = session.state.realm.units.find(({ cardId }) => cardId === 'disabled-vikings')!;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'vikings-warded-enemy' && descriptor.cell === 'C2');
  const warded = session.state.realm.units.find(({ cardId }) => cardId === 'vikings-warded-enemy')!;
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'vikings-stealthed-enemy' && descriptor.cell === 'C2');
  const stealthed = session.state.realm.units.find(({ cardId }) => cardId === 'vikings-stealthed-enemy')!;
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'vikings-submerged-enemy'
    && descriptor.cell === 'C2' && descriptor.region === 'underwater');
  const submerged = session.state.realm.units.find(({ cardId }) => cardId === 'vikings-submerged-enemy')!;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'vikings-ally' && descriptor.cell === 'C2');
  const ally = session.state.realm.units.find(({ cardId }) => cardId === 'vikings-ally')!;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.players.south.avatar.card.instanceId
    && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

  const checkpoint = session;
  const activations = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === vikings.instanceId);
  assert.deepEqual(activations.flatMap(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage' ? [descriptor.targetLocation.cell] : []), ['C2', 'C4']);
  assert.equal(activations.every(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage' && descriptor.targetLocation.region === 'surface'), true);
  assert.equal(observeGame(checkpoint.state, 'north').realm.units.find(({ instanceId }) =>
    instanceId === disabled.instanceId)?.disabled, true);
  assert.equal(legalGameActions(checkpoint.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === disabled.instanceId), false);

  const result = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'activate-area-damage'
      && descriptor.sourceInstanceId === vikings.instanceId
      && descriptor.targetLocation.cell === 'C2'));
  assert.equal(result.accepted, true);
  session = result.session;
  const events = result.receipt.events;
  assert.equal(events.some(({ type }) => type === 'strike-damage-allocated'), false);
  assert.deepEqual(events.slice(0, 2).map(({ type }) => type), ['area-damage-activated', 'stealth-lost']);
  assert.equal(events.slice(2, 6).every(({ type }) => type === 'area-damage-allocated'), true);
  assert.deepEqual(events.filter(({ type }) => type === 'area-damage-allocated')
    .map(({ payload }) => (payload as unknown as Readonly<Record<string, unknown>>).targetInstanceId)
    .sort(), [
    ally.instanceId,
    session.state.players.south.avatar.card.instanceId,
    stealthed.instanceId,
    warded.instanceId,
  ].sort());
  assert.equal(events.filter(({ type }) => type === 'minion-died').length, 2);
  assert.equal(events.some(({ payload, type }) => type === 'damage-dealt'
    && (payload as unknown as Readonly<Record<string, unknown>>).instanceId === stealthed.instanceId
    && (payload as unknown as Readonly<Record<string, unknown>>).amount === 2), true);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === ally.instanceId), false);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === stealthed.instanceId), false);
  assert.equal(session.state.realm.artifacts?.some((artifact) =>
    'bearer' in artifact && artifact.bearer.instanceId === vikings.instanceId), true);
  const wardedAfter = session.state.realm.units.find(({ instanceId }) => instanceId === warded.instanceId)!;
  assert.deepEqual({ damage: wardedAfter.damage, warded: wardedAfter.warded }, { damage: 0, warded: false });
  const submergedAfter = session.state.realm.units.find(({ instanceId }) => instanceId === submerged.instanceId)!;
  assert.deepEqual({ damage: submergedAfter.damage, region: submergedAfter.region }, {
    damage: 0,
    region: 'underwater',
  });
  assert.equal(session.state.players.south.avatar.life, 18);
  assert.equal(session.state.realm.units.find(({ instanceId }) => instanceId === vikings.instanceId)?.tapped, true);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-area-damage' && descriptor.sourceInstanceId === vikings.instanceId), false);
  assert.deepEqual(result.receipt.randomDraws, []);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 a minion mana ability requires readiness, taps, and expires at End Phase', () => {
  let session = keep(createGameSession(manifest(39, {
    spell: {
      manaCost: 1,
      stealth: true,
      tapForMana: 2,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  const unit = session.state.realm.units[0];
  assert.ok(unit);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === unit.instanceId), false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const before = session.state.players.north.mana;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === unit.instanceId));
  assert.equal(session.state.players.north.mana, before + 2);
  assert.equal(session.state.realm.units[0]?.tapped, true);
  assert.equal(session.state.realm.units[0]?.stealthed, false);
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === unit.instanceId), false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  assert.equal(session.state.players.north.mana, 1);
  assert.equal(session.state.realm.units[0]?.tapped, false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 mana and every elemental threshold gate minion actions without being spent together', () => {
  let session = northSecondMain(43, false, {
    manaCost: 2,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  assert.equal(session.state.players.north.mana, 1);
  assert.equal(observeGame(session.state, 'north').players.north.affinity.earth, 1);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion'), false);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  assert.equal(session.state.players.north.mana, 2);
  assert.equal(observeGame(session.state, 'north').players.north.affinity.earth, 2);
  const cardId = session.state.players.north.hand.spellbook[0]?.cardId;
  assert.ok(cardId);
  const cells = legalGameActions(session.state, 'north')
    .filter(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cardId === cardId)
    .map(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell);
  assert.deepEqual(cells, ['C3', 'C4']);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === cardId && descriptor.cell === 'C4'));
  assert.equal(session.state.players.north.mana, 0);
  assert.equal(observeGame(session.state, 'north').players.north.affinity.earth, 2);
  assert.equal(verifyGameReplay(session), true);
});

function northAttacksAtC2(
  seed: number,
  spell?: SpellFacts,
  avatar?: AvatarFacts,
  emptyAtlasAfterOpening = false,
  southSpell?: SpellFacts,
): Readonly<{
  attackerInstanceId: string;
  defenderInstanceId: string;
  session: GameSession;
  targetInstanceId: string;
}> {
  const shortDecks = emptyAtlasAfterOpening
    ? { north: deck('north', 3), south: deck('south', 3) }
    : {};
  let session = keep(createGameSession(manifest(seed, {
    ...shortDecks,
    ...(spell ? { spell } : {}),
    ...(southSpell ? { southSpell } : {}),
    ...(avatar ? { avatar } : {}),
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const attackerInstanceId = session.state.realm.units.find(({ controller }) => controller === 'north')?.instanceId;
  assert.ok(attackerInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const defenderInstanceId = session.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
  assert.ok(defenderInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attackerInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C2'));
  const targetInstanceId = session.state.realm.units
    .find(({ controller, location }) => controller === 'south' && location === 'C2')?.instanceId;
  assert.ok(targetInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attackerInstanceId
      && descriptor.from.cell === 'C3'
      && descriptor.to.cell === 'C2'));
  return { attackerInstanceId, defenderInstanceId, session, targetInstanceId };
}

test('RULE-04 a restricted attacker can target units but not sites', () => {
  const setup = northAttacksAtC2(114, {
    attack: 4,
    cannotAttackSites: true,
    charge: true,
    defense: 4,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  let { session } = setup;
  const actions = legalGameActions(session.state, 'north');
  const targets = actions.flatMap(({ descriptor }) =>
    descriptor.kind === 'declare-attack' ? [descriptor.target] : []);
  assert.equal(targets.some(({ instanceId, kind }) =>
    kind === 'minion' && instanceId === setup.targetInstanceId), true);
  assert.equal(targets.some(({ kind }) => kind === 'site'), false);
  assert.equal(actions.some(({ descriptor }) => descriptor.kind === 'decline-attack'), true);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 attacking-only first strike resolves deaths before normal strikes and is inactive while defending', () => {
  const vanilla = {
    attack: 3,
    defense: 3,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const firstStrike = { ...vanilla, strikesFirstWhileAttacking: true };
  const attacking = northAttacksAtC2(117, firstStrike, undefined, false, vanilla);
  let session = accept(attacking.session, action(attacking.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === attacking.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === attacking.attackerInstanceId)?.damage, 0);
  assert.equal(session.state.players.south.cemetery
    .some(({ instanceId }) => instanceId === attacking.targetInstanceId), true);
  assert.equal(verifyGameReplay(session), true);

  const defending = northAttacksAtC2(118, vanilla, undefined, false, firstStrike);
  session = accept(defending.session, action(defending.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === defending.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(session.state.players.north.cemetery
    .some(({ instanceId }) => instanceId === defending.attackerInstanceId), true);
  assert.equal(session.state.players.south.cemetery
    .some(({ instanceId }) => instanceId === defending.targetInstanceId), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/04 Genesis sleep ends on real damage without retroactive strikes', () => {
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
  const fight = (seed: number, northSpell: SpellFacts, southSpell: SpellFacts) => {
    const setup = northAttacksAtC2(seed, northSpell, undefined, false, southSpell);
    assert.equal(observeGame(setup.session.state, 'north').realm.units
      .find(({ instanceId }) => instanceId === setup.targetInstanceId)?.disabled, true);
    let session = accept(setup.session, action(setup.session, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    return { ...setup, session };
  };

  const ordinary = fight(119, attacker, sleeper);
  assert.equal(ordinary.session.state.realm.units
    .find(({ instanceId }) => instanceId === ordinary.attackerInstanceId)?.damage, 0);
  assert.equal(ordinary.session.state.realm.units
    .find(({ instanceId }) => instanceId === ordinary.targetInstanceId)?.damage, 2);
  assert.equal(observeGame(ordinary.session.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === ordinary.targetInstanceId)?.disabled, false);
  assert.equal(ordinary.session.transcript.at(-1)?.events
    .filter(({ type }) => type === 'minion-awakened').length, 1);
  assert.equal(verifyGameReplay(ordinary.session), true);

  const early = fight(120, { ...attacker, strikesFirstWhileAttacking: true }, sleeper);
  assert.equal(early.session.state.realm.units
    .find(({ instanceId }) => instanceId === early.attackerInstanceId)?.damage, 5);
  assert.equal(early.session.state.realm.units
    .find(({ instanceId }) => instanceId === early.targetInstanceId)?.damage, 2);
  assert.equal(observeGame(early.session.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === early.targetInstanceId)?.disabled, false);
  assert.equal(verifyGameReplay(early.session), true);

  const warded = fight(121, attacker, { ...sleeper, ward: true });
  assert.equal(warded.session.state.realm.units
    .find(({ instanceId }) => instanceId === warded.targetInstanceId)?.damage, 0);
  assert.equal(observeGame(warded.session.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === warded.targetInstanceId)?.disabled, true);
  assert.equal(warded.session.transcript.at(-1)?.events
    .some(({ type }) => type === 'minion-awakened'), false);
  assert.equal(verifyGameReplay(warded.session), true);
});

test('RULE-04 Lance buffs and breaks on the next unit strike but not a site strike', () => {
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

  const attacking = northAttacksAtC2(124, lance, undefined, false, twoPower);
  assert.equal(observeGame(attacking.session.state, 'south').realm.units
    .find(({ instanceId }) => instanceId === attacking.attackerInstanceId)?.carriedLanceCount, 1);
  assert.equal(attacking.session.transcript.some(({ events }) => events.some(({ payload, type }) =>
    type === 'lance-gained'
      && canonicalJson(payload) === canonicalJson({
        bearerInstanceId: attacking.attackerInstanceId,
        count: 1,
        sourceInstanceId: attacking.attackerInstanceId,
      }))), true);
  let session = accept(attacking.session, action(attacking.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === attacking.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  const attackEvents = session.transcript.at(-1)!.events;
  assert.deepEqual(attackEvents.find(({ type }) => type === 'lance-broken')?.payload, {
    bearerInstanceId: attacking.attackerInstanceId,
    count: 1,
    sourceInstanceId: attacking.attackerInstanceId,
  });
  assert.ok(attackEvents.findIndex(({ type }) => type === 'lance-broken')
    < attackEvents.findIndex(({ type }) => type === 'minion-died'));
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === attacking.attackerInstanceId)?.damage, 0);
  assert.equal(session.state.players.south.cemetery
    .some(({ instanceId }) => instanceId === attacking.targetInstanceId), true);
  assert.equal(observeGame(session.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === attacking.attackerInstanceId)?.carriedLanceCount, undefined);
  assert.equal(verifyGameReplay(session), true);

  const site = northAttacksAtC2(125, lance, undefined, false, twoPower);
  session = accept(site.session, action(site.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'close-defend'));
  const siteEvents = session.transcript.at(-1)!.events;
  assert.equal(siteEvents.some(({ type }) => type === 'lance-broken'), false);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === site.attackerInstanceId)?.carriedLanceCount, 1);
  const siteStruck = siteEvents.find(({ type }) => type === 'undefended-site-struck');
  assert.ok(siteStruck);
  assert.equal(canonicalJson(siteStruck.payload).includes('"amount":1'), true);
  assert.equal(verifyGameReplay(session), true);

  const tripleLance = { ...lance, defense: 3, lanceCount: 3 as const };
  const defending = northAttacksAtC2(126, {
    ...lance,
    defense: 4,
  }, undefined, false, tripleLance);
  session = accept(defending.session, action(defending.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === defending.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  const defendEvents = session.transcript.at(-1)!.events;
  assert.equal(session.state.players.north.cemetery
    .some(({ instanceId }) => instanceId === defending.attackerInstanceId), true);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === defending.targetInstanceId)?.damage, 2);
  assert.deepEqual(defendEvents.find(({ payload, type }) =>
    type === 'lance-broken' && canonicalJson(payload).includes(defending.targetInstanceId))?.payload, {
    bearerInstanceId: defending.targetInstanceId,
    count: 3,
    sourceInstanceId: defending.targetInstanceId,
  });
  assert.deepEqual(defendEvents.find(({ payload, type }) =>
    type === 'damage-dealt' && canonicalJson(payload).includes(defending.attackerInstanceId))?.payload, {
    accumulated: 4,
    amount: 4,
    direct: true,
    instanceId: defending.attackerInstanceId,
    seat: 'north',
  });
  assert.equal(verifyGameReplay(session), true);

  const ranged = northAttacksAtC2(127, { ...lance, ranged: true }, undefined, false, {
    ...twoPower,
    defense: 3,
    ward: true,
  });
  session = accept(ranged.session, action(ranged.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  if (session.state.phase === 'intercept') {
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'close-intercept'));
  }
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === ranged.attackerInstanceId
      && descriptor.hit?.instanceId === ranged.targetInstanceId));
  const rangedEvents = session.transcript.at(-1)!.events;
  assert.deepEqual(rangedEvents.map(({ type }) => type), [
    'projectile-shot',
    'strike-damage-allocated',
    'damage-dealt',
    'ward-broken',
    'lance-broken',
  ]);
  assert.equal(canonicalJson(rangedEvents[1]!.payload).includes('"amount":2'), true);
  const rangedTarget = session.state.realm.units
    .find(({ instanceId }) => instanceId === ranged.targetInstanceId);
  assert.deepEqual(
    rangedTarget && { damage: rangedTarget.damage, warded: rangedTarget.warded },
    { damage: 0, warded: false },
  );
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === ranged.attackerInstanceId)?.carriedLanceCount, undefined);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Ranged strikes without return damage and Ward prevents the first positive damage event', () => {
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
  let session = keep(createGameSession(wardManifest));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const shooterInstanceId = session.state.realm.units
    .find(({ controller }) => controller === 'north')?.instanceId;
  assert.ok(shooterInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const targetInstanceId = session.state.realm.units
    .find(({ controller }) => controller === 'south')?.instanceId;
  assert.ok(targetInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === shooterInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  let longRange = session;
  longRange = accept(longRange, action(longRange, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  longRange = accept(longRange, action(longRange, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  longRange = accept(longRange, action(longRange, ({ descriptor }) => descriptor.kind === 'end-turn'));
  longRange = accept(longRange, action(longRange, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const longRangeShots = legalGameActions(longRange.state, 'north')
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
  longRange = accept(longRange, longRangeShot);
  assert.equal(verifyGameReplay(longRange), true);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C2'));
  const secondTargetInstanceId = session.state.realm.units
    .find(({ controller, location }) => controller === 'south' && location === 'C2')?.instanceId;
  assert.ok(secondTargetInstanceId);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === targetInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const shots = legalGameActions(session.state, 'north')
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
  session = accept(session, shot);

  const shooter = session.state.realm.units.find(({ instanceId }) => instanceId === shooterInstanceId);
  const wardedTarget = session.state.realm.units.find(({ instanceId }) => instanceId === targetInstanceId);
  assert.deepEqual({ damage: shooter?.damage, location: shooter?.location, tapped: shooter?.tapped }, {
    damage: 0,
    location: 'C3',
    tapped: true,
  });
  assert.deepEqual({ damage: wardedTarget?.damage, warded: wardedTarget?.warded }, { damage: 0, warded: false });
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === secondTargetInstanceId), true);
  assert.deepEqual(session.transcript.at(-1)?.events.map(({ type }) => type), [
    'projectile-shot',
    'strike-damage-allocated',
    'damage-dealt',
    'ward-broken',
  ]);
  assert.equal(observeGame(session.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.warded, false);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === shooterInstanceId
      && descriptor.hit?.instanceId === targetInstanceId));
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) => instanceId === targetInstanceId), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 a drag projectile stops at the first visible unit and may fight after arrival', () => {
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
  const setup = (seed: number): Readonly<{
    blockerInstanceId: string;
    pudgeInstanceId: string;
    session: GameSession;
    targetInstanceId: string;
  }> => {
    const base = manifest(seed, { northSpell: pudge, southSpell: target });
    const preview = createGameSession(base);
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
    let session = keep(createGameSession(custom));
    session = keep(session);
    const pudgeCard = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === pudgeCardId);
    const blockerCard = session.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === blockerCardId);
    assert.ok(pudgeCard);
    assert.ok(blockerCard);
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === pudgeCard.instanceId
        && descriptor.cell === 'C4'));
    assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile'
        && descriptor.shooterInstanceId === pudgeCard.instanceId), false);
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === blockerCard.instanceId
        && descriptor.cell === 'C3'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
    const targetCard = session.state.players.south.hand.spellbook[0];
    assert.ok(targetCard);
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === targetCard.instanceId
        && descriptor.cell === 'C2'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    return {
      blockerInstanceId: blockerCard.instanceId,
      pudgeInstanceId: pudgeCard.instanceId,
      session,
      targetInstanceId: targetCard.instanceId,
    };
  };

  const checkpoint = setup(142);
  const choices = legalGameActions(checkpoint.session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile'
      && descriptor.shooterInstanceId === checkpoint.pudgeInstanceId
      && descriptor.direction === 'south'
      && descriptor.hit?.instanceId === checkpoint.targetInstanceId);
  assert.deepEqual(choices.map(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival), [false, true]);
  assert.equal(choices.every(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile'
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'), true);
  assert.equal(choices.some(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile'
      && descriptor.hit?.instanceId === checkpoint.blockerInstanceId), false);
  const noFight = accept(checkpoint.session, choices.find(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile' && !descriptor.fightOnArrival)!);
  const dragged = noFight.state.realm.units
    .find(({ instanceId }) => instanceId === checkpoint.targetInstanceId);
  assert.deepEqual({ location: dragged?.location, tapped: dragged?.tapped, warded: dragged?.warded }, {
    location: 'C4',
    tapped: false,
    warded: true,
  });
  assert.equal(noFight.state.realm.units
    .find(({ instanceId }) => instanceId === checkpoint.pudgeInstanceId)?.tapped, true);
  assert.deepEqual(noFight.transcript.at(-1)?.events.map(({ type }) => type), [
    'projectile-shot',
    'unit-dragged',
  ]);
  const draggedPayload = noFight.transcript.at(-1)?.events[1]?.payload;
  const draggedJson = canonicalJson(draggedPayload ?? null);
  assert.equal(draggedJson.includes('"steps":2'), true);
  assert.match(
    draggedJson,
    /"path":\[{"cell":"C2","region":"surface"},{"cell":"C3","region":"surface"},{"cell":"C4","region":"surface"}\]/,
  );
  assert.equal(verifyGameReplay(noFight), true);

  const fight = accept(checkpoint.session, choices.find(({ descriptor }) =>
    descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival)!);
  assert.deepEqual(fight.transcript.at(-1)?.events.map(({ type }) => type), [
    'projectile-shot',
    'unit-dragged',
    'fight-started',
    'strike-damage-allocated',
    'damage-dealt',
    'damage-dealt',
    'ward-broken',
  ]);
  const foughtPudge = fight.state.realm.units
    .find(({ instanceId }) => instanceId === checkpoint.pudgeInstanceId);
  const foughtTarget = fight.state.realm.units
    .find(({ instanceId }) => instanceId === checkpoint.targetInstanceId);
  assert.deepEqual({ damage: foughtPudge?.damage, location: foughtPudge?.location }, {
    damage: 3,
    location: 'C4',
  });
  assert.deepEqual({ damage: foughtTarget?.damage, location: foughtTarget?.location, warded: foughtTarget?.warded }, {
    damage: 0,
    location: 'C4',
    warded: false,
  });
  assert.equal(verifyGameReplay(fight), true);
});

test('RULE-03 Granary Rats suppresses its site threshold while enabled', () => {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const north: GameDeckSpec = {
    atlas: Array(3).fill('dual-site'),
    avatar: 'north-avatar',
    spellbook: ['rats', 'gated', 'filler'],
  };
  const south: GameDeckSpec = {
    atlas: Array(3).fill('dual-site'),
    avatar: 'south-avatar',
    spellbook: Array(3).fill('filler'),
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
      manaCost: 1,
      siteProvidesNoThreshold: true,
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
  let session = keep(keep(createGameSession(gameManifest)));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  const gated = session.state.players.north.hand.spellbook.find(({ cardId }) => cardId === 'gated');
  const rats = session.state.players.north.hand.spellbook.find(({ cardId }) => cardId === 'rats');
  assert.ok(gated);
  assert.ok(rats);
  const baseline = session;
  const canSummonGated = (checkpoint: GameSession) => legalGameActions(checkpoint.state, 'north')
    .some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'gated' && descriptor.cell === 'C4');
  assert.deepEqual(observeGame(baseline.state, 'north').players.north.affinity,
    { air: 0, earth: 1, fire: 1, water: 0 });
  assert.equal(canSummonGated(baseline), true);

  const unit = {
    ...rats,
    controller: 'north' as const,
    damage: 0,
    location: 'C4' as const,
    region: 'underground' as const,
    stealthed: false,
    summoningSickness: false,
    tapped: false,
    warded: false,
  };
  const withUnits = (units: GameSession['state']['realm']['units']): GameSession => ({
    ...baseline,
    state: {
      ...baseline.state,
      realm: { ...baseline.state.realm, units },
    },
  });
  const voidRat = withUnits([{ ...unit, region: 'void' }]);
  assert.equal(canSummonGated(voidRat), true);
  const enemyInstanceId =
    'sha256:2222222222222222222222222222222222222222222222222222222222222222' as const;
  const suppressed = withUnits([
    unit,
    { ...unit, controller: 'south', instanceId: enemyInstanceId, region: 'underwater' },
  ]);
  assert.deepEqual(observeGame(suppressed.state, 'north').players.north.affinity,
    { air: 0, earth: 0, fire: 0, water: 0 });
  assert.equal(canSummonGated(suppressed), false);

  const immutableSuppressed: GameSession = {
    ...suppressed,
    state: {
      ...suppressed.state,
      cards: {
        ...suppressed.state.cards,
        'dual-site': {
          ...suppressed.state.cards['dual-site']!,
          cannotBeMovedDestroyedOrModified: true,
        } as GameCardDefinition,
      },
    },
  };
  assert.deepEqual(observeGame(immutableSuppressed.state, 'north').players.north.affinity,
    { air: 0, earth: 1, fire: 1, water: 0 });
  assert.equal(canSummonGated(immutableSuppressed), true);

  const oneDisabled = withUnits(suppressed.state.realm.units.map((candidate) =>
    candidate.instanceId === unit.instanceId
      ? {
        ...candidate,
        disableEffects: [{ expiresAtSeat: 'north' as const, sourceInstanceId: candidate.instanceId }],
      }
      : candidate));
  assert.equal(canSummonGated(oneDisabled), false);
  const allDisabled = withUnits(oneDisabled.state.realm.units.map((candidate) => ({
    ...candidate,
    disableEffects: [{ expiresAtSeat: 'north' as const, sourceInstanceId: candidate.instanceId }],
  })));
  assert.deepEqual(observeGame(allDisabled.state, 'north').players.north.affinity,
    { air: 0, earth: 1, fire: 1, water: 0 });
  assert.equal(canSummonGated(allDisabled), true);
});

test('RULE-03 a provider adds affinity until that minion dies', () => {
  const setup = northAttacksAtC2(46, {
    attack: 1,
    defense: 1,
    manaCost: 1,
    provides: 'earth',
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  let { session } = setup;
  assert.equal(observeGame(session.state, 'north').players.north.affinity.earth, 3);
  assert.equal(observeGame(session.state, 'south').players.south.affinity.earth, 4);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(observeGame(session.state, 'north').players.north.affinity.earth, 2);
  assert.equal(observeGame(session.state, 'south').players.south.affinity.earth, 3);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Lethal kills a tougher minion with positive damage but not zero damage', () => {
  const resolve = (attack: number, seed: number): GameSession => {
    const setup = northAttacksAtC2(seed, {
      attack,
      defense: 5,
      lethal: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    });
    let { session } = setup;
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId));
    return accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  };

  const positive = resolve(1, 45);
  assert.equal(positive.state.players.north.cemetery.length, 1);
  assert.equal(positive.state.players.south.cemetery.length, 1);
  assert.equal(verifyGameReplay(positive), true);

  const zero = resolve(0, 44);
  assert.equal(zero.state.players.north.cemetery.length, 0);
  assert.equal(zero.state.players.south.cemetery.length, 0);
  assert.equal(verifyGameReplay(zero), true);
});

test('RULE-04 a prohibited minion cannot move to Defend but can still Intercept', () => {
  const spell = {
    attack: 2,
    cannotDefend: true,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const defend = northAttacksAtC2(50, spell);
  let session = accept(defend.session, action(defend.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === defend.targetInstanceId));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === defend.defenderInstanceId), false);

  const stationary = northAttacksAtC2(51, spell);
  const site = stationary.session.state.realm.sites.C2;
  assert.ok(site);
  session = accept(stationary.session, action(stationary.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === site.instanceId));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === stationary.targetInstanceId), true);

  const intercept = northAttacksAtC2(52, spell);
  session = accept(intercept.session, action(intercept.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'intercept' && descriptor.unitInstanceId === intercept.targetInstanceId), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Immobile units attack and Defend in place but cannot move themselves', () => {
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

  const movingDefend = northAttacksAtC2(145, vanilla, undefined, false, immobile);
  let session = accept(movingDefend.session, action(movingDefend.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === movingDefend.targetInstanceId));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === movingDefend.defenderInstanceId), false);
  assert.equal(verifyGameReplay(session), true);

  const stationary = northAttacksAtC2(146, vanilla, undefined, false, immobile);
  const site = stationary.session.state.realm.sites.C2;
  assert.ok(site);
  session = accept(stationary.session, action(stationary.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === site.instanceId));
  const stationaryDefend = action(session, ({ descriptor }) => descriptor.kind === 'defend'
    && descriptor.unitInstanceId === stationary.targetInstanceId
    && descriptor.path.length === 1);
  session = accept(session, stationaryDefend);
  assert.equal(verifyGameReplay(session), true);

  const local = northAttacksAtC2(147, vanilla, undefined, false, immobile);
  session = accept(local.session, action(local.session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'close-intercept'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const moves = legalGameActions(session.state, 'south').filter(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === local.targetInstanceId);
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
  const ignored = stepGame(session, forged);
  assert.equal(ignored.accepted, true);
  session = ignored.session;
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === local.targetInstanceId)?.location, 'C2');
  assert.doesNotMatch(canonicalJson(ignored.receipt.events), /"C3"/);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === local.attackerInstanceId));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 a fully prohibited minion cannot use Defend or Intercept', () => {
  const spell = {
    attack: 4,
    cannotDefendOrIntercept: true,
    defense: 4,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const defend = northAttacksAtC2(112, spell);
  let session = accept(defend.session, action(defend.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === defend.targetInstanceId));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === defend.defenderInstanceId), false);
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates), true);

  const intercept = northAttacksAtC2(113, spell);
  session = accept(intercept.session, action(intercept.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(session.state.phase, 'main');
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'intercept'), false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Airborne moves diagonally and restricts attacks and Intercept', () => {
  const ground = {
    attack: 3,
    defense: 3,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const airborne = { ...ground, airborne: true };
  const ranged = { ...ground, ranged: true };
  const canTarget = (
    session: GameSession,
    targetInstanceId: string,
  ): boolean => legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === targetInstanceId);
  const addDiagonalSite = (initial: GameSession): GameSession => {
    let session = initial;
    if (session.state.phase === 'intercept') {
      session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'close-intercept'));
    }
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    return accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B3'));
  };

  const rangedAttacksAirborne = northAttacksAtC2(118, ranged, undefined, false, airborne);
  assert.equal(canTarget(rangedAttacksAirborne.session, rangedAttacksAirborne.targetInstanceId), false);

  const airborneAttacksAirborne = northAttacksAtC2(119, airborne, undefined, false, airborne);
  assert.equal(canTarget(airborneAttacksAirborne.session, airborneAttacksAirborne.targetInstanceId), true);
  let session = accept(airborneAttacksAirborne.session, action(airborneAttacksAirborne.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(session.state.phase, 'intercept');
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'intercept'
      && descriptor.unitInstanceId === airborneAttacksAirborne.targetInstanceId), true);
  assert.equal(verifyGameReplay(session), true);

  const groundIntercept = northAttacksAtC2(120, airborne, undefined, false, ground);
  session = accept(groundIntercept.session, action(groundIntercept.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(session.state.phase, 'main');
  session = addDiagonalSite(session);
  const diagonal = action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === groundIntercept.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,B3');
  session = accept(session, diagonal);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === groundIntercept.attackerInstanceId)?.location, 'B3');
  assert.equal(verifyGameReplay(session), true);

  const rangedIntercept = northAttacksAtC2(121, airborne, undefined, false, ranged);
  session = accept(rangedIntercept.session, action(rangedIntercept.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(session.state.phase, 'intercept');
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'intercept' && descriptor.unitInstanceId === rangedIntercept.targetInstanceId), true);
  assert.equal(verifyGameReplay(session), true);

  const groundMovement = northAttacksAtC2(122, ground, undefined, false, ground);
  session = accept(groundMovement.session, action(groundMovement.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  session = addDiagonalSite(session);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === groundMovement.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,B3'), false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Updraft Ridge gives only Airborne minions a free departure to move or Defend', () => {
  const base = manifest(162, {
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  });
  const preview = createGameSession(base);
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

  let session = keep(keep(createGameSession(gameManifest)));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cardId === updraftId && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === airborneId));
  const airborneInstanceId = session.state.realm.units.at(-1)?.instanceId;
  assert.ok(airborneInstanceId);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === groundId));
  const groundInstanceId = session.state.realm.units.at(-1)?.instanceId;
  assert.ok(groundInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const attackerInstanceId = session.state.realm.units.at(-1)?.instanceId;
  assert.ok(attackerInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  const hasPath = (unitInstanceId: string): boolean => legalGameActions(session.state, 'north')
    .some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
  assert.equal(hasPath(airborneInstanceId), true);
  assert.equal(hasPath(groundInstanceId), false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === groundInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'), false);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === airborneInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'));
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === airborneInstanceId)?.location, 'C2');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Mountain Pass blocks only occupied ground-minion entry', () => {
  const base = manifest(161, {
    site: { blocksGroundMinionEntryWhileMinionAtop: true },
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 0,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  });
  const preview = createGameSession(base);
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

  let session = keep(keep(createGameSession(gameManifest)));
  const northGround = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === northGroundCardId);
  const northAirborne = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === northAirborneCardId);
  assert.ok(northGround);
  assert.ok(northAirborne);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === northGround.instanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === northAirborne.instanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const southGround = session.state.players.south.hand.spellbook
    .find(({ cardId }) => cardId === southGroundCardId);
  assert.ok(southGround);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === southGround.instanceId));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northGround.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northAirborne.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const southAirborne = session.state.players.south.hand.spellbook
    .find(({ cardId }) => cardId === southAirborneCardId);
  assert.ok(southAirborne);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === southAirborne.instanceId
      && descriptor.cell === 'C2'));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C2'), true);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const moves = legalGameActions(session.state, 'north');
  const entersC2 = (instanceId: string): boolean => moves.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2');
  assert.equal(entersC2(northGround.instanceId), false);
  assert.equal(entersC2(northAirborne.instanceId), true);
  assert.equal(moves.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === session.state.players.north.avatar.card.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'), true);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northAirborne.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === southAirborne.instanceId));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === southGround.instanceId), false);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Stealth blocks attacks, Defend, Intercept, and projectiles until interaction', () => {
  const ground = {
    attack: 1,
    defense: 5,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const stealth = { ...ground, attack: 3, stealth: true };
  const setup = northAttacksAtC2(123, stealth, undefined, false, ground);
  let session = accept(setup.session, action(setup.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === setup.attackerInstanceId)?.stealthed, true);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.targetInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2'));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.attackerInstanceId), false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId));
  assert.equal(session.state.phase, 'main');
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'defend-window-closed'), false);
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === setup.attackerInstanceId)?.stealthed, false);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.targetInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2'));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.attackerInstanceId), true);
  assert.equal(verifyGameReplay(session), true);

  const ranged = northAttacksAtC2(
    124,
    { ...ground, ranged: true, stealth: true },
    undefined,
    false,
    stealth,
  );
  session = ranged.session;
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === ranged.targetInstanceId), false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const shots = legalGameActions(session.state, 'north')
    .filter(({ descriptor }) => descriptor.kind === 'shoot-projectile');
  assert.equal(shots.some(({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.hit?.instanceId === ranged.targetInstanceId), false);
  const miss = shots.find(({ descriptor }) =>
    descriptor.kind === 'shoot-projectile' && descriptor.direction === 'north');
  assert.ok(miss);
  session = accept(session, miss);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === ranged.attackerInstanceId)?.stealthed, false);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === ranged.targetInstanceId)?.stealthed, true);
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Sly Fox gains Stealth once at the end of its controller turn', () => {
  let session = keep(createGameSession(manifest(126, {
    spell: {
      attack: 1,
      defense: 1,
      gainsStealthAtEndOfTurn: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  assert.equal(session.state.realm.units[0]?.stealthed, false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.state.realm.units[0]?.stealthed, true);
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['stealth-gained', 'turn-ended', 'turn-started'],
  );
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-gained'), false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Scent Hounds permanently removes nearby enemy Stealth', () => {
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

  let session = keep(keep(createGameSession(gameManifest)));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const houndInstanceId = session.state.realm.units[0]!.instanceId;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const targetInstanceId = session.state.realm.units
    .find(({ controller }) => controller === 'south')!.instanceId;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === houndInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  const checkpoint = session;
  const moved = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === targetInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  assert.equal(moved.accepted, true);
  if (!moved.accepted) return;
  session = moved.session;
  assert.deepEqual(moved.receipt.events.map(({ type }) => type), [
    'move-and-attack-activated',
    'stealth-lost',
  ]);
  assert.deepEqual(moved.receipt.events[1]?.payload, {
    instanceId: targetInstanceId,
    seat: 'south',
    sourceInstanceId: houndInstanceId,
  });
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));

  const regained = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(regained.accepted, true);
  if (!regained.accepted) return;
  session = regained.session;
  assert.deepEqual(regained.receipt.events.map(({ type }) => type), [
    'stealth-gained',
    'stealth-lost',
    'turn-ended',
    'turn-started',
  ]);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === houndInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
  assert.equal(verifyGameReplay(session), true);

  const disabledCheckpoint: GameSession = {
    ...checkpoint,
    state: {
      ...checkpoint.state,
      realm: {
        ...checkpoint.state.realm,
        units: checkpoint.state.realm.units.map((unit) => unit.instanceId === houndInstanceId
          ? {
            ...unit,
            disableEffects: [{
              expiresAtSeat: 'south' as const,
              sourceInstanceId: unit.instanceId,
            }],
          }
          : unit),
      },
    },
  };
  const disabledMove = stepGame(disabledCheckpoint, action(disabledCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === targetInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  assert.equal(disabledMove.accepted, true);
  if (!disabledMove.accepted) return;
  assert.equal(disabledMove.receipt.events.some(({ type }) => type === 'stealth-lost'), false);
  assert.equal(disabledMove.session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, true);
});

test('RULE-04 Malakhim untaps at its controller End Phase unless Disabled', () => {
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

  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const malakhim = session.state.realm.units[0];
  assert.ok(malakhim);
  const readyEnd = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(readyEnd.accepted, true);
  if (!readyEnd.accepted) return;
  assert.equal(readyEnd.receipt.events.some(({ type }) => type === 'minion-untapped'), false);
  assert.deepEqual(readyEnd.receipt.randomDraws, []);
  session = readyEnd.session;

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const attackerInstanceId = session.state.realm.units
    .find(({ controller }) => controller === 'south')?.instanceId;
  assert.ok(attackerInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === malakhim.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  const tappedMalakhim = session.state.realm.units
    .find(({ instanceId }) => instanceId === malakhim.instanceId);
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

  const damagedCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        units: session.state.realm.units.map((unit) => unit.instanceId === malakhim.instanceId
          ? { ...unit, damage: 2 }
          : unit),
      },
    },
  };
  const damagedEnd = stepGame(damagedCheckpoint, action(damagedCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(damagedEnd.accepted, true);
  if (!damagedEnd.accepted) return;
  assert.deepEqual(damagedEnd.receipt.events.map(({ type }) => type), [
    'minion-untapped',
    'turn-ended',
    'turn-started',
  ]);
  assert.deepEqual(damagedEnd.session.state.realm.units
    .filter(({ instanceId }) => instanceId === malakhim.instanceId)
    .map(({ damage, tapped, warded }) => ({ damage, tapped, warded })), [{
    damage: 0,
    tapped: false,
    warded: true,
  }]);

  const disabledCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        units: session.state.realm.units.map((unit) => unit.instanceId === malakhim.instanceId
          ? {
            ...unit,
            damage: 2,
            disableEffects: [{
              expiresAtSeat: 'south' as const,
              sourceInstanceId: unit.instanceId,
            }],
          }
          : unit),
      },
    },
  };
  const disabledEnd = stepGame(disabledCheckpoint, action(disabledCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(disabledEnd.accepted, true);
  if (!disabledEnd.accepted) return;
  assert.equal(disabledEnd.receipt.events.some(({ type }) => type === 'minion-untapped'), false);
  assert.deepEqual(disabledEnd.session.state.realm.units
    .filter(({ instanceId }) => instanceId === malakhim.instanceId)
    .map(({ damage, disableEffects, tapped, warded }) => ({
      damage,
      disableEffects,
      tapped,
      warded,
    })), [{
    damage: 0,
    disableEffects: undefined,
    tapped: true,
    warded: true,
  }]);

  const ended = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(ended.accepted, true);
  if (!ended.accepted) return;
  assert.deepEqual(ended.receipt.events.map(({ type }) => type), [
    'minion-untapped',
    'turn-ended',
    'turn-started',
  ]);
  assert.deepEqual(ended.receipt.events[0]?.payload, {
    instanceId: malakhim.instanceId,
    seat: 'north',
    sourceInstanceId: malakhim.instanceId,
  });
  assert.equal(ended.receipt.events[0]?.type, 'minion-untapped');
  assert.deepEqual(ended.receipt.randomDraws, []);
  assert.deepEqual(ended.session.state.realm.units
    .filter(({ instanceId }) => instanceId === malakhim.instanceId)
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
  assert.equal(observeGame(ended.session.state, 'south').realm.units
    .find(({ instanceId }) => instanceId === malakhim.instanceId)?.warded, true);
  session = ended.session;

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2,C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === malakhim.instanceId));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Ignited dies before turn cleanup unless it is Disabled', () => {
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

  let checkpoint = keep(createGameSession(gameManifest));
  checkpoint = keep(checkpoint);
  checkpoint = accept(checkpoint, action(checkpoint, ({ descriptor }) => descriptor.kind === 'play-site'));
  checkpoint = accept(checkpoint, action(checkpoint, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  const ignited = checkpoint.state.realm.units[0];
  assert.ok(ignited);

  const ended = stepGame(checkpoint, action(checkpoint, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(ended.accepted, true);
  if (!ended.accepted) return;
  assert.deepEqual(ended.receipt.events.map(({ type }) => type), [
    'minion-died',
    'turn-ended',
    'turn-started',
  ]);
  assert.equal(ended.session.state.realm.units.some(({ instanceId }) =>
    instanceId === ignited.instanceId), false);
  assert.equal(ended.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === ignited.instanceId), true);
  assert.deepEqual({
    activeSeat: ended.session.state.activeSeat,
    mana: ended.session.state.players.south.mana,
    phase: ended.session.state.phase,
    stateVersion: ended.session.state.stateVersion,
  }, {
    activeSeat: 'south',
    mana: 0,
    phase: 'draw',
    stateVersion: checkpoint.state.stateVersion + 1,
  });
  assert.equal(verifyGameReplay(ended.session), true);

  const disabledCheckpoint: GameSession = {
    ...checkpoint,
    state: {
      ...checkpoint.state,
      realm: {
        ...checkpoint.state.realm,
        units: [{
          ...ignited,
          damage: 2,
          disableEffects: [{ expiresAtSeat: 'south', sourceInstanceId: ignited.instanceId }],
        }],
      },
    },
  };
  const disabledEnd = stepGame(disabledCheckpoint, action(disabledCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(disabledEnd.accepted, true);
  if (!disabledEnd.accepted) return;
  assert.deepEqual(disabledEnd.receipt.events.map(({ type }) => type), [
    'turn-ended',
    'minion-disable-expired',
    'turn-started',
  ]);
  assert.deepEqual(disabledEnd.session.state.realm.units.map((unit) => ({
    damage: unit.damage,
    disableEffects: unit.disableEffects,
    instanceId: unit.instanceId,
  })), [{ damage: 0, disableEffects: undefined, instanceId: ignited.instanceId }]);
});

test('RULE-04 Sedge Crabs can move themselves only sideways', () => {
  const crab = {
    attack: 3,
    defense: 3,
    manaCost: 1,
    movementBonus: 1 as const,
    movesOnlySideways: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  const defendSetup = northAttacksAtC2(128, undefined, undefined, false, crab);
  const defendSession = accept(defendSetup.session, action(defendSetup.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
  assert.equal(legalGameActions(defendSession.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === defendSetup.defenderInstanceId), false);
  assert.equal(verifyGameReplay(defendSession), true);

  let session = keep(createGameSession(manifest(127, {
    spell: crab,
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'B3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C3'));
  const crabInstanceId = session.state.realm.units[0]?.instanceId;
  assert.ok(crabInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const moves = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
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
  const sideways = moves.find(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,B3');
  assert.ok(sideways);
  session = accept(session, sideways);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(session.state.realm.units[0]?.location, 'B3');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Dalcean Phalanx can move itself only forward for its seat', () => {
  const phalanx = {
    attack: 5,
    connectsTopBottom: true,
    defense: 5,
    manaCost: 1,
    movementBonus: 1 as const,
    movesOnlyForward: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  const defendSetup = northAttacksAtC2(141, undefined, undefined, false, phalanx);
  const defendSession = accept(defendSetup.session, action(defendSetup.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
  assert.equal(legalGameActions(defendSession.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === defendSetup.defenderInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'), true);
  assert.equal(verifyGameReplay(defendSession), true);

  let session = keep(createGameSession(manifest(142, { spell: phalanx })));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C3');
  const instanceId = session.state.realm.units[0]?.instanceId;
  assert.ok(instanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const paths = legalGameActions(session.state, 'north').flatMap(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === instanceId
      ? [descriptor.path.map(({ cell }) => cell).join(',')]
      : []);
  assert.equal(paths.includes('C3'), true);
  assert.equal(paths.includes('C3,C2'), true);
  assert.equal(paths.includes('C3,C2,C1'), true);
  assert.equal(paths.includes('C3,C4'), false);
  assert.equal(paths.includes('C3,B3'), false);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === instanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2,C1');
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), true);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const edgePaths = legalGameActions(session.state, 'north').flatMap(({ descriptor }) =>
    descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === instanceId
      ? [descriptor.path.map(({ cell }) => cell).join(',')]
      : []);
  assert.equal(edgePaths.includes('C1,C4'), true);
  assert.equal(edgePaths.includes('C1,C2'), false);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === instanceId
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C4');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(session.state.realm.units[0]?.location, 'C4');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-02/04 a unit can move across connected top and bottom realm edges', () => {
  let session = keep(createGameSession(manifest(128, {
    spell: {
      connectsTopBottom: true,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'summon-minion'));
  const unit = session.state.realm.units[0];
  assert.ok(unit);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const actions = legalGameActions(session.state, 'north');
  const wraps = ({ descriptor }: GameLegalAction): boolean => descriptor.kind === 'move-and-attack'
    && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C1';
  assert.equal(actions.some((candidate) =>
    wraps(candidate) && candidate.descriptor.kind === 'move-and-attack'
      && candidate.descriptor.unitInstanceId === unit.instanceId), true);
  assert.equal(actions.some((candidate) =>
    wraps(candidate) && candidate.descriptor.kind === 'move-and-attack'
      && candidate.descriptor.unitInstanceId === session.state.players.north.avatar.card.instanceId), false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unit.instanceId
    && descriptor.to.cell === 'C1'));
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), true);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Submerge uses underwater summons, movement, and region-isolated combat', () => {
  const submerge = {
    attack: 2,
    defense: 2,
    manaCost: 1,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
  } as const;
  const ordinary = { ...submerge, submerge: false };
  let session = keep(createGameSession(manifest(129, {
    northSpell: submerge,
    site: { elements: ['water'] },
    southSpell: ordinary,
  })));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site');
  const northSummons = legalGameActions(session.state, 'north');
  assert.equal(northSummons.some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === undefined), true);
  assert.equal(northSummons.some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), true);
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underwater');
  const northUnitId = session.state.realm.units[0]?.instanceId;
  assert.ok(northUnitId);
  assert.equal(observeGame(session.state, 'north').realm.units[0]?.region, 'underwater');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), false);
  take(({ descriptor }) => descriptor.kind === 'summon-minion');
  const southUnitId = session.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
  assert.ok(southUnitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  const underwaterMoves = legalGameActions(session.state, 'north');
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
  let surfaced = accept(session, surfaces);
  surfaced = accept(surfaced, action(surfaced, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(surfaced.state.realm.units.find(({ instanceId }) => instanceId === northUnitId)?.region, 'surface');
  assert.equal(verifyGameReplay(surfaced), true);

  session = accept(session, swims);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), false);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === southUnitId
      && descriptor.to.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C3/underwater,C2/underwater');
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === southUnitId), false);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(session.state.phase, 'main');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C2/underwater,C2/surface');
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === southUnitId), true);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'close-intercept');
  assert.equal(verifyGameReplay(session), true);

  let land = keep(createGameSession(manifest(130, { northSpell: submerge })));
  land = keep(land);
  land = accept(land, action(land, ({ descriptor }) => descriptor.kind === 'play-site'));
  assert.equal(legalGameActions(land.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), false);
  assert.equal(verifyGameReplay(land), true);
});

test('RULE-04 Burrowing uses underground summons and movement only at land sites', () => {
  const burrowing = {
    attack: 2,
    burrowing: true,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const ordinary = { ...burrowing, burrowing: false };
  let session = keep(createGameSession(manifest(131, { northSpell: burrowing, southSpell: ordinary })));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site');
  const northSummons = legalGameActions(session.state, 'north');
  assert.equal(northSummons.some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === undefined), true);
  assert.equal(northSummons.some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), true);
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underground');
  const northUnitId = session.state.realm.units[0]?.instanceId;
  assert.ok(northUnitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  const moves = legalGameActions(session.state, 'north');
  assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === northUnitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C4/underground,C3/underground'), true);
  assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === northUnitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C4/underground,C4/surface'), true);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === northUnitId
    && descriptor.to.cell === 'C3'
    && descriptor.to.region === 'underground');
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), false);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(verifyGameReplay(session), true);

  let water = keep(createGameSession(manifest(132, {
    northSpell: burrowing,
    site: { elements: ['water'] },
  })));
  water = keep(water);
  water = accept(water, action(water, ({ descriptor }) => descriptor.kind === 'play-site'));
  assert.equal(legalGameActions(water.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), false);
  assert.equal(verifyGameReplay(water), true);
});

test('RULE-04 Secret Tunnel connects burrowed allies only to the controller\'s other sites', () => {
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
  let session = keep(createGameSession(tunnelManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  const tunnel = session.state.players.north.hand.atlas.find(({ cardId }) => cardId === tunnelCardId);
  const land = session.state.players.north.hand.atlas.find(({ cardId }) =>
    cardId !== tunnelCardId && cardId !== waterCardId);
  const water = session.state.players.north.hand.atlas.find(({ cardId }) => cardId === waterCardId);
  assert.ok(tunnel);
  assert.ok(land);
  assert.ok(water);

  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === tunnel.instanceId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C4' && descriptor.region === 'underground');
  const unitId = session.state.realm.units[0]?.instanceId;
  assert.ok(unitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === land.instanceId && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === water.instanceId && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const pathFor = (candidate: GameLegalAction): string => candidate.descriptor.kind === 'move-and-attack'
    ? candidate.descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
    : '';
  const moves = legalGameActions(session.state, 'north');
  const unitPaths = moves
    .filter(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId)
    .map(pathFor);
  assert.equal(unitPaths.filter((path) => path === 'C4/underground,C3/underground').length, 1);
  assert.equal(unitPaths.includes('C4/underground,C2/underwater'), true);
  assert.equal(unitPaths.includes('C4/underground,C1/underground'), false);
  assert.equal(unitPaths.includes('C4/underground,C2/underwater,C4/underground'), false);
  const avatarId = session.state.players.north.avatar.card.instanceId;
  const avatarPaths = moves
    .filter(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === avatarId)
    .map(pathFor);
  assert.equal(avatarPaths.includes('C4/surface,C3/surface'), true);
  assert.equal(avatarPaths.includes('C4/surface,C2/surface'), false);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C4/underground,C2/underwater');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.deepEqual(session.state.realm.units[0]?.location, 'C2');
  assert.deepEqual(session.state.realm.units[0]?.region, 'underwater');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 a must-be-burrowed cast restriction suppresses only non-underground casts', () => {
  let session = keep(createGameSession(manifest(139, {
    northSpell: {
      attack: 3,
      burrowing: true,
      defense: 3,
      manaCost: 1,
      mustBeCastBurrowed: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  })));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  const summons = legalGameActions(session.state, 'north');
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C4' && descriptor.region === undefined), false);
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C4' && descriptor.region === 'underground'), true);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C4' && descriptor.region === 'underground');
  const unitId = session.state.realm.units[0]?.instanceId;
  assert.ok(unitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C4/underground,C4/surface');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 a must-be-submerged cast restriction suppresses only non-underwater casts', () => {
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
  let session = keep(createGameSession(submergedManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  const water = session.state.players.north.hand.atlas.find(({ cardId }) => {
    const definition = session.state.cards[cardId];
    return definition?.cardType === 'site' && definition.elements.includes('water');
  });
  const land = session.state.players.north.hand.atlas.find(({ cardId }) => {
    const definition = session.state.cards[cardId];
    return definition?.cardType === 'site' && !definition.elements.includes('water');
  });
  assert.ok(water);
  assert.ok(land);
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === water.instanceId && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === land.instanceId && descriptor.cell === 'C3');
  const summons = legalGameActions(session.state, 'north');
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.region === undefined), false);
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C3' && descriptor.region === 'underground'), false);
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.region === 'void'), false);
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C4' && descriptor.region === 'underwater'), true);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C4' && descriptor.region === 'underwater');
  const unitId = session.state.realm.units[0]?.instanceId;
  assert.ok(unitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const moves = legalGameActions(session.state, 'north');
  assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.to.cell === 'C3' && descriptor.to.region === 'underground'), true);
  assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId && descriptor.to.region === 'void'), true);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C4/underwater,C4/surface');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 combined region abilities permit eligible cross-region adjacency steps', () => {
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
  let session = keep(createGameSession(crossManifest));
  session = keep(session);
  const north = session.state.players.north;
  const land = north.hand.atlas.find(({ cardId }) => {
    const definition = session.state.cards[cardId];
    return definition?.cardType === 'site' && !definition.elements.includes('water');
  });
  const water = north.hand.atlas.find(({ cardId }) => {
    const definition = session.state.cards[cardId];
    return definition?.cardType === 'site' && definition.elements.includes('water');
  });
  assert.ok(land);
  assert.ok(water);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cardInstanceId === land.instanceId);
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underground');
  const unitId = session.state.realm.units[0]?.instanceId;
  assert.ok(unitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardInstanceId === water.instanceId
    && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C4/underground,C3/underwater');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'C3/underwater,C4/underground,B4/void');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Voidwalk summons to any void and moves between adjacent void and surface locations', () => {
  const voidwalk = {
    attack: 2,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    voidwalk: true,
  } as const;
  let session = keep(createGameSession(manifest(134, {
    northSpell: voidwalk,
    site: { elements: ['air'] },
  })));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site');
  const summons = legalGameActions(session.state, 'north');
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'C4' && descriptor.region === undefined), true);
  assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'B4' && descriptor.region === 'void'), true);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'B4' && descriptor.region === 'void');
  const coveredUnitId = session.state.realm.units[0]?.instanceId;
  assert.ok(coveredUnitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.region === 'void'), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
  assert.equal(session.state.realm.units.find(({ instanceId }) => instanceId === coveredUnitId)?.region, 'surface');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'B3' && descriptor.region === 'void');
  const unitId = session.state.realm.units.find(({ region }) => region === 'void')?.instanceId;
  assert.ok(unitId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const moves = legalGameActions(session.state, 'north');
  assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'B3/void,A3/void'), true);
  assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'B3/void,B4/surface'), true);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.to.cell === 'B4' && descriptor.to.region === 'surface');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 an outer-column cast restriction filters surface and Voidwalk summons, not movement', () => {
  let session = keep(createGameSession(manifest(135, {
    northSpell: {
      attack: 3,
      defense: 3,
      manaCost: 3,
      mustBeCastToOuterColumn: true,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
    },
    site: { elements: ['air'] },
  })));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A4');

  const summons = legalGameActions(session.state, 'north')
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
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'E2' && descriptor.region === 'void');
  const unitId = session.state.realm.units.find(({ location }) => location === 'E2')?.instanceId;
  assert.ok(unitId);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'E2/void,D2/void');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Movement +1 issues exact two-step and returning Move and Attack paths', () => {
  const setup = northAttacksAtC2(53, {
    attack: 2,
    defense: 2,
    manaCost: 1,
    movementBonus: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  let session = accept(setup.session, action(setup.session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'close-intercept'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const actions = legalGameActions(session.state, 'north');
  const twoStep = actions.find(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4');
  assert.ok(twoStep);
  assert.equal(actions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2'), true);
  const result = stepGame(session, twoStep);
  assert.equal(result.accepted, true);
  session = result.session;
  assert.equal(session.state.pendingCombat?.cell, 'C4');
  assert.match(canonicalJson(result.receipt.events[0]?.payload ?? null), /\"steps\":2/);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Movement +1 can take an exact two-step path to Defend', () => {
  let session = keep(createGameSession(manifest(54, {
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      movementBonus: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const defenderInstanceId = session.state.realm.units[0]?.instanceId;
  assert.ok(defenderInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const attackerInstanceId = session.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
  assert.ok(attackerInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
  const defend = action(session, ({ descriptor }) =>
    descriptor.kind === 'defend'
      && descriptor.unitInstanceId === defenderInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
  session = accept(session, defend);
  assert.equal(session.state.realm.units.find(({ instanceId }) => instanceId === defenderInstanceId)?.location, 'C2');
  assert.match(canonicalJson(session.transcript.at(-1)?.events[0]?.payload ?? null), /\"steps\":2/);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Movement +2 issues exact three-step paths and attacks after moving', () => {
  const setup = northAttacksAtC2(125, {
    attack: 2,
    defense: 2,
    manaCost: 1,
    movementBonus: 2,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  let session = accept(setup.session, action(setup.session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'close-intercept'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const actions = legalGameActions(session.state, 'north');
  assert.equal(actions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2'), true);
  assert.equal(actions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2,C3'), false);
  assert.equal(actions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4,C3'), true);
  const threeStep = actions.find(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2,C1');
  assert.ok(threeStep);
  assert.equal(actions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.path.length > 4), false);
  const result = stepGame(session, threeStep);
  assert.equal(result.accepted, true);
  session = result.session;
  assert.match(canonicalJson(result.receipt.events[0]?.payload ?? null), /\"steps\":3/);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.defenderInstanceId), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-05 Deathrite draws sites before simultaneous deaths enter their cemeteries', () => {
  const spell = {
    attack: 1,
    deathriteDrawSite: true,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const setup = northAttacksAtC2(48, spell);
  const before = setup.session.state.players;
  let session = accept(setup.session, action(setup.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));

  for (const seat of ['north', 'south'] as const) {
    assert.equal(session.state.players[seat].atlas.length, before[seat].atlas.length - 1);
    assert.equal(session.state.players[seat].hand.atlas.length, before[seat].hand.atlas.length + 1);
  }
  const events = session.transcript.at(-1)?.events ?? [];
  const firstCemeteryEvent = events.findIndex(({ type }) => type === 'minion-died');
  assert.ok(firstCemeteryEvent > 0);
  assert.equal(events.slice(0, firstCemeteryEvent).filter(({ type }) => type === 'site-drawn').length, 2);
  assert.equal(session.state.players.north.cemetery.length, 1);
  assert.equal(session.state.players.south.cemetery.length, 1);
  assert.equal(verifyGameReplay(session), true);

  const empty = northAttacksAtC2(49, spell, undefined, true);
  let deckOut = accept(empty.session, action(empty.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === empty.targetInstanceId));
  deckOut = accept(deckOut, action(deckOut, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.deepEqual(deckOut.state.terminal, {
    reason: 'simultaneous_defeat',
    result: 'draw',
    status: 'finished',
  });
  assert.equal(verifyGameReplay(deckOut), true);
});

test('RULE-05 Bladderblimp Deathrite makes each player lose life for their nearby sites', () => {
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

  const setup = northAttacksAtC2(
    147,
    blimp,
    { attack: 1, defense: 1, drawSpell: false, life: 2 },
    false,
    ordinary,
  );
  let session = accept(setup.session, action(setup.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(session.state.players.north.avatar.life, 1);
  assert.equal(session.state.players.south.avatar.life, 0);
  assert.deepEqual(session.state.terminal, { status: 'active' });
  const events = session.transcript.at(-1)?.events ?? [];
  const lifeEvents = events.filter(({ type }) => type === 'avatar-life-lost');
  assert.deepEqual(lifeEvents.map(({ payload }) => payload), [
    {
      amount: 1,
      life: 1,
      seat: 'north',
      sourceInstanceId: setup.attackerInstanceId,
    },
    {
      amount: 2,
      life: 0,
      seat: 'south',
      sourceInstanceId: setup.attackerInstanceId,
    },
  ]);
  const deathsDoor = events.find(({ type }) => type === 'avatar-reached-deaths-door');
  assert.deepEqual(deathsDoor?.payload, {
    seat: 'south',
    sourceInstanceId: setup.attackerInstanceId,
    turnNumber: session.state.turnNumber,
  });
  const firstDeath = events.findIndex(({ type }) => type === 'minion-died');
  assert.ok(firstDeath > events.findIndex(({ type }) => type === 'avatar-reached-deaths-door'));
  assert.equal(events.some(({ type }) => type === 'game-ended'), false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-05 Deathrite damages each other remaining unit here in simultaneous chained batches', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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

  let session = keep(keep(createGameSession(createGameManifest({
    authority,
    cards,
    decks,
    firstSeat: 'north',
    seed,
  }))));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  for (const cardId of [...scarabCardIds, chainedCardId]) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === cardId
      && descriptor.cell === 'C1');
  }
  const southUnits = [...session.state.realm.units];
  const scarabInstanceIds = southUnits
    .filter(({ cardId }) => scarabCardIds.includes(cardId))
    .map(({ instanceId }) => instanceId)
    .sort();
  const chainedInstanceId = southUnits.find(({ cardId }) => cardId === chainedCardId)?.instanceId;
  assert.equal(scarabInstanceIds.length, 2);
  assert.ok(chainedInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const sourceInstanceId = session.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === sourceCardId)?.instanceId;
  assert.ok(sourceInstanceId);
  const result = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === sourceInstanceId
      && descriptor.cell === 'C1'));
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;

  assert.equal(session.state.players.north.avatar.life, 20);
  assert.equal(session.state.players.south.avatar.life, 16);
  assert.equal(session.state.realm.units.length, 1);
  assert.deepEqual(session.state.realm.units[0] && {
    damage: session.state.realm.units[0].damage,
    instanceId: session.state.realm.units[0].instanceId,
    location: session.state.realm.units[0].location,
  }, { damage: 3, instanceId: sourceInstanceId, location: 'C1' });
  assert.deepEqual(
    session.state.players.south.cemetery.map(({ instanceId }) => instanceId).sort(),
    [...scarabInstanceIds, chainedInstanceId].sort(),
  );

  const events = result.receipt.events;
  const allocations = events.filter(({ type }) => type === 'deathrite-damage-allocated');
  assert.equal(allocations.length, 7);
  const sourceOrder = allocations
    .map(({ payload }) => payload !== null && typeof payload === 'object'
      && 'sourceInstanceId' in payload ? payload.sourceInstanceId : undefined)
    .filter((source, index, sources) => source !== undefined && sources.indexOf(source) === index);
  assert.deepEqual(sourceOrder, [...scarabInstanceIds, chainedInstanceId]);
  const southAvatarId = session.state.players.south.avatar.card.instanceId;
  for (const [source, targets] of [
    [scarabInstanceIds[0]!, [sourceInstanceId, southAvatarId, chainedInstanceId].sort()],
    [scarabInstanceIds[1]!, [sourceInstanceId, southAvatarId].sort()],
    [chainedInstanceId, [sourceInstanceId, southAvatarId].sort()],
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
  const northAvatarId = session.state.players.north.avatar.card.instanceId;
  assert.equal(allocations.some(({ payload }) =>
    payload !== null
      && typeof payload === 'object'
      && 'targetInstanceId' in payload
      && payload.targetInstanceId === northAvatarId), false);
  assert.equal(events.some(({ type }) =>
    type === 'fight-started' || type === 'strike-damage-allocated'), false);
  assert.ok(events.findIndex(({ type }) => type === 'minion-died')
    > events.findLastIndex(({ type }) => type === 'deathrite-damage-allocated'));
  assert.equal(events.filter(({ type }) => type === 'minion-died').length, 3);
  assert.equal(result.receipt.randomDraws.length, 0);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-05 Deathrite uses its moved last location with Ward, reduction, and Lethal', () => {
  const deathrite = {
    attack: 0,
    deathriteDamageEachUnitHere: 2,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const resolve = (
    seed: number,
    source: SpellFacts,
    target: SpellFacts,
  ): ReturnType<typeof northAttacksAtC2> => {
    const setup = northAttacksAtC2(seed, source, undefined, false, target);
    let session = accept(setup.session, action(setup.session, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    return { ...setup, session };
  };

  const warded = resolve(266, deathrite, {
    attack: 2,
    defense: 10,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    ward: true,
  });
  const wardedEvents = warded.session.transcript.at(-1)?.events ?? [];
  const wardedAllocationIndex = wardedEvents.findIndex(({ type }) =>
    type === 'deathrite-damage-allocated');
  assert.deepEqual(wardedEvents[wardedAllocationIndex]?.payload, {
    amount: 2,
    sourceInstanceId: warded.attackerInstanceId,
    targetInstanceId: warded.targetInstanceId,
  });
  assert.deepEqual(wardedEvents.slice(wardedAllocationIndex + 1).find(({ payload, type }) =>
    type === 'damage-dealt'
      && canonicalJson(payload).includes(warded.targetInstanceId))?.payload, {
    amount: 0,
    attemptedAmount: 2,
    direct: true,
    instanceId: warded.targetInstanceId,
    prevented: true,
    seat: 'south',
  });
  assert.ok(wardedEvents.findIndex(({ type }) => type === 'ward-broken') > wardedAllocationIndex);
  const sourceMoves = warded.session.transcript.flatMap(({ events }) =>
    events.filter(({ payload, type }) =>
      type === 'move-and-attack-activated'
        && canonicalJson(payload).includes(warded.attackerInstanceId)));
  assert.equal(sourceMoves.length, 2);
  assert.equal(canonicalJson(sourceMoves.at(-1)!.payload).includes('"to":{"cell":"C2"'), true);
  const wardedTarget = warded.session.state.realm.units.find(({ instanceId }) =>
    instanceId === warded.targetInstanceId);
  const distantWarded = warded.session.state.realm.units.find(({ instanceId }) =>
    instanceId === warded.defenderInstanceId);
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
  assert.equal(verifyGameReplay(warded.session), true);

  const lethal = resolve(267, { ...deathrite, lethal: true }, {
    attack: 2,
    defense: 10,
    manaCost: 1,
    takesLessDamage: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  const lethalEvents = lethal.session.transcript.at(-1)?.events ?? [];
  const lethalAllocationIndex = lethalEvents.findIndex(({ type }) =>
    type === 'deathrite-damage-allocated');
  assert.deepEqual(lethalEvents[lethalAllocationIndex]?.payload, {
    amount: 2,
    sourceInstanceId: lethal.attackerInstanceId,
    targetInstanceId: lethal.targetInstanceId,
  });
  assert.deepEqual(lethalEvents.slice(lethalAllocationIndex + 1).find(({ payload, type }) =>
    type === 'damage-dealt'
      && canonicalJson(payload).includes(lethal.targetInstanceId))?.payload, {
    accumulated: 1,
    amount: 1,
    attemptedAmount: 2,
    direct: true,
    instanceId: lethal.targetInstanceId,
    prevented: true,
    seat: 'south',
  });
  assert.ok(lethalEvents.findIndex(({ payload, type }) =>
    type === 'minion-died' && canonicalJson(payload).includes(lethal.targetInstanceId))
      > lethalAllocationIndex);
  assert.equal(lethal.session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === lethal.targetInstanceId), true);
  const distantReduced = lethal.session.state.realm.units.find(({ instanceId }) =>
    instanceId === lethal.defenderInstanceId);
  assert.deepEqual(distantReduced && {
    damage: distantReduced.damage,
    location: distantReduced.location,
  }, { damage: 0, location: 'C1' });
  assert.equal(verifyGameReplay(lethal.session), true);
});

test('RULE-05 Deathrite healing caps at maximum, fails at Death\'s Door, and precedes cemetery entry', () => {
  const resolveHealingFight = (life: number, seed: number): GameSession => {
    const setup = northAttacksAtC2(seed, {
      attack: 1,
      deathriteHeal: 3,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    }, { attack: 1, defense: 1, drawSpell: false, life });
    let { session } = setup;
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates));
    assert.equal(session.state.players.south.avatar.life, life - 1);
    session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.defenderInstanceId
        && descriptor.to.cell === 'C2'));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.attackerInstanceId));
    return accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  };

  const capped = resolveHealingFight(5, 56);
  assert.equal(capped.state.players.south.avatar.life, 5);
  const cappedEvents = capped.transcript.at(-1)?.events ?? [];
  assert.equal(cappedEvents.filter(({ type }) => type === 'avatar-healed').some(({ payload }) =>
    canonicalJson(payload).includes('"amount":1')
      && canonicalJson(payload).includes('"seat":"south"')), true);
  assert.ok(Math.max(...cappedEvents.map(({ type }, index) => type === 'avatar-healed' ? index : -1))
    < cappedEvents.findIndex(({ type }) => type === 'minion-died'));
  assert.equal(verifyGameReplay(capped), true);

  const deathDoor = resolveHealingFight(1, 57);
  assert.equal(deathDoor.state.players.south.avatar.life, 0);
  assert.equal(deathDoor.transcript.at(-1)?.events.some(({ type }) =>
    type === 'avatar-healed'), false);
  assert.equal(verifyGameReplay(deathDoor), true);
});

test('RULE-04 Move and Attack stages movement before an undefended enemy-site strike', () => {
  const setup = northAttacksAtC2(47);
  const { attackerInstanceId } = setup;
  let { session } = setup;
  assert.equal(session.state.activeSeat, 'north');
  assert.equal(session.state.decisionSeat, 'north');
  assert.equal(session.state.phase, 'attack');
  const attacker = session.state.realm.units.find(({ instanceId }) => instanceId === attackerInstanceId);
  assert.deepEqual({
    location: attacker?.location,
    region: attacker?.region,
    tapped: attacker?.tapped,
  }, { location: 'C2', region: 'surface', tapped: true });

  const site = session.state.realm.sites.C2;
  assert.ok(site);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === site.instanceId));
  assert.equal(session.state.activeSeat, 'north');
  assert.equal(session.state.decisionSeat, 'south');
  assert.equal(session.state.phase, 'defend');

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates));
  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.decisionSeat, 'north');
  assert.equal(session.state.players.south.avatar.life, 19);
  assert.equal(session.state.players.south.avatar.deathDoorTurn, null);
  assert.equal(session.state.realm.units.length, 3);
  assert.deepEqual(
    session.transcript.at(-1)?.events.map(({ type }) => type),
    ['defend-window-closed', 'undefended-site-struck', 'avatar-life-lost'],
  );
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Defend moves a unit into a simultaneous fight and stages exact split damage', () => {
  const setup = northAttacksAtC2(53);
  const { attackerInstanceId, defenderInstanceId, targetInstanceId } = setup;
  let { session } = setup;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === defenderInstanceId));
  const movedDefender = session.state.realm.units.find(({ instanceId }) => instanceId === defenderInstanceId);
  assert.deepEqual({ location: movedDefender?.location, tapped: movedDefender?.tapped }, {
    location: 'C2',
    tapped: true,
  });

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(session.state.phase, 'allocate');
  assert.equal(session.state.activeSeat, 'north');
  assert.equal(session.state.decisionSeat, 'north');
  const firstAllocation = legalGameActions(session.state, 'north')
    .find(({ descriptor }) => descriptor.kind === 'allocate-strike' && descriptor.amount === 0);
  assert.ok(firstAllocation);
  session = accept(session, firstAllocation);
  const finalAllocation = action(session, ({ descriptor }) =>
    descriptor.kind === 'allocate-strike' && descriptor.amount === 1);
  const damagedInstanceId = finalAllocation.descriptor.kind === 'allocate-strike'
    ? finalAllocation.descriptor.targetInstanceId
    : '';
  session = accept(session, finalAllocation);

  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === attackerInstanceId), false);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === damagedInstanceId), false);
  assert.equal(session.state.realm.units.filter(({ controller }) => controller === 'south').length, 1);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) => instanceId === attackerInstanceId), true);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) => instanceId === damagedInstanceId), true);
  assert.equal(session.transcript.flatMap(({ events }) => events).filter(({ type }) => type === 'minion-died').length, 2);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 takes-less prevention applies to each simultaneous damage source before Lethal', () => {
  for (const scenario of [
    { attack: 2, defense: 2, enemyPower: 1, expectedDamage: 0, lethal: true, seed: 142 },
    { attack: 4, defense: 3, enemyPower: 2, expectedDamage: 2, lethal: false, seed: 143 },
  ] as const) {
    const setup = northAttacksAtC2(
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
    );
    let { session } = setup;
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === setup.defenderInstanceId));
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    const allocated = stepGame(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'allocate-strike' && descriptor.amount === scenario.enemyPower));
    assert.equal(allocated.accepted, true);
    const fought = stepGame(allocated.session, action(allocated.session, ({ descriptor }) =>
      descriptor.kind === 'allocate-strike' && descriptor.amount === scenario.enemyPower));
    assert.equal(fought.accepted, true);
    session = fought.session;

    assert.equal(session.state.realm.units.find(({ instanceId }) =>
      instanceId === setup.attackerInstanceId)?.damage, scenario.expectedDamage);
    assert.equal(session.state.realm.units.filter(({ controller }) => controller === 'south').length, 0);
    assert.deepEqual(fought.receipt.events.find(({ payload, type }) =>
      type === 'damage-dealt'
        && canonicalJson(payload).includes(setup.attackerInstanceId))?.payload, {
      accumulated: scenario.expectedDamage,
      amount: scenario.expectedDamage,
      attemptedAmount: scenario.enemyPower * 2,
      direct: true,
      instanceId: setup.attackerInstanceId,
      prevented: true,
      seat: 'north',
    });
    assert.equal(fought.receipt.randomDraws.length, 0);
    assert.equal(verifyGameReplay(session), true);
  }
});

test('RULE-04 declining an attack gives only co-located ready enemies an Intercept window', () => {
  const setup = northAttacksAtC2(59);
  const { attackerInstanceId, defenderInstanceId, targetInstanceId } = setup;
  let { session } = setup;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(session.state.phase, 'intercept');
  assert.equal(session.state.activeSeat, 'north');
  assert.equal(session.state.decisionSeat, 'south');
  const interceptors = legalGameActions(session.state, 'south')
    .filter(({ descriptor }) => descriptor.kind === 'intercept');
  assert.deepEqual(
    interceptors.map(({ descriptor }) => descriptor.kind === 'intercept' && descriptor.unitInstanceId),
    [targetInstanceId],
  );
  assert.equal(interceptors.some(({ descriptor }) =>
    descriptor.kind === 'intercept' && descriptor.unitInstanceId === defenderInstanceId), false);

  session = accept(session, interceptors[0]!);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'close-intercept'));
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === attackerInstanceId), false);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === targetInstanceId), false);
  const eventTypes = session.transcript.flatMap(({ events }) => events.map(({ type }) => type));
  assert.equal(eventTypes.includes('attack-declared'), false);
  assert.equal(eventTypes.includes('interceptor-joined'), true);
  assert.equal(eventTypes.includes('fight-started'), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 surviving minion damage persists through the turn and clears in End Phase', () => {
  const setup = northAttacksAtC2(61, {
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  const { targetInstanceId } = setup;
  let { session } = setup;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === targetInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.deepEqual(session.state.realm.units
    .filter(({ location }) => location === 'C2')
    .map(({ damage }) => damage), [1, 1]);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.state.realm.units.every(({ damage }) => damage === 0), true);
  assert.equal(verifyGameReplay(session), true);
});

function northAvatarAttacksSouthAtC2(seed: number): Readonly<{
  northAvatarInstanceId: string;
  northMinionInstanceId: string;
  session: GameSession;
  southAvatarInstanceId: string;
}> {
  let session = keep(createGameSession(manifest(seed, {
    avatar: { attack: 2, defense: 1, drawSpell: false, life: 1 },
  })));
  session = keep(session);
  const northAvatarInstanceId = session.state.players.north.avatar.card.instanceId;
  const southAvatarInstanceId = session.state.players.south.avatar.card.instanceId;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const northMinionInstanceId = session.state.realm.units[0]?.instanceId;
  assert.ok(northMinionInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northMinionInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northMinionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northAvatarInstanceId
      && descriptor.to.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === southAvatarInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northAvatarInstanceId
      && descriptor.to.cell === 'C2'));
  return { northAvatarInstanceId, northMinionInstanceId, session, southAvatarInstanceId };
}

test("RULE-04 Death's Door prevents same-turn direct damage and later simultaneous death blows draw", () => {
  const setup = northAvatarAttacksSouthAtC2(67);
  const { northAvatarInstanceId, northMinionInstanceId, southAvatarInstanceId } = setup;
  let { session } = setup;
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'avatar'
      && descriptor.target.instanceId === southAvatarInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(session.state.players.north.avatar.life, 0);
  assert.equal(session.state.players.south.avatar.life, 0);
  assert.equal(session.state.players.north.avatar.deathDoorTurn, 7);
  assert.equal(session.state.players.south.avatar.deathDoorTurn, 7);
  assert.deepEqual(session.state.terminal, { status: 'active' });
  const firstFightEvents = session.transcript.at(-1)?.events ?? [];
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

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northMinionInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'avatar'
      && descriptor.target.instanceId === southAvatarInstanceId));
  const immunityResult = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(immunityResult.accepted, true);
  session = immunityResult.session;
  assert.deepEqual(session.state.terminal, { status: 'active' });
  assert.equal(session.state.players.south.avatar.life, 0);
  assert.equal(immunityResult.receipt.events.some(({ payload, type }) =>
    type === 'damage-dealt'
      && typeof payload === 'object'
      && payload !== null
      && !Array.isArray(payload)
      && 'prevented' in payload
      && payload.prevented === true), true);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === southAvatarInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'avatar'
      && descriptor.target.instanceId === northAvatarInstanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.deepEqual(session.state.terminal, {
    reason: 'simultaneous_avatar_defeat',
    result: 'draw',
    status: 'finished',
  });
  assert.equal(session.transcript.at(-1)?.events.filter(({ type }) => type === 'death-blow').length, 2);
  assert.equal(verifyGameReplay(session), true);
});

test("RULE-04 later undefended site strikes cannot deliver Death's Door death blows", () => {
  const setup = northAttacksAtC2(
    71,
    undefined,
    { attack: 1, defense: 1, drawSpell: false, life: 1 },
  );
  let { session } = setup;
  const site = session.state.realm.sites.C2;
  assert.ok(site);
  const strikeSite = (): GameLegalAction => action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'site'
      && descriptor.target.instanceId === site.instanceId);
  session = accept(session, strikeSite());
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates));
  assert.equal(session.state.players.south.avatar.life, 0);
  assert.equal(session.state.players.south.avatar.deathDoorTurn, 5);
  assert.deepEqual(session.state.terminal, { status: 'active' });

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === setup.attackerInstanceId
      && descriptor.to.cell === 'C2'));
  session = accept(session, strikeSite());
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates));
  assert.equal(session.state.players.south.avatar.life, 0);
  assert.deepEqual(session.state.terminal, { status: 'active' });
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'death-blow'), false);
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'avatar-life-lost'), false);
  assert.equal(verifyGameReplay(session), true);
});

test('shared stale rejection leaves game state, PRNG, and accepted transcript unchanged', () => {
  const initial = createGameSession(manifest(17));
  const command = action(initial, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0);
  const accepted = stepGame(initial, command);
  assert.equal(accepted.accepted, true);
  const before = canonicalJson(accepted.session.state);
  const beforeHash = hashGameState(accepted.session.state);
  const stale = stepGame(accepted.session, command);

  assert.equal(stale.accepted, false);
  assert.equal(stale.reason.code, 'stale_version');
  assert.equal(stale.reason.currentStateHash, beforeHash);
  assert.equal(canonicalJson(stale.session.state), before);
  assert.equal(stale.session.transcript.length, 1);
  assert.equal(stale.session.attempts.length, 2);
});

test('RULE-01 attempting to draw from an empty deck immediately loses', () => {
  const short = deck('short', 3, 3);
  let session = keep(createGameSession(manifest(19, { north: short, south: short })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));

  assert.deepEqual(session.state.terminal, {
    loser: 'south',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'north',
  });
  assert.equal(session.state.phase, 'terminal');
  assert.deepEqual(legalGameActions(session.state, 'south'), []);
  assert.equal(session.transcript.at(-1)?.events[0]?.type, 'game-ended');
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 Pick Up and Drop manage local carried Artifacts once per unit turn', () => {
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
  let gameManifest: GameManifest | undefined;
  for (let seed = 83; seed < 256; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const opening = createGameSession(candidate).state.players.north.hand.spellbook;
    if (opening.filter(({ cardId }) => cardId === 'sword-and-shield').length >= 2
      && opening.some(({ cardId }) => cardId === 'artifact-bearer')) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
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

  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): GameSession => {
    session = accept(session, action(session, predicate));
    return session;
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'artifact-bearer' && descriptor.cell === 'C4');
  const bearer = session.state.realm.units.find(({ cardId }) => cardId === 'artifact-bearer')!;
  assert.equal(bearer.summoningSickness, true);
  assert.equal(session.state.players.north.avatar.tapped, true);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'sword-and-shield'
    && descriptor.casterInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.cell === 'C4'
    && descriptor.bearer === undefined);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'sword-and-shield'
    && descriptor.casterInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.cell === 'C4'
    && descriptor.bearer === undefined);

  const surfaceArtifacts = (session.state.realm.artifacts ?? [])
    .flatMap((artifact) => 'bearer' in artifact ? [] : [artifact]);
  assert.equal(surfaceArtifacts.length, 2);
  const artifactInstanceIds = surfaceArtifacts.map(({ instanceId }) => instanceId).sort();
  const pickupDescriptors = (checkpoint: GameSession) =>
    legalGameActions(checkpoint.state, checkpoint.state.decisionSeat)
      .flatMap(({ descriptor }) => descriptor.kind === 'pick-up-artifacts' ? [descriptor] : []);
  const dropDescriptors = (checkpoint: GameSession) =>
    legalGameActions(checkpoint.state, checkpoint.state.decisionSeat)
      .flatMap(({ descriptor }) => descriptor.kind === 'drop-artifacts' ? [descriptor] : []);
  const expectedSubsets = [
    artifactInstanceIds[0],
    artifactInstanceIds[1],
    artifactInstanceIds.join(','),
  ].sort();
  const initialPickups = pickupDescriptors(session);
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

  const remoteId = 'sha256:2222222222222222222222222222222222222222222222222222222222222222' as const;
  const underwaterId = 'sha256:3333333333333333333333333333333333333333333333333333333333333333' as const;
  const carriedId = 'sha256:4444444444444444444444444444444444444444444444444444444444444444' as const;
  const firstArtifact = surfaceArtifacts.find(({ instanceId }) =>
    instanceId === artifactInstanceIds[0])!;
  const filteredCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        artifacts: [
          ...surfaceArtifacts.map((artifact) => artifact.instanceId === artifactInstanceIds[0]
            ? { ...artifact, owner: 'south' as const }
            : artifact),
          { ...firstArtifact, instanceId: remoteId, location: 'C3' as const },
          { ...firstArtifact, instanceId: underwaterId, region: 'underwater' as const },
          {
            bearer: {
              instanceId: session.state.players.north.avatar.card.instanceId,
              kind: 'avatar' as const,
              seat: 'north' as const,
            },
            cardId: firstArtifact.cardId,
            instanceId: carriedId,
            owner: firstArtifact.owner,
            source: firstArtifact.source,
          },
        ],
        units: session.state.realm.units.map((unit) => unit.instanceId === bearer.instanceId
          ? { ...unit, tapped: true }
          : unit),
      },
    },
  };
  const filteredPickups = pickupDescriptors(filteredCheckpoint);
  assert.equal(filteredPickups.length, 6);
  assert.equal(filteredPickups.every(({ artifactInstanceIds: ids }) =>
    ids.every((instanceId) => artifactInstanceIds.includes(instanceId))), true);
  const enemyOwnedPick = stepGame(filteredCheckpoint, action(filteredCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'pick-up-artifacts'
      && descriptor.unit.kind === 'minion'
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === artifactInstanceIds[0]));
  assert.equal(enemyOwnedPick.accepted, true);
  if (!enemyOwnedPick.accepted) throw new Error('expected enemy-owned Artifact Pick Up to be accepted');
  assert.equal(enemyOwnedPick.session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === artifactInstanceIds[0])?.owner, 'south');
  assert.equal(enemyOwnedPick.session.state.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId)?.tapped, true);

  const readyAvatar = { ...filteredCheckpoint.state.players.north.avatar };
  delete readyAvatar.lastInteractedTurn;
  const avatarRef = {
    instanceId: readyAvatar.card.instanceId,
    kind: 'avatar' as const,
    seat: 'north' as const,
  };
  const minionRef = { instanceId: bearer.instanceId, kind: 'minion' as const, seat: 'north' as const };
  const dropReady: GameSession = {
    ...filteredCheckpoint,
    state: {
      ...filteredCheckpoint.state,
      players: {
        ...filteredCheckpoint.state.players,
        north: { ...filteredCheckpoint.state.players.north, avatar: readyAvatar },
      },
      realm: {
        ...filteredCheckpoint.state.realm,
        artifacts: [
          ...surfaceArtifacts.map((artifact) => ({
            bearer: minionRef,
            cardId: artifact.cardId,
            instanceId: artifact.instanceId,
            owner: artifact.owner,
            source: artifact.source,
          })),
          ...[remoteId, underwaterId].map((instanceId) => ({
            bearer: avatarRef,
            cardId: firstArtifact.cardId,
            instanceId,
            owner: firstArtifact.owner,
            source: firstArtifact.source,
          })),
        ],
      },
    },
  };
  const initialDrops = dropDescriptors(dropReady);
  assert.equal(initialDrops.length, 6);
  assert.deepEqual(initialDrops.filter(({ unit }) => unit.kind === 'minion')
    .map(({ artifactInstanceIds: ids }) => ids.join(',')).sort(), expectedSubsets);
  assert.deepEqual(initialDrops.filter(({ unit }) => unit.kind === 'avatar')
    .map(({ artifactInstanceIds: ids }) => ids.join(',')).sort(), [
    remoteId,
    underwaterId,
    [remoteId, underwaterId].sort().join(','),
  ].sort());
  const droppedOnce = accept(dropReady, action(dropReady, ({ descriptor }) =>
    descriptor.kind === 'drop-artifacts'
      && descriptor.unit.kind === 'minion'
      && descriptor.artifactInstanceIds.length === 1));
  assert.equal(dropDescriptors(droppedOnce).some(({ unit }) => unit.kind === 'minion'), false);

  const disabledCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        units: session.state.realm.units.map((unit) => unit.instanceId === bearer.instanceId
          ? {
            ...unit,
            disableEffects: [{ expiresAtSeat: 'south' as const, sourceInstanceId: unit.instanceId }],
          }
          : unit),
      },
    },
  };
  assert.deepEqual(pickupDescriptors(disabledCheckpoint).map(({ unit }) => unit.kind),
    ['avatar', 'avatar', 'avatar']);
  const disabledDrop: GameSession = {
    ...dropReady,
    state: {
      ...dropReady.state,
      realm: { ...dropReady.state.realm, units: disabledCheckpoint.state.realm.units },
    },
  };
  assert.equal(dropDescriptors(disabledDrop).some(({ unit }) => unit.kind === 'minion'), false);

  const beforeForge = hashGameState(session.state);
  const forged = stepGame(session, {
    actionId: 'sha256:9999999999999999999999999999999999999999999999999999999999999999',
    seat: 'north',
    stateVersion: session.state.stateVersion,
  });
  assert.equal(forged.accepted, false);
  assert.equal(forged.reason.code, 'unknown_action');
  assert.equal(hashGameState(forged.session.state), beforeForge);

  const manaBeforePickUp = session.state.players.north.mana;
  const picked = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'pick-up-artifacts'
      && descriptor.unit.kind === 'minion'
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === artifactInstanceIds[0]));
  assert.equal(picked.accepted, true);
  if (!picked.accepted) throw new Error('expected Artifact Pick Up to be accepted');
  session = picked.session;
  assert.deepEqual(picked.receipt.events.map(({ payload, type }) => ({ payload, type })), [{
    payload: {
      artifactInstanceIds: [artifactInstanceIds[0]],
      seat: 'north',
      unitInstanceId: bearer.instanceId,
      unitKind: 'minion',
    },
    type: 'artifacts-picked-up',
  }]);
  assert.deepEqual(picked.receipt.randomDraws, []);
  assert.equal(session.state.players.north.mana, manaBeforePickUp);
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.deepEqual(session.state.realm.units
    .filter(({ instanceId }) => instanceId === bearer.instanceId)
    .map(({ stealthed, summoningSickness, tapped }) => ({ stealthed, summoningSickness, tapped })), [{
    stealthed: true,
    summoningSickness: true,
    tapped: false,
  }]);
  assert.equal(session.state.realm.artifacts?.find(({ instanceId }) =>
    instanceId === artifactInstanceIds[0])?.owner, 'north');
  const voluntarilyDropped = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'drop-artifacts'
      && descriptor.unit.instanceId === bearer.instanceId
      && descriptor.artifactInstanceIds[0] === artifactInstanceIds[0]));
  assert.equal(voluntarilyDropped.accepted, true);
  if (!voluntarilyDropped.accepted) throw new Error('expected voluntary Artifact Drop to be accepted');
  assert.deepEqual(voluntarilyDropped.receipt.events.map(({ payload, type }) => ({ payload, type })), [{
    payload: {
      artifactInstanceIds: [artifactInstanceIds[0]],
      seat: 'north',
      unitInstanceId: bearer.instanceId,
      unitKind: 'minion',
    },
    type: 'artifacts-dropped',
  }]);
  assert.deepEqual(voluntarilyDropped.receipt.randomDraws, []);
  assert.deepEqual(observeGame(voluntarilyDropped.session.state, 'north').realm.artifacts
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
  assert.deepEqual(voluntarilyDropped.session.state.realm.units
    .filter(({ instanceId }) => instanceId === bearer.instanceId)
    .map(({ damage, stealthed, summoningSickness, tapped }) => ({
      damage, stealthed, summoningSickness, tapped,
    })), [{ damage: 0, stealthed: true, summoningSickness: true, tapped: false }]);
  assert.equal(voluntarilyDropped.session.state.players.north.mana, manaBeforePickUp);
  assert.equal(verifyGameReplay(voluntarilyDropped.session), true);
  assert.equal(pickupDescriptors(session).some(({ unit }) => unit.kind === 'minion'), false);
  assert.equal(pickupDescriptors(session).some(({ unit }) => unit.kind === 'avatar'), true);
  let northView = observeGame(session.state, 'north');
  assert.equal(northView.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId)?.attack, 3);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  assert.equal(pickupDescriptors(session).some(({ unit }) => unit.kind === 'minion'), true);
  const pickedAgain = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'pick-up-artifacts'
      && descriptor.unit.instanceId === bearer.instanceId
      && descriptor.artifactInstanceIds.length === 1
      && descriptor.artifactInstanceIds[0] === artifactInstanceIds[1]));
  assert.equal(pickedAgain.accepted, true);
  if (!pickedAgain.accepted) throw new Error('expected next-turn Artifact Pick Up to be accepted');
  session = pickedAgain.session;
  assert.deepEqual(pickedAgain.receipt.randomDraws, []);
  let movedOnly = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === bearer.instanceId
      && descriptor.to.cell === 'C3'));
  movedOnly = accept(movedOnly, action(movedOnly, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  assert.equal(dropDescriptors(movedOnly).some(({ unit }) => unit.instanceId === bearer.instanceId), true);
  assert.equal(movedOnly.state.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId)?.stealthed, true);

  const activated = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === bearer.instanceId));
  assert.equal(dropDescriptors(activated).some(({ unit }) => unit.instanceId === bearer.instanceId), false);

  const dummyCard = [
    ...session.state.players.south.hand.spellbook,
    ...session.state.players.south.spellbook,
  ].find(({ cardId }) => cardId === 'artifact-dummy');
  assert.ok(dummyCard);
  const strikeCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        units: [...session.state.realm.units, {
          ...dummyCard,
          controller: 'south',
          damage: 0,
          location: 'C4',
          region: 'surface',
          stealthed: false,
          summoningSickness: false,
          tapped: false,
          warded: false,
        }],
      },
    },
  };
  let struck = accept(strikeCheckpoint, action(strikeCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === bearer.instanceId
      && descriptor.path.length === 1));
  struck = accept(struck, action(struck, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === dummyCard.instanceId));
  assert.equal(dropDescriptors(struck).some(({ unit }) => unit.instanceId === bearer.instanceId), false);
  northView = observeGame(session.state, 'north');
  const observedBearer = northView.realm.units.find(({ instanceId }) => instanceId === bearer.instanceId);
  assert.equal(observedBearer?.attack, 5);
  assert.equal(observedBearer?.defense, 5);

  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'sword-and-shield'
    && descriptor.casterInstanceId === bearer.instanceId
    && descriptor.cell === 'C3'
    && descriptor.bearer === undefined);
  assert.equal(dropDescriptors(session).some(({ unit }) => unit.instanceId === bearer.instanceId), false);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId)?.stealthed, false);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === bearer.instanceId && descriptor.to.cell === 'C3');
  northView = observeGame(session.state, 'north');
  assert.deepEqual(northView.realm.artifacts?.map(({ bearer: artifactBearer, controller, location, region }) => ({
    bearer: artifactBearer?.instanceId,
    controller,
    location,
    region,
  })), [
    { bearer: bearer.instanceId, controller: 'north', location: 'C3', region: 'surface' },
    { bearer: bearer.instanceId, controller: 'north', location: 'C3', region: 'surface' },
    { bearer: undefined, controller: null, location: 'C3', region: 'surface' },
  ]);
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'artifact-enemy' && descriptor.cell === 'C3');
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === 'artifact-enemy')!;
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === enemy.instanceId && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion' && descriptor.target.instanceId === bearer.instanceId);
  const fought = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(fought.accepted, true);
  session = fought.session;
  const events = fought.receipt.events;
  const dropIndexes = events.flatMap(({ type }, index) => type === 'artifact-dropped' ? [index] : []);
  const deathIndex = events.findIndex(({ payload, type }) => type === 'minion-died'
    && canonicalJson(payload).includes(bearer.instanceId));
  assert.equal(dropIndexes.length, 2);
  assert.equal(dropIndexes.every((index) => index < deathIndex), true);
  northView = observeGame(session.state, 'north');
  assert.deepEqual(northView.realm.artifacts?.map((artifact) => ({
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
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) => instanceId === bearer.instanceId), true);
  assert.equal(session.state.players.north.cemetery.some(({ cardId }) => cardId === 'sword-and-shield'), false);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 dropping a power Artifact immediately kills a lethally wounded bearer', () => {
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
  let session = keep(createGameSession(createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-drop-state-based-death-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 214,
  })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === 'drop-bearer'));
  const bearer = session.state.realm.units.find(({ cardId }) => cardId === 'drop-bearer');
  assert.ok(bearer);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'drop-sword'
      && descriptor.bearer?.instanceId === bearer.instanceId));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardId === 'drop-static-servant'));
  assert.deepEqual(observeGame(session.state, 'north').realm.units
    .filter(({ instanceId }) => instanceId === bearer.instanceId)
    .map(({ damage, defense }) => ({ damage, defense })), [{ damage: 1, defense: 3 }]);

  const beforeDropVersion = session.state.stateVersion;
  const dropped = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'drop-artifacts'
      && descriptor.unit.instanceId === bearer.instanceId
      && descriptor.artifactInstanceIds.length === 1));
  assert.equal(dropped.accepted, true);
  if (!dropped.accepted) return;
  session = dropped.session;

  assert.equal(session.state.stateVersion, beforeDropVersion + 1);
  assert.deepEqual(dropped.receipt.events.map(({ type }) => type), [
    'artifacts-dropped',
    'minion-died',
  ]);
  assert.deepEqual(dropped.receipt.randomDraws, []);
  assert.equal(session.state.realm.units.some(({ instanceId }) => instanceId === bearer.instanceId), false);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === bearer.instanceId), true);
  assert.deepEqual(observeGame(session.state, 'north').realm.artifacts?.map((artifact) => ({
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
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 a carried Lethal Artifact kills on positive strike damage and drops with its bearer', () => {
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

  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'dagger-bearer' && descriptor.cell === 'C4');
  const bearer = session.state.realm.units.find(({ cardId }) => cardId === 'dagger-bearer')!;
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'poisonous-dagger'
    && descriptor.bearer?.instanceId === bearer.instanceId);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === bearer.instanceId && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'dagger-enemy' && descriptor.cell === 'C3');
  const enemy = session.state.realm.units.find(({ cardId }) => cardId === 'dagger-enemy')!;
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === enemy.instanceId && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion' && descriptor.target.instanceId === bearer.instanceId);
  const fought = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(fought.accepted, true);
  session = fought.session;

  const events = fought.receipt.events;
  const lethalDamage = events.find(({ payload, type }) => type === 'damage-dealt'
    && canonicalJson(payload).includes(enemy.instanceId));
  assert.ok(lethalDamage);
  assert.match(canonicalJson(lethalDamage.payload), /"amount":2/);
  assert.equal(events.filter(({ type }) => type === 'minion-died').length, 2);
  const dropIndex = events.findIndex(({ type }) => type === 'artifact-dropped');
  const bearerDeathIndex = events.findIndex(({ payload, type }) => type === 'minion-died'
    && canonicalJson(payload).includes(bearer.instanceId));
  assert.ok(dropIndex >= 0 && dropIndex < bearerDeathIndex);
  assert.equal(session.state.realm.units.some(({ instanceId }) =>
    instanceId === bearer.instanceId || instanceId === enemy.instanceId), false);
  assert.deepEqual(observeGame(session.state, 'north').realm.artifacts?.map((artifact) => ({
    bearer: artifact.bearer,
    controller: artifact.controller,
    location: artifact.location,
    region: artifact.region,
  })), [{ bearer: undefined, controller: null, location: 'C3', region: 'surface' }]);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Siege Ballista taps its bearer and another ally for measured artifact damage', () => {
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
      attack: 1,
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

  let session = keep(keep(createGameSession(manifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'ballista-bearer'
    && descriptor.cell === 'C4'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'ballista-helper'
    && descriptor.cell === 'C4'
    && descriptor.region === undefined);
  const bearer = session.state.realm.units.find(({ cardId }) => cardId === 'ballista-bearer');
  const helper = session.state.realm.units.find(({ cardId }) => cardId === 'ballista-helper');
  assert.ok(bearer && helper);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'siege-ballista'
    && descriptor.bearer?.kind === 'minion'
    && descriptor.bearer.instanceId === bearer.instanceId);
  const ballista = session.state.realm.artifacts?.find(({ cardId }) => cardId === 'siege-ballista');
  assert.ok(ballista);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.artifactInstanceId === ballista.instanceId
      && descriptor.helper.instanceId === helper.instanceId), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'ballista-far-target' && descriptor.cell === 'C1');
  const farTarget = session.state.realm.units.find(({ cardId }) => cardId === 'ballista-far-target');
  assert.ok(farTarget);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.artifactInstanceId === ballista.instanceId
      && descriptor.target.instanceId === farTarget.instanceId), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'ballista-near-target'
    && descriptor.cell === 'C2'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'ballista-burrowed-target'
    && descriptor.cell === 'C2'
    && descriptor.region === 'underground');
  const nearTarget = session.state.realm.units.find(({ cardId }) => cardId === 'ballista-near-target');
  const burrowedTarget = session.state.realm.units.find(({ cardId }) =>
    cardId === 'ballista-burrowed-target');
  assert.ok(nearTarget && burrowedTarget);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

  const abilities = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.artifactInstanceId === ballista.instanceId);
  assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
    && descriptor.helper.instanceId === helper.instanceId
    && descriptor.target.instanceId === nearTarget.instanceId), true, JSON.stringify({
    abilities: abilities.map(({ descriptor }) => descriptor),
    bearer: bearer.instanceId,
    burrowedTarget: burrowedTarget.instanceId,
    farTarget: farTarget.instanceId,
    helper: helper.instanceId,
    nearTarget: nearTarget.instanceId,
  }));
  assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
    && descriptor.target.instanceId === farTarget.instanceId), false);
  assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
    && descriptor.target.instanceId === burrowedTarget.instanceId), false);
  assert.equal(abilities.some(({ descriptor }) => descriptor.kind === 'activate-artifact-damage'
    && descriptor.target.instanceId === bearer.instanceId), true);

  const noHelperState = {
    ...session.state,
    realm: {
      ...session.state.realm,
      units: session.state.realm.units.map((unit) => unit.instanceId === helper.instanceId
        ? { ...unit, location: 'C3' as const }
        : unit),
    },
  };
  assert.equal(legalGameActions(noHelperState, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.helper.instanceId === helper.instanceId), false);
  const tappedHelperState = {
    ...session.state,
    realm: {
      ...session.state.realm,
      units: session.state.realm.units.map((unit) => unit.instanceId === helper.instanceId
        ? { ...unit, tapped: true }
        : unit),
    },
  };
  assert.equal(legalGameActions(tappedHelperState, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.helper.instanceId === helper.instanceId), false);
  const disabledBearerState = {
    ...session.state,
    realm: {
      ...session.state.realm,
      units: session.state.realm.units.map((unit) => unit.instanceId === bearer.instanceId
        ? { ...unit, disabledUntilDamaged: true as const }
        : unit),
    },
  };
  assert.equal(legalGameActions(disabledBearerState, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.artifactInstanceId === ballista.instanceId), false);
  const uncarriedState = {
    ...session.state,
    realm: {
      ...session.state.realm,
      artifacts: session.state.realm.artifacts!.map((artifact) =>
        artifact.instanceId === ballista.instanceId
          ? {
            cardId: artifact.cardId,
            instanceId: artifact.instanceId,
            location: 'C4' as const,
            owner: artifact.owner,
            region: 'surface' as const,
            source: artifact.source,
          }
          : artifact),
    },
  };
  assert.equal(legalGameActions(uncarriedState, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.artifactInstanceId === ballista.instanceId), false);
  const hiddenTargetState = {
    ...session.state,
    realm: {
      ...session.state.realm,
      units: session.state.realm.units.map((unit) => unit.instanceId === nearTarget.instanceId
        ? { ...unit, stealthed: true }
        : unit),
    },
  };
  assert.equal(legalGameActions(hiddenTargetState, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.target.instanceId === nearTarget.instanceId), false);
  const undergroundState = {
    ...session.state,
    realm: {
      ...session.state.realm,
      units: session.state.realm.units.map((unit) =>
        unit.instanceId === bearer.instanceId || unit.instanceId === helper.instanceId
          ? { ...unit, region: 'underground' as const }
          : unit),
    },
  };
  assert.equal(legalGameActions(undergroundState, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.artifactInstanceId === ballista.instanceId
      && descriptor.helper.instanceId === helper.instanceId
      && descriptor.target.instanceId === burrowedTarget.instanceId), true);

  const overlap = abilities.find(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.helper.instanceId === helper.instanceId
      && descriptor.target.instanceId === helper.instanceId);
  assert.ok(overlap);
  const overlapResult = stepGame(session, overlap);
  assert.equal(overlapResult.accepted, true);
  if (!overlapResult.accepted) return;
  assert.equal(overlapResult.session.state.realm.units.some(({ instanceId }) =>
    instanceId === helper.instanceId), false);
  assert.equal(overlapResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId)?.tapped, true);
  assert.equal(verifyGameReplay(overlapResult.session), true);

  const activation = abilities.find(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-damage'
      && descriptor.helper.instanceId === helper.instanceId
      && descriptor.target.instanceId === nearTarget.instanceId);
  assert.ok(activation);
  const result = stepGame(session, activation);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
  const survivingBearer = session.state.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId);
  const survivingHelper = session.state.realm.units.find(({ instanceId }) =>
    instanceId === helper.instanceId);
  const damagedTarget = session.state.realm.units.find(({ instanceId }) =>
    instanceId === nearTarget.instanceId);
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
  assert.equal(canonicalJson(activated.payload).includes(ballista.instanceId), true);
  assert.deepEqual(allocated.payload, {
    amount: 3,
    sourceInstanceId: ballista.instanceId,
    targetInstanceId: nearTarget.instanceId,
  });
  assert.deepEqual(result.receipt.events.map(({ type }) => type), [
    'artifact-damage-activated',
    'artifact-damage-allocated',
    'damage-dealt',
  ]);
  assert.equal(result.receipt.randomDraws.length, 0);
  assert.equal(session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Payload Trebuchet discards a card for measured location damage', () => {
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

  let session = keep(keep(createGameSession(manifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'payload-bearer' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'payload-helper' && descriptor.cell === 'C4');
  const bearer = session.state.realm.units.find(({ cardId }) => cardId === 'payload-bearer');
  const helper = session.state.realm.units.find(({ cardId }) => cardId === 'payload-helper');
  assert.ok(bearer && helper);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'payload-trebuchet'
    && descriptor.bearer?.instanceId === bearer.instanceId);
  const payload = session.state.realm.artifacts?.find(({ cardId }) => cardId === 'payload-trebuchet');
  assert.ok(payload);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-discard-area-damage'), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'payload-target'
    && descriptor.cell === 'C1'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'payload-warded-target'
    && descriptor.cell === 'C1'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'payload-burrowed-target'
    && descriptor.cell === 'C1'
    && descriptor.region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

  const discard = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === 'payload-discard');
  const siteDiscard = session.state.players.north.hand.atlas[0];
  const target = session.state.realm.units.find(({ cardId }) => cardId === 'payload-target');
  const warded = session.state.realm.units.find(({ cardId }) => cardId === 'payload-warded-target');
  const burrowed = session.state.realm.units.find(({ cardId }) =>
    cardId === 'payload-burrowed-target');
  assert.ok(discard && siteDiscard && target && warded && burrowed);
  const choices = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-discard-area-damage'
      && descriptor.artifactInstanceId === payload.instanceId
      && descriptor.helper.instanceId === helper.instanceId);
  assert.equal(choices.some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-discard-area-damage'
      && descriptor.discardCardInstanceId === discard.instanceId
      && descriptor.discardZone === 'spellbook'
      && descriptor.targetLocation.cell === 'C1'
      && descriptor.targetLocation.region === 'surface'), true);
  assert.equal(choices.some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-discard-area-damage'
      && descriptor.targetLocation.cell === 'C1'
      && descriptor.targetLocation.region === 'underground'), false);

  const friendly = choices.find(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-discard-area-damage'
      && descriptor.discardCardInstanceId === discard.instanceId
      && descriptor.targetLocation.cell === 'C4');
  assert.ok(friendly);
  const friendlyResult = stepGame(session, friendly);
  assert.equal(friendlyResult.accepted, true);
  if (!friendlyResult.accepted) return;
  const friendlyBearer = friendlyResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId);
  assert.deepEqual({
    avatarLife: friendlyResult.session.state.players.north.avatar.life,
    bearerDamage: friendlyBearer?.damage,
    bearerLance: friendlyBearer?.carriedLanceCount,
    bearerStealth: friendlyBearer?.stealthed,
    helperDamage: friendlyResult.session.state.realm.units.find(({ instanceId }) =>
      instanceId === helper.instanceId)?.damage,
  }, {
    avatarLife: 16,
    bearerDamage: 4,
    bearerLance: 1,
    bearerStealth: true,
    helperDamage: 4,
  });
  assert.equal(verifyGameReplay(friendlyResult.session), true);

  const zero = choices.find(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-discard-area-damage'
      && descriptor.discardCardInstanceId === siteDiscard.instanceId
      && descriptor.discardZone === 'atlas'
      && descriptor.targetLocation.cell === 'C1');
  assert.ok(zero);
  const zeroResult = stepGame(session, zero);
  assert.equal(zeroResult.accepted, true);
  if (!zeroResult.accepted) return;
  assert.equal(zeroResult.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === siteDiscard.instanceId), true);
  assert.equal(zeroResult.receipt.events.filter(({ type }) =>
    type === 'artifact-discard-area-damage-allocated').every(({ payload }) =>
    (payload as { amount: number }).amount === 0), true);
  assert.equal(zeroResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === warded.instanceId)?.warded, true);
  assert.equal(zeroResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId)?.damage, 0);
  assert.equal(verifyGameReplay(zeroResult.session), true);

  const activation = choices.find(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-discard-area-damage'
      && descriptor.discardCardInstanceId === discard.instanceId
      && descriptor.discardZone === 'spellbook'
      && descriptor.targetLocation.cell === 'C1');
  assert.ok(activation);
  const result = stepGame(session, activation);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
  const survivingBearer = session.state.realm.units.find(({ instanceId }) =>
    instanceId === bearer.instanceId);
  const survivingHelper = session.state.realm.units.find(({ instanceId }) =>
    instanceId === helper.instanceId);
  const survivingWard = session.state.realm.units.find(({ instanceId }) =>
    instanceId === warded.instanceId);
  assert.deepEqual({
    bearerDamage: survivingBearer?.damage,
    bearerLance: survivingBearer?.carriedLanceCount,
    bearerStealth: survivingBearer?.stealthed,
    bearerTapped: survivingBearer?.tapped,
    burrowedDamage: session.state.realm.units.find(({ instanceId }) =>
      instanceId === burrowed.instanceId)?.damage,
    helperDamage: survivingHelper?.damage,
    helperTapped: survivingHelper?.tapped,
    southLife: session.state.players.south.avatar.life,
    targetPresent: session.state.realm.units.some(({ instanceId }) =>
      instanceId === target.instanceId),
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
  assert.equal(session.state.players.north.hand.spellbook.some(({ instanceId }) =>
    instanceId === discard.instanceId), false);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === discard.instanceId), true);
  const allocations = result.receipt.events.filter(({ type }) =>
    type === 'artifact-discard-area-damage-allocated');
  assert.equal(allocations.length, 3);
  assert.equal(allocations.every(({ payload: allocationPayload }) => {
    const allocation = allocationPayload as {
      amount?: number;
      sourceInstanceId?: string;
    };
    return allocation.amount === 4 && allocation.sourceInstanceId === payload.instanceId;
  }), true);
  assert.deepEqual(result.receipt.events.slice(0, 2).map(({ type }) => type), [
    'card-discarded',
    'artifact-discard-area-damage-activated',
  ]);
  assert.equal(result.receipt.events.some(({ type }) =>
    type === 'fight-started' || type === 'strike-damage-allocated' || type === 'lance-broken'), false);
  assert.equal(result.receipt.randomDraws.length, 0);
  assert.equal(session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Rolling Boulder rolls maximally and damages other units along its path', () => {
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

  let session = keep(keep(createGameSession(rollingManifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'boulder-pusher' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'boulder-origin-target' && descriptor.cell === 'C4');
  const pusher = session.state.realm.units.find(({ cardId }) => cardId === 'boulder-pusher');
  const originTarget = session.state.realm.units.find(({ cardId }) =>
    cardId === 'boulder-origin-target');
  assert.ok(pusher && originTarget);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'rolling-boulder'
    && descriptor.bearer === undefined
    && descriptor.cell === 'C4');
  const boulder = session.state.realm.artifacts?.find(({ cardId }) => cardId === 'rolling-boulder');
  assert.ok(boulder);
  const freshRolls = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-roll-damage');
  assert.equal(freshRolls.some(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-roll-damage'
      && descriptor.pusher.instanceId === pusher.instanceId), false);
  assert.deepEqual({
    avatarLocation: session.state.players.north.avatar.location,
    avatarTapped: session.state.players.north.avatar.tapped,
    pushers: freshRolls.flatMap(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage'
        ? [descriptor.pusher.kind]
        : []),
  }, {
    avatarLocation: 'C4',
    avatarTapped: true,
    pushers: [],
  });
  take(({ descriptor }) => descriptor.kind === 'end-turn');

  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'boulder-warded-target'
    && descriptor.cell === 'C2'
    && descriptor.region === undefined);
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'boulder-underground-target'
    && descriptor.cell === 'C2'
    && descriptor.region === 'underground');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'boulder-off-path-target'
    && descriptor.cell === 'B1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

  const warded = session.state.realm.units.find(({ cardId }) => cardId === 'boulder-warded-target');
  const underground = session.state.realm.units.find(({ cardId }) =>
    cardId === 'boulder-underground-target');
  const offPath = session.state.realm.units.find(({ cardId }) =>
    cardId === 'boulder-off-path-target');
  assert.ok(warded && underground && offPath);
  const rolls = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-artifact-roll-damage'
      && descriptor.artifactInstanceId === boulder.instanceId
      && descriptor.pusher.instanceId === pusher.instanceId);
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
  const beforeForge = hashGameState(session.state);
  const forged = stepGame(session, {
    actionId: opaqueActionId(
      'sorcery-core-v1',
      'north',
      session.state.stateVersion,
      shortenedDescriptor,
    ),
    seat: 'north',
    stateVersion: session.state.stateVersion,
  });
  assert.equal(forged.accepted, false);
  assert.equal(forged.reason.code, 'unknown_action');
  assert.equal(hashGameState(forged.session.state), beforeForge);

  const zeroResult = stepGame(session, zeroRoll);
  assert.equal(zeroResult.accepted, true);
  if (!zeroResult.accepted) return;
  assert.equal(zeroResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === pusher.instanceId)?.tapped, true);
  assert.equal(zeroResult.session.state.realm.units.find(({ instanceId }) =>
    instanceId === originTarget.instanceId)?.damage, 0);
  assert.deepEqual(zeroResult.receipt.events.map(({ type }) => type), [
    'artifact-roll-damage-activated',
  ]);
  assert.equal(observeGame(zeroResult.session.state, 'north').realm.artifacts
    ?.find(({ instanceId }) => instanceId === boulder.instanceId)?.location, 'C4');
  assert.equal(verifyGameReplay(zeroResult.session), true);

  const carriedSession: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        artifacts: session.state.realm.artifacts!.map((artifact) =>
          artifact.instanceId === boulder.instanceId
            ? {
              bearer: { instanceId: pusher.instanceId, kind: 'minion' as const, seat: 'north' as const },
              cardId: artifact.cardId,
              instanceId: artifact.instanceId,
              owner: artifact.owner,
              source: artifact.source,
            }
            : artifact),
      },
    },
  };
  const carriedResult = stepGame(carriedSession, action(carriedSession, ({ descriptor }) =>
    descriptor.kind === 'activate-artifact-roll-damage'
      && descriptor.artifactInstanceId === boulder.instanceId
      && descriptor.pusher.instanceId === pusher.instanceId
      && descriptor.direction === 'south'));
  assert.equal(carriedResult.accepted, true);
  if (!carriedResult.accepted) return;
  assert.deepEqual(observeGame(carriedResult.session.state, 'north').realm.artifacts
    ?.filter(({ instanceId }) => instanceId === boulder.instanceId)
    .map(({ bearer, controller, location, region }) => ({ bearer, controller, location, region })),
  [{ bearer: undefined, controller: null, location: 'C1', region: 'surface' }]);

  const result = stepGame(session, southRoll);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;
  const survivingPusher = session.state.realm.units.find(({ instanceId }) =>
    instanceId === pusher.instanceId);
  const survivingWard = session.state.realm.units.find(({ instanceId }) =>
    instanceId === warded.instanceId);
  assert.deepEqual({
    northLife: session.state.players.north.avatar.life,
    offPathDamage: session.state.realm.units.find(({ instanceId }) =>
      instanceId === offPath.instanceId)?.damage,
    originTargetPresent: session.state.realm.units.some(({ instanceId }) =>
      instanceId === originTarget.instanceId),
    pusherDamage: survivingPusher?.damage,
    pusherLance: survivingPusher?.carriedLanceCount,
    pusherStealth: survivingPusher?.stealthed,
    pusherTapped: survivingPusher?.tapped,
    southLife: session.state.players.south.avatar.life,
    undergroundDamage: session.state.realm.units.find(({ instanceId }) =>
      instanceId === underground.instanceId)?.damage,
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
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === originTarget.instanceId), true);
  assert.deepEqual(observeGame(session.state, 'north').realm.artifacts
    ?.filter(({ instanceId }) => instanceId === boulder.instanceId)
    .map(({ bearer, controller, location, region }) => ({ bearer, controller, location, region })),
  [{ bearer: undefined, controller: null, location: 'C1', region: 'surface' }]);
  const allocations = result.receipt.events.filter(({ type }) =>
    type === 'artifact-roll-damage-allocated');
  const expectedTargetIds = [
    session.state.players.north.avatar.card.instanceId,
    originTarget.instanceId,
    session.state.players.south.avatar.card.instanceId,
    warded.instanceId,
  ].sort((left, right) => left.localeCompare(right));
  assert.deepEqual(allocations.map(({ payload }) =>
    (payload as { targetInstanceId: string }).targetInstanceId), expectedTargetIds);
  assert.equal(allocations.every(({ payload }) => {
    const allocation = payload as { amount?: number; sourceInstanceId?: string };
    return allocation.amount === 4 && allocation.sourceInstanceId === boulder.instanceId;
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
  assert.equal(session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03/05 Mesmerism transfers a minion and its Deathrite to the new controller', () => {
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

  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'mesmerism-target'
    && descriptor.casterInstanceId === session.state.players.south.avatar.card.instanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'mesmerism-warded'
    && descriptor.casterInstanceId === session.state.players.south.avatar.card.instanceId
    && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'mesmerism-attacker'
    && descriptor.casterInstanceId === session.state.players.south.avatar.card.instanceId
    && descriptor.cell === 'C1');
  const hiddenTarget = session.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-target')!;
  const wardedTarget = session.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-warded')!;
  const attacker = session.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-attacker')!;

  let hidden = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  hidden = accept(hidden, action(hidden, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const hiddenTargetIds = legalGameActions(hidden.state, 'north').flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.kind === 'minion'
      ? [descriptor.target.instanceId]
      : []);
  assert.deepEqual([...new Set(hiddenTargetIds)], [wardedTarget.instanceId]);
  assert.equal(verifyGameReplay(hidden), true);

  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'mesmerism-artifact'
    && descriptor.casterInstanceId === hiddenTarget.instanceId
    && descriptor.bearer?.instanceId === hiddenTarget.instanceId);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === hiddenTarget.instanceId)?.stealthed, false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  const checkpoint = session;
  const targetBefore = checkpoint.state.realm.units.find(({ instanceId }) =>
    instanceId === hiddenTarget.instanceId)!;
  const carriedBefore = checkpoint.state.realm.artifacts?.find((artifact) =>
    'bearer' in artifact && artifact.bearer.instanceId === hiddenTarget.instanceId);
  assert.ok(carriedBefore);
  assert.ok('bearer' in carriedBefore);
  const casts = legalGameActions(checkpoint.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic');
  assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.kind === 'minion'
      ? [descriptor.target.instanceId]
      : []))].sort(), [hiddenTarget.instanceId, wardedTarget.instanceId].sort());

  const warded = stepGame(checkpoint, action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === wardedTarget.instanceId));
  assert.equal(warded.accepted, true);
  if (!warded.accepted) return;
  assert.deepEqual(warded.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'ward-broken',
    'magic-resolved',
  ]);
  assert.deepEqual({
    controller: warded.session.state.realm.units.find(({ instanceId }) =>
      instanceId === wardedTarget.instanceId)?.controller,
    warded: warded.session.state.realm.units.find(({ instanceId }) =>
      instanceId === wardedTarget.instanceId)?.warded,
  }, { controller: 'south', warded: false });
  assert.equal(verifyGameReplay(warded.session), true);

  const gainedAction = action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === hiddenTarget.instanceId);
  const sourceInstanceId = gainedAction.descriptor.kind === 'cast-magic'
    ? gainedAction.descriptor.cardInstanceId
    : '';
  const gained = stepGame(checkpoint, gainedAction);
  assert.equal(gained.accepted, true);
  if (!gained.accepted) return;
  session = gained.session;
  const controlled = session.state.realm.units.find(({ instanceId }) =>
    instanceId === hiddenTarget.instanceId)!;
  assert.deepEqual(controlled, { ...targetBefore, controller: 'north' });
  assert.deepEqual(gained.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'minion-control-changed',
    'magic-resolved',
  ]);
  assert.deepEqual(gained.receipt.events[1]?.payload, {
    fromSeat: 'south',
    instanceId: hiddenTarget.instanceId,
    seat: 'north',
    sourceInstanceId,
  });
  const carried = session.state.realm.artifacts?.find((artifact) =>
    'bearer' in artifact && artifact.bearer.instanceId === hiddenTarget.instanceId);
  assert.ok(carried);
  assert.ok('bearer' in carried);
  assert.deepEqual(carried, {
    ...carriedBefore,
    bearer: { ...carriedBefore.bearer, seat: 'north' },
  });
  const northView = observeGame(session.state, 'north');
  assert.deepEqual({
    artifactController: northView.realm.artifacts?.[0]?.controller,
    artifactSeat: northView.realm.artifacts?.[0]?.bearer?.seat,
    attack: northView.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId)?.attack,
    defense: northView.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId)?.defense,
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
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === hiddenTarget.instanceId
      && descriptor.to.cell === 'C3'), true);

  const ownNoOp = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.casterInstanceId === session.state.players.north.avatar.card.instanceId
      && descriptor.target?.instanceId === hiddenTarget.instanceId));
  assert.equal(ownNoOp.accepted, true);
  if (!ownNoOp.accepted) return;
  session = ownNoOp.session;
  assert.deepEqual(ownNoOp.receipt.events.map(({ type }) => type), ['magic-cast', 'magic-resolved']);
  assert.deepEqual(session.state.realm.units.find(({ instanceId }) =>
    instanceId === hiddenTarget.instanceId), controlled);
  assert.deepEqual(session.state.realm.artifacts?.find((artifact) =>
    'bearer' in artifact && artifact.bearer.instanceId === hiddenTarget.instanceId), carried);
  assert.equal(gained.receipt.randomDraws.length + ownNoOp.receipt.randomDraws.length, 0);
  assert.equal(session.state.players.north.mana, checkpoint.state.players.north.mana - 2);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === attacker.instanceId && descriptor.to.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === hiddenTarget.instanceId);
  const playersBeforeDeathrite = session.state.players;
  const fought = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(fought.accepted, true);
  if (!fought.accepted) return;
  session = fought.session;
  assert.equal(session.state.realm.units.some(({ instanceId }) =>
    instanceId === hiddenTarget.instanceId), false);
  assert.equal(session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === hiddenTarget.instanceId), true);
  assert.equal(session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === hiddenTarget.instanceId), false);
  const deathEvents = fought.receipt.events;
  assert.equal(session.state.players.north.atlas.length, playersBeforeDeathrite.north.atlas.length - 1);
  assert.equal(session.state.players.north.hand.atlas.length,
    playersBeforeDeathrite.north.hand.atlas.length + 1);
  assert.equal(session.state.players.south.atlas.length, playersBeforeDeathrite.south.atlas.length);
  assert.equal(session.state.players.south.hand.atlas.length,
    playersBeforeDeathrite.south.hand.atlas.length);
  assert.deepEqual(deathEvents.find(({ type }) => type === 'site-drawn')?.payload, {
    seat: 'north',
    sourceInstanceId: hiddenTarget.instanceId,
  });
  const drawIndex = deathEvents.findIndex(({ type }) => type === 'site-drawn');
  const dropIndex = deathEvents.findIndex(({ type }) => type === 'artifact-dropped');
  const deathIndex = deathEvents.findIndex(({ payload, type }) => type === 'minion-died'
    && canonicalJson(payload).includes(hiddenTarget.instanceId));
  assert.ok(drawIndex >= 0 && drawIndex < dropIndex && dropIndex < deathIndex);
  assert.equal(observeGame(session.state, 'north').realm.artifacts?.[0]?.controller, null);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Fatality kills only a wounded minion in the caster region', () => {
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

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'fatality-target' && descriptor.cell === 'C4');
  const target = session.state.realm.units.find(({ cardId }) => cardId === 'fatality-target');
  assert.ok(target);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === session.state.players.north.avatar.card.instanceId
    && descriptor.path.length === 1);
  take(({ descriptor }) => descriptor.kind === 'declare-attack'
    && descriptor.target.kind === 'minion'
    && descriptor.target.instanceId === target.instanceId);
  take(({ descriptor }) => descriptor.kind === 'close-defend'
    && descriptor.originalTargetParticipates);
  const checkpoint = session;
  const wounded = checkpoint.state.realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId);
  const fatality = checkpoint.state.players.north.hand.spellbook
    .find(({ cardId }) => cardId === 'fatality');
  assert.equal(wounded?.damage, 1);
  assert.ok(fatality);

  const healthyId = 'sha256:2222222222222222222222222222222222222222222222222222222222222222' as const;
  const hiddenId = 'sha256:3333333333333333333333333333333333333333333333333333333333333333' as const;
  const undergroundId = 'sha256:4444444444444444444444444444444444444444444444444444444444444444' as const;
  const alliedId = 'sha256:5555555555555555555555555555555555555555555555555555555555555555' as const;
  const filteredState = {
    ...checkpoint.state,
    realm: {
      ...checkpoint.state.realm,
      units: [
        ...checkpoint.state.realm.units,
        { ...target, damage: 0, instanceId: healthyId },
        { ...target, damage: 1, instanceId: hiddenId, stealthed: true },
        { ...target, damage: 1, instanceId: undergroundId, region: 'underground' as const },
        {
          ...target,
          controller: 'north' as const,
          damage: 1,
          instanceId: alliedId,
          owner: 'north' as const,
          stealthed: true,
        },
      ],
    },
  };
  const targetRefs = legalGameActions(filteredState, 'north').flatMap(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === fatality.instanceId
      && descriptor.target
      ? [descriptor.target]
      : []);
  assert.equal(targetRefs.some(({ kind }) => kind === 'avatar'), false);
  assert.deepEqual(targetRefs.map(({ instanceId }) => instanceId).sort(), [
    alliedId,
    target.instanceId,
  ].sort());

  const wardedCheckpoint: GameSession = {
    ...checkpoint,
    state: {
      ...checkpoint.state,
      realm: {
        ...checkpoint.state.realm,
        units: checkpoint.state.realm.units.map((unit) => unit.instanceId === target.instanceId
          ? { ...unit, warded: true }
          : unit),
      },
    },
  };
  const warded = stepGame(wardedCheckpoint, action(wardedCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === target.instanceId));
  assert.equal(warded.accepted, true);
  if (!warded.accepted) return;
  assert.deepEqual(warded.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'ward-broken',
    'magic-resolved',
  ]);
  assert.deepEqual(warded.session.state.realm.units
    .filter(({ instanceId }) => instanceId === target.instanceId)
    .map(({ damage, warded: hasWard }) => ({ damage, warded: hasWard })), [{
    damage: 1,
    warded: false,
  }]);

  const cast = action(checkpoint, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === target.instanceId);
  const sourceInstanceId = cast.descriptor.kind === 'cast-magic'
    ? cast.descriptor.cardInstanceId
    : '';
  const killed = stepGame(checkpoint, cast);
  assert.equal(killed.accepted, true);
  if (!killed.accepted) return;
  assert.deepEqual(killed.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'minion-killed',
    'minion-died',
    'magic-resolved',
  ]);
  assert.deepEqual(killed.receipt.events[1]?.payload, {
    cardId: target.cardId,
    instanceId: target.instanceId,
    owner: 'south',
    seat: 'south',
    sourceInstanceId,
  });
  assert.equal(killed.receipt.events.some(({ type }) => type === 'damage-dealt'), false);
  assert.deepEqual(killed.receipt.randomDraws, []);
  assert.equal(killed.session.state.realm.units.some(({ instanceId }) =>
    instanceId === target.instanceId), false);
  assert.equal(killed.session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === target.instanceId), true);
  assert.equal(killed.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === sourceInstanceId), true);
  assert.equal(verifyGameReplay(killed.session), true);
});

test('RULE-03 Sparkmage may tap for zero damage at a nearby location with no other unit', () => {
  let session = keep(keep(createGameSession(manifest(417, {
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
  }))));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));

  const activation = action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-sparkmage'
      && descriptor.targetLocation.cell === 'C4'
      && descriptor.targetLocation.region === 'surface');
  assert.match(activation.label, /deal 0 to a random other unit at C4/);
  const result = stepGame(session, activation);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;

  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.players.north.airThresholdsCastThisTurn, 0);
  assert.deepEqual(result.receipt.randomDraws, []);
  assert.deepEqual(result.receipt.events.map(({ type }) => type), ['sparkmage-activated']);
  assert.doesNotMatch(canonicalJson(result.receipt.events[0]!.payload), /targetInstanceId/);
  assert.equal(observeGame(session.state, 'south').players.north.airThresholdsCastThisTurn, 0);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Sparkmage counts every player-cast spell source, resets, and damages one other unit', () => {
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
    { defense: 4, manaCost: 0, thresholds: { air: 1, earth: 0, fire: 0, water: 0 } },
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
  const preview = createGameSession(gameManifest).state.players.north;
  const opening = preview.hand.spellbook.map(({ cardId }) => cardId);
  assert.equal(['sparkmage-caster', 'sparkmage-artifact', 'sparkmage-magic']
    .every((cardId) => opening.includes(cardId)), true);
  assert.equal(preview.spellbook[0]?.cardId, 'sparkmage-magic');
  assert.deepEqual(gameManifest.cards['sparkmage-avatar'], {
    attack: 1,
    cardType: 'avatar',
    defense: 1,
    drawSpell: false,
    life: 20,
    tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true,
  });

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'sparkmage-caster' && descriptor.cell === 'C4');
  const caster = session.state.realm.units.find(({ cardId }) => cardId === 'sparkmage-caster');
  assert.ok(caster);
  assert.equal(session.state.players.north.airThresholdsCastThisTurn, 1);
  take(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === 'sparkmage-artifact'
    && descriptor.casterInstanceId === caster.instanceId
    && descriptor.bearer?.instanceId === caster.instanceId);
  assert.equal(session.state.players.north.airThresholdsCastThisTurn, 2);
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardId === 'sparkmage-magic'
    && descriptor.casterInstanceId === caster.instanceId);
  assert.equal(session.state.players.north.airThresholdsCastThisTurn, 3);
  const opponentView = observeGame(session.state, 'south');
  assert.equal(opponentView.players.north.airThresholdsCastThisTurn, 3);
  assert.equal(typeof opponentView.players.north.hand.spellbook, 'number');

  const resetCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      players: {
        ...session.state.players,
        south: { ...session.state.players.south, airThresholdsCastThisTurn: 2 },
      },
    },
  };
  const reset = stepGame(resetCheckpoint, action(resetCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(reset.accepted, true);
  if (!reset.accepted) return;
  assert.equal(reset.session.state.players.north.airThresholdsCastThisTurn, 0);
  assert.equal(reset.session.state.players.south.airThresholdsCastThisTurn, 0);

  take(({ descriptor }) => descriptor.kind === 'end-turn');
  assert.equal(session.state.players.north.airThresholdsCastThisTurn, 0);
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardId === 'sparkmage-magic'
    && descriptor.casterInstanceId === caster.instanceId);
  assert.equal(session.state.players.north.airThresholdsCastThisTurn, 1);

  const activation = action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-sparkmage'
      && descriptor.targetLocation.cell === 'C4'
      && descriptor.targetLocation.region === 'surface');
  assert.doesNotMatch(canonicalJson(activation.descriptor), new RegExp(caster.instanceId));
  const result = stepGame(session, activation);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;

  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === caster.instanceId)?.damage, 1);
  assert.equal(result.receipt.randomDraws.length, 1);
  assert.equal(
    result.receipt.randomDraws[0]?.purpose,
    'sparkmage_random_other_unit_at_nearby_location',
  );
  assert.deepEqual(result.receipt.events.map(({ type }) => type), [
    'sparkmage-activated',
    'damage-dealt',
  ]);
  assert.match(canonicalJson(result.receipt.events[0]!.payload), new RegExp(caster.instanceId));
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-03 Sparkmage chooses among multiple other units with deterministic private RNG', () => {
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
  const preview = createGameSession(gameManifest).state.players.north;
  assert.ok(preview.hand.spellbook.filter(({ cardId }) =>
    cardId === 'sparkmage-many-target').length >= 2);
  assert.equal(preview.spellbook[0]?.cardId, 'sparkmage-many-magic');

  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  const targets = session.state.players.north.hand.spellbook
    .filter(({ cardId }) => cardId === 'sparkmage-many-target')
    .slice(0, 2);
  assert.equal(targets.length, 2);
  for (const target of targets) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === target.instanceId
      && descriptor.cell === 'C4');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'cast-magic'
    && descriptor.cardId === 'sparkmage-many-magic');

  const activation = action(session, ({ descriptor }) =>
    descriptor.kind === 'activate-sparkmage'
      && descriptor.targetLocation.cell === 'C4'
      && descriptor.targetLocation.region === 'surface');
  for (const target of targets) {
    assert.doesNotMatch(canonicalJson(activation.descriptor), new RegExp(target.instanceId));
  }
  const result = stepGame(session, activation);
  assert.equal(result.accepted, true);
  if (!result.accepted) return;
  session = result.session;

  assert.ok(result.receipt.randomDraws.length >= 1);
  assert.ok(result.receipt.randomDraws.every(({ purpose }) =>
    purpose === 'sparkmage_random_other_unit_at_nearby_location'));
  const activated = result.receipt.events.find(({ type }) => type === 'sparkmage-activated');
  const damaged = result.receipt.events.filter(({ type }) => type === 'damage-dealt');
  assert.ok(activated);
  assert.equal(damaged.length, 1);
  const activatedJson = canonicalJson(activated.payload);
  const damagedJson = canonicalJson(damaged[0]!.payload);
  const selected = targets.find(({ instanceId }) => activatedJson.includes(instanceId));
  assert.ok(selected);
  assert.match(damagedJson, new RegExp(selected.instanceId));
  assert.equal(session.state.realm.units.filter(({ damage }) => damage === 1).length, 1);
  assert.equal(session.state.players.north.airThresholdsCastThisTurn, 1);
  assert.equal(verifyGameReplay(session), true);
});
