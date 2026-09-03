import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpoint,
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
  verifyGameReplay,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameLegalAction,
  type GameSession,
} from '../../src/engine/game.ts';
import {
  accept,
  action,
  deck,
  keep,
  manifest,
  northAttacksAtC2,
  northSecondMain,
  SYNTHETIC_AUTHORITY_HASH,
  takeAction,
} from './game-setup-helpers.ts';
import { withSetup } from './rust-setup-session.ts';

test('RULE-03 Aura occupies any canonical 2x2 area and grounds site minions for three controller turns', async () => {
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === northSiteId && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === airborneCardId && descriptor.cell === 'C4'
      && descriptor.region === undefined);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
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
    await take(({ descriptor }) => descriptor.kind === 'cast-aura'
      && descriptor.cells.every((cell, index) => cell === affectedCells[index]));

    const auraInstance = ctx.state.realm.auras?.[0];
    const airborne = ctx.state.realm.units.find(({ cardId }) => cardId === airborneCardId);
    const burrowing = ctx.state.realm.units.find(({ cardId }) => cardId === burrowingCardId);
    assert.ok(auraInstance);
    assert.ok(airborne);
    assert.ok(burrowing);
    let view = observeGame(ctx.state, 'north');
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

    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(observeGame(ctx.state, 'north').realm.auras?.[0]?.turnCounters, 1);
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === southSiteId && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === voidwalkCardId && descriptor.cell === 'B3'
      && descriptor.region === 'void');
    const voidwalk = ctx.state.realm.units.find(({ cardId }) => cardId === voidwalkCardId);
    assert.ok(voidwalk);
    view = observeGame(ctx.state, 'south');
    assert.equal(view.realm.units.find(({ instanceId }) =>
      instanceId === voidwalk.instanceId)?.immobile, false);

    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(observeGame(ctx.state, 'north').realm.auras?.[0]?.turnCounters, 2);
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');

    view = observeGame(ctx.state, 'north');
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

test('RULE-03 an end-turn Aura damages a random affected unit before its optional step', async () => {
  // The Rust engine's PRNG stream for this fixture diverges from the legacy TS
  // engine's, so the fixed seed that used to yield two distinct random Aura
  // outcomes no longer reliably does; search seeds like the Lucky Charm proof
  // in game-setup-02.test.ts ("commits the random action before exposing two
  // deterministic outcomes").
  let succeeded = false;
  for (let seed = 1; seed <= 50 && !succeeded; seed += 1) {
    const base = manifest(seed);
    const preview = createGameSession(base);
    const [luckyCharmId, auraCardId, minionCardId] =
      preview.state.players.north.hand.spellbook.map(({ cardId }) => cardId);
    const siteCardId = preview.state.players.north.hand.atlas[0]?.cardId;
    assert.ok(luckyCharmId && auraCardId && minionCardId && siteCardId);
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
    const gameManifest = createGameManifest({
      authority: base.authority,
      cards,
      decks: base.decks,
      firstSeat: base.firstSeat,
      seed,
    });

    await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === siteCardId && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === luckyCharmId && descriptor.bearer?.kind === 'avatar');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === minionCardId && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-aura'
      && descriptor.cardId === auraCardId
      && descriptor.cells.join(',') === 'B3,B4,C3,C4');
    const beforeEndTurn = createGameCheckpoint(ctx.session);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    const committed = createGameCheckpoint(ctx.session);

    if ((await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'resolve-end-turn-aura-random').length !== 2) {
      return;
    }
    succeeded = true;
    await ctx.resume(beforeEndTurn);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'resolve-end-turn-aura-random'), false);
    await ctx.resume(committed);
    assert.equal(ctx.state.phase, 'end-turn-aura');
    const committedTranscript = ctx.session.transcript.at(-1)!;
    assert.equal(committedTranscript.events.some(({ type }) => type === 'damage-dealt'), false);
    assert.equal(committedTranscript.events.some(({ type }) => type === 'turn-ended'), false);
    assert.equal(committedTranscript.randomDraws.length, 2);
    assert.equal(committedTranscript.randomDraws.every(({ purpose }) =>
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
    const beforeForgeHash = ctx.stateHash();
    const forged = await ctx.stepRequest({
      actionId: opaqueActionId('sorcery-core-v1', 'north', ctx.state.stateVersion, forgedDescriptor),
      seat: 'north',
      stateVersion: ctx.state.stateVersion,
    });
    assert.equal(forged.accepted, false);
    if (!forged.accepted) assert.equal(forged.reason.code, 'unknown_action');
    assert.equal(hashGameState(forged.session.state), beforeForgeHash);

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'resolve-end-turn-aura-move'
        && descriptor.cells?.join(',') === 'C3,C4,D3,D4');
    assert.deepEqual(ctx.state.realm.auras?.[0]?.cells, ['C3', 'C4', 'D3', 'D4']);
    assert.equal(ctx.state.phase, 'draw');
    assert.equal(ctx.session.transcript.at(-1)?.events.some(({ type }) => type === 'turn-ended'), true);

    for (let controllerTurn = 2; controllerTurn <= 3; controllerTurn += 1) {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      if (!ctx.state.players.south.domainEstablished) {
        await takeAction(ctx, ({ descriptor }) =>
          descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      }
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'resolve-end-turn-aura-random');
      await takeAction(ctx, ({ descriptor }) =>
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
    });
  }
  assert.equal(succeeded, true, 'expected a seed producing two distinct end-turn Aura random outcomes');
});

