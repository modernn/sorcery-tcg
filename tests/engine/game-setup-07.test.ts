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
  manifest,
  SYNTHETIC_AUTHORITY_HASH,
  takeAction,
  withNorthAttacksAtC2,
  withNorthAvatarAttacksSouthAtC2,
  type NorthAttacksAtC2Ids,
  type SpellFacts,
} from './game-setup-helpers.ts';
import { SetupCtx, withSetup } from './rust-setup-session.ts';

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

  await withNorthAttacksAtC2({
    seed: 147,
    spell: blimp,
    avatar: { attack: 1, defense: 1, drawSpell: false, life: 2 },
    southSpell: ordinary,
  }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
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
      turnNumber: ctx.state.turnNumber,
    });
    const firstDeath = events.findIndex(({ type }) => type === 'minion-died');
    assert.ok(firstDeath > events.findIndex(({ type }) => type === 'avatar-reached-deaths-door'));
    assert.equal(events.some(({ type }) => type === 'game-ended'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
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
  // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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

  await withSetup(createGameManifest({
    authority,
    cards,
    decks,
    firstSeat: 'north',
    seed,
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
    for (const cardId of [...scarabCardIds, chainedCardId]) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

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
    assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(canonicalJson(forged.session.state), beforeOrder);

    const interruptedCheckpoint = createGameCheckpoint(ctx.session);
    const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
      interruptedCheckpoint,
    )));
    assert.equal(
      canonicalJson(restored as unknown as JsonValue),
      canonicalJson(ctx.session as unknown as JsonValue),
    );
    assert.deepEqual(
      (await ctx.legalActions('south')).map(({ actionId }) => actionId),
      orderActions.map(({ actionId }) => actionId),
    );

    const southAvatarId = ctx.state.players.south.avatar.card.instanceId;
    const northAvatarId = ctx.state.players.north.avatar.card.instanceId;
    const branchPoint = createGameCheckpoint(ctx.session);
    const branchHashes: string[] = [];
    for (const orderedFirst of orderActions) {
      assert.equal(orderedFirst.descriptor.kind, 'order-deathrites');
      if (orderedFirst.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
      const chosenInstanceId = orderedFirst.descriptor.sourceInstanceId;
      const otherInstanceId = scarabInstanceIds.find((instanceId) => instanceId !== chosenInstanceId);
      assert.ok(otherInstanceId);
      await ctx.resume(branchPoint);
      const ordered = await ctx.step(orderedFirst);
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
      assert.equal(ctx.state.players.north.avatar.life, 20);
      assert.equal(ctx.state.players.south.avatar.life, 16);
      assert.equal(ctx.state.realm.units.length, 1);
      assert.deepEqual(ctx.state.realm.units[0] && {
        damage: ctx.state.realm.units[0].damage,
        instanceId: ctx.state.realm.units[0].instanceId,
        location: ctx.state.realm.units[0].location,
      }, { damage: 3, instanceId: sourceInstanceId, location: 'C1' });
      assert.deepEqual(
        ctx.state.players.south.cemetery.map(({ instanceId }) => instanceId).sort(),
        [...scarabInstanceIds, chainedInstanceId].sort(),
      );
      assert.equal(ordered.receipt.randomDraws.length, 0);
      assert.equal(await ctx.verifyReplay(), true);

      const beforeStale = canonicalJson(ctx.state);
      const stale = await ctx.step(orderedFirst);
      assert.equal(stale.accepted, false);
      assert.equal(stale.reason.code, 'stale_version');
      assert.equal(canonicalJson(stale.session.state), beforeStale);
      branchHashes.push(hashGameState(ctx.state));
    }
    assert.equal(new Set(branchHashes).size, 1);
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
    check: (ctx: SetupCtx, ids: NorthAttacksAtC2Ids) => Promise<void>,
  ): Promise<void> => {
    await withNorthAttacksAtC2({ seed, spell: source, southSpell: target }, async (ctx, ids) => {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === ids.targetInstanceId);
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      await check(ctx, ids);
    });
  };

  await resolve(266, deathrite, {
    attack: 2,
    defense: 10,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    ward: true,
  }, async (ctx, warded) => {
    const wardedEvents = ctx.session.transcript.at(-1)?.events ?? [];
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
    const sourceMoves = ctx.session.transcript.flatMap(({ events }) =>
      events.filter(({ payload, type }) =>
        type === 'move-and-attack-activated'
          && canonicalJson(payload).includes(warded.attackerInstanceId)));
    assert.equal(sourceMoves.length, 2);
    assert.equal(canonicalJson(sourceMoves.at(-1)!.payload).includes('"to":{"cell":"C2"'), true);
    const wardedTarget = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === warded.targetInstanceId);
    const distantWarded = ctx.state.realm.units.find(({ instanceId }) =>
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
    assert.equal(await ctx.verifyReplay(), true);
  });

  await resolve(267, { ...deathrite, lethal: true }, {
    attack: 2,
    defense: 10,
    manaCost: 1,
    takesLessDamage: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  }, async (ctx, lethal) => {
    const lethalEvents = ctx.session.transcript.at(-1)?.events ?? [];
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
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === lethal.targetInstanceId), true);
    const distantReduced = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === lethal.defenderInstanceId);
    assert.deepEqual(distantReduced && {
      damage: distantReduced.damage,
      location: distantReduced.location,
    }, { damage: 0, location: 'C1' });
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-05 Deathrite preserves its unit-source power snapshot before cemetery entry', async () => {
  await withNorthAttacksAtC2({
    seed: 171,
    spell: {
      attack: 4,
      deathriteDamageEachUnitHere: 1,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    southSpell: {
      attack: 1,
      defense: 10,
      manaCost: 1,
      preventsDamageFromUnitsWithPowerAtLeast: 4,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId);
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    if (!fought.accepted) return;
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === setup.attackerInstanceId), true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === setup.targetInstanceId)?.damage, 0);
    assert.deepEqual(fought.receipt.events.filter(({ payload, type }) =>
      type === 'damage-dealt'
        && canonicalJson(payload).includes(setup.targetInstanceId)).map(({ payload }) => payload), [{
      accumulated: 0,
      amount: 0,
      attemptedAmount: 4,
      direct: true,
      instanceId: setup.targetInstanceId,
      prevented: true,
      seat: 'south',
    }, {
      accumulated: 0,
      amount: 0,
      attemptedAmount: 1,
      direct: true,
      instanceId: setup.targetInstanceId,
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
    check: (ctx: SetupCtx) => Promise<void>,
  ): Promise<void> => {
    await withNorthAttacksAtC2({
      seed,
      spell: {
        attack: 1,
        deathriteHeal: 3,
        defense: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
      avatar: { attack: 1, defense: 1, drawSpell: false, life },
    }, async (ctx, setup) => {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
      assert.equal(ctx.state.players.south.avatar.life, life - 1);
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.defenderInstanceId
        && descriptor.to.cell === 'C2');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.attackerInstanceId);
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      await check(ctx);
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
  await withNorthAttacksAtC2({ seed: 47 }, async (ctx, setup) => {
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal(ctx.state.decisionSeat, 'north');
    assert.equal(ctx.state.phase, 'attack');
    const attacker = ctx.state.realm.units.find(({ instanceId }) => instanceId === setup.attackerInstanceId);
    assert.deepEqual({
      location: attacker?.location,
      region: attacker?.region,
      tapped: attacker?.tapped,
    }, { location: 'C2', region: 'surface', tapped: true });

    const site = ctx.state.realm.sites.C2;
    assert.ok(site);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === site.instanceId);
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal(ctx.state.decisionSeat, 'south');
    assert.equal(ctx.state.phase, 'defend');

    await takeAction(ctx, ({ descriptor }) =>
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
  await withNorthAttacksAtC2({ seed: 53 }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === setup.defenderInstanceId);
    const movedDefender = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === setup.defenderInstanceId);
    assert.deepEqual({ location: movedDefender?.location, tapped: movedDefender?.tapped }, {
      location: 'C2',
      tapped: true,
    });

    await takeAction(ctx, ({ descriptor }) =>
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
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === setup.attackerInstanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === damagedInstanceId), false);
    assert.equal(ctx.state.realm.units.filter(({ controller }) => controller === 'south').length, 1);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === setup.attackerInstanceId), true);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === damagedInstanceId), true);
    assert.equal(ctx.session.transcript.flatMap(({ events }) => events)
      .filter(({ type }) => type === 'minion-died').length, 2);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 takes-less prevention applies to each simultaneous damage source before Lethal', async () => {
  for (const scenario of [
    { attack: 2, defense: 2, enemyPower: 1, expectedDamage: 0, lethal: true, seed: 142 },
    { attack: 4, defense: 3, enemyPower: 2, expectedDamage: 2, lethal: false, seed: 143 },
  ] as const) {
    await withNorthAttacksAtC2({
      seed: scenario.seed,
      spell: {
        attack: scenario.attack,
        defense: scenario.defense,
        manaCost: 1,
        takesLessDamage: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
      southSpell: {
        attack: scenario.enemyPower,
        defense: scenario.enemyPower,
        lethal: scenario.lethal,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    }, async (ctx, setup) => {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === setup.targetInstanceId);
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'defend'
          && descriptor.unitInstanceId === setup.defenderInstanceId);
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      const allocated = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'allocate-strike' && descriptor.amount === scenario.enemyPower));
      assert.equal(allocated.accepted, true);
      const fought = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'allocate-strike' && descriptor.amount === scenario.enemyPower));
      assert.equal(fought.accepted, true);

      assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === setup.attackerInstanceId)?.damage, scenario.expectedDamage);
      assert.equal(ctx.state.realm.units.filter(({ controller }) => controller === 'south').length, 0);
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
      assert.equal(await ctx.verifyReplay(), true);
    });
  }
});

