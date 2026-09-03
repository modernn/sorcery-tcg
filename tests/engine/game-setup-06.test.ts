import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import {
  createGameManifest,
  type GameCardDefinition,
  type GameLegalAction,
} from '../../src/engine/game.ts';
import {
  cardsFor,
  deck,
  manifest,
  SYNTHETIC_AUTHORITY_HASH,
  takeAction,
  withNorthAttacksAtC2,
} from './game-setup-helpers.ts';
import { SetupCtx, withSetup } from './rust-setup-session.ts';

test('RULE-04 conditional end-turn Stealth requires no nearby enemy in the same region', async () => {
  await withSetup(manifest(241, {
    northSpell: {
      attack: 2,
      defense: 2,
      gainsStealthAtEndOfTurnIfNoEnemiesNearby: true,
      manaCost: 0,
      submerge: true,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    site: { elements: ['water'] },
    southSpell: {
      attack: 1,
      defense: 1,
      manaCost: 0,
      stealth: true,
      summonToAnySite: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B2');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B3' && (descriptor.region ?? 'surface') === 'surface');
    const enemyInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(enemyInstanceId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const checkpoint = createGameCheckpoint(ctx.session);

    const summon = async (cell: 'C1' | 'C4', region: 'surface' | 'underwater'): Promise<void> => {
      await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cell === cell && (descriptor.region ?? 'surface') === region);
    };
    const sourceId = (): string => {
      const instanceId = ctx.state.realm.units
        .find(({ controller }) => controller === 'north')?.instanceId;
      assert.ok(instanceId);
      return instanceId;
    };

    await summon('C1', 'surface');
    const avatarBlockedId = sourceId();
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === avatarBlockedId)?.stealthed, false);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
    await summon('C1', 'underwater');
    const otherRegionId = sourceId();
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === otherRegionId)?.stealthed, true);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'stealth-gained',
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(checkpoint);
    await summon('C4', 'surface');
    const minionBlockedId = sourceId();
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === minionBlockedId)?.stealthed, false);
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === enemyInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'B3,B2');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === minionBlockedId)?.stealthed, true);
    assert.deepEqual(ctx.session.transcript.at(-1)?.events.map(({ type }) => type), [
      'stealth-gained',
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Scent Hounds permanently removes nearby enemy Stealth', async () => {
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const houndInstanceId = ctx.state.realm.units[0]!.instanceId;
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const targetInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')!.instanceId;
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === houndInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    const moved = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === targetInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
    assert.equal(moved.accepted, true);
    if (!moved.accepted) return;
    assert.deepEqual(moved.receipt.events.map(({ type }) => type), [
      'move-and-attack-activated',
      'stealth-lost',
    ]);
    assert.deepEqual(moved.receipt.events[1]?.payload, {
      instanceId: targetInstanceId,
      seat: 'south',
      sourceInstanceId: houndInstanceId,
    });
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');

    const regained = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    assert.equal(regained.accepted, true);
    if (!regained.accepted) return;
    assert.deepEqual(regained.receipt.events.map(({ type }) => type), [
      'stealth-gained',
      'stealth-lost',
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === houndInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units
      .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
    assert.equal(await ctx.verifyReplay(), true);

    // A disabled Scent Hound withholding Stealth-strip until its disable expires
    // (event order stealth-gained,turn-ended | turn-ended,minion-disable-expired,
    // stealth-lost,turn-started) is proven in Rust
    // `rule_catalog_0107_scent_hounds_permanently_remove_nearby_enemy_stealth`
    // (crates/sorcery-engine/tests/combat_rules.rs), which reaches the disabled
    // state through a legal Freeze cast rather than a synthetic state edit.
  });
});

