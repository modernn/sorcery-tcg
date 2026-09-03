import assert from 'node:assert/strict';
import { after } from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import { RustSessionClient } from '../../src/engine/rust-engine.ts';
import { parseExportedSession } from '../../src/engine/rust-session-helpers.ts';

import {
  createGameManifest,
  createGameSession,
  legalGameActions,
  stepGame,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
} from '../../src/engine/game.ts';
import { SetupCtx, withSetup } from './rust-setup-session.ts';

export const SYNTHETIC_AUTHORITY_HASH =
  'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const;

export function deck(prefix: string, atlasCount = 30, spellbookCount = 50): GameDeckSpec {
  return {
    atlas: Array.from({ length: atlasCount }, (_, index) => `${prefix}-site-${index + 1}`),
    avatar: `${prefix}-avatar`,
    spellbook: Array.from({ length: spellbookCount }, (_, index) => `${prefix}-spell-${index + 1}`),
  };
}

export type SpellFacts = Readonly<{
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

export type AvatarFacts = Readonly<{
  attack: number;
  defense: number;
  drawSpell: boolean;
  earthSitePlayCreatesAdjacentRubble?: true;
  life: number;
  replaceAdjacentRubbleWithTopAtlasSite?: true;
  tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn?: true;
}>;

export type SiteFacts = Readonly<{
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

export function cardsFor(
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

export function manifest(
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

export function action(
  session: GameSession,
  predicate: (candidate: GameLegalAction) => boolean,
): GameLegalAction {
  const found = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  assert.ok(found, 'expected legal action');
  return found;
}

export function accept(session: GameSession, candidate: GameLegalAction): GameSession {
  const result = stepGame(session, candidate);
  assert.equal(result.accepted, true);
  return result.session;
}

export function keep(session: GameSession): GameSession {
  return accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
}

export function northSecondMain(seed = 23, shortDecks = false, spell?: SpellFacts): GameSession {
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

/** Advances one Rust setup session through both opening turns into north's second main. */
export async function toNorthSecondMain(ctx: SetupCtx): Promise<void> {
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
}

export function numericGenesisSession(remainingCount: number, seed: number): GameSession {
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
  let session = keep(createGameSession(createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: `synthetic-genesis-spells-${remainingCount}-v1`,
    },
    cards,
    decks,
    firstSeat: 'north',
    seed,
  })));
  session = keep(session);
  return accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
}

export function northAttacksAtC2(
  seed: number,
  spell?: SpellFacts,
  avatar?: AvatarFacts,
  emptyAtlasAfterOpening = false,
  southSpell?: SpellFacts,
  extraSouthMinionsAtC1 = 0,
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
  for (let index = 0; index < extraSouthMinionsAtC1; index += 1) {
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  }
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

export function northAvatarAttacksSouthAtC2(seed: number): Readonly<{
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

export function devilsEggManifest(kind: 'both' | 'carried' | 'regions', seed: number) {
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
  return { gameManifest, ids };
}

export function devilsEggFixture(kind: 'both' | 'carried' | 'regions', seed: number) {
  const { gameManifest, ids } = devilsEggManifest(kind, seed);
  let checkpoint = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    checkpoint = accept(checkpoint, action(checkpoint, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  if (kind === 'carried') {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === ids.carrier && descriptor.cell === 'C4');
    const carrier = checkpoint.state.realm.units.find(({ cardId }) => cardId === ids.carrier);
    assert.ok(carrier);
    take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === ids.northEgg
      && descriptor.bearer?.instanceId === carrier.instanceId);
  } else {
    for (let index = 0; index < (kind === 'regions' ? 3 : 1); index += 1) {
      take(({ descriptor }) => descriptor.kind === 'cast-artifact'
        && descriptor.cardId === ids.northEgg && descriptor.cell === 'C4');
    }
  }
  return { checkpoint, gameManifest, ids };
}

// ---------------------------------------------------------------------------
// Rust-backed twins of the legacy TypeScript fixtures above. Migrated proofs use
// these; the sync helpers above are deleted once no proof calls them.
// ---------------------------------------------------------------------------

/** Finds one legal action on a Rust setup session and requires acceptance. */
export async function takeAction(
  ctx: SetupCtx,
  predicate: (candidate: GameLegalAction) => boolean,
): Promise<void> {
  await ctx.accept(await ctx.action(predicate));
}

/** Runs one callback in north's second main phase on a Rust setup session. */
export async function withNorthSecondMain(
  options: Readonly<{ seed?: number; shortDecks?: boolean; spell?: SpellFacts }>,
  run: (ctx: SetupCtx) => Promise<void>,
): Promise<void> {
  const { seed = 23, shortDecks = false, spell } = options;
  const manifestOptions = shortDecks
    ? { north: deck('north', 3, 4), south: deck('south', 3, 4), ...(spell ? { spell } : {}) }
    : spell ? { spell } : {};
  await withSetup(manifest(seed, manifestOptions), async (ctx) => {
    await toNorthSecondMain(ctx);
    await run(ctx);
  });
}

/** Runs one callback after north plays its first site with a numeric Genesis draw deck. */
export async function withNumericGenesisSession(
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await run(ctx);
  });
}

export type NorthAttacksAtC2Ids = Readonly<{
  attackerInstanceId: string;
  defenderInstanceId: string;
  targetInstanceId: string;
}>;

/** Runs one callback once north's minion has declared its attack into C2 on a Rust session. */
export async function withNorthAttacksAtC2(
  options: Readonly<{
    seed: number;
    spell?: SpellFacts;
    avatar?: AvatarFacts;
    emptyAtlasAfterOpening?: boolean;
    southSpell?: SpellFacts;
    extraSouthMinionsAtC1?: number;
  }>,
  run: (ctx: SetupCtx, ids: NorthAttacksAtC2Ids) => Promise<void>,
): Promise<void> {
  const { seed, spell, avatar, emptyAtlasAfterOpening = false, southSpell, extraSouthMinionsAtC1 = 0 } = options;
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const attackerInstanceId = ctx.state.realm.units.find(({ controller }) => controller === 'north')?.instanceId;
    assert.ok(attackerInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const defenderInstanceId = ctx.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(defenderInstanceId);
    for (let index = 0; index < extraSouthMinionsAtC1; index += 1) {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C2');
    const targetInstanceId = ctx.state.realm.units
      .find(({ controller, location }) => controller === 'south' && location === 'C2')?.instanceId;
    assert.ok(targetInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.from.cell === 'C3'
        && descriptor.to.cell === 'C2');
    await run(ctx, { attackerInstanceId, defenderInstanceId, targetInstanceId });
  });
}

export type NorthAvatarAttacksSouthAtC2Ids = Readonly<{
  northAvatarInstanceId: string;
  northMinionInstanceId: string;
  southAvatarInstanceId: string;
}>;

/** Runs one callback once north's Avatar has declared its attack into C2 on a Rust session. */
export async function withNorthAvatarAttacksSouthAtC2(
  seed: number,
  run: (ctx: SetupCtx, ids: NorthAvatarAttacksSouthAtC2Ids) => Promise<void>,
): Promise<void> {
  await withSetup(manifest(seed, {
    avatar: { attack: 2, defense: 1, drawSpell: false, life: 1 },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const northAvatarInstanceId = ctx.state.players.north.avatar.card.instanceId;
    const southAvatarInstanceId = ctx.state.players.south.avatar.card.instanceId;
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const northMinionInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(northMinionInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northMinionInstanceId
        && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northMinionInstanceId
        && descriptor.to.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northAvatarInstanceId
        && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === southAvatarInstanceId
        && descriptor.to.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northAvatarInstanceId
        && descriptor.to.cell === 'C2');
    await run(ctx, { northAvatarInstanceId, northMinionInstanceId, southAvatarInstanceId });
  });
}

/** Runs one callback once the Devil's Egg fixture has cast its Artifacts on a Rust session. */
export async function withDevilsEggFixture(
  kind: 'both' | 'carried' | 'regions',
  seed: number,
  run: (ctx: SetupCtx, fixture: ReturnType<typeof devilsEggManifest>) => Promise<void>,
): Promise<void> {
  const fixture = devilsEggManifest(kind, seed);
  const { ids } = fixture;
  await withSetup(fixture.gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    if (kind === 'carried') {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === ids.carrier && descriptor.cell === 'C4');
      const carrier = ctx.state.realm.units.find(({ cardId }) => cardId === ids.carrier);
      assert.ok(carrier);
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
        && descriptor.cardId === ids.northEgg
        && descriptor.bearer?.instanceId === carrier.instanceId);
    } else {
      for (let index = 0; index < (kind === 'regions' ? 3 : 1); index += 1) {
        await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
          && descriptor.cardId === ids.northEgg && descriptor.cell === 'C4');
      }
    }
    await run(ctx, fixture);
  });
}

let sharedPeekClient: Promise<RustSessionClient> | null = null;

after(async () => {
  if (sharedPeekClient) await (await sharedPeekClient).close();
});

/**
 * Returns the deterministic opening session for one manifest from the Rust engine.
 *
 * Seed searches call this thousands of times, so one `session-json` process is shared and
 * re-pointed at each manifest instead of spawning a process per peek.
 */
export async function peekOpening(manifest: GameManifest): Promise<GameSession> {
  sharedPeekClient ??= RustSessionClient.start();
  const client = await sharedPeekClient;
  await client.newSession(canonicalJson(manifest as unknown as JsonValue));
  return parseExportedSession(await client.exportSession(), manifest);
}