test('RULE-04 declining an attack gives only co-located ready enemies an Intercept window', async () => {
  await withNorthAttacksAtC2({ seed: 59 }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'intercept');
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal(ctx.state.decisionSeat, 'south');
    const interceptors = (await ctx.legalActions('south'))
      .filter(({ descriptor }) => descriptor.kind === 'intercept');
    assert.deepEqual(
      interceptors.map(({ descriptor }) => descriptor.kind === 'intercept' && descriptor.unitInstanceId),
      [setup.targetInstanceId],
    );
    assert.equal(interceptors.some(({ descriptor }) =>
      descriptor.kind === 'intercept' && descriptor.unitInstanceId === setup.defenderInstanceId), false);

    await ctx.accept(interceptors[0]!);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === setup.attackerInstanceId), false);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === setup.targetInstanceId), false);
    const eventTypes = ctx.session.transcript.flatMap(({ events }) => events.map(({ type }) => type));
    assert.equal(eventTypes.includes('attack-declared'), false);
    assert.equal(eventTypes.includes('interceptor-joined'), true);
    assert.equal(eventTypes.includes('fight-started'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 surviving minion damage persists through the turn and clears in End Phase', async () => {
  await withNorthAttacksAtC2({
    seed: 61,
    spell: {
      defense: 2,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.deepEqual(ctx.state.realm.units
      .filter(({ location }) => location === 'C2')
      .map(({ damage }) => damage), [1, 1]);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units.every(({ damage }) => damage === 0), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test("RULE-04 Death's Door prevents same-turn direct damage and later simultaneous death blows draw", async () => {
  await withNorthAvatarAttacksSouthAtC2(67, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'avatar'
        && descriptor.target.instanceId === setup.southAvatarInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
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
        { amount: 2, direct: true, instanceId: setup.northAvatarInstanceId, seat: 'north' },
        { amount: 2, direct: true, instanceId: setup.southAvatarInstanceId, seat: 'south' },
      ],
    );
    assert.deepEqual(
      firstFightEvents.filter(({ type }) => type === 'avatar-life-lost').map(({ payload }) => payload),
      [
        { amount: 1, life: 0, seat: 'north' },
        { amount: 1, life: 0, seat: 'south' },
      ],
    );

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.northMinionInstanceId
        && descriptor.to.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'avatar'
        && descriptor.target.instanceId === setup.southAvatarInstanceId);
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

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.southAvatarInstanceId
        && descriptor.to.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'avatar'
        && descriptor.target.instanceId === setup.northAvatarInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.deepEqual(ctx.state.terminal, {
      reason: 'simultaneous_avatar_defeat',
      result: 'draw',
      status: 'finished',
    });
    assert.equal(ctx.session.transcript.at(-1)?.events.filter(({ type }) =>
      type === 'death-blow').length, 2);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test("RULE-04 later undefended site strikes cannot deliver Death's Door death blows", async () => {
  await withNorthAttacksAtC2({
    seed: 71,
    avatar: { attack: 1, defense: 1, drawSpell: false, life: 1 },
  }, async (ctx, setup) => {
    const site = ctx.state.realm.sites.C2;
    assert.ok(site);
    const strikeSite = async (): Promise<void> => {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'site'
          && descriptor.target.instanceId === site.instanceId);
    };
    await strikeSite();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
    assert.equal(ctx.state.players.south.avatar.life, 0);
    assert.equal(ctx.state.players.south.avatar.deathDoorTurn, 5);
    assert.deepEqual(ctx.state.terminal, { status: 'active' });

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.attackerInstanceId
        && descriptor.to.cell === 'C2');
    await strikeSite();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && !descriptor.originalTargetParticipates);
    assert.equal(ctx.state.players.south.avatar.life, 0);
    assert.deepEqual(ctx.state.terminal, { status: 'active' });
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'death-blow'), false);
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) =>
      type === 'avatar-life-lost'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('shared stale rejection leaves game state, PRNG, and accepted transcript unchanged', async () => {
  await withSetup(manifest(17), async (ctx) => {
    const command = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'mulligan'
        && descriptor.atlasOrder.length === 0
        && descriptor.spellbookOrder.length === 0);
    const accepted = await ctx.step(command);
    assert.equal(accepted.accepted, true);
    const before = canonicalJson(accepted.session.state);
    const beforeHash = hashGameState(accepted.session.state);
    const stale = await ctx.step(command);

    assert.equal(stale.accepted, false);
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');

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
  let gameManifest: GameManifest | undefined;
  for (let seed = 83; seed < 256; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    // Seed peek via TS createGameSession (cheap); play path uses SetupCtx.
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'artifact-bearer' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'artifact-bearer')!;
    assert.equal(bearer.summoningSickness, true);
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword-and-shield'
      && descriptor.casterInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.cell === 'C4'
      && descriptor.bearer === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword-and-shield'
      && descriptor.casterInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.cell === 'C4'
      && descriptor.bearer === undefined);

    const surfaceArtifacts = (ctx.state.realm.artifacts ?? [])
      .flatMap((artifact) => 'bearer' in artifact ? [] : [artifact]);
    assert.equal(surfaceArtifacts.length, 2);
    const artifactInstanceIds = surfaceArtifacts.map(({ instanceId }) => instanceId).sort();
    // Forged-state probes still use TS legality; live probes use Rust SetupCtx.
    const pickupDescriptorsFrom = (checkpoint: GameSession) =>
      legalGameActions(checkpoint.state, checkpoint.state.decisionSeat)
        .flatMap(({ descriptor }) => descriptor.kind === 'pick-up-artifacts' ? [descriptor] : []);
    const dropDescriptorsFrom = (checkpoint: GameSession) =>
      legalGameActions(checkpoint.state, checkpoint.state.decisionSeat)
        .flatMap(({ descriptor }) => descriptor.kind === 'drop-artifacts' ? [descriptor] : []);
    const pickupDescriptorsLive = async () =>
      (await ctx.legalActions()).flatMap(({ descriptor }) =>
        descriptor.kind === 'pick-up-artifacts' ? [descriptor] : []);
    const dropDescriptorsLive = async () =>
      (await ctx.legalActions()).flatMap(({ descriptor }) =>
        descriptor.kind === 'drop-artifacts' ? [descriptor] : []);
    const expectedSubsets = [
      artifactInstanceIds[0],
      artifactInstanceIds[1],
      artifactInstanceIds.join(','),
    ].sort();
    const initialPickups = await pickupDescriptorsLive();
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
      ...ctx.session,
      state: {
        ...ctx.state,
        realm: {
          ...ctx.state.realm,
          artifacts: [
            ...surfaceArtifacts.map((artifact) => artifact.instanceId === artifactInstanceIds[0]
              ? { ...artifact, owner: 'south' as const }
              : artifact),
            { ...firstArtifact, instanceId: remoteId, location: 'C3' as const },
            { ...firstArtifact, instanceId: underwaterId, region: 'underwater' as const },
            {
              bearer: {
                instanceId: ctx.state.players.north.avatar.card.instanceId,
                kind: 'avatar' as const,
                seat: 'north' as const,
              },
              cardId: firstArtifact.cardId,
              instanceId: carriedId,
              owner: firstArtifact.owner,
              source: firstArtifact.source,
            },
          ],
          units: ctx.state.realm.units.map((unit) => unit.instanceId === bearer.instanceId
            ? { ...unit, tapped: true }
            : unit),
        },
      },
    };
    const filteredPickups = pickupDescriptorsFrom(filteredCheckpoint);
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
    const initialDrops = dropDescriptorsFrom(dropReady);
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
    assert.equal(dropDescriptorsFrom(droppedOnce).some(({ unit }) => unit.kind === 'minion'), false);

    const disabledCheckpoint: GameSession = {
      ...ctx.session,
      state: {
        ...ctx.state,
        realm: {
          ...ctx.state.realm,
          units: ctx.state.realm.units.map((unit) => unit.instanceId === bearer.instanceId
            ? {
              ...unit,
              disableEffects: [{ expiresAtSeat: 'south' as const, sourceInstanceId: unit.instanceId }],
            }
            : unit),
        },
      },
    };
    assert.deepEqual(pickupDescriptorsFrom(disabledCheckpoint).map(({ unit }) => unit.kind),
      ['avatar', 'avatar', 'avatar']);
    const disabledDrop: GameSession = {
      ...dropReady,
      state: {
        ...dropReady.state,
        realm: { ...dropReady.state.realm, units: disabledCheckpoint.state.realm.units },
      },
    };
    assert.equal(dropDescriptorsFrom(disabledDrop).some(({ unit }) => unit.kind === 'minion'), false);

    const beforeForge = hashGameState(ctx.state);
    const forged = await ctx.stepRequest({
      actionId: 'sha256:9999999999999999999999999999999999999999999999999999999999999999',
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(hashGameState(forged.session.state), beforeForge);

    const manaBeforePickUp = ctx.state.players.north.mana;
    const picked = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'pick-up-artifacts'
        && descriptor.unit.kind === 'minion'
        && descriptor.artifactInstanceIds.length === 1
        && descriptor.artifactInstanceIds[0] === artifactInstanceIds[0]));
    assert.equal(picked.accepted, true);
    if (!picked.accepted) throw new Error('expected Artifact Pick Up to be accepted');
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
    assert.equal(ctx.state.players.north.mana, manaBeforePickUp);
    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.deepEqual(ctx.state.realm.units
      .filter(({ instanceId }) => instanceId === bearer.instanceId)
      .map(({ stealthed, summoningSickness, tapped }) => ({ stealthed, summoningSickness, tapped })), [{
      stealthed: true,
      summoningSickness: true,
      tapped: false,
    }]);
    assert.equal(ctx.state.realm.artifacts?.find(({ instanceId }) =>
      instanceId === artifactInstanceIds[0])?.owner, 'north');

    const beforeVoluntaryDrop = createGameCheckpoint(ctx.session);
    const voluntarilyDropped = await ctx.step(await ctx.action(({ descriptor }) =>
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
    assert.deepEqual(observeGame(ctx.state, 'north').realm.artifacts
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
    assert.deepEqual(ctx.state.realm.units
      .filter(({ instanceId }) => instanceId === bearer.instanceId)
      .map(({ damage, stealthed, summoningSickness, tapped }) => ({
        damage, stealthed, summoningSickness, tapped,
      })), [{ damage: 0, stealthed: true, summoningSickness: true, tapped: false }]);
    assert.equal(ctx.state.players.north.mana, manaBeforePickUp);
    assert.equal(await ctx.verifyReplay(), true);
    await ctx.resume(beforeVoluntaryDrop);

    assert.equal((await pickupDescriptorsLive()).some(({ unit }) => unit.kind === 'minion'), false);
    assert.equal((await pickupDescriptorsLive()).some(({ unit }) => unit.kind === 'avatar'), true);
    let northView = observeGame(ctx.state, 'north');
    assert.equal(northView.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId)?.attack, 3);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    assert.equal((await pickupDescriptorsLive()).some(({ unit }) => unit.kind === 'minion'), true);
    const pickedAgain = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'pick-up-artifacts'
        && descriptor.unit.instanceId === bearer.instanceId
        && descriptor.artifactInstanceIds.length === 1
        && descriptor.artifactInstanceIds[0] === artifactInstanceIds[1]));
    assert.equal(pickedAgain.accepted, true);
    if (!pickedAgain.accepted) throw new Error('expected next-turn Artifact Pick Up to be accepted');
    assert.deepEqual(pickedAgain.receipt.randomDraws, []);

    const beforeMove = createGameCheckpoint(ctx.session);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === bearer.instanceId
        && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal((await dropDescriptorsLive()).some(({ unit }) =>
      unit.instanceId === bearer.instanceId), true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId)?.stealthed, true);
    await ctx.resume(beforeMove);

    const beforeActivate = createGameCheckpoint(ctx.session);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'activate-mana' && descriptor.unitInstanceId === bearer.instanceId);
    assert.equal((await dropDescriptorsLive()).some(({ unit }) =>
      unit.instanceId === bearer.instanceId), false);
    await ctx.resume(beforeActivate);

    const dummyCard = [
      ...ctx.state.players.south.hand.spellbook,
      ...ctx.state.players.south.spellbook,
    ].find(({ cardId }) => cardId === 'artifact-dummy');
    assert.ok(dummyCard);
    const strikeCheckpoint: GameSession = {
      ...ctx.session,
      state: {
        ...ctx.state,
        realm: {
          ...ctx.state.realm,
          units: [...ctx.state.realm.units, {
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
    assert.equal(dropDescriptorsFrom(struck).some(({ unit }) =>
      unit.instanceId === bearer.instanceId), false);
    northView = observeGame(ctx.state, 'north');
    const observedBearer = northView.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId);
    assert.equal(observedBearer?.attack, 5);
    assert.equal(observedBearer?.defense, 5);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword-and-shield'
      && descriptor.casterInstanceId === bearer.instanceId
      && descriptor.cell === 'C3'
      && descriptor.bearer === undefined);
    assert.equal((await dropDescriptorsLive()).some(({ unit }) =>
      unit.instanceId === bearer.instanceId), false);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId)?.stealthed, false);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === bearer.instanceId && descriptor.to.cell === 'C3');
    northView = observeGame(ctx.state, 'north');
    assert.deepEqual(northView.realm.artifacts?.map(({
      bearer: artifactBearer, controller, location, region,
    }) => ({
      bearer: artifactBearer?.instanceId,
      controller,
      location,
      region,
    })), [
      { bearer: bearer.instanceId, controller: 'north', location: 'C3', region: 'surface' },
      { bearer: bearer.instanceId, controller: 'north', location: 'C3', region: 'surface' },
      { bearer: undefined, controller: null, location: 'C3', region: 'surface' },
    ]);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'artifact-enemy' && descriptor.cell === 'C3');
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === 'artifact-enemy')!;
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === enemy.instanceId && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion' && descriptor.target.instanceId === bearer.instanceId);
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    const events = fought.receipt.events;
    const dropIndexes = events.flatMap(({ type }, index) => type === 'artifact-dropped' ? [index] : []);
    const deathIndex = events.findIndex(({ payload, type }) => type === 'minion-died'
      && canonicalJson(payload).includes(bearer.instanceId));
    assert.equal(dropIndexes.length, 2);
    assert.equal(dropIndexes.every((index) => index < deathIndex), true);
    northView = observeGame(ctx.state, 'north');
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
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === bearer.instanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ cardId }) =>
      cardId === 'sword-and-shield'), false);
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === 'drop-bearer');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'drop-bearer');
    assert.ok(bearer);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'cast-artifact'
        && descriptor.cardId === 'drop-sword'
        && descriptor.bearer?.instanceId === bearer.instanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === 'drop-static-servant');
    assert.deepEqual(observeGame(ctx.state, 'north').realm.units
      .filter(({ instanceId }) => instanceId === bearer.instanceId)
      .map(({ damage, defense }) => ({ damage, defense })), [{ damage: 1, defense: 3 }]);

    const beforeDropVersion = ctx.state.stateVersion;
    const dropped = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'drop-artifacts'
        && descriptor.unit.instanceId === bearer.instanceId
        && descriptor.artifactInstanceIds.length === 1));
    assert.equal(dropped.accepted, true);
    if (!dropped.accepted) return;

    assert.equal(ctx.state.stateVersion, beforeDropVersion + 1);
    assert.deepEqual(dropped.receipt.events.map(({ type }) => type), [
      'artifacts-dropped',
      'minion-died',
    ]);
    assert.deepEqual(dropped.receipt.randomDraws, []);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === bearer.instanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === bearer.instanceId), true);
    assert.deepEqual(observeGame(ctx.state, 'north').realm.artifacts?.map((artifact) => ({
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'dagger-bearer' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'dagger-bearer')!;
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'cast-artifact'
        && descriptor.cardId === 'poisonous-dagger'
        && descriptor.bearer?.instanceId === bearer.instanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === bearer.instanceId && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'dagger-enemy' && descriptor.cell === 'C3');
    const enemy = ctx.state.realm.units.find(({ cardId }) => cardId === 'dagger-enemy')!;
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === enemy.instanceId && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion' && descriptor.target.instanceId === bearer.instanceId);
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);

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
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === bearer.instanceId || instanceId === enemy.instanceId), false);
    assert.deepEqual(observeGame(ctx.state, 'north').realm.artifacts?.map((artifact) => ({
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
  const gameManifest = createGameManifest({ ...input, seed: 2 });
  assert.deepEqual(gameManifest.cards['siege-ballista'], cards['siege-ballista']);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-bearer'
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-helper'
      && descriptor.cell === 'C4'
      && descriptor.region === undefined);
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-bearer');
    const helper = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-helper');
    assert.ok(bearer && helper);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'siege-ballista'
      && descriptor.bearer?.kind === 'minion'
      && descriptor.bearer.instanceId === bearer.instanceId);
    const ballista = ctx.state.realm.artifacts?.find(({ cardId }) => cardId === 'siege-ballista');
    assert.ok(ballista);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.artifactInstanceId === ballista.instanceId
        && descriptor.helper.instanceId === helper.instanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-far-target' && descriptor.cell === 'C1');
    const farTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-far-target');
    assert.ok(farTarget);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.artifactInstanceId === ballista.instanceId
        && descriptor.target.instanceId === farTarget.instanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-near-target'
      && descriptor.cell === 'C2'
      && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'ballista-burrowed-target'
      && descriptor.cell === 'C2'
      && descriptor.region === 'underground');
    const nearTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'ballista-near-target');
    const burrowedTarget = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'ballista-burrowed-target');
    assert.ok(nearTarget && burrowedTarget);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const abilities = (await ctx.legalActions('north')).filter(({ descriptor }) =>
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

    // Forged-state probes still use TS legality.
    const noHelperState = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        units: ctx.state.realm.units.map((unit) => unit.instanceId === helper.instanceId
          ? { ...unit, location: 'C3' as const }
          : unit),
      },
    };
    assert.equal(legalGameActions(noHelperState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.helper.instanceId === helper.instanceId), false);
    const tappedHelperState = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        units: ctx.state.realm.units.map((unit) => unit.instanceId === helper.instanceId
          ? { ...unit, tapped: true }
          : unit),
      },
    };
    assert.equal(legalGameActions(tappedHelperState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.helper.instanceId === helper.instanceId), false);
    const disabledBearerState = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        units: ctx.state.realm.units.map((unit) => unit.instanceId === bearer.instanceId
          ? { ...unit, disabledUntilDamaged: true as const }
          : unit),
      },
    };
    assert.equal(legalGameActions(disabledBearerState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.artifactInstanceId === ballista.instanceId), false);
    const uncarriedState = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        artifacts: ctx.state.realm.artifacts!.map((artifact) =>
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
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        units: ctx.state.realm.units.map((unit) => unit.instanceId === nearTarget.instanceId
          ? { ...unit, stealthed: true }
          : unit),
      },
    };
    assert.equal(legalGameActions(hiddenTargetState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.target.instanceId === nearTarget.instanceId), false);
    const undergroundState = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        units: ctx.state.realm.units.map((unit) =>
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
    const beforeOverlap = createGameCheckpoint(ctx.session);
    const overlapResult = await ctx.step(overlap);
    assert.equal(overlapResult.accepted, true);
    if (!overlapResult.accepted) return;
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === helper.instanceId), false);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId)?.tapped, true);
    assert.equal(await ctx.verifyReplay(), true);
    await ctx.resume(beforeOverlap);

    const activation = abilities.find(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-damage'
        && descriptor.helper.instanceId === helper.instanceId
        && descriptor.target.instanceId === nearTarget.instanceId);
    assert.ok(activation);
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const survivingBearer = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId);
    const survivingHelper = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === helper.instanceId);
    const damagedTarget = ctx.state.realm.units.find(({ instanceId }) =>
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
  const gameManifest = createGameManifest({ ...input, seed: 5 });
  assert.deepEqual(gameManifest.cards['payload-trebuchet'], cards['payload-trebuchet']);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-bearer' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-helper' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-bearer');
    const helper = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-helper');
    assert.ok(bearer && helper);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'payload-trebuchet'
      && descriptor.bearer?.instanceId === bearer.instanceId);
    const payloadArtifact = ctx.state.realm.artifacts?.find(({ cardId }) =>
      cardId === 'payload-trebuchet');
    assert.ok(payloadArtifact);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-target'
      && descriptor.cell === 'C1'
      && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-warded-target'
      && descriptor.cell === 'C1'
      && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'payload-burrowed-target'
      && descriptor.cell === 'C1'
      && descriptor.region === 'underground');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const discard = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === 'payload-discard');
    const siteDiscard = ctx.state.players.north.hand.atlas[0];
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-target');
    const warded = ctx.state.realm.units.find(({ cardId }) => cardId === 'payload-warded-target');
    const burrowed = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'payload-burrowed-target');
    assert.ok(discard && siteDiscard && target && warded && burrowed);
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'
        && descriptor.artifactInstanceId === payloadArtifact.instanceId
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
    const beforeFriendly = createGameCheckpoint(ctx.session);
    const friendlyResult = await ctx.step(friendly);
    assert.equal(friendlyResult.accepted, true);
    if (!friendlyResult.accepted) return;
    const friendlyBearer = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId);
    assert.deepEqual({
      avatarLife: ctx.state.players.north.avatar.life,
      bearerDamage: friendlyBearer?.damage,
      bearerLance: friendlyBearer?.carriedLanceCount,
      bearerStealth: friendlyBearer?.stealthed,
      helperDamage: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === helper.instanceId)?.damage,
    }, {
      avatarLife: 16,
      bearerDamage: 4,
      bearerLance: 1,
      bearerStealth: true,
      helperDamage: 4,
    });
    assert.equal(await ctx.verifyReplay(), true);
    await ctx.resume(beforeFriendly);

    const zero = choices.find(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'
        && descriptor.discardCardInstanceId === siteDiscard.instanceId
        && descriptor.discardZone === 'atlas'
        && descriptor.targetLocation.cell === 'C1');
    assert.ok(zero);
    const beforeZero = createGameCheckpoint(ctx.session);
    const zeroResult = await ctx.step(zero);
    assert.equal(zeroResult.accepted, true);
    if (!zeroResult.accepted) return;
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === siteDiscard.instanceId), true);
    assert.equal(zeroResult.receipt.events.filter(({ type }) =>
      type === 'artifact-discard-area-damage-allocated').every(({ payload }) =>
      (payload as { amount: number }).amount === 0), true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === warded.instanceId)?.warded, true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId)?.damage, 0);
    assert.equal(await ctx.verifyReplay(), true);
    await ctx.resume(beforeZero);

    const activation = choices.find(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-discard-area-damage'
        && descriptor.discardCardInstanceId === discard.instanceId
        && descriptor.discardZone === 'spellbook'
        && descriptor.targetLocation.cell === 'C1');
    assert.ok(activation);
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const survivingBearer = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === bearer.instanceId);
    const survivingHelper = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === helper.instanceId);
    const survivingWard = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === warded.instanceId);
    assert.deepEqual({
      bearerDamage: survivingBearer?.damage,
      bearerLance: survivingBearer?.carriedLanceCount,
      bearerStealth: survivingBearer?.stealthed,
      bearerTapped: survivingBearer?.tapped,
      burrowedDamage: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === burrowed.instanceId)?.damage,
      helperDamage: survivingHelper?.damage,
      helperTapped: survivingHelper?.tapped,
      southLife: ctx.state.players.south.avatar.life,
      targetPresent: ctx.state.realm.units.some(({ instanceId }) =>
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
    assert.equal(ctx.state.players.north.hand.spellbook.some(({ instanceId }) =>
      instanceId === discard.instanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === discard.instanceId), true);
    const allocations = result.receipt.events.filter(({ type }) =>
      type === 'artifact-discard-area-damage-allocated');
    assert.equal(allocations.length, 3);
    assert.equal(allocations.every(({ payload: allocationPayload }) => {
      const allocation = allocationPayload as {
        amount?: number;
        sourceInstanceId?: string;
      };
      return allocation.amount === 4 && allocation.sourceInstanceId === payloadArtifact.instanceId;
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-pusher' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-origin-target' && descriptor.cell === 'C4');
    const pusher = ctx.state.realm.units.find(({ cardId }) => cardId === 'boulder-pusher');
    const originTarget = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'boulder-origin-target');
    assert.ok(pusher && originTarget);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'rolling-boulder'
      && descriptor.bearer === undefined
      && descriptor.cell === 'C4');
    const boulder = ctx.state.realm.artifacts?.find(({ cardId }) => cardId === 'rolling-boulder');
    assert.ok(boulder);
    const freshRolls = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage');
    assert.equal(freshRolls.some(({ descriptor }) =>
      descriptor.kind === 'activate-artifact-roll-damage'
        && descriptor.pusher.instanceId === pusher.instanceId), false);
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-warded-target'
      && descriptor.cell === 'C2'
      && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-underground-target'
      && descriptor.cell === 'C2'
      && descriptor.region === 'underground');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'boulder-off-path-target'
      && descriptor.cell === 'B1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const warded = ctx.state.realm.units.find(({ cardId }) => cardId === 'boulder-warded-target');
    const underground = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'boulder-underground-target');
    const offPath = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'boulder-off-path-target');
    assert.ok(warded && underground && offPath);
    const rolls = (await ctx.legalActions('north')).filter(({ descriptor }) =>
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
    const beforeForge = hashGameState(ctx.state);
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
    assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(hashGameState(forged.session.state), beforeForge);

    const beforeZero = createGameCheckpoint(ctx.session);
    const zeroResult = await ctx.step(zeroRoll);
    assert.equal(zeroResult.accepted, true);
    if (!zeroResult.accepted) return;
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === pusher.instanceId)?.tapped, true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === originTarget.instanceId)?.damage, 0);
    assert.deepEqual(zeroResult.receipt.events.map(({ type }) => type), [
      'artifact-roll-damage-activated',
    ]);
    assert.equal(observeGame(ctx.state, 'north').realm.artifacts
      ?.find(({ instanceId }) => instanceId === boulder.instanceId)?.location, 'C4');
    assert.equal(await ctx.verifyReplay(), true);
    await ctx.resume(beforeZero);

    // Forged carried-state probe still uses TS legality/step.
    const carriedSession: GameSession = {
      ...ctx.session,
      state: {
        ...ctx.state,
        realm: {
          ...ctx.state.realm,
          artifacts: ctx.state.realm.artifacts!.map((artifact) =>
            artifact.instanceId === boulder.instanceId
              ? {
                bearer: {
                  instanceId: pusher.instanceId,
                  kind: 'minion' as const,
                  seat: 'north' as const,
                },
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
      .map(({ bearer, controller, location, region }) => ({
        bearer, controller, location, region,
      })),
    [{ bearer: undefined, controller: null, location: 'C1', region: 'surface' }]);

    const result = await ctx.step(southRoll);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const survivingPusher = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === pusher.instanceId);
    const survivingWard = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === warded.instanceId);
    assert.deepEqual({
      northLife: ctx.state.players.north.avatar.life,
      offPathDamage: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === offPath.instanceId)?.damage,
      originTargetPresent: ctx.state.realm.units.some(({ instanceId }) =>
        instanceId === originTarget.instanceId),
      pusherDamage: survivingPusher?.damage,
      pusherLance: survivingPusher?.carriedLanceCount,
      pusherStealth: survivingPusher?.stealthed,
      pusherTapped: survivingPusher?.tapped,
      southLife: ctx.state.players.south.avatar.life,
      undergroundDamage: ctx.state.realm.units.find(({ instanceId }) =>
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
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === originTarget.instanceId), true);
    assert.deepEqual(observeGame(ctx.state, 'north').realm.artifacts
      ?.filter(({ instanceId }) => instanceId === boulder.instanceId)
      .map(({ bearer, controller, location, region }) => ({
        bearer, controller, location, region,
      })),
    [{ bearer: undefined, controller: null, location: 'C1', region: 'surface' }]);
    const allocations = result.receipt.events.filter(({ type }) =>
      type === 'artifact-roll-damage-allocated');
    const expectedTargetIds = [
      ctx.state.players.north.avatar.card.instanceId,
      originTarget.instanceId,
      ctx.state.players.south.avatar.card.instanceId,
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'mesmerism-target'
      && descriptor.casterInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'mesmerism-warded'
      && descriptor.casterInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'mesmerism-attacker'
      && descriptor.casterInstanceId === ctx.state.players.south.avatar.card.instanceId
      && descriptor.cell === 'C1');
    const hiddenTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-target')!;
    const wardedTarget = ctx.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-warded')!;
    const attacker = ctx.state.realm.units.find(({ cardId }) => cardId === 'mesmerism-attacker')!;

    const beforeHidden = createGameCheckpoint(ctx.session);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const hiddenTargetIds = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.kind === 'minion'
        ? [descriptor.target.instanceId]
        : []);
    assert.deepEqual([...new Set(hiddenTargetIds)], [wardedTarget.instanceId]);
    assert.equal(await ctx.verifyReplay(), true);
    await ctx.resume(beforeHidden);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'mesmerism-artifact'
      && descriptor.casterInstanceId === hiddenTarget.instanceId
      && descriptor.bearer?.instanceId === hiddenTarget.instanceId);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId)?.stealthed, false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const checkpoint = createGameCheckpoint(ctx.session);
    const manaBeforeCasts = ctx.state.players.north.mana;
    const targetBefore = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId)!;
    const carriedBefore = ctx.state.realm.artifacts?.find((artifact) =>
      'bearer' in artifact && artifact.bearer.instanceId === hiddenTarget.instanceId);
    assert.ok(carriedBefore);
    assert.ok('bearer' in carriedBefore);
    const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic');
    assert.deepEqual([...new Set(casts.flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.kind === 'minion'
        ? [descriptor.target.instanceId]
        : []))].sort(), [hiddenTarget.instanceId, wardedTarget.instanceId].sort());

    const warded = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === wardedTarget.instanceId));
    assert.equal(warded.accepted, true);
    if (!warded.accepted) return;
    assert.deepEqual(warded.receipt.events.map(({ type }) => type), [
      'magic-cast',
      'ward-broken',
      'magic-resolved',
    ]);
    assert.deepEqual({
      controller: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === wardedTarget.instanceId)?.controller,
      warded: ctx.state.realm.units.find(({ instanceId }) =>
        instanceId === wardedTarget.instanceId)?.warded,
    }, { controller: 'south', warded: false });
    assert.equal(await ctx.verifyReplay(), true);
    await ctx.resume(checkpoint);

    const gainedAction = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === hiddenTarget.instanceId);
    const sourceInstanceId = gainedAction.descriptor.kind === 'cast-magic'
      ? gainedAction.descriptor.cardInstanceId
      : '';
    const gained = await ctx.step(gainedAction);
    assert.equal(gained.accepted, true);
    if (!gained.accepted) return;
    const controlled = ctx.state.realm.units.find(({ instanceId }) =>
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
    const carried = ctx.state.realm.artifacts?.find((artifact) =>
      'bearer' in artifact && artifact.bearer.instanceId === hiddenTarget.instanceId);
    assert.ok(carried);
    assert.ok('bearer' in carried);
    assert.deepEqual(carried, {
      ...carriedBefore,
      bearer: { ...carriedBefore.bearer, seat: 'north' },
    });
    const northView = observeGame(ctx.state, 'north');
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
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === hiddenTarget.instanceId
        && descriptor.to.cell === 'C3'), true);

    const ownNoOp = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.casterInstanceId === ctx.state.players.north.avatar.card.instanceId
        && descriptor.target?.instanceId === hiddenTarget.instanceId));
    assert.equal(ownNoOp.accepted, true);
    if (!ownNoOp.accepted) return;
    assert.deepEqual(ownNoOp.receipt.events.map(({ type }) => type), ['magic-cast', 'magic-resolved']);
    assert.deepEqual(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId), controlled);
    assert.deepEqual(ctx.state.realm.artifacts?.find((artifact) =>
      'bearer' in artifact && artifact.bearer.instanceId === hiddenTarget.instanceId), carried);
    assert.equal(gained.receipt.randomDraws.length + ownNoOp.receipt.randomDraws.length, 0);
    assert.equal(ctx.state.players.north.mana, manaBeforeCasts - 2);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attacker.instanceId && descriptor.to.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === hiddenTarget.instanceId);
    const playersBeforeDeathrite = ctx.state.players;
    const fought = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    if (!fought.accepted) return;
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId), false);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === hiddenTarget.instanceId), false);
    const deathEvents = fought.receipt.events;
    assert.equal(ctx.state.players.north.atlas.length, playersBeforeDeathrite.north.atlas.length - 1);
    assert.equal(ctx.state.players.north.hand.atlas.length,
      playersBeforeDeathrite.north.hand.atlas.length + 1);
    assert.equal(ctx.state.players.south.atlas.length, playersBeforeDeathrite.south.atlas.length);
    assert.equal(ctx.state.players.south.hand.atlas.length,
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
    assert.equal(observeGame(ctx.state, 'north').realm.artifacts?.[0]?.controller, null);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

