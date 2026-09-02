import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameCheckpoint,
  parseGameCheckpoint,
  resumeGameCheckpoint,
  serializeGameCheckpoint,
} from '../src/engine/checkpoint.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  stepGame,
  verifyGameReplay,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
} from '../src/engine/game.ts';

const FIXTURE_PATH = fileURLToPath(new URL(
  '../tests/engine/fixtures/sacrifice-summon-action-v1.json',
  import.meta.url,
));

const SUMMONER_ID = 'synthetic-tithe-beast';
const FODDER_ID = 'synthetic-fodder-minion';

function manifest(deathrites = false): GameManifest {
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
  const input = {
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
  for (let seed = 1; seed <= 4_096; seed += 1) {
    const candidate = createGameManifest({ ...input, seed });
    const north = createGameSession(candidate).state.players.north;
    if (north.hand.spellbook.some(({ cardId }) => cardId === SUMMONER_ID)
      && north.hand.spellbook.filter(({ cardId }) => cardId === FODDER_ID).length === 2
      && north.spellbook[0]?.cardId === FODDER_ID) return candidate;
  }
  throw new Error('expected a deterministic synthetic sacrifice fixture seed');
}

function readyToSummon(gameManifest: GameManifest): GameSession {
  let session = createGameSession(gameManifest);
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    session = take(session, predicate);
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

  step(keep);
  step(keep);
  step(playSite('C4'));
  step(summonFodder);
  step(summonFodder);
  step(endTurn);
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  step(playSite('C1'));
  step(endTurn);
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step(playSite('C3'));
  step(summonFodder);
  return session;
}

function take(
  session: GameSession,
  predicate: (action: GameLegalAction) => boolean,
): GameSession {
  const action = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!action) throw new Error('expected deterministic sacrifice-summon setup action');
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued setup action was rejected: ${result.reason.code}`);
  return result.session;
}

export function captureSacrificeSummonActionParityFixture(): JsonValue {
  const gameManifest = manifest();
  const session = readyToSummon(gameManifest);

  const chosen = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === SUMMONER_ID)?.instanceId;
  if (!chosen) throw new Error('expected synthetic sacrifice summoner in the north hand');
  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === chosen
      && descriptor.cell === 'C4');
  if (actions.length !== 8) throw new Error(`expected eight sacrifice-payment actions, received ${actions.length}`);
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
  const result = stepGame(session, selectedAction);
  if (!result.accepted) throw new Error(`issued sacrifice-payment action was rejected: ${result.reason.code}`);
  if (result.receipt.randomDraws.length !== 0) throw new Error('sacrifice payment unexpectedly used randomness');

  const deathriteManifest = manifest(true);
  const deathriteSession = readyToSummon(deathriteManifest);
  const deathriteChosen = deathriteSession.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === SUMMONER_ID)?.instanceId;
  if (!deathriteChosen) throw new Error('expected Deathrite sacrifice summoner in the north hand');
  const paymentAction = legalGameActions(deathriteSession.state, 'north').find(({ descriptor }) =>
    descriptor.kind === 'summon-minion'
      && descriptor.cardInstanceId === deathriteChosen
      && descriptor.cell === 'C4'
      && descriptor.sacrificedMinionInstanceIds?.length === 2);
  if (!paymentAction) throw new Error('expected a two-minion Deathrite sacrifice payment');
  const interrupted = stepGame(deathriteSession, paymentAction);
  if (!interrupted.accepted) throw new Error(`Deathrite payment was rejected: ${interrupted.reason.code}`);
  const pendingCheckpoint = createGameCheckpoint(interrupted.session);
  const serializedCheckpoint = serializeGameCheckpoint(pendingCheckpoint);
  const restored = resumeGameCheckpoint(parseGameCheckpoint(serializedCheckpoint));
  if (canonicalJson(restored as unknown as JsonValue)
    !== canonicalJson(interrupted.session as unknown as JsonValue)) {
    throw new Error('Deathrite checkpoint did not restore byte-identically');
  }
  const orderActions = legalGameActions(restored.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'order-deathrites');
  if (orderActions.length !== 2) {
    throw new Error(`expected two Deathrite order actions, received ${orderActions.length}`);
  }
  const selectedOrderAction = orderActions[0]!;
  const resolved = stepGame(restored, selectedOrderAction);
  if (!resolved.accepted) throw new Error(`Deathrite order was rejected: ${resolved.reason.code}`);
  if (!verifyGameReplay(resolved.session)) throw new Error('resolved Deathrite replay failed verification');
  const resolvedCheckpoint = createGameCheckpoint(resolved.session);

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    deathrite: {
      manifestId: deathriteManifest.manifestId,
      paymentAction,
      pending: {
        checkpointId: pendingCheckpoint.checkpointId,
        checkpointRoundTrip: true,
        decisionSeat: interrupted.session.state.decisionSeat,
        expectedSessionHash: pendingCheckpoint.expectedSessionHash,
        orderActions,
        phase: interrupted.session.state.phase,
        receipt: interrupted.receipt,
        serializedCheckpointHash: identityHash(serializedCheckpoint),
        stateHash: hashGameState(interrupted.session.state),
        stateVersion: interrupted.session.state.stateVersion,
      },
      resolved: {
        checkpointId: resolvedCheckpoint.checkpointId,
        decisionSeat: resolved.session.state.decisionSeat,
        expectedSessionHash: resolvedCheckpoint.expectedSessionHash,
        phase: resolved.session.state.phase,
        receipt: resolved.receipt,
        replayVerified: true,
        selectedOrderActionId: selectedOrderAction.actionId,
        serializedCheckpointHash: identityHash(serializeGameCheckpoint(resolvedCheckpoint)),
        stateHash: hashGameState(resolved.session.state),
        stateVersion: resolved.session.state.stateVersion,
      },
    },
    eligibleSacrificeCandidateIds,
    manifestId: gameManifest.manifestId,
    schemaVersion: 1,
    seat: 'north',
    source: 'typescript-legality-engine',
    stateVersion: session.state.stateVersion,
    transition: {
      receipt: result.receipt,
      selectedActionId: selectedAction.actionId,
    },
  };
}

export function serializeSacrificeSummonActionParityFixture(): string {
  return `${canonicalJson(captureSacrificeSummonActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeSacrificeSummonActionParityFixture();
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
