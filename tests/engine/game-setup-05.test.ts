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
  hashGameState,
  legalGameActions,
  observeGame,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameLegalAction,
  type GameSession,
} from '../../src/engine/game.ts';
import {
  manifest,
  SYNTHETIC_AUTHORITY_HASH,
  takeAction,
  withNorthAttacksAtC2,
  type NorthAttacksAtC2Ids,
  type SpellFacts,
} from './game-setup-helpers.ts';
import { SetupCtx, withPreview, withSetup } from './rust-setup-session.ts';

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
    check: (ctx: SetupCtx, ids: NorthAttacksAtC2Ids) => Promise<void>,
  ): Promise<void> => {
    await withNorthAttacksAtC2({ seed, spell: northSpell, southSpell }, async (ctx, ids) => {
      assert.equal(observeGame(ctx.state, 'north').realm.units
        .find(({ instanceId }) => instanceId === ids.targetInstanceId)?.disabled, true);
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === ids.targetInstanceId);
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
      await check(ctx, ids);
    });
  };

  await fight(119, attacker, sleeper, async (ctx, ordinary) => {
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === ordinary.attackerInstanceId)?.damage, 0);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === ordinary.targetInstanceId)?.damage, 2);
    assert.equal(observeGame(ctx.state, 'north').realm.units
      .find(({ instanceId }) => instanceId === ordinary.targetInstanceId)?.disabled, false);
    assert.equal(ctx.session.transcript.at(-1)?.events
      .filter(({ type }) => type === 'minion-awakened').length, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await fight(120, { ...attacker, strikesFirstWhileAttacking: true }, sleeper, async (ctx, early) => {
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === early.attackerInstanceId)?.damage, 5);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === early.targetInstanceId)?.damage, 2);
    assert.equal(observeGame(ctx.state, 'north').realm.units
      .find(({ instanceId }) => instanceId === early.targetInstanceId)?.disabled, false);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await fight(121, attacker, { ...sleeper, ward: true }, async (ctx, warded) => {
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === warded.targetInstanceId)?.damage, 0);
    assert.equal(observeGame(ctx.state, 'north').realm.units
      .find(({ instanceId }) => instanceId === warded.targetInstanceId)?.disabled, true);
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

  await withNorthAttacksAtC2({ seed: 124, spell: lance, southSpell: twoPower }, async (ctx, attacking) => {
    assert.equal(observeGame(ctx.state, 'south').realm.units
      .find(({ instanceId }) => instanceId === attacking.attackerInstanceId)?.carriedLanceCount, 1);
    assert.equal(ctx.session.transcript.some(({ events }) => events.some(({ payload, type }) =>
      type === 'lance-gained'
        && canonicalJson(payload) === canonicalJson({
          bearerInstanceId: attacking.attackerInstanceId,
          count: 1,
          sourceInstanceId: attacking.attackerInstanceId,
        }))), true);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === attacking.targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    const attackEvents = ctx.session.transcript.at(-1)!.events;
    assert.deepEqual(attackEvents.find(({ type }) => type === 'lance-broken')?.payload, {
      bearerInstanceId: attacking.attackerInstanceId,
      count: 1,
      sourceInstanceId: attacking.attackerInstanceId,
    });
    assert.ok(attackEvents.findIndex(({ type }) => type === 'lance-broken')
      < attackEvents.findIndex(({ type }) => type === 'minion-died'));
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === attacking.attackerInstanceId)?.damage, 0);
    assert.equal(ctx.state.players.south.cemetery
      .some(({ instanceId }) => instanceId === attacking.targetInstanceId), true);
    assert.equal(observeGame(ctx.state, 'north').realm.units
      .find(({ instanceId }) => instanceId === attacking.attackerInstanceId)?.carriedLanceCount, undefined);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({ seed: 125, spell: lance, southSpell: twoPower }, async (ctx, site) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-defend');
    const siteEvents = ctx.session.transcript.at(-1)!.events;
    assert.equal(siteEvents.some(({ type }) => type === 'lance-broken'), false);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === site.attackerInstanceId)?.carriedLanceCount, 1);
    const siteStruck = siteEvents.find(({ type }) => type === 'undefended-site-struck');
    assert.ok(siteStruck);
    assert.equal(canonicalJson(siteStruck.payload).includes('"amount":1'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  const tripleLance = { ...lance, defense: 3, lanceCount: 3 as const };
  await withNorthAttacksAtC2({
    seed: 126,
    southSpell: tripleLance,
    spell: { ...lance, defense: 4 },
  }, async (ctx, defending) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === defending.targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    const defendEvents = ctx.session.transcript.at(-1)!.events;
    assert.equal(ctx.state.players.north.cemetery
      .some(({ instanceId }) => instanceId === defending.attackerInstanceId), true);
    assert.equal(ctx.state.realm.units
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
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 127,
    southSpell: { ...twoPower, defense: 3, ward: true },
    spell: { ...lance, ranged: true },
  }, async (ctx, ranged) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    if (ctx.state.phase === 'intercept') {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === ranged.attackerInstanceId
        && descriptor.hit?.instanceId === ranged.targetInstanceId);
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
      .find(({ instanceId }) => instanceId === ranged.targetInstanceId);
    assert.deepEqual(
      rangedTarget && { damage: rangedTarget.damage, warded: rangedTarget.warded },
      { damage: 0, warded: false },
    );
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === ranged.attackerInstanceId)?.carriedLanceCount, undefined);
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const shooterInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'north')?.instanceId;
    assert.ok(shooterInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const targetInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(targetInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === shooterInstanceId
        && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    const branchPoint = createGameCheckpoint(ctx.session);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const longRangeShots = (await ctx.legalActions('north'))
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
    await ctx.accept(longRangeShot);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(branchPoint);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C2');
    const secondTargetInstanceId = ctx.state.realm.units
      .find(({ controller, location }) => controller === 'south' && location === 'C2')?.instanceId;
    assert.ok(secondTargetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === targetInstanceId
        && descriptor.to.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
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
    assert.equal(observeGame(ctx.state, 'north').realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.warded, false);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === shooterInstanceId
        && descriptor.hit?.instanceId === targetInstanceId);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) => instanceId === targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a surviving enabled minion may take one legal step after its Ranged strike', async () => {
  await withNorthAttacksAtC2({
    seed: 176,
    southSpell: {
      attack: 1,
      defense: 5,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    spell: {
      attack: 1,
      defense: 3,
      manaCost: 1,
      mayStepAfterRangedStrike: true,
      ranged: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    if (ctx.state.phase === 'intercept') {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const shot = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === setup.attackerInstanceId
        && descriptor.hit?.instanceId === setup.targetInstanceId));
    assert.equal(shot.accepted, true);
    if (!shot.accepted) return;
    const pending = shot.session;
    assert.deepEqual({
      decisionSeat: pending.state.decisionSeat,
      phase: pending.state.phase,
      sourceInstanceId: pending.state.pendingRangedStep?.sourceInstanceId,
    }, {
      decisionSeat: 'north',
      phase: 'ranged-step',
      sourceInstanceId: setup.attackerInstanceId,
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

    const pendingCheckpoint = createGameCheckpoint(pending);
    const declined = await ctx.step(decline);
    assert.equal(declined.accepted, true);
    if (!declined.accepted) return;
    assert.equal(declined.session.state.phase, 'main');
    assert.equal(declined.session.state.pendingRangedStep, null);
    assert.deepEqual(declined.receipt.events, []);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(pendingCheckpoint);
    const stepped = await ctx.step(step);
    assert.equal(stepped.accepted, true);
    if (!stepped.accepted) return;
    const steppedUnit = stepped.session.state.realm.units.find(({ instanceId }) =>
      instanceId === setup.attackerInstanceId);
    assert.deepEqual(
      steppedUnit && { location: steppedUnit.location, tapped: steppedUnit.tapped },
      { location: 'C3', tapped: true },
    );
    assert.deepEqual(stepped.receipt.events.map(({ type }) => type), ['unit-stepped']);
    assert.deepEqual(stepped.receipt.events[0]?.payload, {
      from: { cell: 'C2', region: 'surface' },
      instanceId: setup.attackerInstanceId,
      seat: 'north',
      sourceInstanceId: setup.attackerInstanceId,
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const shooterInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(shooterInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === shooterInstanceId
        && descriptor.hit === null);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.pendingRangedStep, undefined);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 a Ranged striker that dies during the hit cannot leave a pending step', async () => {
  await withNorthAttacksAtC2({
    seed: 178,
    southSpell: {
      attack: 1,
      deathriteDamageEachUnitHere: 1,
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
    spell: {
      attack: 1,
      defense: 1,
      manaCost: 1,
      mayStepAfterRangedStrike: true,
      ranged: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    if (ctx.state.phase === 'intercept') {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.shooterInstanceId === setup.attackerInstanceId
        && descriptor.hit?.instanceId === setup.targetInstanceId);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.pendingRangedStep, undefined);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === setup.attackerInstanceId), true);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === setup.targetInstanceId), true);
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
  await withSetup(base, async (preview) => {
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
      const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
        await takeAction(ctx, predicate);
      };

      await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === shooterCard.cardId && descriptor.cell === 'C4');
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === attackerCard.cardId && descriptor.cell === 'C1');
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === visibleCard.cardId && descriptor.cell === 'C3');
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === stealthedCard.cardId && descriptor.cell === 'C3');
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === defendedCard.cardId && descriptor.cell === 'C2');
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
      await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ctx.state.realm.units
          .find(({ cardId }) => cardId === attackerCard.cardId)?.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
      await take(({ descriptor }) => descriptor.kind === 'decline-attack');
      if (ctx.state.phase === 'intercept') {
        await take(({ descriptor }) => descriptor.kind === 'close-intercept');
      }
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

      const checkpoint = ctx.session;
      const branchPoint = createGameCheckpoint(checkpoint);
      const shooter = checkpoint.state.realm.units.find(({ cardId }) =>
        cardId === shooterCard.cardId);
      const defended = checkpoint.state.realm.units.find(({ cardId }) =>
        cardId === defendedCard.cardId);
      const attacker = checkpoint.state.realm.units.find(({ cardId }) =>
        cardId === attackerCard.cardId);
      const visible = checkpoint.state.realm.units.find(({ cardId }) =>
        cardId === visibleCard.cardId);
      const stealthed = checkpoint.state.realm.units.find(({ cardId }) =>
        cardId === stealthedCard.cardId);
      assert.ok(shooter);
      assert.ok(defended);
      assert.ok(attacker);
      assert.ok(visible);
      assert.ok(stealthed);
      const findUnit = (state: typeof checkpoint.state, instanceId: string) =>
        state.realm.units.find((unit) => unit.instanceId === instanceId);
      const shooterShots = async () =>
        (await ctx.legalActions('north')).filter(({ descriptor }) =>
          descriptor.kind === 'shoot-projectile'
            && descriptor.shooterInstanceId === shooter.instanceId);

      const move = await ctx.action(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === shooter.instanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
      const moveStarted = await ctx.step(move);
      assert.equal(moveStarted.accepted, true);
      if (!moveStarted.accepted) return;
      assert.equal(ctx.state.phase, 'movement');
      assert.equal(findUnit(ctx.state, shooter.instanceId)?.location, 'C4');
      const originShots = await shooterShots();
      const originShot = originShots.find(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.direction === 'south'
          && descriptor.hit?.instanceId === visible.instanceId);
      assert.ok(originShot);
      assert.deepEqual(originShot.descriptor.kind === 'shoot-projectile'
        ? originShot.descriptor.path.map(({ cell }) => cell)
        : [], ['C4', 'C3']);
      assert.equal(originShots.some(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.hit?.instanceId === stealthed.instanceId), false);
      assert.equal(originShots.some(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.hit?.instanceId === attacker.instanceId), false);

      const originFired = await ctx.step(originShot);
      assert.equal(originFired.accepted, true);
      if (!originFired.accepted) return;
      assert.equal(ctx.state.phase, 'movement');
      assert.equal(findUnit(ctx.state, visible.instanceId)?.damage, 1);
      assert.equal(findUnit(ctx.state, shooter.instanceId)?.stealthed, false);
      assert.equal(originFired.receipt.events.some(({ payload, type }) =>
        type === 'stealth-lost' && canonicalJson(payload).includes(shooter.instanceId)), true);
      assert.equal((await shooterShots()).length, 0);

      const beforeForgeHash = hashGameState(ctx.state);
      const beforeForgeTranscript = ctx.session.transcript.length;
      const forged = await ctx.stepRequest({
        actionId: opaqueActionId(
          'sorcery-core-v1',
          'north',
          ctx.state.stateVersion,
          originShot.descriptor,
        ),
        seat: 'north',
        stateVersion: ctx.state.stateVersion,
      });
      assert.equal(forged.accepted, false);
      if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
      assert.equal(hashGameState(forged.session.state), beforeForgeHash);
      assert.equal(forged.session.transcript.length, beforeForgeTranscript);
      await take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
      assert.equal(findUnit(ctx.state, shooter.instanceId)?.location, 'C3');
      await take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
      assert.equal(findUnit(ctx.state, shooter.instanceId)?.location, 'C2');
      await take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
      assert.equal(ctx.state.phase, 'attack');
      const movedShooter = findUnit(ctx.state, shooter.instanceId);
      assert.deepEqual(movedShooter && {
        location: movedShooter.location,
        tapped: movedShooter.tapped,
      }, { location: 'C2', tapped: true });
      assert.equal(ctx.session.transcript.flatMap(({ events }) => events)
        .filter(({ type }) => type === 'projectile-shot').length, 1);
      assert.equal(await ctx.verifyReplay(), true);

      await ctx.resume(branchPoint);
      await take(({ descriptor }) => descriptor.kind === 'end-turn');
      await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await take(({ descriptor }) =>
        descriptor.kind === 'move-and-attack'
          && descriptor.unitInstanceId === attacker.instanceId
          && descriptor.path.length === 1);
      await take(({ descriptor }) =>
        descriptor.kind === 'declare-attack'
          && descriptor.target.kind === 'minion'
          && descriptor.target.instanceId === defended.instanceId);
      const defend = await ctx.action(({ descriptor }) =>
        descriptor.kind === 'defend'
          && descriptor.unitInstanceId === shooter.instanceId
          && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
      const defendStarted = await ctx.step(defend);
      assert.equal(defendStarted.accepted, true);
      if (!defendStarted.accepted) return;
      await take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
      assert.equal(ctx.state.phase, 'movement');
      assert.equal(findUnit(ctx.state, shooter.instanceId)?.location, 'C3');
      const intermediateShots = await shooterShots();
      const intermediateShot = intermediateShots.find(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.hit?.instanceId === visible.instanceId);
      assert.ok(intermediateShot);
      assert.deepEqual(intermediateShot.descriptor.kind === 'shoot-projectile'
        ? intermediateShot.descriptor.path.map(({ cell }) => cell)
        : [], ['C3']);
      assert.equal(intermediateShots.some(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.hit?.instanceId === stealthed.instanceId), false);
      assert.equal(intermediateShots.some(({ descriptor }) =>
        descriptor.kind === 'shoot-projectile'
          && descriptor.hit?.instanceId === attacker.instanceId), false);
      const intermediateFired = await ctx.step(intermediateShot);
      assert.equal(intermediateFired.accepted, true);
      if (!intermediateFired.accepted) return;
      assert.equal(findUnit(ctx.state, visible.instanceId)?.damage, 1);
      assert.equal((await shooterShots()).length, 0);
      await take(({ descriptor }) => descriptor.kind === 'continue-basic-movement');
      assert.equal(findUnit(ctx.state, shooter.instanceId)?.location, 'C2');
      const defendFinished = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'continue-basic-movement'));
      assert.equal(defendFinished.accepted, true);
      if (!defendFinished.accepted) return;
      assert.equal(defendFinished.session.state.phase, 'defend');
      assert.equal(defendFinished.session.state.pendingCombat?.defenders.some(({ instanceId }) =>
        instanceId === shooter.instanceId), true);
      assert.equal(defendFinished.receipt.events.some(({ type }) => type === 'defender-joined'), true);
      assert.equal(defendFinished.session.transcript.flatMap(({ events }) => events)
        .filter(({ type }) => type === 'projectile-shot').length, 1);
      assert.equal(await ctx.verifyReplay(), true);
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
  let pudgeCardId: string | undefined;
  let blockerCardId: string | undefined;
  await withPreview(base, async (preview) => {
    pudgeCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    blockerCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
  });
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === pudgeCard.instanceId
        && descriptor.cell === 'C4');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile'
        && descriptor.shooterInstanceId === pudgeCard.instanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === blockerCard.instanceId
        && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    const targetCard = ctx.state.players.south.hand.spellbook[0];
    assert.ok(targetCard);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === targetCard.instanceId
        && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const branchPoint = createGameCheckpoint(ctx.session);
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile'
        && descriptor.shooterInstanceId === pudgeCard.instanceId
        && descriptor.direction === 'south'
        && descriptor.hit?.instanceId === targetCard.instanceId);
    assert.deepEqual(choices.map(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival), [false, true]);
    assert.equal(choices.every(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile'
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'), true);
    assert.equal(choices.some(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile'
        && descriptor.hit?.instanceId === blockerCard.instanceId), false);

    const noFightChoice = choices.find(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile' && !descriptor.fightOnArrival);
    assert.ok(noFightChoice);
    await ctx.accept(noFightChoice);
    const dragged = ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetCard.instanceId);
    assert.deepEqual({ location: dragged?.location, tapped: dragged?.tapped, warded: dragged?.warded }, {
      location: 'C4',
      tapped: false,
      warded: true,
    });
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === pudgeCard.instanceId)?.tapped, true);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'projectile-shot',
      'unit-dragged',
    ]);
    const draggedPayload = ctx.session.transcript.at(-1)?.events[1]?.payload;
    const draggedJson = canonicalJson(draggedPayload ?? null);
    assert.equal(draggedJson.includes('"steps":2'), true);
    assert.match(
      draggedJson,
      /"path":\[{"cell":"C2","region":"surface"},{"cell":"C3","region":"surface"},{"cell":"C4","region":"surface"}\]/,
    );
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(branchPoint);
    const fightChoice = choices.find(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile' && descriptor.fightOnArrival);
    assert.ok(fightChoice);
    await ctx.accept(fightChoice);
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
      .find(({ instanceId }) => instanceId === pudgeCard.instanceId);
    const foughtTarget = ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetCard.instanceId);
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
  let pudgeCardId: string | undefined;
  let rainCardId: string | undefined;
  let fragileIds: string[] = [];
  let targetCardId: string | undefined;
  await withPreview(base, async (preview) => {
    const players = preview.state.players;
    pudgeCardId = players.north.hand.spellbook[0]?.cardId;
    rainCardId = players.north.hand.spellbook[1]?.cardId;
    fragileIds = players.south.hand.spellbook.slice(0, 2).map(({ cardId }) => cardId);
    targetCardId = players.south.hand.spellbook[2]?.cardId;
  });
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

  const advanceToDragReady = async (
    ctx: SetupCtx,
    fragileCell: 'C1' | 'C2',
  ): Promise<Readonly<{
    fragiles: readonly GameSession['state']['realm']['units'][number][];
    pudge: GameSession['state']['realm']['units'][number];
    target: GameSession['state']['realm']['units'][number];
  }>> => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === pudgeCardId
      && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    if (fragileCell === 'C1') {
      for (const fragileId of fragileIds) {
        await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === fragileId
          && descriptor.cell === fragileCell);
      }
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    if (fragileCell === 'C2') {
      for (const fragileId of fragileIds) {
        await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
          && descriptor.cardId === fragileId
          && descriptor.cell === fragileCell);
      }
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetCardId
      && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic' && descriptor.cardId === rainCardId);
    const pudge = ctx.state.realm.units.find(({ cardId }) => cardId === pudgeCardId);
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetCardId);
    const fragiles = ctx.state.realm.units.filter(({ cardId }) => fragileIds.includes(cardId));
    assert.ok(pudge && target);
    assert.equal(fragiles.length, 2);
    assert.equal(fragiles.every(({ damage }) => damage === 1), true);
    return { fragiles, pudge, target };
  };

  await withSetup(gameManifest, async (ctx) => {
    const { fragiles, pudge, target } = await advanceToDragReady(ctx, 'C1');
    const ready = createGameCheckpoint(ctx.session);
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
      await ctx.resume(ready);
      const beforeVersion = ctx.state.stateVersion;
      const interrupted = await ctx.step(choice);
      assert.equal(interrupted.accepted, true);
      if (!interrupted.accepted) throw new Error('expected drag to reach Deathrite ordering');
      assert.equal(interrupted.session.state.stateVersion, beforeVersion + 1);
      assert.deepEqual(interrupted.receipt.events.map(({ type }) => type), [
        'projectile-shot',
        'unit-dragged',
      ]);
      assert.equal(interrupted.session.state.phase, 'deathrite-order');
      assert.equal(interrupted.session.state.decisionSeat, 'south');
      assert.equal(interrupted.session.state.realm.units.find(({ instanceId }) =>
        instanceId === target.instanceId)?.location, 'C3');
      assert.equal(fragiles.every(({ instanceId }) => !interrupted.session.state.realm.units
        .some((unit) => unit.instanceId === instanceId)), true);
      assert.equal(fragiles.every(({ instanceId }) => !interrupted.session.state.players.south.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      const interruptedCheckpoint = createGameCheckpoint(interrupted.session);
      const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
        interruptedCheckpoint,
      )));
      assert.equal(
        canonicalJson(restored as unknown as JsonValue),
        canonicalJson(interrupted.session as unknown as JsonValue),
      );
      await ctx.resume(interruptedCheckpoint);
      const orders = (await ctx.legalActions('south')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.equal(orders.length, 2);
      assert.equal((await ctx.legalActions('north')).length, 0);
      const restoredVersion = ctx.state.stateVersion;
      const resolved = await ctx.step(orders[choiceIndex]!);
      assert.equal(resolved.accepted, true);
      if (!resolved.accepted) throw new Error('expected drag to resume after Deathrites');
      assert.equal(resolved.session.state.stateVersion, restoredVersion + 1);
      const types = resolved.receipt.events.map(({ type }) => type);
      assert.deepEqual(types.slice(0, 5), [
        'deathrite-order-committed',
        'site-drawn',
        'site-drawn',
        'minion-died',
        'minion-died',
      ]);
      assert.equal(types[5], 'unit-dragged');
      assert.equal(resolved.session.state.phase, 'main');
      assert.equal(resolved.session.state.pendingDeathrites, undefined);
      assert.equal(resolved.session.state.realm.units.find(({ instanceId }) =>
        instanceId === target.instanceId)?.location, 'C4');
      assert.equal(fragiles.every(({ instanceId }) => resolved.session.state.players.south.cemetery
        .some((card) => card.instanceId === instanceId)), true);
      assert.equal(resolved.session.state.players.south.atlas.length, atlasBefore - 2);
      assert.equal(resolved.session.state.players.south.hand.atlas.length, atlasHandBefore + 2);
      assert.equal(types.includes('fight-started'), choice.descriptor.fightOnArrival);
      assert.equal(types.indexOf('fight-started') > types.indexOf('unit-dragged'),
        choice.descriptor.fightOnArrival);
      assert.equal(await ctx.verifyReplay(), true);
      branchHashes.push(hashGameState(resolved.session.state));
    }
    assert.equal(new Set(branchHashes).size, 2);
  });

  await withSetup(gameManifest, async (ctx) => {
    const finalEdge = await advanceToDragReady(ctx, 'C2');
    const ready = createGameCheckpoint(ctx.session);
    const finalChoices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'shoot-drag-projectile'
        && descriptor.shooterInstanceId === finalEdge.pudge.instanceId
        && descriptor.hit?.instanceId === finalEdge.target.instanceId
        && descriptor.direction === 'south');
    for (const [choiceIndex, choice] of finalChoices.entries()) {
      assert.equal(choice.descriptor.kind, 'shoot-drag-projectile');
      if (choice.descriptor.kind !== 'shoot-drag-projectile') throw new Error('unreachable');
      await ctx.resume(ready);
      const beforeVersion = ctx.state.stateVersion;
      const interrupted = await ctx.step(choice);
      assert.equal(interrupted.accepted, true);
      if (!interrupted.accepted) throw new Error('expected final drag edge to reach Deathrites');
      assert.equal(interrupted.session.state.stateVersion, beforeVersion + 1);
      assert.equal(interrupted.session.state.phase, 'deathrite-order');
      assert.equal(interrupted.session.state.realm.units.find(({ instanceId }) =>
        instanceId === finalEdge.target.instanceId)?.location, 'C4');
      assert.equal(interrupted.receipt.events.some(({ type }) => type === 'fight-started'), false);
      const interruptedCheckpoint = createGameCheckpoint(interrupted.session);
      const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
        interruptedCheckpoint,
      )));
      assert.equal(
        canonicalJson(restored as unknown as JsonValue),
        canonicalJson(interrupted.session as unknown as JsonValue),
      );
      await ctx.resume(interruptedCheckpoint);
      const orders = (await ctx.legalActions('south')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      assert.equal(orders.length, 2);
      const restoredVersion = ctx.state.stateVersion;
      const resolved = await ctx.step(orders[choiceIndex]!);
      assert.equal(resolved.accepted, true);
      if (!resolved.accepted) throw new Error('expected final-edge Deathrites to finish');
      assert.equal(resolved.session.state.stateVersion, restoredVersion + 1);
      const types = resolved.receipt.events.map(({ type }) => type);
      assert.equal(types.includes('unit-dragged'), false);
      assert.equal(types.includes('fight-started'), choice.descriptor.fightOnArrival);
      assert.ok(!choice.descriptor.fightOnArrival
        || types.indexOf('fight-started') > types.lastIndexOf('minion-died'));
      assert.equal(resolved.session.state.phase, 'main');
      assert.equal(await ctx.verifyReplay(), true);
    }
  });
});