// TODO(rust-cutover): the Rust engine cannot compute legal actions for TeleportAllyToTargetSite
// or TeleportNearbyAllyThenDrawCard once the controller has any occupiesSquareArea minion
// (crates/sorcery-engine/src/game.rs GameError::UnsupportedManifestFact("teleportAllyToTargetSite:occupiesSquareArea")),
// so this proof stays on the legacy synchronous engine until that gap is closed.
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

    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const top = ctx.state.players.north.atlas[0];
    const swap = ctx.state.players.north.hand.atlas[0];
    assert.ok(top);
    assert.ok(swap);
    const before = canonicalJson(observeGame(ctx.state, 'north') as unknown as JsonValue);
    assert.equal(before.includes(top.cardId), false);
    assert.equal(before.includes(top.instanceId), false);
    const replacement = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'replace-rubble-with-top-atlas-site'
        && descriptor.targetCell === 'C3');
    assert.equal(replacement.label, 'Replace Rubble at C3 with the top site of your Atlas');
    assert.equal(canonicalJson(replacement as unknown as JsonValue).includes(top.cardId), false);
    assert.equal(canonicalJson(replacement as unknown as JsonValue).includes(top.instanceId), false);
    const replacementDeathrites = ctx.state.players.north.hand.spellbook.slice(0, 2);
    assert.equal(replacementDeathrites.length, 2);
    const replacementDeathriteIds = replacementDeathrites.map(({ instanceId }) => instanceId);
    const orderedReplacementState = {
      ...ctx.state,
      cards: {
        ...ctx.state.cards,
        [top.cardId]: {
          ...ctx.state.cards[top.cardId]!,
          elements: ['water'],
        } as GameCardDefinition,
        'north-minion': {
          ...ctx.state.cards['north-minion']!,
          burrowing: true,
          deathriteDrawSite: true,
        } as GameCardDefinition,
      },
      players: {
        ...ctx.state.players,
        north: {
          ...ctx.state.players.north,
          hand: {
            ...ctx.state.players.north.hand,
            spellbook: ctx.state.players.north.hand.spellbook.filter(({ instanceId }) =>
              !replacementDeathriteIds.includes(instanceId)),
          },
        },
      },
      realm: {
        ...ctx.state.realm,
        units: [...ctx.state.realm.units, ...replacementDeathrites.map((card) => ({
          ...card,
          controller: 'north' as const,
          damage: 0,
          location: 'C3' as const,
          region: 'underground' as const,
          stealthed: false,
          summoningSickness: false,
          tapped: false,
          warded: false,
        }))],
      },
    };
    assert.equal(orderedReplacementState.phase, 'main');
    assert.equal(orderedReplacementState.pendingGenesisToken ?? null, null);
    const blockedTopDefinition = orderedReplacementState.cards[top.cardId];
    assert.equal(blockedTopDefinition?.cardType === 'site'
      && blockedTopDefinition.elements.includes('water')
      && blockedTopDefinition.genesisPayOneManaToSummonToken === 'foot-soldier', true);
    assert.equal(legalGameActions(orderedReplacementState, 'north').some(({ descriptor }) =>
      descriptor.kind === 'replace-rubble-with-top-atlas-site'
        && descriptor.targetCell === 'C3'), true);

    const swapped: GameSession = {
      ...ctx.session,
      state: {
        ...ctx.state,
        players: {
          ...ctx.state.players,
          north: {
            ...ctx.state.players.north,
            atlas: [swap, ...ctx.state.players.north.atlas.slice(1)],
            hand: {
              ...ctx.state.players.north.hand,
              atlas: [top, ...ctx.state.players.north.hand.atlas.slice(1)],
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
    const revealed = canonicalJson(observeGame(ctx.state, 'south') as unknown as JsonValue);
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

test('RULE-02/03 site Genesis resumes after ordered terrain-replacement Deathrites', () => {
  const base = manifest(245, {
    avatar: {
      attack: 1,
      defense: 1,
      drawSpell: false,
      life: 20,
      replaceAdjacentRubbleWithTopAtlasSite: true,
    },
  });
  const preview = createGameSession(base).state.players.north;
  const sourceCardId = preview.hand.atlas[0]?.cardId;
  const targetCardId = preview.hand.atlas[1]?.cardId;
  const waterCardId = preview.atlas[0]?.cardId;
  const deathriteCardIds = preview.hand.spellbook.slice(0, 2).map(({ cardId }) => cardId);
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
  const setup = (deathriteCount: 1 | 2) => {
    let setupSession = keep(keep(createGameSession(gameManifest)));
    const take = (predicate: Parameters<typeof action>[1]): void => {
      setupSession = accept(setupSession, action(setupSession, predicate));
    };
    take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === sourceCardId
      && descriptor.cell === 'C4');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardId === targetCardId
      && descriptor.cell === 'C3');
    for (const cardId of deathriteCardIds.slice(0, deathriteCount)) {
      take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === cardId
        && descriptor.cell === 'C3'
        && descriptor.region === 'underground');
    }
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    take(({ descriptor }) => descriptor.kind === 'end-turn');
    take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    take(({ descriptor }) => descriptor.kind === 'activate-site-destruction'
      && descriptor.targetCell === 'C3');
    const deathrites = setupSession.state.realm.units.filter(({ cardId }) =>
      deathriteCardIds.includes(cardId));
    assert.equal(deathrites.length, deathriteCount);
    assert.equal(deathrites.every(({ region }) => region === 'underground'), true);
    const top = setupSession.state.players.north.atlas[0];
    assert.ok(top);
    assert.equal(top.cardId, waterCardId);
    return { deathrites, session: setupSession, top };
  };
  const { deathrites, session, top } = setup(2);
  const manaBefore = session.state.players.north.mana;
  const atlasBefore = session.state.players.north.atlas.length;
  const atlasHandBefore = session.state.players.north.hand.atlas.length;
  const replacement = action(session, ({ descriptor }) =>
    descriptor.kind === 'replace-rubble-with-top-atlas-site'
      && descriptor.targetCell === 'C3');
  const interrupted = stepGame(session, replacement);
  assert.equal(interrupted.accepted, true);
  if (!interrupted.accepted) throw new Error('expected terrain replacement to reach Deathrites');
  assert.equal(interrupted.session.state.stateVersion, session.state.stateVersion + 1);
  assert.deepEqual(interrupted.receipt.events.map(({ type }) => type), [
    'rubble-replaced',
    'site-played',
  ]);
  assert.deepEqual(interrupted.receipt.randomDraws, []);
  assert.equal(interrupted.session.state.phase, 'deathrite-order');
  assert.equal(interrupted.session.state.decisionSeat, 'north');
  assert.equal(interrupted.session.state.realm.sites.C3?.instanceId, top.instanceId);
  assert.equal(interrupted.session.state.players.north.avatar.tapped, true);
  assert.equal(interrupted.session.state.players.north.mana, manaBefore + 1);
  assert.equal(interrupted.session.state.players.north.atlas.length, atlasBefore - 1);
  assert.equal(deathrites.every(({ instanceId }) => !interrupted.session.state.realm.units
    .some((unit) => unit.instanceId === instanceId)), true);
  assert.equal(deathrites.every(({ instanceId }) => !interrupted.session.state.players.north.cemetery
    .some((card) => card.instanceId === instanceId)), true);
  const restored = resumeGameCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
    createGameCheckpoint(interrupted.session),
  )));
  assert.equal(
    canonicalJson(restored as unknown as JsonValue),
    canonicalJson(interrupted.session as unknown as JsonValue),
  );
  const orders = legalGameActions(restored.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'order-deathrites');
  assert.equal(orders.length, 2);
  assert.equal(legalGameActions(restored.state, 'south').length, 0);
  assert.deepEqual(
    legalGameActions(restored.state, 'north').map(({ actionId }) => actionId),
    orders.map(({ actionId }) => actionId),
  );
  const branches = orders.map((order) => {
    assert.equal(order.descriptor.kind, 'order-deathrites');
    if (order.descriptor.kind !== 'order-deathrites') throw new Error('unreachable');
    const chosenInstanceId = order.descriptor.sourceInstanceId;
    const otherInstanceId = deathrites.find(({ instanceId }) =>
      instanceId !== chosenInstanceId)?.instanceId;
    assert.ok(otherInstanceId);
    const resolved = stepGame(restored, order);
    assert.equal(resolved.accepted, true);
    if (!resolved.accepted) throw new Error('expected site Genesis to resume');
    assert.equal(resolved.session.state.stateVersion, restored.state.stateVersion + 1);
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
    assert.equal(resolved.session.state.phase, 'main');
    assert.equal(resolved.session.state.pendingDeathrites, undefined);
    assert.equal(resolved.session.state.realm.sites.C3?.instanceId, top.instanceId);
    assert.equal(deathrites.every(({ instanceId }) => resolved.session.state.players.north.cemetery
      .some((card) => card.instanceId === instanceId)), true);
    assert.equal(resolved.session.state.players.north.atlas.length, atlasBefore - 3);
    assert.equal(resolved.session.state.players.north.hand.atlas.length, atlasHandBefore + 2);
    assert.equal(resolved.session.state.players.north.mana, manaBefore + 2);
    assert.deepEqual(resolved.receipt.randomDraws, []);
    assert.equal(verifyGameReplay(resolved.session), true);
    return resolved.session;
  });
  assert.equal(new Set(branches.map(({ state }) => hashGameState(state))).size, 1);

  const immediate = setup(1);
  const immediateResult = stepGame(immediate.session, action(immediate.session, ({ descriptor }) =>
    descriptor.kind === 'replace-rubble-with-top-atlas-site'
      && descriptor.targetCell === 'C3'));
  assert.equal(immediateResult.accepted, true);
  if (!immediateResult.accepted) throw new Error('expected synchronous site Genesis');
  assert.equal(immediateResult.session.state.stateVersion, immediate.session.state.stateVersion + 1);
  assert.deepEqual(immediateResult.receipt.events.map(({ type }) => type), [
    'rubble-replaced',
    'site-played',
    'site-drawn',
    'minion-died',
    'mana-gained',
  ]);
  assert.equal(immediateResult.session.state.phase, 'main');
  assert.equal(immediateResult.session.state.players.north.cemetery.some(({ instanceId }) =>
    instanceId === immediate.deathrites[0]!.instanceId), true);
  assert.equal(verifyGameReplay(immediateResult.session), true);

  const terminalSetup = setup(1);
  const terminalSession: GameSession = {
    ...terminalSetup.session,
    state: {
      ...terminalSetup.session.state,
      players: {
        ...terminalSetup.session.state.players,
        north: {
          ...terminalSetup.session.state.players.north,
          atlas: [terminalSetup.top],
        },
      },
    },
  };
  const terminalManaBefore = terminalSession.state.players.north.mana;
  const terminalResult = stepGame(terminalSession, action(terminalSession, ({ descriptor }) =>
    descriptor.kind === 'replace-rubble-with-top-atlas-site'
      && descriptor.targetCell === 'C3'));
  assert.equal(terminalResult.accepted, true);
  if (!terminalResult.accepted) throw new Error('expected terminal terrain Deathrite');
  assert.deepEqual(terminalResult.receipt.events.map(({ type }) => type), [
    'rubble-replaced',
    'site-played',
    'minion-died',
    'game-ended',
  ]);
  assert.equal(terminalResult.receipt.events.some(({ type }) => type === 'mana-gained'), false);
  assert.equal(terminalResult.session.state.players.north.mana, terminalManaBefore + 1);
  assert.equal(terminalResult.session.state.phase, 'terminal');
  assert.equal(terminalResult.session.state.pendingDeathrites, undefined);
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

test('RULE-03 Observatory privately reorders the next three spells without drawing', () => {
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

  const checkpoint = keep(keep(createGameSession(gameManifest)));
  const before = checkpoint.state.players.north.spellbook;
  const play = action(checkpoint, ({ descriptor }) => descriptor.kind === 'play-site'
    && descriptor.cardId === 'observatory-site' && descriptor.cell === 'C4');
  if (play.descriptor.kind !== 'play-site') throw new Error('expected Observatory play');
  const played = stepGame(checkpoint, play);
  assert.equal(played.accepted, true);
  if (!played.accepted) return;
  assert.equal(played.session.state.phase, 'genesis');
  assert.deepEqual(played.session.state.players.north.spellbook, before);

  const choices = legalGameActions(played.session.state, 'north');
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
  const kept = stepGame(played.session, identity);
  const reversed = stepGame(played.session, reverse);
  assert.equal(kept.accepted, true);
  assert.equal(reversed.accepted, true);
  if (!kept.accepted || !reversed.accepted) return;
  assert.deepEqual(kept.session.state.players.north.spellbook, before);
  assert.deepEqual(reversed.session.state.players.north.spellbook.slice(0, 3), before.slice(0, 3).reverse());
  assert.deepEqual(reversed.session.state.players.north.spellbook.slice(3), before.slice(3));
  assert.deepEqual(reversed.receipt.events.map(({ payload, type }) => ({ payload, type })), [{
    payload: { count: 3, seat: 'north', sourceInstanceId: play.descriptor.cardInstanceId },
    type: 'spells-reordered',
  }]);
  assert.deepEqual(reversed.receipt.randomDraws, []);
  assert.equal(
    canonicalJson(observeGame(kept.session.state, 'south') as unknown as JsonValue),
    canonicalJson(observeGame(reversed.session.state, 'south') as unknown as JsonValue),
  );
  assert.equal(verifyGameReplay(kept.session), true);
  assert.equal(verifyGameReplay(reversed.session), true);
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

test('RULE-03/04 an undamaged 0/0 Genesis minion survives until it takes positive damage', () => {
  const setup = northAttacksAtC2(131, {
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
  });
  const target = observeGame(setup.session.state, 'north').realm.units
    .find(({ instanceId }) => instanceId === setup.targetInstanceId);
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

  let session = accept(setup.session, action(setup.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === setup.targetInstanceId));
  const fought = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(fought.accepted, true);
  if (!fought.accepted) return;
  session = fought.session;
  assert.equal(session.state.players.south.cemetery
    .some(({ instanceId }) => instanceId === setup.targetInstanceId), true);
  assert.deepEqual(fought.receipt.events.find(({ payload, type }) =>
    type === 'damage-dealt'
      && canonicalJson(payload).includes(setup.targetInstanceId))?.payload, {
    accumulated: 1,
    amount: 1,
    direct: true,
    instanceId: setup.targetInstanceId,
    seat: 'south',
  });
  assert.equal(session.transcript.every(({ randomDraws }) => randomDraws.length === 0), true);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 an active minion prevents damage from a unit at its current-power threshold', () => {
  for (const [seed, sourcePower, disabled, expectedDamage] of [
    [170, 4, false, 0],
    [172, 3, false, 3],
    [173, 4, true, 4],
  ] as const) {
    const setup = northAttacksAtC2(seed, {
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
    });
    assert.equal(observeGame(setup.session.state, 'south').realm.units.find(({ instanceId }) =>
      instanceId === setup.targetInstanceId)?.disabled, disabled);
    let session = accept(setup.session, action(setup.session, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === setup.targetInstanceId));
    const fought = stepGame(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
    assert.equal(fought.accepted, true);
    if (!fought.accepted) return;
    session = fought.session;
    const target = observeGame(session.state, 'south').realm.units.find(({ instanceId }) =>
      instanceId === setup.targetInstanceId);
    assert.deepEqual({ damage: target?.damage, disabled: target?.disabled }, {
      damage: expectedDamage,
      disabled: false,
    });
    assert.equal(observeGame(session.state, 'north').realm.units.find(({ instanceId }) =>
      instanceId === setup.targetInstanceId)?.damage, expectedDamage);
    const targetDamage = fought.receipt.events.find(({ payload, type }) =>
      type === 'damage-dealt' && canonicalJson(payload).includes(setup.targetInstanceId));
    assert.equal((targetDamage?.payload as { amount?: number }).amount, expectedDamage);
    assert.equal((targetDamage?.payload as { prevented?: boolean }).prevented,
      expectedDamage < sourcePower ? true : undefined);
    assert.equal(fought.receipt.randomDraws.length, 0);
    assert.equal(verifyGameReplay(session), true);
  }
});

test('RULE-04 Ranged damage retains its attacking unit source classification', () => {
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
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
  const shooter = session.state.realm.units.find(({ controller }) => controller === 'north');
  assert.ok(shooter);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
  const target = session.state.realm.units.find(({ controller }) => controller === 'south');
  assert.ok(target);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === shooter.instanceId && descriptor.to.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const shot = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'shoot-projectile'
      && descriptor.shooterInstanceId === shooter.instanceId
      && descriptor.hit?.instanceId === target.instanceId));
  assert.equal(shot.accepted, true);
  if (!shot.accepted) return;
  session = shot.session;
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId)?.damage, 0);
  assert.deepEqual(shot.receipt.events.map(({ type }) => type), [
    'projectile-shot',
    'strike-damage-allocated',
    'damage-dealt',
  ]);
  assert.equal(shot.receipt.randomDraws.length, 0);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-04 a ready minion may tap to shoot a fixed-damage projectile to the first visible unit', () => {
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
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
  const shooter = session.state.realm.units.find(({ controller }) => controller === 'north');
  assert.ok(shooter);
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'shoot-damage-projectile'
      && descriptor.shooterInstanceId === shooter.instanceId), false);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
  const target = session.state.realm.units.find(({ controller }) => controller === 'south');
  assert.ok(target);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const shot = legalGameActions(session.state, 'north').find(({ descriptor }) =>
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
  const fired = stepGame(session, shot);
  assert.equal(fired.accepted, true);
  if (!fired.accepted) return;
  session = fired.session;
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
  const firedShooter = session.state.realm.units.find(({ instanceId }) =>
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
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === target.instanceId)?.damage, 4);
  assert.equal(fired.receipt.events.some(({ type }) => type === 'strike-damage-allocated'), false);
  assert.equal(fired.receipt.randomDraws.length, 0);
  assert.equal(verifyGameReplay(session), true);
});

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

  const deathrite = {
    attack: 2,
    deathriteDamageEachUnitHere: 1,
    defense: 3,
    manaCost: 0,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const ordering = northAttacksAtC2(
    280,
    { ...firstStrike, attack: 6, defense: 10 },
    { attack: 2, defense: 1, drawSpell: false, life: 20 },
    false,
    deathrite,
    1,
  );
  const defenderIds = ordering.session.state.realm.units
    .filter(({ controller, location }) => controller === 'south' && location === 'C1')
    .map(({ instanceId }) => instanceId)
    .sort();
  assert.equal(defenderIds.length, 2);
  const deathriteIds = [ordering.targetInstanceId, defenderIds[0]!].sort();
  const survivingDefenderId = defenderIds[1]!;
  session = accept(ordering.session, action(ordering.session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack'
      && descriptor.target.kind === 'minion'
      && descriptor.target.instanceId === ordering.targetInstanceId));
  for (const unitInstanceId of defenderIds) {
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === unitInstanceId));
  }
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates));
  assert.equal(session.state.phase, 'allocate');
  while (session.state.phase === 'allocate') {
    session = accept(session, action(session, ({ descriptor }) =>
      descriptor.kind === 'allocate-strike'
        && descriptor.amount === (deathriteIds.includes(descriptor.targetInstanceId) ? 3 : 0)));
  }

  assert.equal(session.state.phase, 'deathrite-order');
  assert.equal(session.state.decisionSeat, 'south');
  assert.equal(deathriteIds.every((instanceId) => !session.state.players.south.cemetery
    .some((card) => card.instanceId === instanceId)), true);
  assert.equal(session.transcript.at(-1)?.events.some(({ type }) => type === 'minion-died'), false);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === ordering.attackerInstanceId)?.damage, 0);
  assert.equal(session.transcript.at(-1)?.events.some(({ payload, type }) =>
    type === 'damage-dealt'
      && payload !== null
      && typeof payload === 'object'
      && 'instanceId' in payload
      && payload.instanceId === ordering.attackerInstanceId), false);
  const orderActions = legalGameActions(session.state, 'south').filter(({ descriptor }) =>
    descriptor.kind === 'order-deathrites');
  assert.deepEqual(orderActions.flatMap(({ descriptor }) =>
    descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), deathriteIds);
  const ordered = stepGame(session, orderActions[0]!);
  assert.equal(ordered.accepted, true);
  if (!ordered.accepted) return;
  session = ordered.session;

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
      && payload.instanceId === ordering.attackerInstanceId);
  assert.deepEqual(attackerDamage.at(-1)?.payload, {
    accumulated: 4,
    amount: 2,
    direct: true,
    instanceId: ordering.attackerInstanceId,
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
      && payload.instanceId === ordering.attackerInstanceId);
  assert.ok(lastDeathriteIndex < firstDeathIndex && firstDeathIndex < returnStrikeIndex);
  assert.equal(ordered.receipt.events.filter(({ type }) => type === 'minion-died').length, 2);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === ordering.attackerInstanceId)?.damage, 4);
  assert.equal(session.state.players.south.avatar.life, 20);
  assert.equal(deathriteIds.every((instanceId) => session.state.players.south.cemetery
    .some((card) => card.instanceId === instanceId)), true);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === survivingDefenderId)?.damage, 2);
  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.pendingCombat, null);
  assert.equal(session.state.pendingDeathrites ?? null, null);
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
