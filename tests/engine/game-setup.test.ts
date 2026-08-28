import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
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
  deathriteDrawSite?: boolean;
  deathriteHeal?: number;
  defense?: number;
  diesAtEndOfControllerTurn?: true;
  gainsStealthAtEndOfTurn?: boolean;
  genesisDrawSpell?: boolean;
  genesisDrawSite?: boolean;
  genesisLoseControllerLife?: 2;
  immobile?: boolean;
  lethal?: boolean;
  manaCost: number;
  movementBonus?: 1 | 2;
  movesOnlyForward?: boolean;
  movesOnlySideways?: boolean;
  mustBeCastBurrowed?: boolean;
  mustBeCastSubmerged?: boolean;
  mustBeCastToWaterSite?: boolean;
  provides?: 'air' | 'earth' | 'fire' | 'water';
  ranged?: boolean;
  shootsDragProjectile?: boolean;
  stealth?: boolean;
  strikesFirstWhileAttacking?: boolean;
  submerge?: boolean;
  summonToAnySite?: boolean;
  mustBeCastToOuterColumn?: boolean;
  tapForMana?: number;
  thresholds: Readonly<{ air: number; earth: number; fire: number; water: number }>;
  voidwalk?: boolean;
  waterbound?: boolean;
  ward?: boolean;
}>;

type AvatarFacts = Readonly<{
  attack: number;
  defense: number;
  drawSpell: boolean;
  life: number;
}>;

type SiteFacts = Readonly<{
  connectsBurrowedAllies?: boolean;
  elements?: readonly ('air' | 'earth' | 'fire' | 'water')[];
  genesisDiscardTopSpells?: 2;
  genesisDrawSpellPerAdjacentSameCard?: boolean;
  genesisGainMana?: number;
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
      life: avatar.life,
    };
    playerDeck.atlas.forEach((cardId) => {
      cards[cardId] = {
        cardType: 'site',
        connectsBurrowedAllies: site.connectsBurrowedAllies ?? false,
        elements: site.elements ?? ['earth'],
        ...(site.genesisDiscardTopSpells === 2 ? { genesisDiscardTopSpells: 2 as const } : {}),
        genesisDrawSpellPerAdjacentSameCard:
          site.genesisDrawSpellPerAdjacentSameCard ?? false,
        ...(site.genesisGainMana ? { genesisGainMana: site.genesisGainMana } : {}),
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
        deathriteDrawSite: facts.deathriteDrawSite ?? false,
        ...(facts.deathriteHeal ? { deathriteHeal: facts.deathriteHeal } : {}),
        defense: facts.defense ?? 1,
        ...(facts.diesAtEndOfControllerTurn === true
          ? { diesAtEndOfControllerTurn: true as const }
          : {}),
        gainsStealthAtEndOfTurn: facts.gainsStealthAtEndOfTurn ?? false,
        genesisDrawSpell: facts.genesisDrawSpell ?? false,
        genesisDrawSite: facts.genesisDrawSite ?? false,
        ...(facts.genesisLoseControllerLife === 2 ? { genesisLoseControllerLife: 2 as const } : {}),
        immobile: facts.immobile ?? false,
        lethal: facts.lethal ?? false,
        manaCost: facts.manaCost,
        ...(facts.movementBonus ? { movementBonus: facts.movementBonus } : {}),
        movesOnlyForward: facts.movesOnlyForward ?? false,
        movesOnlySideways: facts.movesOnlySideways ?? false,
        mustBeCastBurrowed: facts.mustBeCastBurrowed ?? false,
        mustBeCastSubmerged: facts.mustBeCastSubmerged ?? false,
        mustBeCastToWaterSite: facts.mustBeCastToWaterSite ?? false,
        ...(facts.provides ? { provides: facts.provides } : {}),
        ranged: facts.ranged ?? false,
        shootsDragProjectile: facts.shootsDragProjectile ?? false,
        stealth: facts.stealth ?? false,
        strikesFirstWhileAttacking: facts.strikesFirstWhileAttacking ?? false,
        submerge: facts.submerge ?? false,
        summonToAnySite: facts.summonToAnySite ?? false,
        mustBeCastToOuterColumn: facts.mustBeCastToOuterColumn ?? false,
        ...(facts.tapForMana ? { tapForMana: facts.tapForMana } : {}),
        thresholds: { ...facts.thresholds },
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
  assert.equal(legalGameActions(session.state, 'north').length, 76);
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
        burrowTargetMinion: true,
        cardType: 'magic',
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    },
  });
  assert.deepEqual(buryManifest.cards[firstSpell], {
    burrowTargetMinion: true,
    cardType: 'magic',
    manaCost: 3,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  });
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        burrowTargetMinion: 'yes',
        cardType: 'magic',
        manaCost: 3,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      } as unknown as GameCardDefinition,
    },
  }), /burrowTargetMinion/);
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: {
        burrowTargetMinion: true,
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
  const targetCardId = southSites[1]?.cardId;
  const drownedCardId = southMinions[0]?.cardId;
  const survivorCardId = southMinions[1]?.cardId;
  assert.ok(sourceCardId);
  assert.ok(targetCardId);
  assert.ok(drownedCardId);
  assert.ok(survivorCardId);
  const cards: Record<string, GameCardDefinition> = { ...base.cards };
  cards[sourceCardId] = {
    ...cards[sourceCardId]!,
    sacrificeToDestroyNearbySite: true,
  } as GameCardDefinition;
  cards[targetCardId] = { cardType: 'site', elements: ['water'] };
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
  const gameManifest = createGameManifest({
    authority: base.authority,
    cards,
    decks: base.decks,
    firstSeat: base.firstSeat,
    seed: base.seed,
  });
  let session = keep(keep(createGameSession(gameManifest)));
  const sourceCard = session.state.players.north.hand.atlas.find(({ cardId }) => cardId === sourceCardId);
  const targetCard = session.state.players.south.hand.atlas.find(({ cardId }) => cardId === targetCardId);
  const drownedCard = session.state.players.south.hand.spellbook.find(({ cardId }) => cardId === drownedCardId);
  const survivorCard = session.state.players.south.hand.spellbook.find(({ cardId }) => cardId === survivorCardId);
  assert.ok(sourceCard);
  assert.ok(targetCard);
  assert.ok(drownedCard);
  assert.ok(survivorCard);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId !== sourceCard.instanceId
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
        burrowTargetMinion: true,
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

test('RULE-04 Ranged strikes without return damage and Ward prevents the first positive damage event', () => {
  const base = manifest(116, {
    spell: {
      attack: 3,
      defense: 3,
      manaCost: 1,
      ranged: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
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
