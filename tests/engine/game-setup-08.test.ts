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
} from '../../src/engine/game.ts';
import {
  accept,
  action,
  cardsFor,
  deck,
  keep,
  manifest,
  peekOpening,
  SYNTHETIC_AUTHORITY_HASH,
  takeAction,
  withDevilsEggFixture,
} from './game-setup-helpers.ts';
import { createGameCheckpoint } from '../../src/engine/checkpoint.ts';
import { withSetup, type SetupCtx } from './rust-setup-session.ts';

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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'fatality-target' && descriptor.cell === 'C4');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === 'fatality-target');
    assert.ok(target);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId
      && descriptor.path.length === 1);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === target.instanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    const checkpoint = ctx.session;
    const wounded = checkpoint.state.realm.units.find(({ instanceId }) =>
      instanceId === target.instanceId);
    const fatality = checkpoint.state.players.north.hand.spellbook
      .find(({ cardId }) => cardId === 'fatality');
    assert.equal(wounded?.damage, 1);
    assert.ok(fatality);

    // A healthy/Stealthed-enemy/underground/Stealthed-ally set of extra targets, and a Warded
    // copy of the wounded target, are not reachable through legal play. The shared magic target
    // filter (exclude enemy Stealth, require the caster's region, allow Stealthed allies) is
    // proven through legal play in
    // `rule_catalog_0023_magic_targets_should_stay_in_the_caster_region_and_exclude_enemy_stealth`
    // in crates/sorcery-engine/tests/magic_rules.rs; the avatar-never-a-target and
    // wounded-only filters are proven alongside the kill itself in
    // `rule_catalog_0147_fatality_should_kill_only_a_wounded_minion_in_the_caster_region` in the
    // same file. Ward absorbing the kill outright (breaking instead of killing, the target
    // surviving with its prior damage unchanged) is proven directly on `Position` in
    // `kill_target_wounded_minion_should_break_ward_instead_of_killing` in
    // crates/sorcery-engine/src/game.rs `mod tests`.

    const cast = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === target.instanceId);
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
      cardId: target.cardId,
      instanceId: target.instanceId,
      owner: 'south',
      seat: 'south',
      sourceInstanceId,
    });
    assert.equal(killed.receipt.events.some(({ type }) => type === 'damage-dealt'), false);
    assert.deepEqual(killed.receipt.randomDraws, []);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === target.instanceId), false);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === target.instanceId), true);
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
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
  const preview = (await peekOpening(gameManifest)).state.players.north;
  const opening = preview.hand.spellbook.map(({ cardId }) => cardId);
  assert.equal(['sparkmage-caster', 'sparkmage-artifact', 'sparkmage-magic']
    .every((cardId) => opening.includes(cardId)), true);
  assert.equal(preview.spellbook[0]?.cardId, 'sparkmage-magic');
  assert.deepEqual(gameManifest.cards['sparkmage-avatar'], {
    attack: 4,
    cardType: 'avatar',
    defense: 4,
    drawSpell: false,
    life: 20,
    tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true,
  });

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'sparkmage-caster' && descriptor.cell === 'C4');
    const caster = ctx.state.realm.units.find(({ cardId }) => cardId === 'sparkmage-caster');
    assert.ok(caster);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 1);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sparkmage-artifact'
      && descriptor.casterInstanceId === caster.instanceId
      && descriptor.bearer?.instanceId === caster.instanceId);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 2);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'sparkmage-magic'
      && descriptor.casterInstanceId === caster.instanceId);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 3);
    const opponentView = ctx.observe('south');
    assert.equal(opponentView.players.north.airThresholdsCastThisTurn, 3);
    assert.equal(typeof opponentView.players.north.hand.spellbook, 'number');

    // A nonzero South counter mid-North-turn is not reachable through legal play (South cannot
    // act during North's turn). Rust proves end-turn resets `airThresholdsCastThisTurn` for both
    // seats, not only the seat whose turn ended, directly on `Position` in
    // `end_turn_should_reset_air_thresholds_cast_this_turn_for_both_seats` in
    // crates/sorcery-engine/src/game.rs `mod tests`.

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 0);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'sparkmage-magic'
      && descriptor.casterInstanceId === caster.instanceId);
    assert.equal(ctx.state.players.north.airThresholdsCastThisTurn, 1);

    const activation = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-sparkmage'
        && descriptor.targetLocation.cell === 'C4'
        && descriptor.targetLocation.region === 'surface');
    assert.doesNotMatch(canonicalJson(activation.descriptor), new RegExp(caster.instanceId));
    const result = await ctx.step(activation);
    assert.equal(result.accepted, true);
    if (!result.accepted) return;

    assert.equal(ctx.state.players.north.avatar.tapped, true);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === caster.instanceId)?.damage, 0);
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
      instanceId: caster.instanceId,
      prevented: true,
      seat: 'north',
    });
    assert.match(canonicalJson(result.receipt.events[0]!.payload), new RegExp(caster.instanceId));
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
  const preview = (await peekOpening(gameManifest)).state.players.north;
  assert.ok(preview.hand.spellbook.filter(({ cardId }) =>
    cardId === 'sparkmage-many-target').length >= 2);
  assert.equal(preview.spellbook[0]?.cardId, 'sparkmage-many-magic');

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const targets = ctx.state.players.north.hand.spellbook
      .filter(({ cardId }) => cardId === 'sparkmage-many-target')
      .slice(0, 2);
    assert.equal(targets.length, 2);
    for (const target of targets) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === target.instanceId
        && descriptor.cell === 'C4');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === 'sparkmage-many-magic');

    const activation = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-sparkmage'
        && descriptor.targetLocation.cell === 'C4'
        && descriptor.targetLocation.region === 'surface');
    for (const target of targets) {
      assert.doesNotMatch(canonicalJson(activation.descriptor), new RegExp(target.instanceId));
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
    const selected = targets.find(({ instanceId }) => activatedJson.includes(instanceId));
    assert.ok(selected);
    assert.match(damagedJson, new RegExp(selected.instanceId));
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

  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 16_384; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const preview = (await peekOpening(candidate)).state.players;
    const northAtlas = preview.north.hand.atlas.map(({ cardId }) => cardId);
    const northSpells = preview.north.hand.spellbook.map(({ cardId }) => cardId);
    const southSpells = preview.south.hand.spellbook.map(({ cardId }) => cardId);
    if (northAtlas.includes(towerId)
      && northAtlas.includes(nonTowerId)
      && northSpells.includes(conditionalId)
      && northSpells.includes(magicId)
      && [targetId, disableMagicId].every((cardId) =>
        southSpells.includes(cardId))) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === towerId && descriptor.cell === 'C4');
    const beforeSurfaceSummon = createGameCheckpoint(ctx.session);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === conditionalId && descriptor.cell === 'C4' && !descriptor.region);
    const conditional = ctx.state.realm.units.find(({ cardId }) => cardId === conditionalId);
    assert.ok(conditional);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === targetId && descriptor.cell === 'C4');
    const target = ctx.state.realm.units.find(({ cardId }) => cardId === targetId);
    assert.ok(target);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === nonTowerId && descriptor.cell === 'C3');

    const atopTower = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === conditional.instanceId);
    assert.deepEqual({ attack: atopTower?.attack, defense: atopTower?.defense }, {
      attack: 3,
      defense: 3,
    });
    const tower = ctx.state.realm.sites.C4;
    assert.ok(tower && !('rubble' in tower));
    const foreignTowerState = {
      ...ctx.state,
      realm: {
        ...ctx.state.realm,
        sites: {
          ...ctx.state.realm.sites,
          C4: { ...tower, controller: 'south' as const },
        },
      },
    };
    const atopForeignTower = observeGame(foreignTowerState, 'north').realm.units
      .find(({ instanceId }) => instanceId === conditional.instanceId);
    assert.deepEqual({ attack: atopForeignTower?.attack, defense: atopForeignTower?.defense }, {
      attack: 3,
      defense: 3,
    });
    const towerActions = await ctx.legalActions('north');
    assert.equal(towerActions.some(({ descriptor }) => descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === conditional.instanceId
      && descriptor.hit?.instanceId === target.instanceId), true);
    assert.equal(towerActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === magicId
      && descriptor.casterInstanceId === conditional.instanceId), true);

    const towerCheckpoint = createGameCheckpoint(ctx.session);

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === conditional.instanceId
      && descriptor.from.cell === 'C4'
      && descriptor.to.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    const offTower = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === conditional.instanceId);
    assert.deepEqual({ attack: offTower?.attack, defense: offTower?.defense }, {
      attack: 1,
      defense: 1,
    });
    const offTowerActions = await ctx.legalActions('north');
    assert.equal(offTowerActions.some(({ descriptor }) => descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === conditional.instanceId), false);
    assert.equal(offTowerActions.some(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.casterInstanceId === conditional.instanceId), false);
    assert.equal(ctx.session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(towerCheckpoint);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === disableMagicId
      && descriptor.casterInstanceId === target.instanceId
      && descriptor.target?.instanceId === conditional.instanceId);
    const disabled = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === conditional.instanceId);
    assert.deepEqual({
      attack: disabled?.attack,
      defense: disabled?.defense,
      disabled: disabled?.disabled,
    }, { attack: 1, defense: 1, disabled: true });
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw');
    assert.equal(ctx.state.activeSeat, 'north');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.casterInstanceId === conditional.instanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw');
    const awakened = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === conditional.instanceId);
    assert.deepEqual({
      attack: awakened?.attack,
      defense: awakened?.defense,
      disabled: awakened?.disabled,
    }, { attack: 3, defense: 3, disabled: false });
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === magicId
      && descriptor.casterInstanceId === conditional.instanceId
      && descriptor.target?.instanceId === conditional.instanceId);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === conditional.instanceId)?.damage, 1);
    const destroyed = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-site-destruction'
        && descriptor.sourceSiteInstanceId === ctx.state.realm.sites.C3?.instanceId
        && descriptor.targetCell === 'C4'));
    assert.equal(destroyed.accepted, true);
    if (destroyed.accepted) {
      assert.equal(destroyed.receipt.events.some(({ payload, type }) =>
        type === 'minion-died'
          && canonicalJson(payload).includes(conditional.instanceId)), true);
      assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
        instanceId === conditional.instanceId), false);
      assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === conditional.instanceId), true);
      assert.equal(destroyed.receipt.randomDraws.length, 0);
      assert.equal(await ctx.verifyReplay(), true);
    }

    await ctx.resume(beforeSurfaceSummon);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === conditionalId
      && descriptor.cell === 'C4'
      && descriptor.region === 'underground');
    const buried = ctx.state.realm.units.find(({ cardId }) => cardId === conditionalId);
    assert.ok(buried);
    const buriedObserved = ctx.observe('north').realm.units
      .find(({ instanceId }) => instanceId === buried.instanceId);
    assert.deepEqual({ attack: buriedObserved?.attack, defense: buriedObserved?.defense }, {
      attack: 1,
      defense: 1,
    });
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.casterInstanceId === buried.instanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const sourceCard = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === 'nimbus-source');
    assert.ok(sourceCard);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
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
    if (repeat.accepted) {
      assert.equal(ctx.state.players.north.avatar.life, 14);
      assert.equal(repeat.receipt.randomDraws.length, 1);
      assert.equal(await ctx.verifyReplay(), true);
    }
  });

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'nimbus-source'
      && descriptor.cell === 'C3');
    const emptySource = ctx.state.realm.units.find(({ cardId }) =>
      cardId === 'nimbus-source');
    assert.ok(emptySource);
    const emptyResult = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'activate-discard-random-damage'
        && descriptor.sourceInstanceId === emptySource.instanceId));
    assert.equal(emptyResult.accepted, true);
    if (emptyResult.accepted) {
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
    }
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'nimbus-many-ally' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'nimbus-many-enemy' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
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

    // A disabled or removed source is not reachable through legal play (activation is only ever
    // offered for a unit that is actually on the board and not Disabled). Both facts follow from
    // the shared legality gate every activated ability uses: `legal_actions` only ever iterates
    // `Position.units` (so a removed unit can never produce an action), and every per-unit
    // ability skips a unit for which `minion_is_disabled` is true, in
    // crates/sorcery-engine/src/game.rs. The Disabled branch of that shared gate is proven
    // through legal play (for a sibling ability using the same gate) in
    // `disabled_stealth_should_be_visible_but_disabled_shooter_cannot_fire` in
    // crates/sorcery-engine/tests/damage_projectile_rules.rs.
    const forgedDescriptor = { ...activation.descriptor, targetInstanceId: enemy.instanceId };
    const beforeForge = ctx.stateHash();
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

    // An oversized enemy footprint at the source's cell is not reachable through legal play here
    // (this deck has no oversized card), but it cannot inflate the random candidate pool anyway:
    // `units_at_location` in crates/sorcery-engine/src/game.rs iterates `Position.units` once per
    // unit regardless of how many cells `occupied_cells` spans, so an oversized unit still
    // contributes exactly one candidate. Unit-based (not cell-based) candidate counting is proven
    // through legal play in
    // `rule_catalog_0153_random_other_unit_candidates_should_include_allies_avatars_and_stealth`
    // in crates/sorcery-engine/tests/discard_random_damage_rules.rs.

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
    result: Extract<Awaited<ReturnType<SetupCtx['step']>>, { accepted: true }>;
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
    let output: Readonly<{
      result: Extract<Awaited<ReturnType<SetupCtx['step']>>, { accepted: true }>;
      targetId: string;
    }> | undefined;
    await withSetup(gameManifest, async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'nimbus-prevention-aura' && descriptor.cell === 'C4');
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
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === 'nimbus-prevention-target' && descriptor.cell === 'C3');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
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
      output = { result, targetId: target.instanceId };
    });
    assert.ok(output);
    return output;
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

