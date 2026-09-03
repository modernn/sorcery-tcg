import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameManifest,
  type GameLegalAction,
} from '../src/engine/game.ts';
import {
  RUST_LEGALITY_SOURCE,
  transitionFromCheckpoint,
  withRustSession,
} from '../src/engine/rust-session-helpers.ts';

const FIXTURE_PATH = fileURLToPath(new URL(
  '../tests/engine/fixtures/duel-action-v1.json',
  import.meta.url,
));

const DUEL_ID = 'synthetic-forced-duel';
const ALLY_ID = 'synthetic-duel-ally';
const NORMAL_TARGET_ID = 'synthetic-duel-target';
const WARDED_TARGET_ID = 'synthetic-warded-target';
const UNDERGROUND_ALLY_ID = 'synthetic-underground-duelist';
const UNDERGROUND_TARGET_ID = 'synthetic-underground-deathrite';
const UNDERGROUND_FRAGILE_ID = 'synthetic-underground-fragile';
const UNDERGROUND_BURY_ID = 'synthetic-bury-duelists';

function manifest() {
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
      contentHash: identityHash({ fixture: 'synthetic-duel-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-duel-action-v1',
    },
    cards: {
      [ALLY_ID]: {
        attack: 3,
        cardType: 'minion',
        defense: 4,
        manaCost: 0,
        thresholds: zero,
      },
      [DUEL_ID]: {
        cardType: 'magic',
        fightAllyWithAdjacentEnemy: true,
        manaCost: 1,
        thresholds: { ...zero, earth: 1 },
      },
      [NORMAL_TARGET_ID]: {
        attack: 2,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [WARDED_TARGET_ID]: {
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
      'north-site': { cardType: 'site', elements: ['earth'] },
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
        spellbook: [DUEL_ID, ALLY_ID, 'north-filler'],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [NORMAL_TARGET_ID, WARDED_TARGET_ID, 'south-filler'],
      },
    },
    firstSeat: 'north',
    seed: 719,
  });
}