test('RULE-03 Granary Rats suppresses its site threshold while enabled', async () => {
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
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const gated = ctx.state.players.north.hand.spellbook.find(({ cardId }) => cardId === 'gated');
    const rats = ctx.state.players.north.hand.spellbook.find(({ cardId }) => cardId === 'rats');
    assert.ok(gated);
    assert.ok(rats);
    const baseline = ctx.session;
    // TODO(rust-cutover): synthetic state, needs a Rust-side proof. This test hand-builds
    // GameSession objects (edited region/controller/cards/disableEffects) that are not
    // reachable through legal play, so the Rust engine cannot be handed them; the affinity-
    // suppression legality probes below stay on the legacy TS `legalGameActions`. The overall
    // rule (enabled Granary Rats suppress the site threshold; a disabled Granary Rats or a
    // protected site does not) is also proven directly in Rust by
    // `rule_catalog_0094_granary_rats_suppress_site_threshold_while_enabled` in
    // crates/sorcery-engine/tests/readiness_affinity_rules.rs, but that proof does not cover
    // every branch below (e.g. a void-region rat, or one of two rats disabled), so this test
    // is kept in full on the legacy engine rather than weakened.
    const canSummonGated = (checkpoint: GameSession) => legalGameActions(checkpoint.state, 'north')
      .some(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'gated' && descriptor.cell === 'C4');
    assert.deepEqual(observeGame(baseline.state, 'north').players.north.affinity,
      { air: 0, earth: 1, fire: 1, water: 0 });
    assert.equal(canSummonGated(baseline), true);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'gated' && descriptor.cell === 'C4'), true);

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
});

