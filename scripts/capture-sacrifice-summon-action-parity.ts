import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameManifest,
  type GameLegalAction,
  type GameManifest,
} from '../src/engine/game.ts';
import {
  RUST_LEGALITY_SOURCE,
  RustGameSessionHandle,
  withRustSession,
} from '../src/engine/rust-session-helpers.ts';

const FIXTURE_PATH = fileURLToPath(new URL(
  '../tests/engine/fixtures/sacrifice-summon-action-v1.json',
  import.meta.url,
));

const SUMMONER_ID = 'synthetic-tithe-beast';
const FODDER_ID = 'synthetic-fodder-minion';

function manifestInput(deathrites = false) {
  const fixtureId = deathrites
    ? 'synthetic-sacrifice-summon-deathrite-v1'
    : 'synthetic-sacrifice-summon-action-v1';
  const zero = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  const avatar = {
    attack: 1,
    cardType: 'avatar' as const,
    defense: 1,
    drawSpell: false,
    life: 20,
  };
  return {
    authority: {
      contentHash: identityHash({ fixture: fixtureId }),
      mode: 'synthetic' as const,
      revisionId: fixtureId,
    },
    cards: {
      [FODDER_ID]: {
        attack: 1,
        cardType: 'minion' as const,
        ...(deathrites ? { deathriteDrawSite: true as const } : {}),
        defense: 2,
        manaCost: 0,
        thresholds: zero,
      },
      [SUMMONER_ID]: {
        attack: 8,
        cardType: 'minion' as const,
        defense: 4,
        manaCost: 6,
        sacrificeMinionAtSummoningLocationForManaDiscount: 2 as const,
        thresholds: zero,
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site' as const, elements: ['earth'] as const, genesisGainMana: 5 },
      'south-avatar': avatar,
      'south-minion': {
        attack: 1,
        cardType: 'minion' as const,
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      'south-site': { cardType: 'site' as const, elements: ['water'] as const },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [SUMMONER_ID, FODDER_ID, FODDER_ID, FODDER_ID],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: Array(3).fill('south-minion'),
      },
    },
    firstSeat: 'north' as const,
  };
}

async function manifest(deathrites = false): Promise<GameManifest> {
  const input = manifestInput(deathrites);
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const north = await withRustSession(candidate, async (handle) =>
      handle.snapshot.state.players.north);
    if (north.hand.spellbook.some(({ cardId }) => cardId === SUMMONER_ID)
      && north.hand.spellbook.filter(({ cardId }) => cardId === FODDER_ID).length === 2
      && north.spellbook[0]?.cardId === FODDER_ID) return candidate;
  }
  throw new Error('expected a deterministic synthetic sacrifice fixture seed');
}

