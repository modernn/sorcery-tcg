import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameCheckpoint,
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
  type GameSession,
} from '../src/engine/game.ts';

const FIXTURE_PATH = fileURLToPath(new URL(
  '../tests/engine/fixtures/teleport-action-v1.json',
  import.meta.url,
));

const TELEPORT_ID = 'synthetic-teleport';
const ALLY_ID = 'synthetic-teleport-ally';
const MAIN_SEED = 154;

function manifest(seed: number) {
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
      contentHash: identityHash({ fixture: 'synthetic-teleport-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-teleport-action-v1',
    },
    cards: {
      [ALLY_ID]: {
        attack: 1,
        cardType: 'minion',
        defense: 5,
        immobile: true,
        manaCost: 0,
        stealth: true,
        submerge: true,
        thresholds: zero,
        ward: true,
      },
      [TELEPORT_ID]: {
        cardType: 'magic',
        manaCost: 2,
        teleportAllyToTargetSite: true,
        thresholds: { air: 2, earth: 0, fire: 0, water: 0 },
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['water', 'air'] },
      'south-avatar': avatar,
      'south-filler': {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 0,
        thresholds: zero,
      },
      'south-site': { cardType: 'site', elements: ['earth'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: [ALLY_ID, TELEPORT_ID, ...Array(8).fill(TELEPORT_ID)],
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: Array(10).fill('south-filler'),
      },
    },
    firstSeat: 'north',
    seed,
  });
}

function take(
  session: GameSession,
  predicate: (action: GameLegalAction) => boolean,
): GameSession {
  const action = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!action) throw new Error('expected deterministic Teleport setup action');
  const result = stepGame(session, action);
  if (!result.accepted) {
    throw new Error(`issued Teleport setup action was rejected: ${result.reason.code}`);
  }
  return result.session;
}

function transition(
  session: GameSession,
  action: GameLegalAction,
  summary: JsonValue,
): JsonValue {
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued Teleport action was rejected: ${result.reason.code}`);
  if (!verifyGameReplay(result.session)) throw new Error('Teleport replay failed verification');
  const checkpoint = createGameCheckpoint(result.session);
  return {
    checkpointId: checkpoint.checkpointId,
    expectedSessionHash: checkpoint.expectedSessionHash,
    receipt: result.receipt,
    replayVerified: true,
    selectedActionId: action.actionId,
    serializedCheckpointHash: identityHash(serializeGameCheckpoint(checkpoint)),
    stateHash: hashGameState(result.session.state),
    summary,
  };
}

function unitSummary(resultSession: GameSession, instanceId: string): JsonValue {
  const unit = resultSession.state.realm.units.find((candidate) =>
    candidate.instanceId === instanceId);
  if (unit) {
    return {
      damage: unit.damage ?? 0,
      location: unit.location,
      region: unit.region,
      stealthed: unit.stealthed ?? false,
      tapped: unit.tapped ?? false,
      warded: unit.warded ?? false,
    };
  }
  const avatar = resultSession.state.players.north.avatar;
  return {
    damage: 0,
    location: avatar.location,
    region: avatar.region,
    stealthed: false,
    tapped: avatar.tapped ?? false,
    warded: false,
  };
}

export function captureTeleportActionParityFixture(): JsonValue {
  const gameManifest = manifest(MAIN_SEED);
  let session = createGameSession(gameManifest);
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    session = take(session, predicate);
  };
  const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0;

  step(keep);
  step(keep);
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardId === ALLY_ID
    && action.descriptor.cell === 'C4'
    && action.descriptor.region === 'underwater');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
  step((action) => action.descriptor.kind === 'end-turn');
  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');

  const ally = session.state.realm.units.find(({ cardId }) => cardId === ALLY_ID);
  const teleport = session.state.players.north.hand.spellbook.find(({ cardId }) =>
    cardId === TELEPORT_ID);
  const destinationSite = session.state.realm.sites.C1;
  if (!ally || !teleport || !destinationSite) {
    throw new Error('expected complete synthetic Teleport position');
  }

  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.cardInstanceId === teleport.instanceId);
  if (actions.length !== 6) {
    throw new Error(`expected six Teleport actions, received ${actions.length}`);
  }
  const avatarNoMove = actions.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.ally?.kind === 'avatar'
      && descriptor.targetLocation?.cell === 'C4');
  const allyTeleport = actions.find(({ descriptor }) =>
    descriptor.kind === 'cast-magic'
      && descriptor.ally?.instanceId === ally.instanceId
      && descriptor.targetLocation?.cell === 'C1');
  if (!avatarNoMove || !allyTeleport) {
    throw new Error('expected avatar no-move and ally teleport actions');
  }

  const startCheckpoint = createGameCheckpoint(session);
  const avatarPreview = stepGame(session, avatarNoMove);
  const allyPreview = stepGame(session, allyTeleport);
  if (!avatarPreview.accepted || !allyPreview.accepted) {
    throw new Error('expected Teleport preview steps to succeed');
  }

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    allyInstanceId: ally.instanceId,
    allyTeleportTransition: transition(session, allyTeleport, {
      eventTypes: allyPreview.receipt.events.map(({ type }) => type),
      targetSiteInstanceId: destinationSite.instanceId,
      unit: unitSummary(allyPreview.session, ally.instanceId),
    }),
    avatarNoMoveTransition: transition(session, avatarNoMove, {
      eventTypes: avatarPreview.receipt.events.map(({ type }) => type),
      unit: unitSummary(avatarPreview.session, session.state.players.north.avatar.card.instanceId),
    }),
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    manifestId: gameManifest.manifestId,
    schemaVersion: 1,
    seat: 'north',
    source: 'typescript-legality-engine',
    startCheckpoint: {
      checkpointId: startCheckpoint.checkpointId,
      expectedSessionHash: startCheckpoint.expectedSessionHash,
      serializedCheckpointHash: identityHash(serializeGameCheckpoint(startCheckpoint)),
      stateHash: hashGameState(session.state),
    },
    stateVersion: session.state.stateVersion,
    teleportInstanceId: teleport.instanceId,
    targetSiteInstanceId: destinationSite.instanceId,
  };
}

export function serializeTeleportActionParityFixture(): string {
  return `${canonicalJson(captureTeleportActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeTeleportActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Teleport parity fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}
