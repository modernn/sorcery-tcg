import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameManifest,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
  type GameStepResult,
} from '../src/engine/game.ts';
import {
  RUST_LEGALITY_SOURCE,
  transitionFromCheckpoint,
  withRustSession,
} from '../src/engine/rust-session-helpers.ts';

const FIXTURE_PATH = fileURLToPath(new URL(
  '../tests/engine/fixtures/leap-attack-action-v1.json',
  import.meta.url,
));

const LEAP_ID = 'synthetic-leap-attack';
const ALLY_ID = 'synthetic-leap-ally';
const ORIGIN_ID = 'synthetic-leap-origin';
const STEP_TARGET_ID = 'synthetic-leap-step-target';
const WARDED_ID = 'synthetic-leap-warded';

const DEATHRITE_SOURCE_ID = 'synthetic-leap-source';
const DEATHRITE_FRAGILE_ID = 'synthetic-leap-fragile';
const DEATHRITE_RAIN_ID = 'synthetic-leap-rain';
const DEATHRITE_ENEMY_ID = 'synthetic-leap-enemy';

const MAIN_SEED = 1;
const DEATHRITE_SEED = 1;

function mainManifest(seed: number) {
  const zero = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const avatar = {
    attack: 1,
    cardType: 'avatar' as const,
    defense: 1,
    drawSpell: false,
    life: 20,
  };
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'synthetic-leap-attack-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-leap-attack-action-v1',
    },
    cards: {
      [ALLY_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 4,
        manaCost: 0,
        movementBonus: 2,
        thresholds: zero,
      },
      [LEAP_ID]: {
        cardType: 'magic',
        leapAttackAlly: true,
        manaCost: 1,
        thresholds: { ...zero, fire: 1 },
      },
      [ORIGIN_ID]: {
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [STEP_TARGET_ID]: {
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        stealth: true,
        summonToAnySite: true,
        thresholds: zero,
      },
      [WARDED_ID]: {
        airborne: true,
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
        ward: true,
      },
      'north-avatar': avatar,
      'north-filler': {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      'north-site': { cardType: 'site', elements: ['fire'] },
      'south-avatar': avatar,
      'south-site': { cardType: 'site', elements: ['water'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [LEAP_ID, ALLY_ID, 'north-filler'],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [ORIGIN_ID, STEP_TARGET_ID, WARDED_ID],
      },
    },
    firstSeat: 'north',
    seed,
  });
}

