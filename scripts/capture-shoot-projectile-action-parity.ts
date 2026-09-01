import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../src/authority/canonical-json.ts';
import { identityHash } from '../src/authority/hash.ts';
import {
  createGameManifest,
  createGameSession,
  legalGameActions,
  stepGame,
  type GameLegalAction,
  type GameSession,
} from '../src/engine/game.ts';

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

function take(
  session: GameSession,
  predicate: (action: GameLegalAction) => boolean,
): Readonly<{ action: GameLegalAction; session: GameSession }> {
  const action = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!action) throw new Error('expected deterministic setup action');
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued setup action was rejected: ${result.reason.code}`);
  return { action, session: result.session };
}

export function captureShootProjectileActionParityFixture(): JsonValue {
  const gameManifest = manifest();
  let session = createGameSession(gameManifest);
  const setupActionIds: string[] = [];
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    const result = take(session, predicate);
    setupActionIds.push(result.action.actionId);
    session = result.session;
  };
  const kind = (wanted: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === wanted;

  step((action) => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0);
  step((action) => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0);
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C4');
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardId === 'north-shooter'
    && action.descriptor.cell === 'C4');
  const shooter = session.state.realm.units.find(({ controller }) => controller === 'north');
  if (!shooter) throw new Error('missing synthetic Ranged shooter');
  step(kind('end-turn'));

  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C1');
  step(kind('end-turn'));

  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C3');
  step(kind('end-turn'));

  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  step((action) => action.descriptor.kind === 'play-site' && action.descriptor.cell === 'C2');
  step((action) => action.descriptor.kind === 'summon-minion'
    && action.descriptor.cardId === 'south-target'
    && action.descriptor.cell === 'C3');
  const target = session.state.realm.units.find(({ controller }) => controller === 'south');
  if (!target) throw new Error('missing synthetic Ranged target');
  step(kind('end-turn'));

  step((action) => action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas');
  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'shoot-projectile');
  if (actions.length !== 4) throw new Error(`expected four Ranged actions, received ${actions.length}`);
  if (legalGameActions(session.state, 'north').some(({ descriptor }) =>
    descriptor.kind === 'shoot-damage-projectile')) {
    throw new Error('ordinary Ranged fixture must not include fixed-damage projectile actions');
  }

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    manifestId: gameManifest.manifestId,
    schemaVersion: 1,
    seat: 'north',
    setupActionIds,
    shooterInstanceId: shooter.instanceId,
    source: 'typescript-legality-engine',
    stateVersion: session.state.stateVersion,
    targetInstanceId: target.instanceId,
  };
}

export function serializeShootProjectileActionParityFixture(): string {
  return `${canonicalJson(captureShootProjectileActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeShootProjectileActionParityFixture();
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