test('RULE-04 Malakhim untaps at its controller End Phase unless Disabled', async () => {
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const malakhim = ctx.state.realm.units[0];
    assert.ok(malakhim);
    const readyEnd = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    assert.equal(readyEnd.accepted, true);
    if (!readyEnd.accepted) return;
    assert.equal(readyEnd.receipt.events.some(({ type }) => type === 'minion-untapped'), false);
    assert.deepEqual(readyEnd.receipt.randomDraws, []);

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const attackerInstanceId = ctx.state.realm.units
      .find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(attackerInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === malakhim.instanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    const tappedMalakhim = ctx.state.realm.units
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

    // A damaged (but enabled) tapped Malakhim untapping and resetting its damage
    // together at End Phase, and a disabled tapped Malakhim staying tapped while
    // its damage still resets, are proven in Rust
    // `malakhim_should_untap_only_when_tapped_and_enabled` and
    // `disabled_malakhim_should_stay_tapped_while_damage_resets`
    // (crates/sorcery-engine/tests/end_turn_lifecycle_rules.rs), which reach the
    // disabled state through a legal Freeze cast rather than a synthetic state edit.

    const ended = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
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
    assert.deepEqual(ctx.state.realm.units
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
    assert.equal(ctx.observe('south').realm.units
      .find(({ instanceId }) => instanceId === malakhim.instanceId)?.warded, true);

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2,C3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'defend' && descriptor.unitInstanceId === malakhim.instanceId);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Ignited dies before turn cleanup unless it is Disabled', async () => {
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

  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'summon-minion');
    const ignited = ctx.state.realm.units[0];
    assert.ok(ignited);
    const stateVersionBefore = ctx.state.stateVersion;

    const ended = await ctx.step(await ctx.action(({ descriptor }) => descriptor.kind === 'end-turn'));
    assert.equal(ended.accepted, true);
    if (!ended.accepted) return;
    assert.deepEqual(ended.receipt.events.map(({ type }) => type), [
      'minion-died',
      'turn-ended',
      'turn-started',
    ]);
    assert.equal(ctx.state.realm.units.some(({ instanceId }) =>
      instanceId === ignited.instanceId), false);
    assert.equal(ctx.state.players.north.cemetery.some(({ instanceId }) =>
      instanceId === ignited.instanceId), true);
    assert.deepEqual({
      activeSeat: ctx.state.activeSeat,
      mana: ctx.state.players.south.mana,
      phase: ctx.state.phase,
      stateVersion: ctx.state.stateVersion,
    }, {
      activeSeat: 'south',
      mana: 0,
      phase: 'draw',
      stateVersion: stateVersionBefore + 1,
    });
    assert.equal(await ctx.verifyReplay(), true);

    // A disabled Ignited surviving turn cleanup, resetting its damage, and
    // expiring its disable on the following End Phase (event order
    // turn-ended,minion-disable-expired,turn-started) is proven in Rust
    // `disabled_ignited_should_survive_cleanup_reset_damage_and_expire_disable`
    // (crates/sorcery-engine/tests/end_turn_lifecycle_rules.rs), which reaches
    // the disabled state through legal Genesis damage and a Freeze cast rather
    // than a synthetic state edit.
  });
});

