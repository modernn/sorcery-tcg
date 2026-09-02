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
  '../tests/engine/fixtures/sparkmage-action-v1.json',
  import.meta.url,
));

function manifest() {
  const thresholds = { air: 0, earth: 0, fire: 0, water: 0 } as const;
  return createGameManifest({
    authority: {
      contentHash: identityHash({ fixture: 'sparkmage-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-sparkmage-action-v1',
    },
    cards: {
      'north-avatar': {
        attack: 1,
        cardType: 'avatar',
        defense: 1,
        drawSpell: false,
        life: 20,
        tapDamageRandomOtherUnitAtNearbyLocationPerAirThresholdCastThisTurn: true,
      },
      'north-minion': {
        attack: 1, cardType: 'minion', defense: 1, manaCost: 0, thresholds,
      },
      'north-site': { cardType: 'site', elements: ['air'] },
      'south-avatar': {
        attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20,
      },
      'south-minion': {
        attack: 1, cardType: 'minion', defense: 1, manaCost: 0, thresholds,
      },
      'south-site': { cardType: 'site', elements: ['earth'] },
    },
    decks: {
      north: {
        atlas: Array(9).fill('north-site'),
        avatar: 'north-avatar',
        spellbook: Array(3).fill('north-minion'),
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: Array(3).fill('south-minion'),
      },
    },
    firstSeat: 'north',
    seed: 148,
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

export function captureSparkmageActionParityFixture(): JsonValue {
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
  const playSite = (cell: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'play-site' && action.descriptor.cell === cell;
  const drawAtlas = (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'draw' && action.descriptor.zone === 'atlas';

  step((action) => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0);
  step((action) => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0);

  const northSites = ['C4', 'C3', 'B3', 'B4', 'D3', 'D4'] as const;
  const southSites = ['C1', 'C2', 'B1', 'B2', 'D1'] as const;
  step(playSite(northSites[0]));
  for (let turn = 0; turn < southSites.length; turn += 1) {
    step(kind('end-turn'));
    step(drawAtlas);
    step(playSite(southSites[turn]!));
    step(kind('end-turn'));
    step(drawAtlas);
    step(playSite(northSites[turn + 1]!));
  }
  step(kind('end-turn'));
  step(drawAtlas);
  step(kind('end-turn'));
  step(drawAtlas);

  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'activate-sparkmage');
  if (actions.length !== 6) {
    throw new Error(`expected six Sparkmage actions, received ${actions.length}`);
  }
  const existingNearbySurfaceCells = northSites.filter((cell) =>
    session.state.realm.sites[cell] !== undefined);
  if (existingNearbySurfaceCells.length !== 6) {
    throw new Error('expected six existing nearby surface locations');
  }

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    avatarInstanceId: session.state.players.north.avatar.card.instanceId,
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    manifestId: gameManifest.manifestId,
    schemaVersion: 1,
    seat: 'north',
    setupActionIds,
    source: 'typescript-legality-engine',
    stateVersion: session.state.stateVersion,
  };
}

export function serializeSparkmageActionParityFixture(): string {
  return `${canonicalJson(captureSparkmageActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeSparkmageActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Sparkmage action parity fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}