test('RULE-03 Artifacts make their current site controller lose life at each turn end', async () => {
  await withDevilsEggFixture('both', 73, async (ctx, { gameManifest, ids }) => {
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

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    const southEggCard = ctx.state.players.south.hand.spellbook[0];
    assert.ok(southEggCard);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
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
  await withDevilsEggFixture('carried', 4, async (ctx, { ids }) => {
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

    // A disabled oversized carrier standing on a foreign site is not reachable through legal
    // play. Rust proves the same fact in `carried_egg_should_outlast_a_disabled_bearer` in
    // crates/sorcery-engine/tests/artifact_life_loss_rules.rs.
  });
});

test('RULE-03 end-turn Artifact life loss respects regions, Rubble, stacking, and Death\'s Door', () => {
  // This board (four artifacts stacked across surface/underground/void/Rubble, both avatars at
  // life 1) is not reachable through legal play. Rust proves the same facts directly on
  // `Position` in `end_turn_artifact_life_loss_should_respect_regions_rubble_stacking_and_deaths_door`
  // (stacking on one site through Death's Door, an Avatar already at the door only recording the
  // trigger) plus its `end_turn_artifact_life_loss_should_skip_rubble` (a Rubble cell charges
  // nobody) and `end_turn_artifact_life_loss_should_charge_a_submerged_bearers_site` (a
  // non-surface region still charges the site above it) scenarios, all in
  // crates/sorcery-engine/tests/artifact_life_loss_rules.rs. A Void-region loose artifact sharing
  // a cell with a live site (as this TS test forged) is unreachable in Rust too:
  // `settle_covered_layers` relayers a loose artifact out of Void the instant a site is played on
  // its cell, so no reachable state ever has both at once.
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

  const gameManifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-headless-start-turn-v1',
    },
    cards,
    decks: { north, south },
    firstSeat: 'north',
    seed: 10,
  });

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === luckyCharmCardId
      && descriptor.bearer?.kind === 'avatar');
    for (let count = 0; count < 2; count += 1) {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === sourceCardId
        && descriptor.cell === 'C4');
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === blockedSiteCardId
      && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'headless-blocker'
      && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    const startTurn = ctx.session;
    const branchPoint = createGameCheckpoint(startTurn);
    const sourceIds = startTurn.state.realm.units
      .filter(({ cardId }) => cardId === sourceCardId)
      .map(({ instanceId }) => instanceId)
      .sort();
    const triggers = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'resolve-start-turn-trigger');
    const firstTrigger = triggers.find(({ descriptor }) =>
      descriptor.kind === 'resolve-start-turn-trigger'
        && descriptor.sourceInstanceId === sourceIds[1]);
    assert.ok(firstTrigger);
    const committed = await ctx.step(firstTrigger);
    assert.equal(committed.accepted, true);
    if (!committed.accepted) return;
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'resolve-random-outcome');
    const blockedChoice = choices.find(({ label }) => label === 'Lucky Charm chooses C1 surface');
    assert.ok(blockedChoice);
    const blocked = await ctx.step(blockedChoice);
    assert.equal(blocked.accepted, true);
    if (!blocked.accepted) return;
    const blockedPoint = createGameCheckpoint(blocked.session);
    const secondTrigger = (await ctx.legalActions('north'))
      .find(({ descriptor }) => descriptor.kind === 'resolve-start-turn-trigger');
    assert.ok(secondTrigger);
    const secondCommitted = await ctx.step(secondTrigger);
    assert.equal(secondCommitted.accepted, true);
    if (!secondCommitted.accepted) return;
    const secondCommittedPoint = createGameCheckpoint(secondCommitted.session);
    let movedChoice: GameLegalAction | undefined;
    for (const choice of (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'resolve-random-outcome')) {
      const probe = await ctx.step(choice);
      const teleported = probe.accepted
        && probe.receipt.events.some(({ type }) => type === 'unit-teleported');
      await ctx.resume(secondCommittedPoint);
      if (teleported) {
        movedChoice = choice;
        break;
      }
    }
    assert.ok(movedChoice);
    assert.match(movedChoice.label, /^Lucky Charm chooses [A-E][1-4] (surface|void)$/);

    assert.equal(startTurn.state.phase, 'start-turn');
    assert.deepEqual(triggers.flatMap(({ descriptor }) =>
      descriptor.kind === 'resolve-start-turn-trigger'
        ? [descriptor.sourceInstanceId]
        : []).sort(), sourceIds);
    assert.equal(firstTrigger.descriptor.kind, 'resolve-start-turn-trigger');
    if (firstTrigger.descriptor.kind !== 'resolve-start-turn-trigger') return;
    assert.equal(firstTrigger.descriptor.sourceInstanceId, sourceIds[1]);
    assert.equal(startTurn.state.realm.units.some(({ cardId, location }) =>
      cardId === 'headless-blocker' && location === 'C1'), true);
    assert.equal(startTurn.state.cards[blockedSiteCardId]?.cardType === 'site'
      && startTurn.state.cards[blockedSiteCardId].preventsUnitsWithPowerAtLeastFromEntering, 3);

    await ctx.resume(branchPoint);
    const repeated = await ctx.step(firstTrigger);
    assert.equal(repeated.accepted, true);
    if (!repeated.accepted) return;
    assert.deepEqual(repeated.receipt, committed.receipt);
    assert.equal(committed.receipt.events.length, 0);
    assert.equal(committed.receipt.randomDraws.length, 2);
    assert.equal(committed.receipt.randomDraws.every(({ purpose }) =>
      purpose === 'start_turn_random_teleport'), true);
    assert.equal(committed.session.state.phase, 'random-choice');
    assert.equal(blockedChoice.descriptor.kind, 'resolve-random-outcome');
    if (blockedChoice.descriptor.kind !== 'resolve-random-outcome') return;
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
    assert.equal(blocked.session.state.phase, 'start-turn');
    assert.equal(blocked.session.state.realm.units.find(({ instanceId }) =>
      instanceId === sourceIds[1])?.location, 'C4');
    assert.equal(secondTrigger.descriptor.kind, 'resolve-start-turn-trigger');
    if (secondTrigger.descriptor.kind !== 'resolve-start-turn-trigger') return;
    assert.equal(secondTrigger.descriptor.sourceInstanceId, sourceIds[0]);

    await ctx.resume(blockedPoint);
    const beforeForgeHash = hashGameState(blocked.session.state);
    const beforeForgeTranscript = blocked.session.transcript.length;
    const forgedDescriptor = {
      kind: 'resolve-start-turn-trigger' as const,
      sourceInstanceId: sourceIds[1]!,
    };
    const forged = await ctx.stepRequest({
      actionId: opaqueActionId(
        'sorcery-core-v1',
        'north',
        blocked.session.state.stateVersion,
        forgedDescriptor,
      ),
      seat: 'north',
      stateVersion: blocked.session.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(hashGameState(forged.session.state), beforeForgeHash);
    assert.equal(forged.session.transcript.length, beforeForgeTranscript);

    const committedAfterForge = await ctx.step(secondTrigger);
    assert.equal(committedAfterForge.accepted, true);
    if (!committedAfterForge.accepted) return;
    assert.deepEqual(committedAfterForge.receipt, secondCommitted.receipt);
    const moved = await ctx.step(movedChoice);
    assert.equal(moved.accepted, true);
    if (!moved.accepted) return;
    assert.equal(moved.receipt.events.some(({ payload, type }) =>
      type === 'unit-teleported'
        && canonicalJson(payload).includes(sourceIds[0]!)
        && canonicalJson(payload).includes('"region":"void"')), true);
    assert.deepEqual(moved.receipt.randomDraws, []);
    assert.equal(moved.session.state.phase, 'draw');
    assert.equal(moved.session.state.pendingStartTurn, undefined);
    assert.equal(moved.session.state.realm.units.find(({ instanceId }) =>
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
  await withSetup(emptyManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    assert.equal(ctx.state.players.north.hand.spellbook.some(({ cardId }) =>
      cardId === raiseDeadId), true);
    const emptyCast = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.cardId === raiseDeadId));
    assert.equal(emptyCast.accepted, true);
    if (!emptyCast.accepted) return;
    assert.deepEqual(emptyCast.receipt.events.map(({ type }) => type), ['magic-cast', 'magic-resolved']);
    assert.deepEqual(emptyCast.receipt.randomDraws, []);
    assert.equal(ctx.state.phase, 'main');
    assert.equal(await ctx.verifyReplay(), true);
  });

  // TODO(rust-cutover): this one branch stays on the legacy synchronous engine. The Rust
  // engine never applies `preventsUnitsWithPowerAtLeastFromEntering` to summons: TS filters
  // every summon destination through `unitEntryAllowed(..., 'summon')` (src/engine/game.ts
  // summonLocations), while Rust's `free_summon_destinations`/`summon_regions`
  // (crates/sorcery-engine/src/game.rs) consult only `surface_location_exists`, so
  // `unit_entry_allowed` is reached from movement and teleport but never from a summon.
  // Reproduction: with `blockedManifest` below, after `resolve-random-outcome` selects the
  // South corpse (attack 2) Rust issues `Raise raise-dead-south-corpse at C1 (free)` and
  // `... at C4 (free)` and parks in `cemetery-summon`, where TS issues no placement and
  // resolves `magic-cast, dead-minion-selected, minion-summon-failed, magic-resolved` back
  // into `main`. Needs the Rust summon gate (and a Rust proof) before it can move.
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
  let blocked = keep(keep(createGameSession(blockedManifest)));
  const blockedTake = (predicate: Parameters<typeof action>[1]): void => {
    blocked = accept(blocked, action(blocked, predicate));
  };
  blockedTake(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  blockedTake(({ descriptor }) => descriptor.kind === 'cast-artifact'
    && descriptor.cardId === luckyCharmId && descriptor.bearer?.kind === 'avatar');
  blockedTake(({ descriptor }) => descriptor.kind === 'end-turn');
  blockedTake(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  blockedTake(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  blockedTake(({ descriptor }) => descriptor.kind === 'end-turn');
  blockedTake(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
  const blockedCorpse = blocked.state.players.south.cemetery.find(({ cardId }) =>
    cardId === southCorpseId);
  assert.ok(blockedCorpse);
  const blockedCast = stepGame(blocked, action(blocked, ({ descriptor }) =>
    descriptor.kind === 'cast-magic' && descriptor.cardId === raiseDeadId));
  assert.equal(blockedCast.accepted, true);
  if (!blockedCast.accepted) return;
  const blockedChoice = legalGameActions(blockedCast.session.state, 'north')
    .find(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
      && descriptor.outcomeInstanceId === blockedCorpse.instanceId);
  assert.ok(blockedChoice);
  const failedPlacement = stepGame(blockedCast.session, blockedChoice);
  assert.equal(failedPlacement.accepted, true);
  if (!failedPlacement.accepted) return;
  assert.equal(failedPlacement.session.state.phase, 'main');
  assert.deepEqual(failedPlacement.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'dead-minion-selected',
    'minion-summon-failed',
    'magic-resolved',
  ]);
  assert.equal(failedPlacement.session.state.players.south.cemetery.some(({ instanceId }) =>
    instanceId === blockedCorpse.instanceId), true);
  assert.equal(verifyGameReplay(failedPlacement.session), true);

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === luckyCharmId
      && descriptor.bearer?.kind === 'avatar');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');

    const northCorpse = ctx.state.players.north.cemetery.find(({ cardId }) =>
      cardId === northCorpseId);
    const southCorpse = ctx.state.players.south.cemetery.find(({ cardId }) =>
      cardId === southCorpseId);
    const raiseDead = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === raiseDeadId);
    assert.ok(northCorpse && southCorpse && raiseDead);
    const corpseIds = [northCorpse.instanceId, southCorpse.instanceId].sort();
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

    const beforeCast = createGameCheckpoint(ctx.session);
    const committed = await ctx.step(raiseCast);
    assert.equal(committed.accepted, true);
    if (!committed.accepted) return;
    const committedReceipt = committed.receipt;
    await ctx.resume(beforeCast);
    const repeated = await ctx.step(raiseCast);
    assert.equal(repeated.accepted, true);
    if (!repeated.accepted) return;
    assert.deepEqual(repeated.receipt, committedReceipt);
    const afterCast = createGameCheckpoint(ctx.session);
    const choices = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'resolve-random-outcome');
    assert.equal(ctx.state.phase, 'random-choice');
    assert.deepEqual(choices.flatMap(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
      ? [descriptor.outcomeInstanceId]
      : []).sort(), corpseIds);
    assert.equal(choices.every(({ label }) => label.startsWith('Lucky Charm chooses raise-dead-')), true);
    assert.deepEqual(committedReceipt.events, []);
    assert.equal(committedReceipt.randomDraws.length, 2);
    assert.equal(committedReceipt.randomDraws.every(({ purpose }) =>
      purpose === 'magic_random_dead_minion'), true);

    const northChoice = choices.find(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
      && descriptor.outcomeInstanceId === northCorpse.instanceId);
    assert.ok(northChoice);
    const northSelected = await ctx.step(northChoice);
    assert.equal(northSelected.accepted, true);
    if (!northSelected.accepted) return;
    const genesisLabels = (await ctx.legalActions('north'))
      .filter(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4')
      .map(({ label }) => label);
    assert.equal(genesisLabels.some((label) => label.endsWith('; decline Genesis')), true);
    assert.equal(genesisLabels.some((label) => label.includes('; Genesis targets avatar ')), true);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(afterCast);
    const forcedChoice = choices.find(({ descriptor }) => descriptor.kind === 'resolve-random-outcome'
      && descriptor.outcomeInstanceId === southCorpse.instanceId);
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
      instanceId === southCorpse.instanceId), true);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === northCorpse.instanceId), true);

    const placements = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'summon-minion');
    assert.deepEqual([...new Set(placements.flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion' ? [descriptor.cell] : []))].sort(), ['C1', 'C4']);
    assert.equal(placements.every(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === southCorpse.instanceId
      && descriptor.manaCost === 0), true);
    const placement = placements.find(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C1' && !descriptor.region);
    assert.ok(placement);
    assert.equal(placement.descriptor.kind, 'summon-minion');
    if (placement.descriptor.kind !== 'summon-minion') return;
    const beforePlacementMana = ctx.state.players.north.mana;
    const beforePlacementLife = ctx.state.players.north.avatar.life;
    const beforeForgeHash = ctx.stateHash();
    const beforeForgeTranscript = ctx.session.transcript;
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
    assert.deepEqual(ctx.session.transcript, beforeForgeTranscript);

    const placed = await ctx.step(placement);
    assert.equal(placed.accepted, true);
    if (!placed.accepted) return;
    const raised = ctx.state.realm.units.find(({ instanceId }) =>
      instanceId === southCorpse.instanceId);
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
    assert.equal(ctx.state.players.north.avatar.life, beforePlacementLife - 2);
    assert.equal(ctx.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === southCorpse.instanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === northCorpse.instanceId), true);
    assert.deepEqual(placed.receipt.randomDraws, []);
    assert.deepEqual(placed.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'avatar-life-lost',
      'magic-resolved',
    ]);

    const reusedHash = ctx.stateHash();
    const reusedTranscript = ctx.session.transcript;
    const reused = await ctx.step(placement);
    assert.equal(reused.accepted, false);
    if (!reused.accepted) assert.equal(reused.reason.code, 'stale_version');
    assert.equal(ctx.stateHash(), reusedHash);
    assert.deepEqual(ctx.session.transcript, reusedTranscript);
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
  // The rest of this board (seven units seeded directly across surface/underground/underwater/
  // void/an oversized footprint, a discard-cost forgery, a protected target site, and a repeat
  // + stale-version replay) is not reachable through legal play. Rust proves every one of those
  // facts directly on `Position` in
  // `craterize_should_discard_a_site_destroy_its_target_and_apply_its_damage_grid` in
  // crates/sorcery-engine/src/game.rs `mod tests`: the mandatory discard cost (an empty Atlas
  // hand offers no cast at all), the discard-cost forgery rejected, the protected-site branch
  // (`site-destruction-prevented`, no `rubble-created`, the site and its centre unit's damage
  // unchanged), the printed damage grid landing on every unit (including the oversized and Void
  // units), the region settling (submerged -> underground once its Water layer is destroyed),
  // Stealth and Ward surviving/breaking, avatar life and mana, both cemeteries, the Rubble
  // identity, and a repeated cast reproducing byte-identical events and state. Generic
  // stale-version rejection after a state-advancing action is proven separately, e.g. in
  // crates/sorcery-engine/tests/site_destruction_rules.rs and magic_rules.rs.
});