test('RULE-03 a provider adds affinity until that minion dies', async () => {
  await withNorthAttacksAtC2({
    seed: 46,
    spell: {
      attack: 1,
      defense: 1,
      manaCost: 1,
      provides: 'earth',
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }, async (ctx, setup) => {
    assert.equal(observeGame(ctx.state, 'north').players.north.affinity.earth, 3);
    assert.equal(observeGame(ctx.state, 'south').players.south.affinity.earth, 4);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(observeGame(ctx.state, 'north').players.north.affinity.earth, 2);
    assert.equal(observeGame(ctx.state, 'south').players.south.affinity.earth, 3);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Lethal kills a tougher minion with positive damage but not zero damage', async () => {
  const resolve = async (attack: number, seed: number): Promise<void> => {
    await withNorthAttacksAtC2({
      seed,
      spell: {
        attack,
        defense: 5,
        lethal: true,
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
      if (attack > 0) {
        assert.equal(ctx.state.players.north.cemetery.length, 1);
        assert.equal(ctx.state.players.south.cemetery.length, 1);
      } else {
        assert.equal(ctx.state.players.north.cemetery.length, 0);
        assert.equal(ctx.state.players.south.cemetery.length, 0);
      }
      assert.equal(await ctx.verifyReplay(), true);
    });
  };

  await resolve(1, 45);
  await resolve(0, 44);
});

test('RULE-04 a prohibited minion cannot move to Defend but can still Intercept', async () => {
  const spell = {
    attack: 2,
    cannotDefend: true,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  await withNorthAttacksAtC2({ seed: 50, spell }, async (ctx, defend) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === defend.targetInstanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === defend.defenderInstanceId), false);
  });

  await withNorthAttacksAtC2({ seed: 51, spell }, async (ctx, stationary) => {
    const site = ctx.state.realm.sites.C2;
    assert.ok(site);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === site.instanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === stationary.targetInstanceId), true);
  });

  await withNorthAttacksAtC2({ seed: 52, spell }, async (ctx, intercept) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept' && descriptor.unitInstanceId === intercept.targetInstanceId), true);
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

  await withNorthAttacksAtC2({
    seed: 145,
    spell: vanilla,
    southSpell: immobile,
  }, async (ctx, movingDefend) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === movingDefend.targetInstanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === movingDefend.defenderInstanceId), false);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 146,
    spell: vanilla,
    southSpell: immobile,
  }, async (ctx, stationary) => {
    const site = ctx.state.realm.sites.C2;
    assert.ok(site);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'site'
        && descriptor.target.instanceId === site.instanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'defend'
      && descriptor.unitInstanceId === stationary.targetInstanceId
      && descriptor.path.length === 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 147,
    spell: vanilla,
    southSpell: immobile,
  }, async (ctx, local) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = (await ctx.legalActions('south')).filter(({ descriptor }) =>
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
    const ignored = await ctx.step(forged);
    assert.equal(ignored.accepted, true);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === local.targetInstanceId)?.location, 'C2');
    assert.doesNotMatch(canonicalJson(ignored.accepted ? ignored.receipt.events : []), /"C3"/);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === local.attackerInstanceId);
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
  await withNorthAttacksAtC2({ seed: 112, spell }, async (ctx, defend) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === defend.targetInstanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === defend.defenderInstanceId), false);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates), true);
  });

  await withNorthAttacksAtC2({ seed: 113, spell }, async (ctx) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
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
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B3');
  };

  await withNorthAttacksAtC2({
    seed: 118,
    spell: ranged,
    southSpell: airborne,
  }, async (ctx, ids) => {
    assert.equal(await canTarget(ctx, ids.targetInstanceId), false);
  });

  await withNorthAttacksAtC2({
    seed: 119,
    spell: airborne,
    southSpell: airborne,
  }, async (ctx, ids) => {
    assert.equal(await canTarget(ctx, ids.targetInstanceId), true);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'intercept');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept'
        && descriptor.unitInstanceId === ids.targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 120,
    spell: airborne,
    southSpell: ground,
  }, async (ctx, ids) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'main');
    await addDiagonalSite(ctx);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ids.attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,B3');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === ids.attackerInstanceId)?.location, 'B3');
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 121,
    spell: airborne,
    southSpell: ranged,
  }, async (ctx, ids) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'intercept');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'intercept' && descriptor.unitInstanceId === ids.targetInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 122,
    spell: ground,
    southSpell: ground,
  }, async (ctx, ids) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await addDiagonalSite(ctx);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ids.attackerInstanceId
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

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const sourceSiteId = ctx.state.realm.sites.C4?.instanceId;
    const minionId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(sourceSiteId);
    assert.ok(minionId);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'fly-site'), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');

    const manaBefore = ctx.state.players.north.mana;
    const fly = await ctx.action(({ descriptor }) => descriptor.kind === 'fly-site'
      && descriptor.sourceSiteInstanceId === sourceSiteId
      && descriptor.targetCell === 'D4');
    const result = await ctx.step(fly);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    assert.equal(ctx.state.realm.sites.C4, undefined);
    assert.equal(ctx.state.realm.sites.D4?.instanceId, sourceSiteId);
    assert.equal(ctx.state.players.north.avatar.location, 'D4');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === minionId)?.location, 'D4');
    assert.equal(ctx.state.players.north.mana, manaBefore);
    assert.equal(result.receipt.events.some(({ type }) => type === 'site-flown'), true);
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
  let updraftId: string | undefined;
  let airborneId: string | undefined;
  let groundId: string | undefined;
  await withPreview(base, async (preview) => {
    updraftId = preview.state.players.north.hand.atlas[0]?.cardId;
    airborneId = preview.state.players.north.hand.spellbook[0]?.cardId;
    groundId = preview.state.players.north.hand.spellbook[1]?.cardId;
  });
  assert.ok(updraftId);
  assert.ok(airborneId);
  assert.ok(groundId);
  const updraftCardId = updraftId;
  const airborneCardId = airborneId;
  const groundCardId = groundId;
  const gameManifest = createGameManifest({
    authority: base.authority,
    cards: {
      ...base.cards,
      [airborneCardId]: { ...base.cards[airborneCardId], airborne: true } as GameCardDefinition,
      [updraftCardId]: {
        ...base.cards[updraftCardId],
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
      [updraftCardId]: {
        ...gameManifest.cards[updraftCardId],
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cardId === updraftCardId && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === airborneCardId);
    const airborneInstanceId = ctx.state.realm.units.at(-1)?.instanceId;
    assert.ok(airborneInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === groundCardId);
    const groundInstanceId = ctx.state.realm.units.at(-1)?.instanceId;
    assert.ok(groundInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const attackerInstanceId = ctx.state.realm.units.at(-1)?.instanceId;
    assert.ok(attackerInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    const hasPath = async (unitInstanceId: string): Promise<boolean> =>
      (await ctx.legalActions('north')).some(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unitInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
    assert.equal(await hasPath(airborneInstanceId), true);
    assert.equal(await hasPath(groundInstanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === groundInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2'), false);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === airborneInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === airborneInstanceId)?.location, 'C2');
    assert.equal(await ctx.verifyReplay(), true);
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
  let northGroundCardId: string | undefined;
  let northAirborneCardId: string | undefined;
  let southGroundCardId: string | undefined;
  let southAirborneCardId: string | undefined;
  await withPreview(base, async (preview) => {
    northGroundCardId = preview.state.players.north.hand.spellbook[0]?.cardId;
    northAirborneCardId = preview.state.players.north.hand.spellbook[1]?.cardId;
    southGroundCardId = preview.state.players.south.hand.spellbook[0]?.cardId;
    southAirborneCardId = preview.state.players.south.hand.spellbook[1]?.cardId;
  });
  assert.ok(northGroundCardId);
  assert.ok(northAirborneCardId);
  assert.ok(southGroundCardId);
  assert.ok(southAirborneCardId);
  const northGroundId = northGroundCardId;
  const northAirborneId = northAirborneCardId;
  const southGroundId = southGroundCardId;
  const southAirborneId = southAirborneCardId;
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
      [northAirborneId]: airborne(northAirborneId),
      [southAirborneId]: airborne(southAirborneId),
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
      .find(({ cardId }) => cardId === northGroundId);
    const northAirborne = ctx.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === northAirborneId);
    assert.ok(northGround);
    assert.ok(northAirborne);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === northGround.instanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === northAirborne.instanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const southGround = ctx.state.players.south.hand.spellbook
      .find(({ cardId }) => cardId === southGroundId);
    assert.ok(southGround);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === southGround.instanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northGround.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northAirborne.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const southAirborne = ctx.state.players.south.hand.spellbook
      .find(({ cardId }) => cardId === southAirborneId);
    assert.ok(southAirborne);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === southAirborne.instanceId
        && descriptor.cell === 'C2');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C2'), true);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = await ctx.legalActions('north');
    const entersC2 = (instanceId: string): boolean => moves.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2');
    assert.equal(entersC2(northGround.instanceId), false);
    assert.equal(entersC2(northAirborne.instanceId), true);
    assert.equal(moves.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'), true);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northAirborne.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === southAirborne.instanceId);
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === southGround.instanceId), false);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.equal(await ctx.verifyReplay(), true);
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
  await withNorthAttacksAtC2({
    seed: 123,
    spell: stealth,
    southSpell: ground,
  }, async (ctx, setup) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === setup.attackerInstanceId)?.stealthed, true);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.targetInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.attackerInstanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'defend-window-closed'), false);
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === setup.attackerInstanceId)?.stealthed, false);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === setup.targetInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.attackerInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 124,
    spell: { ...ground, ranged: true, stealth: true },
    southSpell: stealth,
  }, async (ctx, ranged) => {
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === ranged.targetInstanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const shots = (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'shoot-projectile');
    assert.equal(shots.some(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile'
        && descriptor.hit?.instanceId === ranged.targetInstanceId), false);
    const miss = shots.find(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile' && descriptor.direction === 'north');
    assert.ok(miss);
    await ctx.accept(miss);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === ranged.attackerInstanceId)?.stealthed, false);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === ranged.targetInstanceId)?.stealthed, true);
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-lost'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion');
    assert.equal(ctx.state.realm.units[0]?.stealthed, false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units[0]?.stealthed, true);
    assert.deepEqual(
      ctx.session.transcript.at(-1)?.events.map(({ type }) => type),
      ['stealth-gained', 'turn-ended', 'turn-started'],
    );
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'stealth-gained'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});