function deathriteManifest(seed: number) {
  const zero = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const avatar = {
    attack: 1,
    cardType: 'avatar' as const,
    defense: 1,
    drawSpell: false,
    life: 20,
  };
  const fragileA = `${DEATHRITE_FRAGILE_ID}-a`;
  const fragileB = `${DEATHRITE_FRAGILE_ID}-b`;
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'synthetic-leap-attack-deathrite-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-leap-attack-deathrite-v1',
    },
    cards: {
      [LEAP_ID]: {
        cardType: 'magic',
        leapAttackAlly: true,
        manaCost: 0,
        thresholds: zero,
      },
      [DEATHRITE_ENEMY_ID]: {
        attack: 1,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [fragileA]: {
        attack: 0,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      [fragileB]: {
        attack: 0,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      [DEATHRITE_RAIN_ID]: {
        cardType: 'magic',
        damageEachAbovegroundMinion: 1,
        manaCost: 0,
        thresholds: zero,
      },
      [DEATHRITE_SOURCE_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        otherNearbyAlliesPowerBonus: 1,
        thresholds: zero,
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['fire'] },
      'south-avatar': avatar,
      'south-filler': {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      'south-site': { cardType: 'site', elements: ['water'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [
          LEAP_ID,
          DEATHRITE_SOURCE_ID,
          fragileA,
          fragileB,
          DEATHRITE_RAIN_ID,
        ],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [
          DEATHRITE_ENEMY_ID,
          'south-filler',
          'south-filler',
          'south-filler',
          'south-filler',
          'south-filler',
        ],
      },
    },
    firstSeat: 'north',
    seed,
  });
}

async function captureMovementDeathriteParityForManifest(
  gameManifest: GameManifest,
): Promise<JsonValue> {
  return withRustSession(gameManifest, async (handle) => {
    const fragileIds = [`${DEATHRITE_FRAGILE_ID}-a`, `${DEATHRITE_FRAGILE_ID}-b`];
    const step = async (predicate: (action: GameLegalAction) => boolean): Promise<void> => {
      await handle.take(predicate);
    };
    const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
      && action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0;

    await step(keep);
    await step(keep);
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
    for (const fragileId of fragileIds) {
      await step((action) => action.descriptor.kind === 'summon-minion'
        && action.descriptor.cardId === fragileId
        && action.descriptor.cell === 'C4');
    }
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
    await step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === DEATHRITE_SOURCE_ID
      && action.descriptor.cell === 'C3');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
    await step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === DEATHRITE_ENEMY_ID
      && action.descriptor.cell === 'C2');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'cast-magic' && action.descriptor.cardId === DEATHRITE_RAIN_ID);

    const session = handle.snapshot;
    const source = session.state.realm.units.find(({ cardId }) => cardId === DEATHRITE_SOURCE_ID);
    const enemy = session.state.realm.units.find(({ cardId }) => cardId === DEATHRITE_ENEMY_ID);
    const fragiles = session.state.realm.units.filter(({ cardId }) => fragileIds.includes(cardId));
    if (!source || !enemy || fragiles.length !== 2) {
      throw new Error('expected complete Leap Attack Deathrite position');
    }

    const leapAction = (await handle.legalActions('north')).find(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardId === LEAP_ID
        && descriptor.ally?.instanceId === source.instanceId
        && descriptor.allyDestination?.cell === 'C2');
    if (!leapAction) throw new Error('expected Leap Attack Deathrite cast action');

    const interrupted = await handle.stepAction(leapAction);
    if (!interrupted.accepted) {
      throw new Error(`Leap Attack Deathrite was rejected: ${interrupted.reason.code}`);
    }
    const continuation = interrupted.session.state.pendingDeathrites?.continuation;
    if (continuation?.kind !== 'leap-attack') {
      throw new Error('expected leap-attack continuation');
    }
    const pendingStateHash = await handle.stateHash(interrupted.session.state.decisionSeat);
    const pendingSaved = await handle.checkpoint();
    await handle.resume(pendingSaved.checkpoint);
    const restored = handle.snapshot;
    if (canonicalJson(restored as unknown as JsonValue)
      !== canonicalJson(interrupted.session as unknown as JsonValue)) {
      throw new Error('Leap Attack Deathrite checkpoint did not restore byte-identically');
    }
    const orderActions = (await handle.legalActions()).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    if (orderActions.length !== 2) {
      throw new Error(`expected two Deathrite order actions, received ${orderActions.length}`);
    }
    const selectedOrderAction = orderActions[0]!;
    const resolved = await handle.stepAction(selectedOrderAction);
    if (!resolved.accepted) {
      throw new Error(`Leap Attack Deathrite order was rejected: ${resolved.reason.code}`);
    }
    if (!(await handle.verifyReplay())) {
      throw new Error('Leap Attack Deathrite replay failed verification');
    }
    const resolvedStateHash = await handle.stateHash(resolved.session.state.decisionSeat);
    const resolvedSaved = await handle.checkpoint();

    return {
      leapAction,
      manifestId: gameManifest.manifestId,
      pending: {
        checkpointId: pendingSaved.checkpointId,
        checkpointRoundTrip: true,
        continuation,
        decisionSeat: interrupted.session.state.decisionSeat,
        expectedSessionHash: pendingSaved.expectedSessionHash,
        orderActions,
        phase: interrupted.session.state.phase,
        receipt: interrupted.receipt,
        serializedCheckpointHash: pendingSaved.serializedCheckpointHash,
        stateHash: pendingStateHash,
        stateVersion: interrupted.session.state.stateVersion,
      },
      resolved: {
        checkpointId: resolvedSaved.checkpointId,
        decisionSeat: resolved.session.state.decisionSeat,
        expectedSessionHash: resolvedSaved.expectedSessionHash,
        phase: resolved.session.state.phase,
        receipt: resolved.receipt,
        replayVerified: true,
        selectedOrderActionId: selectedOrderAction.actionId,
        serializedCheckpointHash: resolvedSaved.serializedCheckpointHash,
        stateHash: resolvedStateHash,
        stateVersion: resolved.session.state.stateVersion,
      },
    };
  });
}

async function previewStepSummary(
  gameManifest: GameManifest,
  checkpoint: JsonValue,
  action: GameLegalAction,
  build: (result: Extract<GameStepResult, { accepted: true }>, session: GameSession) => JsonValue,
): Promise<JsonValue> {
  return withRustSession(gameManifest, async (preview) => {
    await preview.resume(checkpoint);
    const result = await preview.stepAction(action);
    if (!result.accepted) throw new Error(`preview step was rejected: ${result.reason.code}`);
    return build(result, preview.snapshot);
  });
}