function undergroundManifest() {
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
      contentHash: identityHash({ fixture: 'synthetic-underground-duel-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-underground-duel-action-v1',
    },
    cards: {
      [DUEL_ID]: {
        cardType: 'magic',
        fightAllyWithAdjacentEnemy: true,
        manaCost: 1,
        thresholds: { ...zero, earth: 1 },
      },
      [UNDERGROUND_BURY_ID]: {
        burrowTargetMinionOrArtifact: true,
        cardType: 'magic',
        manaCost: 0,
        thresholds: zero,
      },
      [UNDERGROUND_ALLY_ID]: {
        attack: 3,
        burrowing: true,
        cardType: 'minion',
        defense: 6,
        manaCost: 0,
        strikesFirstWhileAttacking: true,
        thresholds: zero,
      },
      [UNDERGROUND_TARGET_ID]: {
        attack: 2,
        burrowing: true,
        cardType: 'minion',
        deathriteDamageEachUnitHere: 1,
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [UNDERGROUND_FRAGILE_ID]: {
        attack: 0,
        burrowing: true,
        cardType: 'minion',
        deathriteDrawSite: true,
        defense: 1,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['earth'] },
      'south-avatar': avatar,
      'south-site': { cardType: 'site', elements: ['water'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [
          DUEL_ID,
          UNDERGROUND_ALLY_ID,
          UNDERGROUND_BURY_ID,
          UNDERGROUND_BURY_ID,
          UNDERGROUND_BURY_ID,
          UNDERGROUND_BURY_ID,
        ],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [UNDERGROUND_TARGET_ID, UNDERGROUND_FRAGILE_ID, UNDERGROUND_FRAGILE_ID],
      },
    },
    firstSeat: 'north',
    seed: 1,
  });
}

async function captureUndergroundDeathriteParity(): Promise<JsonValue> {
  const gameManifest = undergroundManifest();
  return withRustSession(gameManifest, async (handle) => {
    const step = async (predicate: (action: GameLegalAction) => boolean): Promise<void> => {
      await handle.take(predicate);
    };
    const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
      && action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0;
    const summon = (cardId: string) => (action: GameLegalAction): boolean =>
      action.descriptor.kind === 'summon-minion'
        && action.descriptor.cardId === cardId
        && action.descriptor.cell === 'C4'
        && action.descriptor.region === undefined;

    await step(keep);
    await step(keep);
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
    await step(summon(UNDERGROUND_ALLY_ID));
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
    await step(summon(UNDERGROUND_TARGET_ID));
    await step(summon(UNDERGROUND_FRAGILE_ID));
    await step(summon(UNDERGROUND_FRAGILE_ID));
    await step((action) => action.descriptor.kind === 'end-turn');
    for (let turn = 0; turn < 3; turn += 1) {
      await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
      while (handle.snapshot.state.players.north.hand.spellbook.some(({ cardId }) =>
        cardId === UNDERGROUND_BURY_ID)) {
        const target = handle.snapshot.state.realm.units.find(({ region }) => region === 'surface');
        if (!target) break;
        await step((action) => action.descriptor.kind === 'cast-magic'
          && action.descriptor.cardId === UNDERGROUND_BURY_ID
          && action.descriptor.target?.instanceId === target.instanceId);
      }
      const ready = handle.snapshot.state.realm.units.every(({ region }) => region === 'underground')
        && handle.snapshot.state.players.north.hand.spellbook.some(({ cardId }) => cardId === DUEL_ID);
      if (ready) break;
      await step((action) => action.descriptor.kind === 'end-turn');
      await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
      await step((action) => action.descriptor.kind === 'end-turn');
    }

    const session = handle.snapshot;
    const ally = session.state.realm.units.find(({ cardId }) => cardId === UNDERGROUND_ALLY_ID);
    const target = session.state.realm.units.find(({ cardId }) => cardId === UNDERGROUND_TARGET_ID);
    const duel = session.state.players.north.hand.spellbook.find(({ cardId }) => cardId === DUEL_ID);
    if (!ally || !target || !duel) throw new Error('expected complete underground Duel position');
    const duelAction = (await handle.legalActions('north')).find(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === duel.instanceId
        && descriptor.ally?.instanceId === ally.instanceId
        && descriptor.target?.instanceId === target.instanceId);
    if (!duelAction) throw new Error('expected underground Duel action');
    const interrupted = await handle.stepAction(duelAction);
    if (!interrupted.accepted) throw new Error(`underground Duel was rejected: ${interrupted.reason.code}`);
    if (interrupted.receipt.randomDraws.length !== 0) {
      throw new Error('underground Duel unexpectedly used randomness');
    }
    const continuation = interrupted.session.state.pendingDeathrites?.continuation;
    if (continuation?.kind !== 'first-strike' || continuation.pending.region !== 'underground') {
      throw new Error('expected underground FirstStrike continuation');
    }
    const pendingStateHash = await handle.stateHash(interrupted.session.state.decisionSeat);
    const pendingSaved = await handle.checkpoint();
    await handle.resume(pendingSaved.checkpoint);
    const restored = handle.snapshot;
    if (canonicalJson(restored as unknown as JsonValue)
      !== canonicalJson(interrupted.session as unknown as JsonValue)) {
      throw new Error('underground Duel checkpoint did not restore byte-identically');
    }
    const orderActions = (await handle.legalActions()).filter(({ descriptor }) =>
      descriptor.kind === 'order-deathrites');
    if (orderActions.length !== 2) {
      throw new Error(`expected two underground Deathrite order actions, received ${orderActions.length}`);
    }
    const selectedOrderAction = orderActions[0]!;
    const resolved = await handle.stepAction(selectedOrderAction);
    if (!resolved.accepted) throw new Error(`underground Deathrite order was rejected: ${resolved.reason.code}`);
    if (!(await handle.verifyReplay())) throw new Error('underground Duel replay failed verification');
    const resolvedStateHash = await handle.stateHash(resolved.session.state.decisionSeat);
    const resolvedSaved = await handle.checkpoint();

    return {
      duelAction,
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

export async function captureDuelActionParityFixture(): Promise<JsonValue> {
  const gameManifest = manifest();
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
    const summon = (cardId: string) => (action: GameLegalAction): boolean =>
      action.descriptor.kind === 'summon-minion'
        && action.descriptor.cardId === cardId
        && action.descriptor.cell === 'C4';

    await step(keep);
    await step(keep);
    await step(playSite('C4'));
    await step(summon(ALLY_ID));
    await step(endTurn);
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    await step(playSite('C1'));
    await step(summon(NORMAL_TARGET_ID));
    await step(summon(WARDED_TARGET_ID));
    await step(endTurn);
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');

    const session = handle.snapshot;
    const allyInstanceId = session.state.realm.units.find(({ cardId }) =>
      cardId === ALLY_ID)?.instanceId;
    const normalTargetInstanceId = session.state.realm.units.find(({ cardId }) =>
      cardId === NORMAL_TARGET_ID)?.instanceId;
    const wardedTargetInstanceId = session.state.realm.units.find(({ cardId }) =>
      cardId === WARDED_TARGET_ID)?.instanceId;
    const duelInstanceId = session.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === DUEL_ID)?.instanceId;
    if (!allyInstanceId || !normalTargetInstanceId || !wardedTargetInstanceId || !duelInstanceId) {
      throw new Error('expected complete synthetic Duel position');
    }
    const actions = (await handle.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === duelInstanceId
        && descriptor.ally?.instanceId === allyInstanceId);
    if (actions.length !== 2) throw new Error(`expected two Duel actions, received ${actions.length}`);
    const normalAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId === normalTargetInstanceId);
    const wardedAction = actions.find(({ descriptor }) => descriptor.kind === 'cast-magic'
      && descriptor.target?.instanceId === wardedTargetInstanceId);
    if (!normalAction || !wardedAction) throw new Error('expected normal and warded Duel actions');
    const startSaved = await handle.checkpoint();

    const duelSummary = (
      action: GameLegalAction,
      allyId: string,
      targetId: string,
    ) => withRustSession(gameManifest, async (preview) => {
      await preview.resume(startSaved.checkpoint);
      const result = await preview.stepAction(action);
      if (!result.accepted) throw new Error(`issued Duel action was rejected: ${result.reason.code}`);
      const target = result.session.state.realm.units.find(({ instanceId }) => instanceId === targetId);
      return {
        allyDamage: result.session.state.realm.units.find(({ instanceId }) =>
          instanceId === allyId)?.damage ?? null,
        mana: result.session.state.players.north.mana,
        targetPresent: target !== undefined,
        targetWarded: target?.warded ?? null,
      };
    });

    return {
      actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
      allyInstanceId,
      canonicalActionIds: actions.map(({ actionId }) => actionId),
      contract: 'sorcery-core-v1',
      manifestId: gameManifest.manifestId,
      normalTargetInstanceId,
      normalTransition: await transitionFromCheckpoint(
        gameManifest,
        startSaved.checkpoint,
        normalAction,
        await duelSummary(normalAction, allyInstanceId, normalTargetInstanceId),
      ),
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
      undergroundDeathrite: await captureUndergroundDeathriteParity(),
      wardedTargetInstanceId,
      wardedTransition: await transitionFromCheckpoint(
        gameManifest,
        startSaved.checkpoint,
        wardedAction,
        await duelSummary(wardedAction, allyInstanceId, wardedTargetInstanceId),
      ),
    };
  });
}

export async function serializeDuelActionParityFixture(): Promise<string> {
  return `${canonicalJson(await captureDuelActionParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeDuelActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Duel fixture is stale: ${FIXTURE_PATH}`);
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
