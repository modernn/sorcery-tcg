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
  withRustSession,
} from '../src/engine/rust-session-helpers.ts';

const FIXTURE_PATH = fileURLToPath(new URL(
  '../tests/engine/fixtures/shoot-projectile-action-v1.json',
  import.meta.url,
));

function manifest() {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'shoot-projectile-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-shoot-projectile-action-v1',
    },
    cards: {
      'north-avatar': {
        attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20,
      },
      'north-shooter': {
        attack: 2,
        cardType: 'minion',
        defense: 2,
        manaCost: 0,
        ranged: true,
        thresholds,
      },
      'north-site': { cardType: 'site', elements: ['earth'] },
      'south-avatar': {
        attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20,
      },
      'south-site': { cardType: 'site', elements: ['earth'] },
      'south-target': {
        attack: 1,
        cardType: 'minion',
        defense: 3,
        manaCost: 0,
        summonToAnySite: true,
        thresholds,
      },
    },
    decks: {
      north: {
        atlas: Array(6).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: Array(3).fill('north-shooter'),
      },
      south: {
        atlas: Array(6).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: Array(3).fill('south-target'),
      },
    },
    firstSeat: 'north',
    seed: 81,
  });
}

export async function captureShootProjectileActionParityFixture(): Promise<JsonValue> {
  return withRustSession(manifest(), async (handle) => {
    const setupActionIds: string[] = [];
    const step = async (predicate: (action: GameLegalAction) => boolean): Promise<void> => {
      const { action } = await handle.take(predicate);
      setupActionIds.push(action.actionId);
    };
    const kind = (wanted: string) => (action: GameLegalAction): boolean =>
      action.descriptor.kind === wanted;

    await step((action) => action.descriptor.kind === 'mulligan'
      && action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0);
    await step((action) => action.descriptor.kind === 'mulligan'
      && action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0);
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
    await step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === 'north-shooter'
      && action.descriptor.cell === 'C4');
    const shooter = handle.snapshot.state.realm.units.find(({ controller }) => controller === 'north');
    if (!shooter) throw new Error('missing synthetic Ranged shooter');
    await step(kind('end-turn'));

    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
    await step(kind('end-turn'));

    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
    await step(kind('end-turn'));

    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    await step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
    await step((action) => action.descriptor.kind === 'summon-minion'
      && action.descriptor.cardId === 'south-target'
      && action.descriptor.cell === 'C3');
    const target = handle.snapshot.state.realm.units.find(({ controller }) => controller === 'south');
    if (!target) throw new Error('missing synthetic Ranged target');
    await step(kind('end-turn'));

    await step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
    const actions = (await handle.legalActions('north')).filter(({ descriptor }) =>
      descriptor.kind === 'shoot-projectile');
    if (actions.length !== 4) throw new Error(`expected four Ranged actions, received ${actions.length}`);
    if ((await handle.legalActions('north')).some(({ descriptor }) =>
      descriptor.kind === 'shoot-damage-projectile')) {
      throw new Error('ordinary Ranged fixture must not include fixed-damage projectile actions');
    }

    const gameManifest = manifest();
    return {
      actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
      canonicalActionIds: actions.map(({ actionId }) => actionId),
      contract: 'sorcery-core-v1',
      manifestId: gameManifest.manifestId,
      schemaVersion: 1,
      seat: 'north',
      setupActionIds,
      shooterInstanceId: shooter.instanceId,
      source: RUST_LEGALITY_SOURCE,
      stateVersion: handle.snapshot.state.stateVersion,
      targetInstanceId: target.instanceId,
    };
  });
}

export async function serializeShootProjectileActionParityFixture(): Promise<string> {
  return `${canonicalJson(await captureShootProjectileActionParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeShootProjectileActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Ranged action parity fixture is stale: ${FIXTURE_PATH}`);
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
