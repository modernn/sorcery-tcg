import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import { createGameCheckpoint } from '../../src/engine/checkpoint.ts';
import {
  createGameManifest,
  createGameSession,
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
import {
  accept,
  action,
  cardsFor,
  deck,
  keep,
  manifest,
  SYNTHETIC_AUTHORITY_HASH,
  takeAction,
  withNumericGenesisSession,
  type SpellFacts,
} from './game-setup-helpers.ts';
import { withPreview, withSetup, type SetupCtx } from './rust-setup-session.ts';

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
    let result!: Readonly<{
      artifactInstanceId: string;
      beforeCast: GameSession['state'];
      session: GameSession;
    }>;
    await withSetup(createGameManifest({
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
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'cast-artifact'
        && descriptor.cardId === 'bury-artifact-target'
        && (carried
          ? descriptor.bearer?.kind === 'avatar'
          : descriptor.bearer === undefined && descriptor.cell === 'C1'));
      const artifact = ctx.state.realm.artifacts?.[0];
      assert.ok(artifact);
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
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
      result = { artifactInstanceId: artifact.instanceId, beforeCast, session: ctx.session };
    });
    return result;
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'cave-in-burrower'
      && descriptor.cell === 'C1'
      && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'cave-in-victim' && descriptor.cell === 'C1');
    const burrower = ctx.state.realm.units.find(({ cardId }) => cardId === 'cave-in-burrower');
    const victim = ctx.state.realm.units.find(({ cardId }) => cardId === 'cave-in-victim');
    const artifactCard = ctx.state.players.south.hand.spellbook
      .find(({ cardId }) => cardId === 'cave-in-artifact');
    assert.ok(burrower && victim && artifactCard);
    const preparedCheckpoint = createGameCheckpoint(ctx.session);

    const castCaveIn = async (bearer: 'avatar' | 'minion'): Promise<void> => {
      await ctx.resume(preparedCheckpoint);
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'cast-artifact'
          && descriptor.cardInstanceId === artifactCard.instanceId
          && descriptor.bearer?.kind === bearer
          && (bearer === 'avatar' || descriptor.bearer.instanceId === burrower.instanceId));
      const artifact = ctx.state.realm.artifacts?.find(({ instanceId }) =>
        instanceId === artifactCard.instanceId);
      assert.ok(artifact);
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const spell = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === 'cave-in-magic');
      assert.ok(spell);
      const casts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === spell.instanceId);
      assert.deepEqual(casts.flatMap(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.targetLocation
        ? [`${descriptor.targetLocation.cell}:${descriptor.targetLocation.region}`]
        : []), ['C1:surface']);
      const rubbleId = 'sha256:6666666666666666666666666666666666666666666666666666666666666666' as const;
      const rubbleState = {
        ...ctx.state,
        realm: {
          ...ctx.state.realm,
          sites: {
            ...ctx.state.realm.sites,
            A1: { controller: null, instanceId: rubbleId, rubble: true as const },
          },
        },
      };
      // Forged-state legality probes still use TS legalGameActions.
      assert.equal(legalGameActions(rubbleState, 'north').some(({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.cardInstanceId === spell.instanceId
          && descriptor.targetSiteInstanceId === rubbleId
          && descriptor.targetLocation?.cell === 'A1'), true);
      assert.equal(legalGameActions({
        ...ctx.state,
        players: {
          ...ctx.state.players,
          north: {
            ...ctx.state.players.north,
            avatar: { ...ctx.state.players.north.avatar, region: 'underground' as const },
          },
        },
      }, 'north').some(({ descriptor }) => descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === spell.instanceId), false);
      const result = await ctx.step(casts[0]!);
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
      assert.equal(await ctx.verifyReplay(), true);
    };

    await castCaveIn('minion');
    await castCaveIn('avatar');
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
  let waterSiteId: string | undefined;
  let landSiteId: string | undefined;
  let ordinaryId: string | undefined;
  let survivorId: string | undefined;
  let wardedId: string | undefined;
  let landTargetId: string | undefined;
  let stealthId: string | undefined;
  await withPreview(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed: 246,
  }), async (preview) => {
    waterSiteId = preview.state.players.south.hand.atlas[0]?.cardId;
    landSiteId = preview.state.players.south.hand.atlas[1]?.cardId;
    ordinaryId = preview.state.players.south.hand.spellbook[0]?.cardId;
    survivorId = preview.state.players.south.hand.spellbook[1]?.cardId;
    wardedId = preview.state.players.south.hand.spellbook[2]?.cardId;
    landTargetId = preview.state.players.south.spellbook[0]?.cardId;
    stealthId = preview.state.players.south.spellbook[1]?.cardId;
  });
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
    const waterSite = instance(waterSiteId!);
    const landSite = instance(landSiteId!);
    const ordinary = instance(ordinaryId!);
    const survivor = instance(survivorId!);
    const warded = instance(wardedId!);
    const landTarget = instance(landTargetId!);
    const stealth = instance(stealthId!);

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === waterSite.instanceId
        && descriptor.cell === 'C1');
    for (const target of [ordinary, survivor, warded]) {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === target.instanceId
          && descriptor.cell === 'C1'
          && descriptor.region === undefined);
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === landSite.instanceId
        && descriptor.cell === 'C2');
    for (const target of [landTarget, stealth]) {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === target.instanceId
          && descriptor.cell === 'C2'
          && descriptor.region === undefined);
    }
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const preparedCheckpoint = createGameCheckpoint(ctx.session);
    const checkpointState = ctx.state;
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

    const castAt = async (targetInstanceId: string): Promise<GameSession> => {
      await ctx.resume(preparedCheckpoint);
      const resumeCasts = (await ctx.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.cardInstanceId === drown.instanceId);
      const choice = resumeCasts.find(({ descriptor }) =>
        descriptor.kind === 'cast-magic' && descriptor.target?.instanceId === targetInstanceId);
      assert.ok(choice);
      await ctx.accept(choice);
      assert.equal(await ctx.verifyReplay(), true);
      return ctx.session;
    };
    const verifyCast = (cast: GameSession, checkpointState: GameSession['state']): void => {
      assert.equal(cast.state.stateVersion, checkpointState.stateVersion + 1);
      assert.equal(cast.state.players.north.mana, checkpointState.players.north.mana - 1);
      assert.equal(cast.state.players.north.cemetery.some(({ instanceId }) =>
        instanceId === drown.instanceId), true);
    };

    const dead = await castAt(ordinary.instanceId);
    verifyCast(dead, checkpointState);
    assert.equal(dead.state.realm.units.some(({ instanceId }) => instanceId === ordinary.instanceId), false);
    assert.equal(dead.state.players.south.cemetery.some(({ instanceId }) =>
      instanceId === ordinary.instanceId), true);
    assert.deepEqual(dead.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'minion-submerged',
      'minion-died',
      'magic-resolved',
    ]);

    const submerged = await castAt(survivor.instanceId);
    verifyCast(submerged, checkpointState);
    assert.equal(submerged.state.realm.units.find(({ instanceId }) =>
      instanceId === survivor.instanceId)?.region, 'underwater');
    assert.deepEqual(submerged.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'minion-submerged',
      'magic-resolved',
    ]);

    const protectedByWard = await castAt(warded.instanceId);
    verifyCast(protectedByWard, checkpointState);
    const wardedUnit = protectedByWard.state.realm.units.find(({ instanceId }) =>
      instanceId === warded.instanceId);
    assert.equal(wardedUnit?.region, 'surface');
    assert.equal(wardedUnit?.warded, false);
    assert.deepEqual(protectedByWard.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'ward-broken',
      'magic-resolved',
    ]);

    const unable = await castAt(landTarget.instanceId);
    verifyCast(unable, checkpointState);
    assert.equal(unable.state.realm.units.find(({ instanceId }) =>
      instanceId === landTarget.instanceId)?.region, 'surface');
    assert.deepEqual(unable.transcript.at(-1)?.events.map(({ type }) => type), [
      'magic-cast',
      'magic-resolved',
    ]);
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
    let result!: Readonly<{
      beforeCast: GameSession['state'];
      session: GameSession;
      spellInstanceId: string;
    }>;
    await withSetup(createGameManifest({
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
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C4');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site' && descriptor.cell === 'C1');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'cast-magic'
          && descriptor.target !== undefined
          && descriptor.target.kind === 'avatar'
          && descriptor.target.seat === 'north');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
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
      result = { beforeCast, session: ctx.session, spellInstanceId: spell.instanceId };
    });
    return result;
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
    await withSetup(manifest(111, {
      spell: {
        manaCost: 1,
        summonToAnySite,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
      const cardInstanceId = ctx.state.players.north.hand.spellbook[0]?.instanceId;
      assert.ok(cardInstanceId);
      cells = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
        descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === cardInstanceId
          ? [descriptor.cell]
          : []);
      if (summonToAnySite) {
        await takeAction(ctx, ({ descriptor }) =>
          descriptor.kind === 'summon-minion'
            && descriptor.cardInstanceId === cardInstanceId
            && descriptor.cell === 'C1');
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
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

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

    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northWater.instanceId && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === southLand.instanceId && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northLand.instanceId && descriptor.cell === 'C3');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === southWater.instanceId && descriptor.cell === 'B1');
    const southSummons = await ctx.legalActions('south');
    assert.equal(southSummons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === ordinaryId && descriptor.cell === 'C4'), false);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const summons = await ctx.legalActions('north');
    const cellsFor = (cardId: string): readonly string[] => summons.flatMap(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cardId === cardId ? [descriptor.cell] : []);
    assert.deepEqual(cellsFor(featuredId), ['B1', 'C4']);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === featuredId && descriptor.cell === 'B1');
    assert.equal(ctx.state.realm.units[0]?.location, 'B1');
    assert.equal(ctx.state.realm.units[0]?.controller, 'north');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

