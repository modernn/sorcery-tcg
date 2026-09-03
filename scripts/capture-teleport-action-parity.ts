import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import type { EngineReceipt } from '../src/engine/contract.ts';
import {
  createGameManifest,
  type GameLegalAction,
  type GameSession,
} from '../src/engine/game.ts';
import {
  RUST_LEGALITY_SOURCE,
  transitionFromCheckpoint,
  withRustSession,
} from '../src/engine/rust-session-helpers.ts';

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

async function previewAction(
  gameManifest: ReturnType<typeof manifest>,
  checkpoint: JsonValue,
  action: GameLegalAction,
): Promise<Readonly<{ receipt: EngineReceipt; session: GameSession }>> {
  return withRustSession(gameManifest, async (previewHandle) => {
    await previewHandle.resume(checkpoint);
    const result = await previewHandle.stepAction(action);
    if (!result.accepted) {
      throw new Error(`Teleport preview action was rejected: ${result.reason.code}`);
    }
    return { receipt: result.receipt, session: result.session };
  });
}

export async function captureTeleportActionParityFixture(): Promise<JsonValue> {
  const gameManifest = manifest(MAIN_SEED);
  return withRustSession(gameManifest, async (handle) => {
    const step = async (predicate: (action: GameLegalAction) => boolean): Promise<void> => {
      await handle.take(predicate);
    };
    const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
      && action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0;

    await step(keep);
    await step(keep);
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
    await step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === ALLY_ID
      && action.descriptor.cell === 'C4'
      && action.descriptor.region === 'underwater');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');

    const session = handle.snapshot;
    const ally = session.state.realm.units.find(({ cardId }) => cardId === ALLY_ID);
    const teleport = session.state.players.north.hand.spellbook.find(({ cardId }) =>
      cardId === TELEPORT_ID);
    const destinationSite = session.state.realm.sites.C1;
    if (!ally || !teleport || !destinationSite) {
      throw new Error('expected complete synthetic Teleport position');
    }

    const actions = (await handle.legalActions('north')).filter(({ descriptor }) =>
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

    const startSaved = await handle.checkpoint();
    const startStateHash = await handle.stateHash('north');
    const avatarPreview = await previewAction(gameManifest, startSaved.checkpoint, avatarNoMove);
    const allyPreview = await previewAction(gameManifest, startSaved.checkpoint, allyTeleport);

    return {
      actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
      allyInstanceId: ally.instanceId,
      allyTeleportTransition: await transitionFromCheckpoint(
        gameManifest,
        startSaved.checkpoint,
        allyTeleport,
        {
          eventTypes: allyPreview.receipt.events.map(({ type }) => type),
          targetSiteInstanceId: destinationSite.instanceId,
          unit: unitSummary(allyPreview.session, ally.instanceId),
        },
      ),
      avatarNoMoveTransition: await transitionFromCheckpoint(
        gameManifest,
        startSaved.checkpoint,
        avatarNoMove,
        {
          eventTypes: avatarPreview.receipt.events.map(({ type }) => type),
          unit: unitSummary(
            avatarPreview.session,
            session.state.players.north.avatar.card.instanceId,
          ),
        },
      ),
      canonicalActionIds: actions.map(({ actionId }) => actionId),
      contract: 'sorcery-core-v1',
      manifestId: gameManifest.manifestId,
      schemaVersion: 1,
      seat: 'north',
      source: RUST_LEGALITY_SOURCE,
      startCheckpoint: {
        checkpointId: startSaved.checkpointId,
        expectedSessionHash: startSaved.expectedSessionHash,
        serializedCheckpointHash: startSaved.serializedCheckpointHash,
        stateHash: startStateHash,
      },
      stateVersion: session.state.stateVersion,
      teleportInstanceId: teleport.instanceId,
      targetSiteInstanceId: destinationSite.instanceId,
    };
  });
}

export async function serializeTeleportActionParityFixture(): Promise<string> {
  return `${canonicalJson(await captureTeleportActionParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeTeleportActionParityFixture();
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

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  void main();
}
