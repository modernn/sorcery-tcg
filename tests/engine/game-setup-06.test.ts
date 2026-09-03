import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson, type JsonValue } from '../../src/authority/canonical-json.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpoint,
  serializeGameCheckpoint,
} from '../../src/engine/checkpoint.ts';
import {
  createGameManifest,
  createGameSession,
  legalGameActions,
  observeGame,
  stepGame,
  verifyGameReplay,
  type GameCardDefinition,
  type GameLegalAction,
  type GameSession,
} from '../../src/engine/game.ts';
import {
  accept,
  action,
  cardsFor,
  deck,
  keep,
  manifest,
  northAttacksAtC2,
  SYNTHETIC_AUTHORITY_HASH,
} from './game-setup-helpers.ts';

test('RULE-04 conditional end-turn Stealth requires no nearby enemy in the same region', () => {
  let prepared = keep(keep(createGameSession(manifest(241, {
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
  }))));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    prepared = accept(prepared, action(prepared, predicate));
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
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B1');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'B2');
  take(({ descriptor }) => descriptor.kind === 'summon-minion'
    && descriptor.cell === 'B3' && (descriptor.region ?? 'surface') === 'surface');
  const enemyInstanceId = prepared.state.realm.units
    .find(({ controller }) => controller === 'south')?.instanceId;
  assert.ok(enemyInstanceId);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  const checkpoint = prepared;

  const summon = (cell: 'C1' | 'C4', region: 'surface' | 'underwater'): GameSession =>
    accept(checkpoint, action(checkpoint, ({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cell === cell && (descriptor.region ?? 'surface') === region));
  const sourceId = (session: GameSession): string => {
    const instanceId = session.state.realm.units
      .find(({ controller }) => controller === 'north')?.instanceId;
    assert.ok(instanceId);
    return instanceId;
  };

  let avatarBlocked = summon('C1', 'surface');
  const avatarBlockedId = sourceId(avatarBlocked);
  avatarBlocked = accept(avatarBlocked, action(avatarBlocked, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(avatarBlocked.state.realm.units
    .find(({ instanceId }) => instanceId === avatarBlockedId)?.stealthed, false);
  assert.deepEqual(avatarBlocked.transcript.at(-1)?.events.map(({ type }) => type), [
    'turn-ended',
    'turn-started',
  ]);
  assert.equal(verifyGameReplay(avatarBlocked), true);

  let otherRegion = summon('C1', 'underwater');
  const otherRegionId = sourceId(otherRegion);
  otherRegion = accept(otherRegion, action(otherRegion, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(otherRegion.state.realm.units
    .find(({ instanceId }) => instanceId === otherRegionId)?.stealthed, true);
  assert.deepEqual(otherRegion.transcript.at(-1)?.events.map(({ type }) => type), [
    'stealth-gained',
    'turn-ended',
    'turn-started',
  ]);
  assert.equal(verifyGameReplay(otherRegion), true);

  let minionBlocked = summon('C4', 'surface');
  const minionBlockedId = sourceId(minionBlocked);
  minionBlocked = accept(minionBlocked, action(minionBlocked, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(minionBlocked.state.realm.units
    .find(({ instanceId }) => instanceId === minionBlockedId)?.stealthed, false);
  minionBlocked = accept(minionBlocked, action(minionBlocked, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  minionBlocked = accept(minionBlocked, action(minionBlocked, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === enemyInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'B3,B2'));
  minionBlocked = accept(minionBlocked, action(minionBlocked, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  minionBlocked = accept(minionBlocked, action(minionBlocked, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  minionBlocked = accept(minionBlocked, action(minionBlocked, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  minionBlocked = accept(minionBlocked, action(minionBlocked, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(minionBlocked.state.realm.units
    .find(({ instanceId }) => instanceId === minionBlockedId)?.stealthed, true);
  assert.deepEqual(minionBlocked.transcript.at(-1)?.events.map(({ type }) => type), [
    'stealth-gained',
    'turn-ended',
    'turn-started',
  ]);
  assert.equal(verifyGameReplay(minionBlocked), true);
});

test('RULE-04 Scent Hounds permanently removes nearby enemy Stealth', () => {
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

  let session = keep(keep(createGameSession(gameManifest)));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const houndInstanceId = session.state.realm.units[0]!.instanceId;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const targetInstanceId = session.state.realm.units
    .find(({ controller }) => controller === 'south')!.instanceId;
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === houndInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  const checkpoint = session;
  const moved = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === targetInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  assert.equal(moved.accepted, true);
  if (!moved.accepted) return;
  session = moved.session;
  assert.deepEqual(moved.receipt.events.map(({ type }) => type), [
    'move-and-attack-activated',
    'stealth-lost',
  ]);
  assert.deepEqual(moved.receipt.events[1]?.payload, {
    instanceId: targetInstanceId,
    seat: 'south',
    sourceInstanceId: houndInstanceId,
  });
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));

  const regained = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(regained.accepted, true);
  if (!regained.accepted) return;
  session = regained.session;
  assert.deepEqual(regained.receipt.events.map(({ type }) => type), [
    'stealth-gained',
    'stealth-lost',
    'turn-ended',
    'turn-started',
  ]);
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === houndInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C3,C4'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  assert.equal(session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
  assert.equal(verifyGameReplay(session), true);

  const disabledCheckpoint: GameSession = {
    ...checkpoint,
    state: {
      ...checkpoint.state,
      realm: {
        ...checkpoint.state.realm,
        units: checkpoint.state.realm.units.map((unit) => unit.instanceId === houndInstanceId
          ? {
            ...unit,
            disableEffects: [{
              expiresAtSeat: 'south' as const,
              sourceInstanceId: unit.instanceId,
            }],
          }
          : unit),
      },
    },
  };
  const disabledMove = stepGame(disabledCheckpoint, action(disabledCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === targetInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2'));
  assert.equal(disabledMove.accepted, true);
  if (!disabledMove.accepted) return;
  assert.equal(disabledMove.receipt.events.some(({ type }) => type === 'stealth-lost'), false);
  assert.equal(disabledMove.session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, true);
  let expiry = accept(disabledMove.session, action(disabledMove.session, ({ descriptor }) =>
    descriptor.kind === 'decline-attack'));
  expiry = accept(expiry, action(expiry, ({ descriptor }) => descriptor.kind === 'end-turn'));
  expiry = accept(expiry, action(expiry, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  const reenabled = stepGame(expiry, action(expiry, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(reenabled.accepted, true);
  if (!reenabled.accepted) return;
  assert.deepEqual(reenabled.receipt.events.map(({ type }) => type), [
    'turn-ended',
    'minion-disable-expired',
    'stealth-lost',
    'turn-started',
  ]);
  assert.deepEqual(reenabled.receipt.events[2]?.payload, {
    instanceId: targetInstanceId,
    seat: 'south',
    sourceInstanceId: houndInstanceId,
  });
  assert.equal(reenabled.session.state.stateVersion, expiry.state.stateVersion + 1);
  assert.equal(reenabled.session.state.realm.units
    .find(({ instanceId }) => instanceId === targetInstanceId)?.stealthed, false);
});

test('RULE-04 Malakhim untaps at its controller End Phase unless Disabled', () => {
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

  let session = keep(createGameSession(gameManifest));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C4'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C4'));
  const malakhim = session.state.realm.units[0];
  assert.ok(malakhim);
  const readyEnd = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(readyEnd.accepted, true);
  if (!readyEnd.accepted) return;
  assert.equal(readyEnd.receipt.events.some(({ type }) => type === 'minion-untapped'), false);
  assert.deepEqual(readyEnd.receipt.randomDraws, []);
  session = readyEnd.session;

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cell === 'C1'));
  const attackerInstanceId = session.state.realm.units
    .find(({ controller }) => controller === 'south')?.instanceId;
  assert.ok(attackerInstanceId);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === malakhim.instanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C4,C3'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'decline-attack'));
  const tappedMalakhim = session.state.realm.units
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

  const damagedCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        units: session.state.realm.units.map((unit) => unit.instanceId === malakhim.instanceId
          ? { ...unit, damage: 2 }
          : unit),
      },
    },
  };
  const damagedEnd = stepGame(damagedCheckpoint, action(damagedCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(damagedEnd.accepted, true);
  if (!damagedEnd.accepted) return;
  assert.deepEqual(damagedEnd.receipt.events.map(({ type }) => type), [
    'minion-untapped',
    'turn-ended',
    'turn-started',
  ]);
  assert.deepEqual(damagedEnd.session.state.realm.units
    .filter(({ instanceId }) => instanceId === malakhim.instanceId)
    .map(({ damage, tapped, warded }) => ({ damage, tapped, warded })), [{
    damage: 0,
    tapped: false,
    warded: true,
  }]);

  const disabledCheckpoint: GameSession = {
    ...session,
    state: {
      ...session.state,
      realm: {
        ...session.state.realm,
        units: session.state.realm.units.map((unit) => unit.instanceId === malakhim.instanceId
          ? {
            ...unit,
            damage: 2,
            disableEffects: [{
              expiresAtSeat: 'south' as const,
              sourceInstanceId: unit.instanceId,
            }],
          }
          : unit),
      },
    },
  };
  const disabledEnd = stepGame(disabledCheckpoint, action(disabledCheckpoint, ({ descriptor }) =>
    descriptor.kind === 'end-turn'));
  assert.equal(disabledEnd.accepted, true);
  if (!disabledEnd.accepted) return;
  assert.equal(disabledEnd.receipt.events.some(({ type }) => type === 'minion-untapped'), false);
  assert.deepEqual(disabledEnd.session.state.realm.units
    .filter(({ instanceId }) => instanceId === malakhim.instanceId)
    .map(({ damage, disableEffects, tapped, warded }) => ({
      damage,
      disableEffects,
      tapped,
      warded,
    })), [{
    damage: 0,
    disableEffects: undefined,
    tapped: true,
    warded: true,
  }]);

  const ended = stepGame(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
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
  assert.deepEqual(ended.session.state.realm.units
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
  assert.equal(observeGame(ended.session.state, 'south').realm.units
    .find(({ instanceId }) => instanceId === malakhim.instanceId)?.warded, true);
  session = ended.session;

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === attackerInstanceId
      && descriptor.path.map(({ cell }) => cell).join(',') === 'C1,C2,C3'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'declare-attack' && descriptor.target.kind === 'site'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'defend' && descriptor.unitInstanceId === malakhim.instanceId));
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

test('RULE-04 Planar Gate grants minions Voidwalk only until they leave the void', () => {
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
  let session = keep(keep(createGameSession(gameManifest)));
  const take = (predicate: (candidate: GameLegalAction) => boolean): void => {
    session = accept(session, action(session, predicate));
  };

  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  take(({ descriptor }) => descriptor.kind === 'summon-minion' && descriptor.cell === 'C4');
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
      === 'C4/surface,B4/void');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(session.state.realm.units.find(({ instanceId }) => instanceId === unitId)
    ?.planarGateVoidwalk, true);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C2');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'B4/void,B3/void'), true);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'B4/void,B3/void');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C3');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const exits = legalGameActions(session.state, 'north');
  assert.equal(exits.some(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'B3/void,C3/surface,D3/void'), false);
  take(({ descriptor }) => descriptor.kind === 'move-and-attack'
    && descriptor.unitInstanceId === unitId
    && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
      === 'B3/void,C3/surface');
  take(({ descriptor }) => descriptor.kind === 'decline-attack');
  assert.equal(session.state.realm.units.find(({ instanceId }) => instanceId === unitId)
    ?.planarGateVoidwalk, undefined);
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  assert.equal(legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'move-and-attack'
      && descriptor.unitInstanceId === unitId
      && descriptor.path.map(({ cell, region }) => `${cell}/${region}`).join(',')
        === 'C3/surface,D3/void'), false);
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

test('RULE-05 AP commits before NAP, then NAP Deathrites resolve before AP Deathrites', () => {
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
  const preview = createGameSession(createGameManifest({
    authority,
    cards: baseCards,
    decks,
    firstSeat: 'north',
    seed,
  }));
  const apCardIds = preview.state.players.north.hand.spellbook.slice(0, 2)
    .map(({ cardId }) => cardId);
  const genesisCardId = preview.state.players.north.hand.spellbook[2]?.cardId;
  const napCardIds = preview.state.players.south.hand.spellbook.slice(0, 2)
    .map(({ cardId }) => cardId);
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
  let session = keep(keep(createGameSession(createGameManifest({
    authority,
    cards,
    decks,
    firstSeat: 'north',
    seed,
  }))));
  const take = (predicate: Parameters<typeof action>[1]): void => {
    session = accept(session, action(session, predicate));
  };
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C4');
  for (const cardId of apCardIds) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === cardId && descriptor.cell === 'C4');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');
  take(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell === 'C1');
  for (const cardId of napCardIds) {
    take(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.cardId === cardId && descriptor.cell === 'C4');
  }
  take(({ descriptor }) => descriptor.kind === 'end-turn');
  take(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'spellbook');

  const apInstanceIds = session.state.realm.units
    .filter(({ cardId }) => apCardIds.includes(cardId))
    .map(({ instanceId }) => instanceId)
    .sort();
  const napInstanceIds = session.state.realm.units
    .filter(({ cardId }) => napCardIds.includes(cardId))
    .map(({ instanceId }) => instanceId)
    .sort();
  const genesis = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === genesisCardId);
  assert.equal(apInstanceIds.length, 2);
  assert.equal(napInstanceIds.length, 2);
  assert.ok(genesis);
  const triggered = stepGame(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === genesis.instanceId
      && descriptor.cell === 'C4'));
  assert.equal(triggered.accepted, true);
  if (!triggered.accepted) return;
  session = triggered.session;
  assert.equal(session.state.phase, 'deathrite-order');
  assert.equal(session.state.decisionSeat, 'north');
  assert.deepEqual(legalGameActions(session.state, 'south'), []);
  const apActions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
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
  const apCommitted = stepGame(session, apFirst);
  assert.equal(apCommitted.accepted, true);
  if (!apCommitted.accepted) return;
  session = apCommitted.session;
  assert.equal(session.state.phase, 'deathrite-order');
  assert.equal(session.state.decisionSeat, 'south');
  assert.deepEqual(apCommitted.receipt.events.map(({ type }) => type), ['deathrite-order-committed']);

  const restored = resumeGameCheckpoint(parseGameCheckpoint(serializeGameCheckpoint(
    createGameCheckpoint(session),
  )));
  assert.equal(
    canonicalJson(restored as unknown as JsonValue),
    canonicalJson(session as unknown as JsonValue),
  );
  const napActions = legalGameActions(restored.state, 'south').filter(({ descriptor }) =>
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
  const resolved = stepGame(restored, napFirst);
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
  assert.equal(verifyGameReplay(resolved.session), true);
});

test('RULE-05 NAP then AP Deathrites resolve before simultaneous deaths enter their cemeteries', () => {
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
  const draws = events.filter(({ type }) => type === 'site-drawn');
  assert.ok(firstCemeteryEvent > 0);
  assert.equal(events.slice(0, firstCemeteryEvent).filter(({ type }) => type === 'site-drawn').length, 2);
  assert.deepEqual(draws.map(({ payload }) => payload), [
    { seat: 'south', sourceInstanceId: setup.targetInstanceId },
    { seat: 'north', sourceInstanceId: setup.attackerInstanceId },
  ]);
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