export async function captureLeapAttackActionParityFixture(): Promise<JsonValue> {
  const gameManifest = mainManifest(MAIN_SEED);
  return withRustSession(gameManifest, async (handle) => {
    const step = async (predicate: (action: GameLegalAction) => boolean): Promise<void> => {
      await handle.take(predicate);
    };
    const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
      && action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0;
    const endTurn = (action: GameLegalAction): boolean => action.descriptor.kind === 'end-turn';
    const playSite = (cell: string) => (action: GameLegalAction): boolean =>
      action.descriptor.kind === 'play-site' && action.descriptor.cell === cell;
    const drawAtlas = (action: GameLegalAction): boolean =>
      action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas';
    const summon = (cardId: string, cell: string) => (action: GameLegalAction): boolean =>
      action.descriptor.kind === 'summon-minion'
        && action.descriptor.cardId === cardId
        && action.descriptor.cell === cell;

    await step(keep);
    await step(keep);
    await step(playSite('C4'));
    await step(summon(ALLY_ID, 'C4'));
    await step(endTurn);
    await step(drawAtlas);
    await step(playSite('C1'));
    await step(summon(ORIGIN_ID, 'C4'));
    await step(endTurn);
    await step(drawAtlas);
    await step(playSite('C3'));
    await step(endTurn);
    await step(drawAtlas);
    await step(summon(STEP_TARGET_ID, 'C3'));
    await step(summon(WARDED_ID, 'C3'));
    await step(endTurn);
    await step(drawAtlas);

    const session = handle.snapshot;
    const allyInstanceId = session.state.realm.units.find(({ cardId }) =>
      cardId === ALLY_ID)?.instanceId;
    const originInstanceId = session.state.realm.units.find(({ cardId }) =>
      cardId === ORIGIN_ID)?.instanceId;
    const stepTargetInstanceId = session.state.realm.units.find(({ cardId }) =>
      cardId === STEP_TARGET_ID)?.instanceId;
    const wardedInstanceId = session.state.realm.units.find(({ cardId }) =>
      cardId === WARDED_ID)?.instanceId;
    const leapInstanceId = session.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === LEAP_ID)?.instanceId;
    if (!allyInstanceId || !originInstanceId || !stepTargetInstanceId
      || !wardedInstanceId || !leapInstanceId) {
      throw new Error('expected complete synthetic Leap Attack position');
    }

    const actions = (await handle.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === leapInstanceId
        && descriptor.ally?.instanceId === allyInstanceId);
    if (actions.length !== 2) {
      throw new Error(`expected two Leap Attack actions, received ${actions.length}`);
    }
    const noStepAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.allyDestination?.cell === 'C4');
    const stepAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.allyDestination?.cell === 'C3');
    if (!noStepAction || !stepAction) {
      throw new Error('expected stay and step Leap Attack actions');
    }
    if (stepAction.descriptor.kind !== 'cast-magic') {
      throw new Error('expected Leap Attack cast descriptor');
    }

    const startSaved = await handle.checkpoint();

    return {
      actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
      allyInstanceId,
      canonicalActionIds: actions.map(({ actionId }) => actionId),
      contract: 'sorcery-core-v1',
      manifestId: gameManifest.manifestId,
      movementDeathrite: await captureMovementDeathriteParityForManifest(deathriteManifest(DEATHRITE_SEED)),
      noStepTransition: await transitionFromCheckpoint(
        gameManifest,
        startSaved.checkpoint,
        noStepAction,
        await previewStepSummary(gameManifest, startSaved.checkpoint, noStepAction, (result, previewSession) => {
          const noStepAlly = previewSession.state.realm.units.find(({ instanceId }) =>
            instanceId === allyInstanceId);
          return {
            allyLocation: noStepAlly?.location ?? null,
            allyTapped: noStepAlly?.tapped ?? null,
            c3EnemiesRemain: [stepTargetInstanceId, wardedInstanceId].every((instanceId) =>
              previewSession.state.realm.units.some((unit) => unit.instanceId === instanceId)),
            originDead: !previewSession.state.realm.units.some(({ instanceId }) =>
              instanceId === originInstanceId),
            unitStepped: result.receipt.events.some(({ type }) => type === 'unit-stepped'),
          };
        }),
      ),
      originInstanceId,
      schemaVersion: 1,
      seat: 'north',
      source: RUST_LEGALITY_SOURCE,
      startCheckpoint: {
        checkpointId: startSaved.checkpointId,
        expectedSessionHash: startSaved.expectedSessionHash,
        serializedCheckpointHash: startSaved.serializedCheckpointHash,
        stateHash: await handle.stateHash('north'),
      },
      stateVersion: session.state.stateVersion,
      stepTargetInstanceId,
      stepTransition: await transitionFromCheckpoint(
        gameManifest,
        startSaved.checkpoint,
        stepAction,
        await previewStepSummary(gameManifest, startSaved.checkpoint, stepAction, (result, previewSession) => {
          const stepAlly = previewSession.state.realm.units.find(({ instanceId }) =>
            instanceId === allyInstanceId);
          const stepWarded = previewSession.state.realm.units.find(({ instanceId }) =>
            instanceId === wardedInstanceId);
          const stepped = result.receipt.events.find(({ type }) => type === 'unit-stepped');
          return {
            allyDamage: stepAlly?.damage ?? null,
            allyLocation: stepAlly?.location ?? null,
            allyTapped: stepAlly?.tapped ?? null,
            leapInCemetery: previewSession.state.players.north.cemetery.some(({ cardId }) =>
              cardId === LEAP_ID),
            originAlive: previewSession.state.realm.units.some(({ instanceId }) =>
              instanceId === originInstanceId),
            stepTargetDead: !previewSession.state.realm.units.some(({ instanceId }) =>
              instanceId === stepTargetInstanceId),
            strikeCount: result.receipt.events.filter(({ type }) =>
              type === 'strike-damage-allocated').length,
            unitStepped: stepped?.payload ?? null,
            wardBroken: stepWarded?.warded === false,
          };
        }),
      ),
      wardedInstanceId,
    };
  });
}

export async function serializeLeapAttackActionParityFixture(): Promise<string> {
  return `${canonicalJson(await captureLeapAttackActionParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeLeapAttackActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Leap Attack fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  void main();
}