test('RULE-04 Sedge Crabs can move themselves only sideways', async () => {
  const crab = {
    attack: 3,
    defense: 3,
    manaCost: 1,
    movementBonus: 1 as const,
    movesOnlySideways: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  await withNorthAttacksAtC2({ seed: 128, southSpell: crab }, async (ctx, { defenderInstanceId }) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === defenderInstanceId), false);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(127, { spell: crab }), async (ctx) => {
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
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C3');
    const crabInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(crabInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = (await ctx.legalActions('north')).filter(({ descriptor }) =>
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
    await ctx.accept(sideways);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units[0]?.location, 'B3');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Dalcean Phalanx can move itself only forward for its seat', async () => {
  const phalanx = {
    attack: 5,
    connectsTopBottom: true,
    defense: 5,
    manaCost: 1,
    movementBonus: 1 as const,
    movesOnlyForward: true,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  await withNorthAttacksAtC2({ seed: 141, southSpell: phalanx }, async (ctx, { defenderInstanceId }) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === defenderInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'), true);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(142, { spell: phalanx }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C3');
    const instanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(instanceId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const paths = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === instanceId
        ? [descriptor.path.map(({ cell }) => cell).join(',')]
        : []);
    assert.equal(paths.includes('C3'), true);
    assert.equal(paths.includes('C3,C2'), true);
    assert.equal(paths.includes('C3,C2,C1'), true);
    assert.equal(paths.includes('C3,C4'), false);
    assert.equal(paths.includes('C3,B3'), false);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C2,C1');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), true);
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'close-intercept');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const edgePaths = (await ctx.legalActions('north')).flatMap(({ descriptor }) =>
      descriptor.kind === 'move-and-attack' && descriptor.unitInstanceId === instanceId
        ? [descriptor.path.map(({ cell }) => cell).join(',')]
        : []);
    assert.equal(edgePaths.includes('C1,C4'), true);
    assert.equal(edgePaths.includes('C1,C2'), false);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C4');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units[0]?.location, 'C4');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-02/04 a unit can move across connected top and bottom realm edges', async () => {
  await withSetup(manifest(128, {
    spell: {
      connectsTopBottom: true,
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const actions = await ctx.legalActions('north');
    const wraps = ({ descriptor }: GameLegalAction): boolean => descriptor.kind === 'move-and-attack'
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C1';
    assert.equal(actions.some((candidate) =>
      wraps(candidate) && candidate.descriptor.kind === 'move-and-attack'
        && candidate.descriptor.unitInstanceId === unit.instanceId), true);
    assert.equal(actions.some((candidate) =>
      wraps(candidate) && candidate.descriptor.kind === 'move-and-attack'
        && candidate.descriptor.unitInstanceId === ctx.state.players.north.avatar.card.instanceId), false);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unit.instanceId
      && descriptor.to.cell === 'C1');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), true);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Submerge uses underwater summons, movement, and region-isolated combat', async () => {
  const submerge = {
    attack: 2,
    defense: 2,
    manaCost: 1,
    submerge: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 1 },
  } as const;
  const ordinary = { ...submerge, submerge: false };
  await withSetup(manifest(129, {
    northSpell: submerge,
    site: { elements: ['water'] },
    southSpell: ordinary,
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site');
    const northSummons = await ctx.legalActions('north');
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === undefined), true);
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), true);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underwater');
    const northUnitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(northUnitId);
    assert.equal(ctx.observe('north').realm.units[0]?.region, 'underwater');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');

    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), false);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion');
    const southUnitId = ctx.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
    assert.ok(southUnitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');

    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const underwaterMoves = await ctx.legalActions('north');
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

    const branch = createGameCheckpoint(ctx.session);
    await ctx.accept(surfaces);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === northUnitId)?.region, 'surface');
    assert.equal(await ctx.verifyReplay(), true);

    await ctx.resume(branch);
    await ctx.accept(swims);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), false);
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');

    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === southUnitId
        && descriptor.to.cell === 'C2');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');

    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northUnitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C3/underwater,C2/underwater');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === southUnitId), false);
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.phase, 'main');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');

    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === northUnitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C2/underwater,C2/surface');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === southUnitId), true);
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'close-intercept');
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(130, { northSpell: submerge }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underwater'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Burrowing uses underground summons and movement only at land sites', async () => {
  const burrowing = {
    attack: 2,
    burrowing: true,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  const ordinary = { ...burrowing, burrowing: false };
  await withSetup(manifest(131, { northSpell: burrowing, southSpell: ordinary }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site');
    const northSummons = await ctx.legalActions('north');
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === undefined), true);
    assert.equal(northSummons.some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), true);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underground');
    const northUnitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(northUnitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), false);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    const moves = await ctx.legalActions('north');
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C3/underground'), true);
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C4/surface'), true);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === northUnitId
      && descriptor.to.cell === 'C3'
      && descriptor.to.region === 'underground');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'), false);
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withSetup(manifest(132, {
    northSpell: burrowing,
    site: { elements: ['water'] },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'underground'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Secret Tunnel connects burrowed allies only to the controller\'s other sites', async () => {
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
  await withSetup(tunnelManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    const tunnel = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === tunnelCardId);
    const land = ctx.state.players.north.hand.atlas.find(({ cardId }) =>
      cardId !== tunnelCardId && cardId !== waterCardId);
    const water = ctx.state.players.north.hand.atlas.find(({ cardId }) => cardId === waterCardId);
    assert.ok(tunnel);
    assert.ok(land);
    assert.ok(water);

    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === tunnel.instanceId && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underground');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === land.instanceId && descriptor.cell === 'C3');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === water.instanceId && descriptor.cell === 'C2');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const pathFor = (candidate: GameLegalAction): string => candidate.descriptor.kind === 'move-and-attack'
      ? candidate.descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      : '';
    const moves = await ctx.legalActions('north');
    const unitPaths = moves
      .filter(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unitId)
      .map(pathFor);
    assert.equal(unitPaths.filter((path) => path === 'C4/underground,C3/underground').length, 1);
    assert.equal(unitPaths.includes('C4/underground,C2/underwater'), true);
    assert.equal(unitPaths.includes('C4/underground,C1/underground'), false);
    assert.equal(unitPaths.includes('C4/underground,C2/underwater,C4/underground'), false);
    const avatarId = ctx.state.players.north.avatar.card.instanceId;
    const avatarPaths = moves
      .filter(({ descriptor }) => descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === avatarId)
      .map(pathFor);
    assert.equal(avatarPaths.includes('C4/surface,C3/surface'), true);
    assert.equal(avatarPaths.includes('C4/surface,C2/surface'), false);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C2/underwater');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.deepEqual(ctx.state.realm.units[0]?.location, 'C2');
    assert.deepEqual(ctx.state.realm.units[0]?.region, 'underwater');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a must-be-burrowed cast restriction suppresses only non-underground casts', async () => {
  await withSetup(manifest(139, {
    northSpell: {
      attack: 3,
      burrowing: true,
      defense: 3,
      manaCost: 1,
      mustBeCastBurrowed: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    const summons = await ctx.legalActions('north');
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === undefined), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underground'), true);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underground');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C4/surface');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 a must-be-submerged cast restriction suppresses only non-underwater casts', async () => {
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
  await withSetup(submergedManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    const water = ctx.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    const land = ctx.state.players.north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && !definition.elements.includes('water');
    });
    assert.ok(water);
    assert.ok(land);
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === water.instanceId && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === land.instanceId && descriptor.cell === 'C3');
    const summons = await ctx.legalActions('north');
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.region === undefined), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C3' && descriptor.region === 'underground'), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.region === 'void'), false);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underwater'), true);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === 'underwater');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = await ctx.legalActions('north');
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.to.cell === 'C3' && descriptor.to.region === 'underground'), true);
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId && descriptor.to.region === 'void'), true);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underwater,C4/surface');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 combined region abilities permit eligible cross-region adjacency steps', async () => {
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
  await withSetup(crossManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const north = ctx.state.players.north;
    const land = north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && !definition.elements.includes('water');
    });
    const water = north.hand.atlas.find(({ cardId }) => {
      const definition = ctx.state.cards[cardId];
      return definition?.cardType === 'site' && definition.elements.includes('water');
    });
    assert.ok(land);
    assert.ok(water);
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cardInstanceId === land.instanceId);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.region === 'underground');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === water.instanceId
      && descriptor.cell === 'C3');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/underground,C3/underwater');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C3/underwater,C4/underground,B4/void');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Voidwalk summons to any void and moves between adjacent void and surface locations', async () => {
  const voidwalk = {
    attack: 2,
    defense: 2,
    manaCost: 1,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    voidwalk: true,
  } as const;
  await withSetup(manifest(134, {
    northSpell: voidwalk,
    site: { elements: ['air'] },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site');
    const summons = await ctx.legalActions('north');
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'C4' && descriptor.region === undefined), true);
    assert.equal(summons.some(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B4' && descriptor.region === 'void'), true);
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B4' && descriptor.region === 'void');
    const coveredUnitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(coveredUnitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site');
    assert.equal((await ctx.legalActions('south')).some(({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.region === 'void'), false);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === coveredUnitId)?.region, 'surface');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'B3' && descriptor.region === 'void');
    const unitId = ctx.state.realm.units.find(({ region }) => region === 'void')?.instanceId;
    assert.ok(unitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const moves = await ctx.legalActions('north');
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,A3/void'), true);
    assert.equal(moves.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,B4/surface'), true);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.to.cell === 'B4' && descriptor.to.region === 'surface');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Planar Gate grants minions Voidwalk only until they leave the void', async () => {
  const baseNorth = deck('gate-north');
  const north = { ...baseNorth, atlas: baseNorth.atlas.map(() => 'planar-gate') };
  const withGateEverywhere = manifest(162, {
    north,
    northSpell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      movementBonus: 2,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    },
    site: { minionsHereGainVoidwalkUntilLeavingVoid: true },
  });
  const southSiteIds = new Set(withGateEverywhere.decks.south.atlas);
  const cards = Object.fromEntries(Object.entries(withGateEverywhere.cards).map(([cardId, card]) => {
    if (card.cardType !== 'site' || !southSiteIds.has(cardId)) return [cardId, card];
    const ordinarySite = { ...card };
    delete ordinarySite.minionsHereGainVoidwalkUntilLeavingVoid;
    return [cardId, ordinarySite];
  })) as Record<string, GameCardDefinition>;
  const gameManifest = createGameManifest({ ...withGateEverywhere, cards });
  await withSetup(gameManifest, async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const unitId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(unitId);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C4/surface,B4/void');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === unitId)
      ?.planarGateVoidwalk, true);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'B4/void,B3/void'), true);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B4/void,B3/void');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const exits = await ctx.legalActions('north');
    assert.equal(exits.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,C3/surface,D3/void'), false);
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B3/void,C3/surface');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === unitId)
      ?.planarGateVoidwalk, undefined);
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === unitId
        && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
          === 'C3/surface,D3/void'), false);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-03 an outer-column cast restriction filters surface and Voidwalk summons, not movement', async () => {
  await withSetup(manifest(135, {
    northSpell: {
      attack: 3,
      defense: 3,
      manaCost: 3,
      mustBeCastToOuterColumn: true,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
      voidwalk: true,
    },
    site: { elements: ['air'] },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };

    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B4');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'A4');

    const summons = (await ctx.legalActions('north'))
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
    await take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === 'E2' && descriptor.region === 'void');
    const unitId = ctx.state.realm.units.find(({ location }) => location === 'E2')?.instanceId;
    assert.ok(unitId);

    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'E2/void,D2/void');
    await take(({ descriptor }) => descriptor.kind === 'decline-attack');
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Movement +1 issues exact two-step and returning Move and Attack paths', async () => {
  await withNorthAttacksAtC2({
    seed: 53,
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      movementBonus: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }, async (ctx, { attackerInstanceId }) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const actions = await ctx.legalActions('north');
    const twoStep = actions.find(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4');
    assert.ok(twoStep);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2'), true);
    const result = await ctx.step(twoStep);
    assert.equal(result.accepted, true);
    assert.equal(ctx.state.pendingCombat?.cell, 'C4');
    assert.match(canonicalJson(result.receipt.events[0]?.payload ?? null), /\"steps\":2/);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Movement +1 can take an exact two-step path to Defend', async () => {
  await withSetup(manifest(54, {
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      movementBonus: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
    const defenderInstanceId = ctx.state.realm.units[0]?.instanceId;
    assert.ok(defenderInstanceId);
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');

    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'play-site');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'summon-minion' && descriptor.cell === 'C1');
    const attackerInstanceId = ctx.state.realm.units.find(({ controller }) => controller === 'south')?.instanceId;
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
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site');
    const defend = await ctx.action(({ descriptor }) =>
      descriptor.kind === 'defend'
        && descriptor.unitInstanceId === defenderInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3,C2');
    await ctx.accept(defend);
    assert.equal(ctx.state.realm.units.find(({ instanceId }) => instanceId === defenderInstanceId)?.location, 'C2');
    assert.match(canonicalJson(ctx.session.transcript.at(-1)?.events[0]?.payload ?? null), /\"steps\":2/);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-04 Movement +2 issues exact three-step paths and attacks after moving', async () => {
  await withNorthAttacksAtC2({
    seed: 125,
    spell: {
      attack: 2,
      defense: 2,
      manaCost: 1,
      movementBonus: 2,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    },
  }, async (ctx, { attackerInstanceId, defenderInstanceId }) => {
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'decline-attack');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'close-intercept');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await takeAction(ctx, ({ descriptor }) => descriptor.kind === 'end-turn');
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    const actions = await ctx.legalActions('north');
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2'), true);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2,C3'), false);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C4,C3'), true);
    const threeStep = actions.find(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.map(({ cell }) => cell).join(',') === 'C2,C3,C2,C1');
    assert.ok(threeStep);
    assert.equal(actions.some(({ descriptor }) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === attackerInstanceId
        && descriptor.path.length > 4), false);
    const result = await ctx.step(threeStep);
    assert.equal(result.accepted, true);
    assert.match(canonicalJson(result.receipt.events[0]?.payload ?? null), /\"steps\":3/);
    assert.equal((await ctx.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === defenderInstanceId), true);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-05 AP commits before NAP, then NAP Deathrites resolve before AP Deathrites', async () => {
  const decks = {
    north: deck('ap-nap-north', 5, 6),
    south: deck('ap-nap-south', 5, 6),
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
    revisionId: 'synthetic-ap-nap-deathrites-v1',
  };
  const seed = 281;
  let apCardIds: string[] = [];
  let napCardIds: string[] = [];
  let genesisCardId: string | undefined;
  await withSetup(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (previewCtx) => {
    apCardIds = previewCtx.state.players.north.hand.spellbook.slice(0, 2)
      .map(({ cardId }) => cardId);
    genesisCardId = previewCtx.state.players.north.hand.spellbook[2]?.cardId;
    napCardIds = previewCtx.state.players.south.hand.spellbook.slice(0, 2)
      .map(({ cardId }) => cardId);
  });
  assert.equal(apCardIds.length, 2);
  assert.equal(napCardIds.length, 2);
  assert.ok(genesisCardId);

  const cards: Record<string, GameCardDefinition> = { ...baseCards };
  for (const cardId of [...apCardIds, ...napCardIds]) {
    cards[cardId] = {
      ...baseCards[cardId]!,
      deathriteDrawSite: true,
      summonToAnySite: true,
    } as GameCardDefinition;
  }
  cards[genesisCardId] = {
    ...baseCards[genesisCardId]!,
    defense: 5,
    genesisDamageEachOtherUnitHere: 1,
    summonToAnySite: true,
  } as GameCardDefinition;

  await withSetup(createGameManifest({
    authority,
    cards,
    decks,
    firstSeat: 'north',
    seed,
  }), async (ctx) => {
    await ctx.keep();
    await ctx.keep();
    const take = async (predicate: (candidate: GameLegalAction) => boolean): Promise<void> => {
      await takeAction(ctx, predicate);
    };
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
    for (const cardId of apCardIds) {
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === cardId && descriptor.cell === 'C4');
    }
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
    await take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
    for (const cardId of napCardIds) {
      await take(({ descriptor }) => descriptor.kind === 'summon-minion'
        && descriptor.cardId === cardId && descriptor.cell === 'C4');
    }
    await take(({ descriptor }) => descriptor.kind === 'end-turn');
    await take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

    const apInstanceIds = ctx.state.realm.units
      .filter(({ cardId }) => apCardIds.includes(cardId))
      .map(({ instanceId }) => instanceId)
      .sort();
    const napInstanceIds = ctx.state.realm.units
      .filter(({ cardId }) => napCardIds.includes(cardId))
      .map(({ instanceId }) => instanceId)
      .sort();
    const genesis = ctx.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === genesisCardId);
    assert.equal(apInstanceIds.length, 2);
    assert.equal(napInstanceIds.length, 2);
    assert.ok(genesis);
    const triggered = await ctx.step(await ctx.action(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === genesis.instanceId
        && descriptor.cell === 'C4'));
    assert.equal(triggered.accepted, true);
    if (!triggered.accepted) return;
    assert.equal(ctx.state.phase, 'deathrite-order');
    assert.equal(ctx.state.decisionSeat, 'north');
    assert.deepEqual(await ctx.legalActions('south'), []);
    const apActions = (await ctx.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    assert.deepEqual(apActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), apInstanceIds);
    assert.equal(triggered.receipt.events.some(({ type }) =>
      type === 'site-drawn' || type === 'minion-died'), false);

    const apFirst = apActions[0]!;
    assert.equal(apFirst.descriptor.kind, 'order-deathrites');
    if (apFirst.descriptor.kind !== 'order-deathrites') return;
    const apFirstInstanceId = apFirst.descriptor.sourceInstanceId;
    const apCommittedOrder = [
      apFirstInstanceId,
      apInstanceIds.find((instanceId) => instanceId !== apFirstInstanceId)!,
    ];
    const apCommitted = await ctx.step(apFirst);
    assert.equal(apCommitted.accepted, true);
    if (!apCommitted.accepted) return;
    assert.equal(ctx.state.phase, 'deathrite-order');
    assert.equal(ctx.state.decisionSeat, 'south');
    assert.deepEqual(apCommitted.receipt.events.map(({ type }) => type), ['deathrite-order-committed']);

    const interruptedCheckpoint = createGameCheckpoint(ctx.session);
    const restored = await SetupCtx.resumeCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
      interruptedCheckpoint,
    )));
    assert.equal(
      canonicalJson(restored as unknown as JsonValue),
      canonicalJson(ctx.session as unknown as JsonValue),
    );
    await ctx.resume(interruptedCheckpoint);
    const napActions = (await ctx.legalActions('south')).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    assert.deepEqual(napActions.flatMap(({ descriptor }) =>
      descriptor.kind === 'order-deathrites' ? [descriptor.sourceInstanceId] : []).sort(), napInstanceIds);
    const napFirst = napActions[0]!;
    assert.equal(napFirst.descriptor.kind, 'order-deathrites');
    if (napFirst.descriptor.kind !== 'order-deathrites') return;
    const napFirstInstanceId = napFirst.descriptor.sourceInstanceId;
    const napCommittedOrder = [
      napFirstInstanceId,
      napInstanceIds.find((instanceId) => instanceId !== napFirstInstanceId)!,
    ];
    const resolved = await ctx.step(napFirst);
    assert.equal(resolved.accepted, true);
    if (!resolved.accepted) return;
    const events = resolved.receipt.events;
    assert.deepEqual(events.filter(({ type }) => type === 'site-drawn').map(({ payload }) =>
      payload !== null && typeof payload === 'object' && 'sourceInstanceId' in payload
        ? payload.sourceInstanceId
        : undefined), [...napCommittedOrder, ...apCommittedOrder]);
    assert.ok(events.findIndex(({ type }) => type === 'minion-died')
      > events.findLastIndex(({ type }) => type === 'site-drawn'));
    assert.equal(events.filter(({ type }) => type === 'minion-died').length, 4);
    assert.equal(await ctx.verifyReplay(), true);
  });
});

