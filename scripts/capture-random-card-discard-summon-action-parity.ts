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

export async function captureRandomCardDiscardSummonActionParityFixture(): Promise<JsonValue> {
  return withRustSession(manifest(), async (handle) => {
    const step = async (predicate: (action: GameLegalAction) => boolean): Promise<void> => {
      await handle.take(predicate);
    };
    const keep = (action: GameLegalAction): boolean => action.descriptor.kind === 'mulligan'
      && action.descriptor.atlasOrder.length === 0
      && action.descriptor.spellbookOrder.length === 0;
    const endTurn = (action: GameLegalAction): boolean => action.descriptor.kind === 'end-turn';
    const drawAtlas = (action: GameLegalAction): boolean => action.descriptor.kind === 'draw'
      && action.descriptor.zone === 'atlas';
    const playSite = (cell: string) => (action: GameLegalAction): boolean =>
      action.descriptor.kind === 'play-site' && action.descriptor.cell === cell;

    await step(keep);
    await step(keep);
    await step(playSite('C4'));
    await step(endTurn);
    await step(drawAtlas);
    await step(playSite('C1'));
    await step(endTurn);
    await step(drawAtlas);
    await step(playSite('C3'));

    const session = handle.snapshot;
    const chosen = session.state.players.north.hand.spellbook[0]?.instanceId;
    if (!chosen) throw new Error('expected synthetic random-discard minion in the north hand');
    const actions = (await handle.legalActions('north')).filter(({ descriptor }) =>
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
    const result = await handle.stepAction(selectedAction);
    if (!result.accepted) throw new Error(`issued random-discard action was rejected: ${result.reason.code}`);

    const gameManifest = manifest();
    return {
      actions: actions.map(({ actionId, descriptor, label }) => ({ actionId, descriptor, label })),
      canonicalActionIds: actions.map(({ actionId }) => actionId),
      contract: 'sorcery-core-v1',
      manifestId: gameManifest.manifestId,
      schemaVersion: 1,
      seat: 'north',
      source: RUST_LEGALITY_SOURCE,
      stateVersion: session.state.stateVersion,
      transition: {
        eligibleCandidates,
        receipt: result.receipt,
        selectedActionId: selectedAction.actionId,
      },
    };
  });
}

export async function serializeRandomCardDiscardSummonActionParityFixture(): Promise<string> {
  return `${canonicalJson(await captureRandomCardDiscardSummonActionParityFixture())}\n`;
}

async function main(): Promise<void> {
  const serialized = await serializeRandomCardDiscardSummonActionParityFixture();
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

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  void main();
}
