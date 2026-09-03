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
  '../tests/engine/fixtures/lure-action-v1.json',
  import.meta.url,
));

const LURE_ID = 'synthetic-lure';
const MOBILE_ENEMY_ID = 'synthetic-lure-mobile';
const IMMOBILE_ENEMY_ID = 'synthetic-lure-immobile';
const MAIN_SEED = 100;

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
      contentHash: identityHash({ fixture: 'synthetic-lure-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-lure-action-v1',
    },
    cards: {
      [IMMOBILE_ENEMY_ID]: {
        attack: 1,
        cardType: 'minion',
        defense: 3,
        immobile: true,
        manaCost: 0,
        summonToAnySite: true,
        thresholds: zero,
      },
      [LURE_ID]: {
        cardType: 'magic',
        lureEnemyMinionOneStepCloser: true,
        manaCost: 1,
        thresholds: { ...zero, water: 1 },
      },
      [MOBILE_ENEMY_ID]: {
        attack: 1,
        cardType: 'minion',
        charge: true,
        defense: 3,
        manaCost: 0,
        stealth: true,
        summonToAnySite: true,
        thresholds: zero,
        ward: true,
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['water'] },
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
        spellbook: Array(10).fill(LURE_ID),
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: [MOBILE_ENEMY_ID, IMMOBILE_ENEMY_ID, ...Array(8).fill('south-filler')],
      },
    },
    firstSeat: 'north',
    seed,
  });
}

export async function captureLureActionParityFixture(): Promise<JsonValue> {
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
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
    const immobileCard = handle.snapshot.state.players[handle.snapshot.state.decisionSeat].hand.spellbook
      .find(({ cardId }) => cardId === IMMOBILE_ENEMY_ID);
    if (!immobileCard) throw new Error(`seed ${gameManifest.seed} missing immobile enemy card`);
    await step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardInstanceId === immobileCard.instanceId
      && action.descriptor.cell === 'C3');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'D4');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'D2');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'D3');
    const mobileCard = handle.snapshot.state.players[handle.snapshot.state.decisionSeat].hand.spellbook
      .find(({ cardId }) => cardId === MOBILE_ENEMY_ID);
    if (!mobileCard) throw new Error(`seed ${gameManifest.seed} missing mobile enemy card`);
    await step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardInstanceId === mobileCard.instanceId
      && action.descriptor.cell === 'D3');
    const mobileBeforeMove = handle.snapshot.state.realm.units.find(({ cardId }) =>
      cardId === MOBILE_ENEMY_ID);
    if (!mobileBeforeMove) throw new Error('missing mobile enemy before charge');
    await step((action) => action.descriptor.kind === 'move-and-attack'
      && action.descriptor.unitInstanceId === mobileBeforeMove.instanceId
      && action.descriptor.path.length === 1);
    await step((action) => action.descriptor.kind === 'decline-attack');
    await step((action) => action.descriptor.kind === 'end-turn');
    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'spellbook');

    const session = handle.snapshot;
    const allyInstanceId = session.state.players.north.avatar.card.instanceId;
    const mobile = session.state.realm.units.find(({ cardId }) => cardId === MOBILE_ENEMY_ID);
    const lureCards = session.state.players.north.hand.spellbook.filter(({ cardId }) =>
      cardId === LURE_ID).slice(0, 3);
    if (!mobile || lureCards.length !== 3) {
      throw new Error('expected complete synthetic Lure position');
    }

    const actions = (await handle.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === lureCards[0]?.instanceId);
    if (actions.length !== 2) {
      throw new Error(`expected two Lure actions, received ${actions.length}`);
    }
    const destinationCells = actions.flatMap(({ descriptor }) =>
      descriptor.kind === 'cast-magic' && descriptor.temptedDestination
        ? [descriptor.temptedDestination.cell]
        : []).sort();
    if (destinationCells.join(',') !== 'C3,D4') {
      throw new Error(`unexpected Lure destinations: ${destinationCells.join(',')}`);
    }

    const firstLure = actions.find(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.temptedDestination?.cell === 'C3');
    if (!firstLure) throw new Error('expected C3 Lure action');

    const startSaved = await handle.checkpoint();
    const startStateHash = await handle.stateHash('north');
    const firstResult = await handle.stepAction(firstLure);
    if (!firstResult.accepted) throw new Error('first Lure rejected');
    const secondLure = (await handle.legalActions('north')).find(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === lureCards[1]?.instanceId
        && descriptor.temptedEnemy?.instanceId === mobile.instanceId
        && descriptor.temptedDestination?.cell === 'C4');
    if (!secondLure) throw new Error('expected C4 Lure action after first cast');
    const secondResult = await handle.stepAction(secondLure);
    if (!secondResult.accepted) throw new Error('second Lure rejected');
    const noOpActions = (await handle.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === lureCards[2]?.instanceId);
    if (noOpActions.length !== 1) {
      throw new Error(`expected one no-op Lure action, received ${noOpActions.length}`);
    }
    const noOpAction = noOpActions[0]!;
    const beforeNoOpSaved = await handle.checkpoint();
    const noOpPreview = await handle.stepAction(noOpAction);
    if (!noOpPreview.accepted) throw new Error('no-op Lure rejected');

    return {
      actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
      allyInstanceId,
      canonicalActionIds: actions.map(({ actionId }) => actionId),
      contract: 'sorcery-core-v1',
      firstLureTransition: await transitionFromCheckpoint(
        gameManifest,
        startSaved.checkpoint,
        firstLure,
        {
          eventTypes: firstResult.receipt.events.map(({ type }) => type),
          mobileLocation: firstResult.session.state.realm.units.find(({ instanceId }) =>
            instanceId === mobile.instanceId)?.location ?? null,
        },
      ),
      manifestId: gameManifest.manifestId,
      mobileEnemyInstanceId: mobile.instanceId,
      noOpTransition: await transitionFromCheckpoint(
        gameManifest,
        beforeNoOpSaved.checkpoint,
        noOpAction,
        {
          eventTypes: noOpPreview.receipt.events.map(({ type }) => type),
          mobileLocation: noOpPreview.session.state.realm.units.find(({ instanceId }) =>
            instanceId === mobile.instanceId)?.location ?? null,
        },
      ),
      schemaVersion: 1,
      seat: 'north',
      secondLureAction: {
        actionId: secondLure.actionId,
        descriptor: secondLure.descriptor,
        label: secondLure.label,
      },
      source: RUST_LEGALITY_SOURCE,
      startCheckpoint: {
        checkpointId: startSaved.checkpointId,
        expectedSessionHash: startSaved.expectedSessionHash,
        serializedCheckpointHash: startSaved.serializedCheckpointHash,
        stateHash: startStateHash,
      },
      stateVersion: session.state.stateVersion,
    };
  });
}

export async function serializeLureActionParityFixture(): Promise<string> {
  return `${canonicalJson(await captureLureActionParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeLureActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Lure parity fixture is stale: ${FIXTURE_PATH}`);
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
