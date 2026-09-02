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
  '../tests/engine/fixtures/random-card-discard-summon-action-v1.json',
  import.meta.url,
));

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
      contentHash: identityHash({ fixture: 'synthetic-random-discard-action-v1' }),
      mode: 'synthetic',
      revisionId: 'synthetic-random-discard-action-v1',
    },
    cards: {
      'synthetic-random-discard-minion': {
        attack: 7,
        cardType: 'minion',
        defense: 5,
        discardRandomCardInsteadOfMana: true,
        manaCost: 2,
        thresholds: { ...zero, fire: 1 },
      },
      'north-avatar': avatar,
      'north-site': { cardType: 'site', elements: ['fire'] },
      'south-avatar': avatar,
      'south-minion': {
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
        spellbook: Array(3).fill('synthetic-random-discard-minion'),
      },
      south: {
        atlas: Array(9).fill('south-site'),
        avatar: 'south-avatar',
        spellbook: Array(3).fill('south-minion'),
      },
    },
    firstSeat: 'north',
    seed: 417,
  });
}

function take(
  session: GameSession,
  predicate: (action: GameLegalAction) => boolean,
): GameSession {
  const action = legalGameActions(session.state, session.state.decisionSeat).find(predicate);
  if (!action) throw new Error('expected deterministic random-discard setup action');
  const result = stepGame(session, action);
  if (!result.accepted) throw new Error(`issued setup action was rejected: ${result.reason.code}`);
  return result.session;
}

export function captureRandomCardDiscardSummonActionParityFixture(): JsonValue {
  const gameManifest = manifest();
  let session = createGameSession(gameManifest);
  const step = (predicate: (action: GameLegalAction) => boolean): void => {
    session = take(session, predicate);
  };
  const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
    && action.descriptor.atlasOrder.length === 0
    && action.descriptor.spellbookOrder.length === 0;
  const endTurn = (action: GameLegalAction): boolean => action.descriptor.kind === 'end-turn';
  const drawAtlas = (action: GameLegalAction): boolean => action.descriptor.kind === 'draw'
    && action.descriptor.zone === 'atlas';
  const playSite = (cell: string) => (action: GameLegalAction): boolean =>
    action.descriptor.kind === 'play-site' && action.descriptor.cell === cell;

  step(keep);
  step(keep);
  step(playSite('C4'));
  step(endTurn);
  step(drawAtlas);
  step(playSite('C1'));
  step(endTurn);
  step(drawAtlas);
  step(playSite('C3'));

  const chosen = session.state.players.north.hand.spellbook[0]?.instanceId;
  if (!chosen) throw new Error('expected synthetic random-discard minion in the north hand');
  const actions = legalGameActions(session.state, 'north').filter(({ descriptor }) =>
    descriptor.kind === 'summon-minion' && descriptor.cardInstanceId === chosen);
  if (actions.length !== 4) throw new Error(`expected four random-discard actions, received ${actions.length}`);
  const eligibleCandidates = [
    ...session.state.players.north.hand.atlas.map(({ instanceId }) => ({ instanceId, zone: 'atlas' })),
    ...session.state.players.north.hand.spellbook
      .filter(({ instanceId }) => instanceId !== chosen)
      .map(({ instanceId }) => ({ instanceId, zone: 'spellbook' })),
  ];
  if (eligibleCandidates.length <= 1) throw new Error('expected multiple random discard candidates');
  const selectedAction = actions[0]!;
  const result = stepGame(session, selectedAction);
  if (!result.accepted) throw new Error(`issued random-discard action was rejected: ${result.reason.code}`);

  return {
    actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
    canonicalActionIds: actions.map(({ actionId }) => actionId),
    contract: 'sorcery-core-v1',
    manifestId: gameManifest.manifestId,
    schemaVersion: 1,
    seat: 'north',
    source: 'typescript-legality-engine',
    stateVersion: session.state.stateVersion,
    transition: {
      eligibleCandidates,
      receipt: result.receipt,
      selectedActionId: selectedAction.actionId,
    },
  };
}

export function serializeRandomCardDiscardSummonActionParityFixture(): string {
  return `${canonicalJson(captureRandomCardDiscardSummonActionParityFixture())}\n`;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const serialized = serializeRandomCardDiscardSummonActionParityFixture();
  if (process.argv[2] === '--check') {
    if (readFileSync(FIXTURE_PATH, 'utf8') !== serialized) {
      throw new Error(`Random-card-discard summon fixture is stale: ${FIXTURE_PATH}`);
    }
  } else if (process.argv[2] === '--write') {
    writeFileSync(FIXTURE_PATH, serialized);
  } else {
    process.stdout.write(serialized);
  }
}
