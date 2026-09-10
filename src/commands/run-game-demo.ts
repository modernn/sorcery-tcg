import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, type JsonValue } from '../authority/canonical-json.ts';
import { identityHash } from '../authority/hash.ts';
import {
  createGameManifest,
  hashGameState,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameManifest,
  type GameTerminal,
} from '../engine/game.ts';
import { runRustSyntheticDemo, type Sha256Hash } from '../engine/rust-engine.ts';
import { withRustSession } from '../engine/rust-session-helpers.ts';

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

function demoCards(decks: Readonly<Record<'north' | 'south', GameDeckSpec>>): Record<string, GameCardDefinition> {
  const cards: Record<string, GameCardDefinition> = {};
  for (const deck of Object.values(decks)) {
    cards[deck.avatar] = { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 };
    deck.atlas.forEach((cardId) => {
      cards[cardId] = { cardType: 'site', elements: ['earth'] };
    });
    deck.spellbook.forEach((cardId) => {
      cards[cardId] = {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      };
    });
  }
  return cards;
}

export function createSyntheticDemoManifest(seed = 1): GameManifest {
  const decks = { north: demoDeck('north'), south: demoDeck('south') };
  return createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-setup-fixture-v1',
    },
    cards: demoCards(decks),
    decks,
    firstSeat: 'north',
    seed,
  });
}

export type DeterministicGameReport = Readonly<{
  acceptedActionCount: number;
  classification: 'unranked_partial_rules';
  finalStateHash: Sha256Hash;
  fightCount: number;
  replayVerified: boolean;
  terminal: Extract<GameTerminal, { status: 'finished' }>;
  transcriptHash: Sha256Hash;
  turnCount: number;
}>;

export async function runDeterministicGame(manifest: GameManifest): Promise<DeterministicGameReport> {
  return withRustSession(manifest, async (handle) => {
    while (handle.snapshot.state.terminal.status === 'active'
      && handle.snapshot.transcript.length < MAX_ACTIONS) {
      const result = await handle.stepAction(await handle.selectPolicyAction());
      if (!result.accepted) throw new Error(`deterministic demo action rejected: ${result.reason.code}`);
    }
    const session = handle.snapshot;
    if (session.state.terminal.status !== 'finished') {
      throw new Error('deterministic demo exceeded action limit');
    }
    return Object.freeze({
      acceptedActionCount: session.transcript.length,
      classification: 'unranked_partial_rules',
      finalStateHash: hashGameState(session.state),
      fightCount: session.transcript.flatMap(({ events }) => events)
        .filter(({ type }) => type === 'fight-started').length,
      replayVerified: await handle.verifyReplay(),
      terminal: session.state.terminal,
      transcriptHash: identityHash(session.transcript as unknown as JsonValue),
      turnCount: session.state.turnNumber,
    });
  });
}

export function runGameDemo(seed = 1): DeterministicGameReport {
  const report = runRustSyntheticDemo(seed);
  return Object.freeze({
    acceptedActionCount: report.acceptedActionCount,
    classification: 'unranked_partial_rules',
    fightCount: report.fightCount,
    finalStateHash: report.finalStateHash,
    replayVerified: report.replayVerified,
    terminal: report.terminal as DeterministicGameReport['terminal'],
    transcriptHash: report.transcriptHash,
    turnCount: report.turnCount,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const seed = process.argv[2] === undefined ? 1 : Number(process.argv[2]);
  process.stdout.write(`${canonicalJson(runGameDemo(seed))}\n`);
}
