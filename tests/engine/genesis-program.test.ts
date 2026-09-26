import assert from 'node:assert/strict';
import test from 'node:test';
import { canonicalJson } from '../../src/authority/canonical-json.ts';
import { RustSessionClient } from '../../src/engine/rust-engine.ts';
import {
  createGameManifest,
  type GameCardDefinition,
  type GameDeckSpec,
  type GameManifestInput,
} from '../../src/engine/game.ts';

const authority = {
  contentHash: 'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-genesis-program-test',
};

function deck(seat: 'north' | 'south'): GameDeckSpec {
  return {
    atlas: [`${seat}-site-1`, `${seat}-site-2`, `${seat}-site-3`],
    avatar: `${seat}-avatar`,
    spellbook: [`${seat}-spell-1`, `${seat}-spell-2`, `${seat}-spell-3`],
  };
}

function input(
  genesisProgram: NonNullable<Extract<GameCardDefinition, { cardType: 'minion' }>['genesisProgram']>,
): GameManifestInput {
  const decks = { north: deck('north'), south: deck('south') };
  const cards: Record<string, GameCardDefinition> = {};
  for (const current of Object.values(decks)) {
    cards[current.avatar] = { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 };
    for (const site of current.atlas) cards[site] = { cardType: 'site', elements: ['earth'] };
    for (const spell of current.spellbook) {
      cards[spell] = {
        attack: 1,
        cardType: 'minion',
        defense: 1,
        manaCost: 1,
        thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
      };
    }
  }
  cards['north-spell-1'] = {
    attack: 1,
    cardType: 'minion',
    defense: 1,
    genesisProgram,
    manaCost: 1,
    thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
  };
  return { authority, cards, decks, firstSeat: 'north', seed: 77 };
}

test('genesis programs are cloned at the manifest boundary', () => {
  const relation = { measured: 2 };
  const effects = [{ amount: 1, op: 'damage' as const, recipients: 'target' as const }];
  const genesisProgram = {
    effects,
    selection: { kind: 'unit' as const, relation, unitKind: 'minion' as const },
  };
  const manifest = createGameManifest(input(genesisProgram));
  relation.measured = 99;
  effects.push({ amount: 2, op: 'damage', recipients: 'target' });

  const stored = manifest.cards['north-spell-1'];
  assert.equal(stored?.cardType, 'minion');
  if (stored?.cardType !== 'minion' || stored.genesisProgram === undefined) {
    throw new Error('genesis program missing');
  }
  assert.deepEqual(stored.genesisProgram.selection, {
    kind: 'unit',
    relation: { measured: 2 },
    unitKind: 'minion',
  });
  assert.equal(stored.genesisProgram.effects.length, 1);
});

test('genesis programs are admitted by Rust', async () => {
  const manifest = createGameManifest(input({ effects: [{ op: 'draw-card' }] }));
  const client = await RustSessionClient.start();
  try {
    await client.newSession(canonicalJson(manifest));
    assert.ok((await client.legalActions('north')).length > 0);
    assert.equal(await client.verifyReplay(), true);
  } finally {
    await client.close();
  }
});

test('genesis programs reject legacy Genesis fields and location selection', () => {
  const program = { effects: [{ op: 'draw-card' as const }] };
  const candidate = input(program);
  const card = candidate.cards['north-spell-1'];
  if (card?.cardType !== 'minion') throw new Error('minion fixture missing');
  assert.throws(
    () => createGameManifest({
      ...candidate,
      cards: { ...candidate.cards, 'north-spell-1': { ...card, genesisDrawSite: false } },
    }),
    /cannot be mixed with legacy Genesis effects/,
  );
  assert.throws(
    () => createGameManifest(input({
      effects: [{ op: 'draw-card' }],
      selection: { kind: 'location', relation: 'nearby' },
    })),
    /location selection is unsupported for Genesis programs/,
  );
});