test('RULE-05 NAP then AP Deathrites resolve before simultaneous deaths enter their cemeteries', async () => {
  const spell = {
    attack: 1,
    deathriteDrawSite: true,
    defense: 1,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  } as const;
  await withNorthAttacksAtC2({ seed: 48, spell }, async (ctx, { attackerInstanceId, targetInstanceId }) => {
    const before = ctx.state.players;
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);

    for (const seat of ['north', 'south'] as const) {
      assert.equal(ctx.state.players[seat].atlas.length, before[seat].atlas.length - 1);
      assert.equal(ctx.state.players[seat].hand.atlas.length, before[seat].hand.atlas.length + 1);
    }
    const events = ctx.session.transcript.at(-1)?.events ?? [];
    const firstCemeteryEvent = events.findIndex(({ type }) => type === 'minion-died');
    const draws = events.filter(({ type }) => type === 'site-drawn');
    assert.ok(firstCemeteryEvent > 0);
    assert.equal(events.slice(0, firstCemeteryEvent).filter(({ type }) => type === 'site-drawn').length, 2);
    assert.deepEqual(draws.map(({ payload }) => payload), [
      { seat: 'south', sourceInstanceId: targetInstanceId },
      { seat: 'north', sourceInstanceId: attackerInstanceId },
    ]);
    assert.equal(ctx.state.players.north.cemetery.length, 1);
    assert.equal(ctx.state.players.south.cemetery.length, 1);
    assert.equal(await ctx.verifyReplay(), true);
  });

  await withNorthAttacksAtC2({
    seed: 49,
    spell,
    emptyAtlasAfterOpening: true,
  }, async (ctx, { targetInstanceId }) => {
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'declare-attack'
        && descriptor.target.kind === 'minion'
        && descriptor.target.instanceId === targetInstanceId);
    await takeAction(ctx, ({ descriptor }) =>
      descriptor.kind === 'close-defend' && descriptor.originalTargetParticipates);
    assert.deepEqual(ctx.state.terminal, {
      reason: 'simultaneous_defeat',
      result: 'draw',
      status: 'finished',
    });
    assert.equal(await ctx.verifyReplay(), true);
  });
});