async function readyToSummon(gameManifest: GameManifest): Promise<RustGameSessionHandle> {
  const handle = await RustGameSessionHandle.open(gameManifest);
  const step = async (predicate: (action: GameLegalAction) => boolean): Promise<void> => {
    await handle.take(predicate);
  };
  const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0;
  const endTurn = (action: GameLegalAction): boolean => action.descriptor.kind === 'end-turn';
  const playSite = (cell: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'play-site' && action.descriptor.cell === cell;
  const summonFodder = (action: GameLegalAction): boolean => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardId === FODDER_ID
    && action.descriptor.cell === 'C4';

  await step(keep);
  await step(keep);
  await step(playSite('C4'));
  await step(summonFodder);
  await step(summonFodder);
  await step(endTurn);
  await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  await step(playSite('C1'));
  await step(endTurn);
  await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  await step(playSite('C3'));
  await step(summonFodder);
  return handle;
}

export async function captureSacrificeSummonActionParityFixture(): Promise<JsonValue> {
  const gameManifest = await manifest();
  const handle = await readyToSummon(gameManifest);
  try {
    const session = handle.snapshot;
    const chosen = session.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === SUMMONER_ID)?.instanceId;
    if (!chosen) throw new Error('expected synthetic sacrifice summoner in the north hand');
    const actions = (await handle.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === chosen
        && descriptor.cell === 'C4');
    if (actions.length !== 8) {
      throw new Error(`expected eight sacrifice-payment actions, received ${actions.length}`);
    }
    const eligibleSacrificeCandidateIds = session.state.realm.units
      .filter(({ controller, location, region }) =>
        controller === 'north' && location === 'C4' && region === 'surface')
      .map(({ instanceId }) => instanceId)
      .sort();
    if (eligibleSacrificeCandidateIds.length !== 3) {
      throw new Error(`expected three sacrifice candidates, received ${eligibleSacrificeCandidateIds.length}`);
    }
    const selectedAction = actions.find(({ descriptor }) => descriptor.kind === 'summon-minion'
      && descriptor.sacrificedMinionInstanceIds?.length === 3);
    if (!selectedAction) throw new Error('expected a three-minion sacrifice payment');
    const result = await handle.stepAction(selectedAction);
    if (!result.accepted) throw new Error(`issued sacrifice-payment action was rejected: ${result.reason.code}`);
    if (result.receipt.randomDraws.length !== 0) {
      throw new Error('sacrifice payment unexpectedly used randomness');
    }

    const deathriteManifest = await manifest(true);
    const deathriteHandle = await readyToSummon(deathriteManifest);
    try {
      const deathriteSession = deathriteHandle.snapshot;
      const deathriteChosen = deathriteSession.state.players.north.hand.spellbook.find(({ cardId }) =>
        cardId === SUMMONER_ID)?.instanceId;
      if (!deathriteChosen) throw new Error('expected Deathrite sacrifice summoner in the north hand');
      const paymentAction = (await deathriteHandle.legalActions('north')).find(({ descriptor }) =>
        descriptor.kind === 'summon-minion'
          && descriptor.cardInstanceId === deathriteChosen
          && descriptor.cell === 'C4'
          && descriptor.sacrificedMinionInstanceIds?.length === 2);
      if (!paymentAction) throw new Error('expected a two-minion Deathrite sacrifice payment');
      const interrupted = await deathriteHandle.stepAction(paymentAction);
      if (!interrupted.accepted) throw new Error(`Deathrite payment was rejected: ${interrupted.reason.code}`);
      const pendingStateHash = await deathriteHandle.stateHash(interrupted.session.state.decisionSeat);
      const pendingSaved = await deathriteHandle.checkpoint();
      await deathriteHandle.resume(pendingSaved.checkpoint);
      const restored = deathriteHandle.snapshot;
      if (canonicalJson(restored as unknown as JsonValue)
        !== canonicalJson(interrupted.session as unknown as JsonValue)) {
        throw new Error('Deathrite checkpoint did not restore byte-identically');
      }
      const orderActions = (await deathriteHandle.legalActions('north')).filter(({ descriptor }) =>
        descriptor.kind === 'order-deathrites');
      if (orderActions.length !== 2) {
        throw new Error(`expected two Deathrite order actions, received ${orderActions.length}`);
      }
      const selectedOrderAction = orderActions[0]!;
      const resolved = await deathriteHandle.stepAction(selectedOrderAction);
      if (!resolved.accepted) throw new Error(`Deathrite order was rejected: ${resolved.reason.code}`);
      if (!(await deathriteHandle.verifyReplay())) {
        throw new Error('resolved Deathrite replay failed verification');
      }
      const resolvedStateHash = await deathriteHandle.stateHash(resolved.session.state.decisionSeat);
      const resolvedSaved = await deathriteHandle.checkpoint();

      return {
        actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
        canonicalActionIds: actions.map(({ actionId }) => actionId),
        contract: 'sorcery-core-v1',
        deathrite: {
          manifestId: deathriteManifest.manifestId,
          paymentAction,
          pending: {
            checkpointId: pendingSaved.checkpointId,
            checkpointRoundTrip: true,
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
        },
        eligibleSacrificeCandidateIds,
        manifestId: gameManifest.manifestId,
        schemaVersion: 1,
        seat: 'north',
        source: RUST_LEGALITY_SOURCE,
        stateVersion: session.state.stateVersion,
        transition: {
          receipt: result.receipt,
          selectedActionId: selectedAction.actionId,
        },
      };
    } finally {
      await deathriteHandle.close();
    }
  } finally {
    await handle.close();
  }
}

export async function serializeSacrificeSummonActionParityFixture(): Promise<string> {
  return `${canonicalJson(await captureSacrificeSummonActionParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeSacrificeSummonActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Sacrifice-summon fixture is stale: ${FIXTURE_PATH}`);
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
