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
  gainsStealthAtEndOfTurn?: boolean;
  genesisDrawSpell?: boolean;
  genesisDrawSite?: boolean;
  lethal?: boolean;
  manaCost: number;
  movementBonus?: 1 | 2;
  movesOnlyForward?: boolean;
  movesOnlySideways?: boolean;
  mustBeCastBurrowed?: boolean;
  mustBeCastSubmerged?: boolean;
  provides?: 'air' | 'earth' | 'fire' | 'water';
  ranged?: boolean;
  stealth?: boolean;
  strikesFirstWhileAttacking?: boolean;
  submerge?: boolean;
  summonToAnySite?: boolean;
  mustBeCastToOuterColumn?: boolean;
  tapForMana?: number;
  thresholds: Readonly<{ air: number; earth: number; fire: number; water: number }>;
  voidwalk?: boolean;
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
  genesisDrawSpellPerAdjacentSameCard?: boolean;
  genesisGainMana?: number;
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
        genesisDrawSpellPerAdjacentSameCard:
          site.genesisDrawSpellPerAdjacentSameCard ?? false,
        ...(site.genesisGainMana ? { genesisGainMana: site.genesisGainMana } : {}),
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
        gainsStealthAtEndOfTurn: facts.gainsStealthAtEndOfTurn ?? false,
        genesisDrawSpell: facts.genesisDrawSpell ?? false,
        genesisDrawSite: facts.genesisDrawSite ?? false,
        lethal: facts.lethal ?? false,
        manaCost: facts.manaCost,
        ...(facts.movementBonus ? { movementBonus: facts.movementBonus } : {}),
        movesOnlyForward: facts.movesOnlyForward ?? false,
        movesOnlySideways: facts.movesOnlySideways ?? false,
        mustBeCastBurrowed: facts.mustBeCastBurrowed ?? false,
        mustBeCastSubmerged: facts.mustBeCastSubmerged ?? false,
        ...(facts.provides ? { provides: facts.provides } : {}),
        ranged: facts.ranged ?? false,
        stealth: facts.stealth ?? false,
        strikesFirstWhileAttacking: facts.strikesFirstWhileAttacking ?? false,
        submerge: facts.submerge ?? false,
        summonToAnySite: facts.summonToAnySite ?? false,
        mustBeCastToOuterColumn: facts.mustBeCastToOuterColumn ?? false,
        ...(facts.tapForMana ? { tapForMana: facts.tapForMana } : {}),
        thresholds: { ...facts.thresholds },
        voidwalk: facts.voidwalk ?? false,
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
  }), /unsupported/);
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
  assert.throws(() => createGameManifest({
    ...input,
    cards: {
      ...cards,
      [firstSpell]: { ...cards[firstSpell]!, connectsTopBottom: 'yes' } as unknown as GameCardDefinition,
    },
  }), /connectsTopBottom/);
  const firstSite = decks.north.atlas[0];
  assert.ok(firstSite);
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
  assert.equal(deathDoor.transcript.at(-1)?.events.some(({ payload, type }) =>
    type === 'avatar-healed'
      && canonicalJson(payload).includes('"amount":0')
      && canonicalJson(payload).includes('"attemptedAmount":3')
      && canonicalJson(payload).includes('"seat":"south"')), true);
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
