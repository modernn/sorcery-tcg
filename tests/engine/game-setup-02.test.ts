import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpoint,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import { opaqueActionId, type EngineActionDescriptor } from '../../src/engine/contract.ts';
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
  takeAction,
  type SpellFacts,
} from './game-setup-helpers.ts';
import { withSetup } from './rust-setup-session.ts';

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
  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    // Seed search peeks opening hands via TS createGameSession (cheap); play path uses SetupCtx.
    const opening = createGameSession(candidate).state.players;
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
    if (fragileIds.every((cardId) => northOpening.has(cardId))
      && northBySecondTurn.has(sourceId)
      && [leapId, rainId].every((cardId) => northByThirdTurn.has(cardId))
      && southBySecondTurn.has(enemyId)) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    for (const fragileId of fragileIds) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === fragileId
        && descriptor.cell === 'C4');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === sourceId
      && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === enemyId
      && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic' && descriptor.cardId === rainId);
    const fragiles = ctx.state.realm.units.filter(({ cardId }) => fragileIds.includes(cardId));
    assert.equal(fragiles.length, 2);
    assert.equal(fragiles.every(({ damage }) => damage === 1), true);
    const source = ctx.state.realm.units.find(({ cardId }) => cardId === sourceId);
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === enemyId);
    assert.ok(source && enemy);
    assert.equal(source.location, 'C3');
    assert.deepEqual(fragiles.map(({ instanceId }) => observeGame(ctx.state, 'north').realm.units
      .find((unit) => unit.instanceId === instanceId)?.defense), [2, 2]);
    const atlasBefore = ctx.state.players.north.atlas.length;
    const atlasHandBefore = ctx.state.players.north.hand.atlas.length;
    const cast = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardId === leapId
        && descriptor.ally?.instanceId === source.instanceId
        && descriptor.allyDestination?.cell === 'C2');
    const interrupted = await ctx.step(cast);
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
    const interruptedCheckpoint = createGameCheckpoint(ctx.session);
    const restored = resumeGameCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
      interruptedCheckpoint,
    )));
    assert.equal(
      canonicalJson(restored as unknown as JsonValue),
      canonicalJson(ctx.session as unknown as JsonValue),
    );
    await ctx.resume(interruptedCheckpoint);
    const orderActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(),
    fragiles.map(({ instanceId }) => instanceId).sort());
    const branchPoint = createGameCheckpoint(ctx.session);
    const branchHashes: string[] = [];
    for (const orderAction of orderActions) {
      assert.equal(orderAction.descriptor.kind, 'order-deathrites');
      if (orderAction.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
      const chosenInstanceId = orderAction.descriptor.sourceInstanceId;
      const otherInstanceId = fragiles.find(({ instanceId }) =>
        instanceId !== chosenInstanceId)?.instanceId;
      assert.ok(otherInstanceId);
      await ctx.resume(branchPoint);
      const ordered = await ctx.step(orderAction);
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
      assert.equal(ctx.state.phase, 'main');
      assert.equal(ctx.state.pendingDeathrites, undefined);
      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === source.instanceId)?.location, 'C2');
      assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
        instanceId === enemy.instanceId), false);
      assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
        instanceId === enemy.instanceId), true);
      assert.equal(fragiles.every(({ instanceId }) => ctx.state.players.north.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      assert.equal(ctx.state.players.north.atlas.length, atlasBefore - 2);
      assert.equal(ctx.state.players.north.hand.atlas.length, atlasHandBefore + 2);
      assert.equal(await ctx.verifyReplay(), true);
      branchHashes.push(hashGameState(ctx.state));
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
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'summon-minion' && (descriptor.region ?? 'surface') === region);
      const target = ctx.state.realm.units.find(({ controller }) => controller === 'south');
      assert.ok(target);
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      assert.equal(await ctx.verifyReplay(), true);
      if (targetNearby) {
        assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
          descriptor.kind === 'cast-magic'
            && descriptor.target !== undefined
            && descriptor.target.kind === 'avatar'
            && descriptor.target.seat === 'north'), true);
      }
      legal = (await ctx.legalActions('north')).some(({ descriptor }) =>
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const nearbyTarget = ctx.state.realm.units.find(({ controller, location }) =>
      controller === 'south' && location === 'C4');
    const distantTarget = ctx.state.realm.units.find(({ controller, location }) =>
      controller === 'south' && location === 'C1');
    assert.ok(nearbyTarget);
    assert.ok(distantTarget);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'activate-mana'
      && descriptor.unitInstanceId === nearbyTarget.instanceId);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === nearbyTarget.instanceId)?.tapped, true);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const spell = ctx.state.players.north.hand.spellbook.find(({ cardId }) => {
      const definition = gameManifest.cards[cardId];
      return definition?.cardType === 'magic' && definition.untapTargetMinionAfterDamage === true;
    });
    assert.ok(spell);
    const lashActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
    assert.deepEqual(lashActions.map(({ descriptor }) =>
      descriptor.kind === 'cast-magic' ? descriptor.target?.instanceId : undefined), [nearbyTarget.instanceId]);
    assert.equal(lashActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.kind === 'avatar'), false);
    assert.equal(lashActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId === distantTarget.instanceId), false);

    const before = ctx.state;
    const result = await ctx.step(lashActions[0]!);
    assert.equal(result.accepted, true);
    const targetAfter = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === nearbyTarget.instanceId);
    assert.equal(targetAfter?.damage, 1);
    assert.equal(targetAfter?.tapped, false);
    assert.equal(ctx.state.players.north.mana, before.players.north.mana - 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === spell.instanceId), true);
    assert.equal(ctx.state.stateVersion, before.stateVersion + 1);
    assert.deepEqual(result.accepted ? result.receipt.events.map(({ type }) => type) : [], [
      'magic-cast',
      'magic-damage-allocated',
      'damage-dealt',
      'minion-untapped',
      'magic-resolved',
    ]);
    assert.deepEqual(result.accepted ? result.receipt.events[3]?.payload : undefined, {
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
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === allyCardId && descriptor.cell === 'C4');
    const ally = ctx.state.realm.units.find(({ cardId }) => cardId === allyCardId);
    assert.ok(ally);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetCardId && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === stealthCardId && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === wardCardId && descriptor.cell === 'C2');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetCardId);
    const stealthed = ctx.state.realm.units.find(({ cardId }) => cardId === stealthCardId);
    const wardedTarget = ctx.state.realm.units.find(({ cardId }) => cardId === wardCardId);
    assert.ok(target);
    assert.ok(stealthed);
    assert.ok(wardedTarget);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.from.cell === 'C4'
      && descriptor.to.cell === 'C3'
      && descriptor.path.length === 2);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    const checkpoint = createGameCheckpoint(ctx.session);
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

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    const unfrozenKinds = (await ctx.legalActions('south')).flatMap(({ descriptor }) =>
      'unitInstanceId' in descriptor && descriptor.unitInstanceId === target.instanceId
        ? [descriptor.kind]
        : 'shooterInstanceId' in descriptor && descriptor.shooterInstanceId === target.instanceId
          ? [descriptor.kind]
          : []);
    assert.equal(unfrozenKinds.includes('move-and-attack'), true);
    assert.equal(unfrozenKinds.includes('shoot-projectile'), true);
    assert.equal(unfrozenKinds.includes('activate-mana'), true);

    await ctx.resume(checkpoint);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === freezeCards[0]?.instanceId
        && descriptor.target?.instanceId === ally.instanceId);
    const disabledAlly = ctx.state.realm.units.find(({ instanceId }) => instanceId === ally.instanceId);
    assert.equal(observeGame(ctx.state, 'north').realm.units.find(({ instanceId }) =>
      instanceId === ally.instanceId)?.disabled, true);
    assert.deepEqual({ stealthed: disabledAlly?.stealthed, warded: disabledAlly?.warded }, {
      stealthed: false,
      warded: false,
    });
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'minion-disabled',
      'magic-resolved',
    ]);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events[1]?.payload, {
      expiresAtSeat: 'north',
      instanceId: ally.instanceId,
      seat: 'north',
      sourceInstanceId: freezeCards[0]!.instanceId,
      stealthRemoved: true,
      wardRemoved: true,
    });
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
    const warded = await ctx.step(await ctx.action(({ descriptor }) =>
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
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === wardedTarget.instanceId)?.disableEffects, undefined);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
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
    assert.equal(observeGame(ctx.state, 'south').realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.disabled, true);
    assert.equal(observeGame(ctx.state, 'south').players.south.affinity.air, 0);
    assert.deepEqual(freeze.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-disabled',
      'magic-resolved',
    ]);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) =>
      type === 'minion-disable-expired'), false);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    assert.equal(observeGame(ctx.state, 'south').realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.disabled, true);
    const disabledKinds = (await ctx.legalActions('south')).flatMap(({ descriptor }) =>
      'unitInstanceId' in descriptor && descriptor.unitInstanceId === target.instanceId
        ? [descriptor.kind]
        : 'shooterInstanceId' in descriptor && descriptor.shooterInstanceId === target.instanceId
          ? [descriptor.kind]
          : []);
    assert.deepEqual(disabledKinds, []);
    const expiration = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
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
    assert.equal(observeGame(ctx.state, 'north').realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.disabled, false);
    assert.equal(observeGame(ctx.state, 'north').players.south.affinity.air, 1);
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const checkpoint = createGameCheckpoint(ctx.session);

    const freezeAt = async (region: 'underwater' | 'void', cell: 'C4' | 'B4') => {
      await ctx.resume(checkpoint);
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === casterCardId && descriptor.cell === cell
        && descriptor.region === region);
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === targetCardId && descriptor.cell === cell
        && descriptor.region === region);
      const caster = ctx.state.realm.units.find(({ cardId }) => cardId === casterCardId);
      const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetCardId);
      const freeze = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === freezeCardId);
      assert.ok(caster && target && freeze);
      const result = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === freeze.instanceId
          && descriptor.casterInstanceId === caster.instanceId
          && descriptor.target?.instanceId === target.instanceId));
      assert.equal(result.accepted, true);
      if (!result.accepted) throw new Error('expected Freeze to resolve');
      return { caster, freeze, result, target };
    };

    const underwater = await freezeAt('underwater', 'C4');
    assert.deepEqual(underwater.result.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-disabled',
      'minion-died',
      'magic-resolved',
    ]);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === underwater.target.instanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId, region }) =>
      instanceId === underwater.caster.instanceId && region === 'underwater'), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === underwater.target.instanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === underwater.freeze.instanceId), true);
    assert.equal(underwater.result.receipt.randomDraws.length, 0);
    assert.equal(await ctx.verifyReplay(), true);

    const voided = await freezeAt('void', 'B4');
    assert.deepEqual(voided.result.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'minion-disabled',
      'minion-banished',
      'magic-resolved',
    ]);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === voided.target.instanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId, region }) =>
      instanceId === voided.caster.instanceId && region === 'void'), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === voided.target.instanceId), false);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === voided.target.instanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === voided.freeze.instanceId), true);
    assert.equal(voided.result.receipt.randomDraws.length, 0);
    assert.equal(await ctx.verifyReplay(), true);
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    for (let count = 0; count < 2; count += 1) {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
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

    const before = ctx.state;
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
    assert.equal(ctx.state.stateVersion, before.stateVersion + 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === bolt.instanceId), true);
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

  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 100; seed += 1) {
    const candidate = createGameManifest({
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
    // Seed search peeks opening hands via TS createGameSession (cheap); play path uses SetupCtx.
    if (!createGameSession(candidate).state.players.north.hand.spellbook.some(({ cardId }) =>
      cardId === luckyCharmId)) continue;
    gameManifest = candidate;
    let foundPair = false;
    await withSetup(candidate, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'cast-artifact'
          && descriptor.cardId === luckyCharmId
          && descriptor.bearer?.kind === 'avatar');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      for (let count = 0; count < 2; count += 1) {
        await takeAction(ctx, ({ descriptor }) =>
          descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
      }
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const bolt = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId !== luckyCharmId);
      if (!bolt) return;
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === bolt.instanceId
          && descriptor.targetLocation?.cell === 'C1');
      if (casts.length !== 1) return;
      assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
        descriptor.kind === 'resolve-random-outcome'), false);
      const committed = await ctx.step(casts[0]!);
      if (!committed.accepted) return;
      const choices = (await ctx.legalActions('north')).filter(
        ({ descriptor }) => descriptor.kind === 'resolve-random-outcome',
      );
      if (choices.length !== 2) return;
      foundPair = true;

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
    });
    if (foundPair) break;
    gameManifest = undefined;
  }
  assert.ok(gameManifest);
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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
  cards[stealthCardId] = {
    ...baseCards[stealthCardId]!,
    deathriteDrawSite: true,
    stealth: true,
  } as GameCardDefinition;
  const gameManifest = createGameManifest({ authority, cards, decks, firstSeat: 'north', seed });

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    for (const cardId of [deathriteCardId, wardedCardId, stealthCardId]) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === cardId && descriptor.cell === 'C2');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.to.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === allyCardId && descriptor.cell === 'C2');

    const checkpointVersion = ctx.state.stateVersion;
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
    assert.equal(ctx.state.stateVersion, checkpointVersion + 1);
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
    assert.equal(ctx.state.stateVersion, checkpointVersion + 2);
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === firstTargetCard.cardId && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === secondTargetCard.cardId && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'A1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B4');

    const checkpointTranscriptLength = ctx.session.transcript.length;
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

    // Forged-state mana probe still uses TS legalGameActions (Geomancer pattern).
    const lowMana: GameSession = {
      ...ctx.session,
      state: {
        ...ctx.state,
        players: {
          ...ctx.state.players,
          north: { ...ctx.state.players.north, mana: 1 },
        },
      },
    };
    assert.equal(legalGameActions(lowMana.state, 'north').some(({ descriptor }) =>
      descriptor.kind === 'begin-chain-magic'
        && descriptor.cardInstanceId === chain.instanceId), false);

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

    const extensions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'extend-chain-magic');
    assert.deepEqual(extensions.flatMap(({ descriptor }) => descriptor.kind === 'extend-chain-magic'
      ? [descriptor.target.instanceId]
      : []).sort(), [avatarId, secondTarget.instanceId].sort());
    assert.equal(extensions.some(({ descriptor }) => descriptor.kind === 'extend-chain-magic'
      && descriptor.target.instanceId === firstTarget.instanceId), false);
    assert.equal(extensions.some(({ descriptor }) => descriptor.kind === 'extend-chain-magic'
      && descriptor.target.instanceId === southAvatarId), false);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor, label }) =>
      descriptor.kind === 'resolve-chain-magic' && /1 chosen unit \(2 mana\)/.test(label)), true);

    // Forged-state region/stealth probes still use TS legalGameActions.
    const undergroundState: GameSession['state'] = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        units: ctx.state.realm.units.map((unit) => unit.instanceId === secondTarget.instanceId
          ? { ...unit, region: 'underground' as const }
          : unit),
      },
    };
    assert.equal(legalGameActions(undergroundState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'extend-chain-magic'
        && descriptor.target.instanceId === secondTarget.instanceId), false);

    const stealthState: GameSession['state'] = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        units: ctx.state.realm.units.map((unit) => unit.instanceId === secondTarget.instanceId
          ? { ...unit, controller: 'south' as const, stealthed: true }
          : unit),
      },
    };
    assert.equal(legalGameActions(stealthState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'extend-chain-magic'
        && descriptor.target.instanceId === secondTarget.instanceId), false);

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
    assert.equal(ctx.session.transcript.length, checkpointTranscriptLength + 3);
    assert.equal(await ctx.verifyReplay(), true);
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === deathriteCardId && descriptor.cell === 'C4' && !descriptor.region);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === burrowedCardId && descriptor.cell === 'C4'
      && descriptor.region === 'underground');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === stealthedCardId && descriptor.cell === 'C1' && !descriptor.region);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === wardedCardId && descriptor.cell === 'C1' && !descriptor.region);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === submergedCardId && descriptor.cell === 'C1'
      && descriptor.region === 'underwater');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === voidCardId && descriptor.cell === 'B4' && descriptor.region === 'void');

    const checkpointVersion = ctx.state.stateVersion;
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
    assert.equal(ctx.state.stateVersion, checkpointVersion + 1);
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === printedChargeCardId
      && descriptor.cell === 'C4'
      && descriptor.region === 'underground');
    const printedCharge = ctx.state.realm.units.find(({ cardId }) => cardId === printedChargeCardId);
    assert.ok(printedCharge);
    assert.equal(printedCharge.summoningSickness, true);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === printedCharge.instanceId), true);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === enemyCardId && descriptor.cell === 'C1');
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === enemyCardId);
    assert.ok(enemy);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === summonedCardId && descriptor.cell === 'C4');

    const checkpoint = createGameCheckpoint(ctx.session);
    const checkpointVersion = ctx.state.stateVersion;
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

    const avatarGrant = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === chargeCards[0]?.instanceId
        && descriptor.ally?.kind === 'avatar'));
    assert.equal(avatarGrant.accepted, true);
    if (!avatarGrant.accepted) return;
    assert.deepEqual(avatarGrant.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'charge-granted',
      'magic-resolved',
    ]);
    assert.equal(ctx.state.realm.units.every(({ temporaryChargeSources }) =>
      temporaryChargeSources === undefined), true);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
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
    assert.equal(ctx.state.stateVersion, checkpointVersion + 2);
    assert.equal(first.receipt.randomDraws.length + second.receipt.randomDraws.length, 0);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === summoned.instanceId
      && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === fighterCardId && descriptor.cell === 'C4' && !descriptor.region);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === hiddenCardId && descriptor.cell === 'C4'
      && descriptor.region === 'underground');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === enemyCardId && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === disabledCardId && descriptor.cell === 'C4');

    const checkpoint = createGameCheckpoint(ctx.session);
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

    const avatarGrant = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === overpower.instanceId
        && descriptor.ally?.kind === 'avatar'));
    assert.equal(avatarGrant.accepted, true);
    if (!avatarGrant.accepted) return;
    assert.deepEqual({
      attack: observeGame(ctx.state, 'north').players.north.avatar.attack,
      defense: observeGame(ctx.state, 'north').players.north.avatar.defense,
    }, { attack: 3, defense: 3 });
    assert.deepEqual(avatarGrant.receipt.events.slice(1, 2).map(({ payload, type }) => ({
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
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
    const disabledGrant = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === overpower.instanceId
        && descriptor.ally?.instanceId === disabled.instanceId));
    assert.equal(disabledGrant.accepted, true);
    if (!disabledGrant.accepted) return;
    const disabledView = observeGame(ctx.state, 'north').realm.units.find(({ instanceId }) =>
      instanceId === disabled.instanceId);
    assert.deepEqual({
      attack: disabledView?.attack,
      defense: disabledView?.defense,
      disabled: disabledView?.disabled,
    }, { attack: 3, defense: 3, disabled: true });
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
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
    const poweredView = observeGame(ctx.state, 'north').realm.units.find(({ instanceId }) =>
      instanceId === fighter.instanceId);
    assert.deepEqual({ attack: poweredView?.attack, defense: poweredView?.defense }, {
      attack: 4,
      defense: 4,
    });
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === fighter.instanceId && descriptor.path.length === 1);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'declare-attack'
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
    const expired = observeGame(ctx.state, 'north').realm.units.find(({ instanceId }) =>
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    for (const card of [allyCard, sourceCard, disabledSourceCard]) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
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
    const view = observeGame(ctx.state, 'north');
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

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    for (const card of [northMortalCard, northKingACard, northKingBCard]) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
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
    const northOpening = observeGame(ctx.state, 'north');
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

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    for (const card of [southMortalCard, southNonMortalCard]) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === card.cardId
        && descriptor.cell === 'C1');
    }
    const southMortal = ctx.state.realm.units.find(({ cardId }) =>
      cardId === southMortalCard.cardId);
    const southNonMortal = ctx.state.realm.units.find(({ cardId }) =>
      cardId === southNonMortalCard.cardId);
    assert.ok(southMortal);
    assert.ok(southNonMortal);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northKingA.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    const separated = observeGame(ctx.state, 'north').realm.units.find(({ instanceId }) =>
      instanceId === northMortal.instanceId);
    assert.deepEqual([separated?.attack, separated?.defense], [3, 3]);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === southMortal.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic'
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

    const finalView = observeGame(ctx.state, 'north');
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
      && opening.north.spellbook[0]?.cardId === 'defend-fragile'
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
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cardId === 'defend-fragile'
    && descriptor.cell === 'C4');
  const fragiles = session.state.realm.units.filter(({ cardId }) => cardId === 'defend-fragile');
  assert.equal(fragiles.length, 2);
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
  assert.equal(session.state.phase, 'deathrite-order');
  assert.equal(session.state.decisionSeat, 'north');
  assert.deepEqual(defended.receipt.events.map(({ type }) => type), ['basic-movement-started']);
  assert.equal(fragiles.every(({ instanceId }) => !session.state.realm.units
    .some((unit) => unit.instanceId === instanceId)), true);
  assert.equal(fragiles.every(({ instanceId }) => !session.state.players.north.cemetery
    .some((card) => card.instanceId === instanceId)), true);
  const orderActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'order-deathrites');
  assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
    descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(),
  fragiles.map(({ instanceId }) => instanceId).sort());

  session = resumeGameCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
    createGameCheckpoint(session),
  )));
  const ordered = stepGame(session, orderActions[0]!);
  assert.equal(ordered.accepted, true);
  if (!ordered.accepted) return;
  session = ordered.session;
  assert.equal(session.state.phase, 'movement');
  assert.equal(ordered.receipt.events.filter(({ type }) => type === 'minion-died').length, 2);
  assert.equal(fragiles.every(({ instanceId }) => session.state.players.north.cemetery
    .some((card) => card.instanceId === instanceId)), true);

  const events = [...defended.receipt.events, ...ordered.receipt.events];
  while (session.state.phase === 'movement') {
    const continued = stepGame(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'continue-basic-movement'));
    assert.equal(continued.accepted, true);
    if (!continued.accepted) return;
    session = continued.session;
    events.push(...continued.receipt.events);
  }
  assert.equal(session.state.phase, 'defend');
  assert.deepEqual(session.state.pendingCombat?.defenders.map(({ instanceId }) => instanceId), [
    source.instanceId,
  ]);
  assert.equal(events.filter(({ type }) => type === 'basic-movement-started').length, 1);
  assert.equal(events.filter(({ type }) => type === 'basic-movement-continued').length, 2);
  assert.equal(events.filter(({ type }) => type === 'defender-joined').length, 1);
  assert.ok(events.findIndex(({ type }) => type === 'defender-joined')
    > events.findLastIndex(({ type }) => type === 'minion-died'));
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
