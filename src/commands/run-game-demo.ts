import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson } from '../authority/canonical-json.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  stepGame,
  verifyGameReplay,
  type GameDeckSpec,
  type GameLegalAction,
  type GameSession,
} from '../engine/game.ts';

const SYNTHETIC_AUTHORITY_HASH =
  'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const;
const MAX_ACTIONS = 500;

function demoDeck(prefix: string): GameDeckSpec {
  return {
    atlas: Array.from({ length: 30 }, (_, index) => `${prefix}-site-${index + 1}`),
    avatar: `${prefix}-avatar`,
    spellbook: Array.from({ length: 50 }, (_, index) => `${prefix}-spell-${index + 1}`),
  };
}

function selectAction(session: GameSession): GameLegalAction {
  const actions = legalGameActions(session.state, session.state.activeSeat);
  const selected = actions.find(({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0)
    ?? actions.find(({ descriptor }) => descriptor.kind === 'play-site')
    ?? actions.find(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone === 'atlas')
    ?? actions.find(({ descriptor }) => descriptor.kind === 'end-turn');
  if (!selected) throw new Error('deterministic demo agent has no supported legal action');
  return selected;
}

export function runGameDemo(seed = 1): Readonly<{
  acceptedActionCount: number;
  classification: 'unranked_partial_rules';
  finalStateHash: ReturnType<typeof hashGameState>;
  loser: 'north' | 'south';
  reason: 'deck_empty';
  replayVerified: boolean;
  turnCount: number;
  winner: 'north' | 'south';
}> {
  const manifest = createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-setup-fixture-v1',
    },
    decks: { north: demoDeck('north'), south: demoDeck('south') },
    firstSeat: 'north',
    seed,
  });
  let session = createGameSession(manifest);
  while (session.state.terminal.status === 'active' && session.transcript.length < MAX_ACTIONS) {
    const result = stepGame(session, selectAction(session));
    if (!result.accepted) throw new Error(`deterministic demo action rejected: ${result.reason.code}`);
    session = result.session;
  }
  if (session.state.terminal.status !== 'finished') throw new Error('deterministic demo exceeded action limit');
  return Object.freeze({
    acceptedActionCount: session.transcript.length,
    classification: 'unranked_partial_rules',
    finalStateHash: hashGameState(session.state),
    loser: session.state.terminal.loser,
    reason: session.state.terminal.reason,
    replayVerified: verifyGameReplay(session),
    turnCount: session.state.turnNumber,
    winner: session.state.terminal.winner,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const seed = process.argv[2] === undefined ? 1 : Number(process.argv[2]);
  process.stdout.write(`${canonicalJson(runGameDemo(seed))}\n`);
}