// TODO(rust-cutover): the Rust engine does not yet emit `stealth-lost` when a stealthed
// minion is revealed by moving or is killed by site destruction, so this proof still runs
// on the legacy TypeScript legality engine. Needs a Rust-side fix plus a Rust proof.
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
  assert.equal(observeGame(underwater.state, 'north').realm.units[0]?.stealthed, true);
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
  assert.equal(verifyGameReplay(destroyed.session), true);

  const surface = summoned('surface');
  assert.equal(observeGame(surface.state, 'north').realm.units[0]?.stealthed, true);
  let activeTargetCheckpoint = accept(surface, action(surface, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  activeTargetCheckpoint = accept(activeTargetCheckpoint, action(activeTargetCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  assert.equal(legalGameActions(activeTargetCheckpoint.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId === waterbound.instanceId), false);
  session = northSecondMain(surface);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === waterbound.instanceId
      && descriptor.to.cell === 'C3'));
  assert.deepEqual(session.transcript.at(-1)?.events.map(({ type }) => type), [
    'move-and-attack-activated',
    'stealth-lost',
  ]);
  assert.equal(session.state.realm.units.find(({ instanceId }) =>
    instanceId === waterbound.instanceId)?.stealthed, false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(observeGame(session.state, 'north').realm.units[0]?.disabled, true);
  assert.equal(observeGame(session.state, 'north').realm.units[0]?.stealthed, false);
  assert.equal(observeGame(session.state, 'north').players.north.affinity.water, 1);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId === waterbound.instanceId), true);
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
  assert.equal(observeGame(session.state, 'north').realm.units[0]?.stealthed, false);
  assert.equal(observeGame(session.state, 'north').players.north.affinity.water, 2);
  const enabledActions = legalGameActions(session.state, 'north');
  assert.equal(enabledActions.some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === waterbound.instanceId), true);
  assert.equal(enabledActions.some(({ descriptor }) =>
    descriptor.kind === 'activate-mana'
      && descriptor.unitInstanceId === waterbound.instanceId), true);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  assert.equal(legalGameActions(session.state, 'south').some(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId === waterbound.instanceId), true);
  assert.equal(verifyGameReplay(session), true);
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
  let waterSiteId: string | undefined;
  let landSiteId: string | undefined;
  let featuredId: string | undefined;
  let targetId: string | undefined;
  await withPreview(base, async (preview) => {
    waterSiteId = preview.state.players.north.hand.atlas[0]?.cardId;
    landSiteId = preview.state.players.north.hand.atlas[1]?.cardId;
    featuredId = preview.state.players.north.hand.spellbook[0]?.cardId;
    targetId = preview.state.players.north.hand.spellbook[1]?.cardId;
  });
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
    const opening = createGameCheckpoint(ctx.session);
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

    const playSite = async (cardInstanceId: string): Promise<void> => {
      await takeAction(ctx, ({ descriptor }) =>
        descriptor.kind === 'play-site'
          && descriptor.cardInstanceId === cardInstanceId
          && descriptor.cell === 'C4');
    };

    await playSite(landSite.instanceId);
    const land = createGameCheckpoint(ctx.session);
    const landStateVersion = ctx.state.stateVersion;
    const atlasBeforeDeath = ctx.state.players.north.atlas.length;
    const died = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === featured.instanceId
        && descriptor.cell === 'C4'
        && descriptor.region === 'underground'));
    assert.equal(died.accepted, true);
    if (!died.accepted) return;
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
    assert.equal(died.session.state.stateVersion, landStateVersion + 1);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(land);
    const banished = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === featured.instanceId
        && descriptor.cell === 'A4'
        && descriptor.region === 'void'));
    assert.equal(banished.accepted, true);
    if (!banished.accepted) return;
    assert.deepEqual(banished.receipt.events.map(({ type }) => type), [
      'minion-summoned',
      'minion-banished',
    ]);
    assert.equal(banished.session.state.realm.units.some(({ instanceId }) =>
      instanceId === featured.instanceId), false);
    assert.equal(banished.session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === featured.instanceId), false);
    assert.equal(banished.session.state.stateVersion, landStateVersion + 1);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(opening);
    await playSite(waterSite.instanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === featured.instanceId
        && descriptor.cell === 'C4'
        && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const beforeMoveVersion = ctx.state.stateVersion;
    const moved = await ctx.step(await ctx.action(({ descriptor }) =>
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
    if (!moved.accepted) return;
    assert.deepEqual(moved.receipt.events.map(({ type }) => type), [
      'move-and-attack-activated',
      'minion-banished',
    ]);
    assert.deepEqual(moved.receipt.events[0]?.payload, {
      from: { cell: 'C4', region: 'surface' },
      path: [
        { cell: 'C4', region: 'surface' },
        { cell: 'B4', region: 'void' },
        { cell: 'A4', region: 'void' },
      ],
      seat: 'north',
      steps: 2,
      to: { cell: 'A4', region: 'void' },
      unitInstanceId: featured.instanceId,
    });
    assert.equal(moved.session.state.realm.units.some(({ instanceId }) =>
      instanceId === featured.instanceId), false);
    assert.equal(moved.session.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === featured.instanceId), false);
    assert.equal(moved.session.state.phase, 'main');
    assert.equal(moved.session.state.pendingCombat, null);
    assert.equal(moved.session.state.stateVersion, beforeMoveVersion + 1);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(opening);
    await playSite(waterSite.instanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === featured.instanceId
        && descriptor.cell === 'C4'
        && descriptor.region === undefined);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === target.instanceId
        && descriptor.cell === 'A4'
        && descriptor.region === 'void');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    const southAttacker = ctx.state.players.south.hand.spellbook[0];
    assert.ok(southAttacker);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === southAttacker.instanceId
        && descriptor.cell === 'A4'
        && descriptor.region === 'void');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === southAttacker.instanceId
        && descriptor.path.length === 1);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === target.instanceId);
    const beforeDefendVersion = ctx.state.stateVersion;
    const defended = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === featured.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,B4,A4'));
    assert.equal(defended.accepted, true);
    if (!defended.accepted) return;
    assert.deepEqual(defended.receipt.events.map(({ type }) => type), [
      'defender-joined',
      'minion-banished',
    ]);
    assert.deepEqual(defended.receipt.events[0]?.payload, {
      from: { cell: 'C4', region: 'surface' },
      instanceId: featured.instanceId,
      path: [
        { cell: 'C4', region: 'surface' },
        { cell: 'B4', region: 'void' },
        { cell: 'A4', region: 'void' },
      ],
      seat: 'north',
      steps: 2,
      to: { cell: 'A4', region: 'void' },
    });
    assert.deepEqual(defended.session.state.pendingCombat?.defenders, []);
    assert.equal(defended.session.state.pendingCombat?.targetRemoved, false);
    assert.equal(defended.session.state.stateVersion, beforeDefendVersion + 1);
    assert.equal(await ctx.verifyReplay(), true);
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
    let matched = false;
    await withPreview(candidate, async (preview) => {
      const hand = preview.state.players.north.hand.spellbook;
      matched = hand.some(({ cardId }) => cardId === 'voidwalker')
        && hand.filter(({ cardId }) => cardId === 'sword').length >= 2;
    });
    if (matched) {
      gameManifest = candidate;
      break;
    }
  }
  assert.ok(gameManifest);
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === 'voidwalker' && descriptor.cell === 'C4');
    const bearer = ctx.state.realm.units.find(({ cardId }) => cardId === 'voidwalker');
    assert.ok(bearer);
    await take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword'
      && descriptor.bearer?.instanceId === bearer.instanceId);
    await take(({ descriptor }) => descriptor.kind === 'cast-artifact'
      && descriptor.cardId === 'sword'
      && descriptor.bearer?.instanceId === bearer.instanceId);
    const [carried, stillCarried] = ctx.state.realm.artifacts ?? [];
    assert.ok(carried);
    assert.ok(stillCarried && 'bearer' in stillCarried);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === bearer.instanceId
      && descriptor.to.cell === 'B4'
      && descriptor.to.region === 'void');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'drop-artifacts'
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion');
    const unit = ctx.state.realm.units[0];
    assert.ok(unit);
    assert.equal(unit.summoningSickness, true);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unit.instanceId
        && descriptor.to.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
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
    assert.doesNotMatch(canonicalJson(observeGame(ctx.state, 'south')), new RegExp(drawn.cardId));
    assert.equal(await ctx.verifyReplay(), true);
  });

  const short = deck('genesis-short', 3, 3);
  await withSetup(manifest(40, { north: short, south: short, spell }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion');
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
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
    assert.doesNotMatch(canonicalJson(observeGame(ctx.state, 'south')), new RegExp(drawn.cardId));
    assert.equal(await ctx.verifyReplay(), true);
  });

  const short = deck('genesis-spell-short', 3, 3);
  await withSetup(manifest(127, { north: short, south: short, spell }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion');
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

test('RULE-03 numeric Genesis spell draw counts draw ordered hidden cards', async () => {
  await withNumericGenesisSession(3, 128, async (ctx) => {
    const before = ctx.state.players.north;
    const expectedDraws = before.spellbook.slice(0, 3);
    assert.equal(expectedDraws.length, 3);

    const result = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'));
    assert.equal(result.accepted, true);
    if (!result.accepted) return;
    const after = result.session.state.players.north;
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
      assert.doesNotMatch(canonicalJson(observeGame(result.session.state, 'south')), new RegExp(drawn.cardId));
    }
    assert.equal(observeGame(result.session.state, 'north').realm.units.some(({ attack, damage, defense }) =>
      attack === 0 && damage === 0 && defense === 0), true);
    assert.deepEqual(result.receipt.randomDraws, []);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 numeric Genesis spell draws exhaust 2, 1, or 0 remaining cards before deck loss', async () => {
  for (const remainingCount of [2, 1, 0]) {
    await withNumericGenesisSession(remainingCount, 129 + remainingCount, async (ctx) => {
      const before = ctx.state.players.north;
      assert.equal(before.spellbook.length, remainingCount);
      const expectedDraws = before.spellbook.slice();

      const result = await ctx.step(await ctx.action(({ descriptor }) =>
        descriptor.kind === 'summon-minion'));
      assert.equal(result.accepted, true);
      if (!result.accepted) return;
      assert.deepEqual(result.session.state.terminal, {
        loser: 'north',
        reason: 'deck_empty',
        status: 'finished',
        winner: 'south',
      });
      assert.equal(result.session.state.players.north.spellbook.length, 0);
      assert.deepEqual(result.receipt.events.map(({ type }) => type), [
        'minion-summoned',
        ...Array.from({ length: remainingCount }, () => 'spell-drawn' as const),
        'game-ended',
      ]);
      for (const drawn of expectedDraws) {
        assert.equal(result.session.state.players.north.hand.spellbook
          .some(({ instanceId }) => instanceId === drawn.instanceId), true);
        assert.doesNotMatch(canonicalJson(result.receipt.events), new RegExp(drawn.cardId));
        assert.doesNotMatch(
          canonicalJson(observeGame(result.session.state, 'south')),
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
  for (const incompatible of [{ stealth: true }, { token: true }, { waterbound: true }]) {
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
        genesisDrawSpells: 1,
      } as unknown as GameCardDefinition,
    },
    seed: 1,
  }), /simultaneous Genesis/);
  let gameManifest: GameManifest | undefined;
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    // Seed search peeks opening hands via TS createGameSession (cheap); play path uses SetupCtx.
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
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (
      predicate: (candidate: GameLegalAction) => boolean,
    ): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === alliedMinionId && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.cardId === teleportId
      && descriptor.ally?.kind === 'avatar'
      && descriptor.allyDestination === undefined
      && descriptor.targetLocation?.cell === 'C4');
    for (const minionId of [enemyMinionId, wardedMinionId]) {
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === minionId
        && descriptor.cell === 'C4'
        && descriptor.region === undefined);
    }
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === undergroundMinionId
      && descriptor.cell === 'C4'
      && descriptor.region === 'underground');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');

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
    const branchPoint = createGameCheckpoint(ctx.session);
    const result = await ctx.step(await ctx.action(({ descriptor }) =>
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
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(branchPoint);
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
    const take = async (
      predicate: (candidate: GameLegalAction) => boolean,
    ): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const checkpoint = createGameCheckpoint(ctx.session);
    const checkpointVersion = ctx.state.stateVersion;
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

    const decline = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === card.instanceId
        && descriptor.cell === 'C4'
        && descriptor.genesisDamageChoice === 'decline'));
    assert.equal(decline.accepted, true);
    assert.equal(decline.session.state.players.north.avatar.life, 20);
    assert.deepEqual(decline.receipt.events.map(({ type }) => type), ['minion-summoned']);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
    const targeted = await ctx.step(await ctx.action(({ descriptor }) =>
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
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
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
  const withReadyToSummon = async (
    life: number,
    seed: number,
    run: (ctx: SetupCtx) => Promise<void>,
  ): Promise<void> => {
    await withSetup(manifest(seed, {
      avatar: { attack: 1, defense: 1, drawSpell: false, life },
      spell: {
        genesisLoseControllerLife: 2,
        manaCost: 0,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      },
    }), async (ctx) => {
      await ctx.keep();
      await ctx.keep();
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
      await run(ctx);
    });
  };
  const summon = async (ctx: SetupCtx) => {
    const beforeVersion = ctx.state.stateVersion;
    const result = await ctx.step(
      await ctx.action(({ descriptor }) => descriptor.kind === 'summon-minion'),
    );
    assert.equal(result.accepted, true);
    assert.equal(result.session.state.stateVersion, beforeVersion + 1);
    return result;
  };

  await withReadyToSummon(3, 226, async (ctx) => {
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

  await withReadyToSummon(2, 227, async (ctx) => {
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

    const atDeathsDoor = lifeTwo.session;
    const deathDoorTurn = atDeathsDoor.state.players.north.avatar.deathDoorTurn;
    const lifeZero = await summon(ctx);
    assert.equal(lifeZero.session.state.players.north.avatar.life, 0);
    assert.equal(lifeZero.session.state.players.north.avatar.deathDoorTurn, deathDoorTurn);
    assert.deepEqual(lifeZero.receipt.events.map(({ type }) => type), ['minion-summoned']);
    assert.deepEqual(lifeZero.session.state.terminal, { status: 'active' });
    assert.equal(await ctx.verifyReplay(), true);
  });
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
