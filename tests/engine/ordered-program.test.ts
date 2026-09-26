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
  revisionId: 'synthetic-ordered-program-test',
};

function deck(seat: 'north' | 'south'): GameDeckSpec {
  return {
    atlas: [`${seat}-site-1`, `${seat}-site-2`, `${seat}-site-3`],
    avatar: `${seat}-avatar`,
    spellbook: [`${seat}-spell-1`, `${seat}-spell-2`, `${seat}-spell-3`],
  };
}

function baseCards(decks: Readonly<Record<'north' | 'south', GameDeckSpec>>): Record<string, GameCardDefinition> {
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
  return cards;
}

function input(effectProgram: NonNullable<Extract<GameCardDefinition, { cardType: 'magic' }>['effectProgram']>): GameManifestInput {
  const decks = { north: deck('north'), south: deck('south') };
  const cards = baseCards(decks);
  cards['north-spell-1'] = {
    cardType: 'magic',
    effectProgram,
    manaCost: 0,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  };
  return { authority, cards, decks, firstSeat: 'north', seed: 77 };
}

test('ordered effect programs preserve selection, operations, and nested relation copies', () => {
  const measured = { measured: 2 };
  const cohort: { query: {
    area: 'source';
    controller: 'allied';
    excludeSource: boolean;
    kind: 'avatar' | 'minion';
  } } = {
    query: {
      area: 'source',
      controller: 'allied',
      excludeSource: true,
      kind: 'minion',
    },
  };
  const effects = [
    {
      alliedOnly: true,
      op: 'choose-unit' as const,
      optional: true,
      relation: 'nearby' as const,
      kind: 'minion' as const,
    },
    {
      amount: 2,
      duration: 'this-turn' as const,
      modifier: 'power' as const,
      op: 'grant' as const,
      recipients: 'chosen' as const,
    },
    { op: 'give-stealth' as const, recipients: cohort },
    { op: 'draw-card' as const },
  ];
  const effectProgram = {
    effects,
    optionalSelection: false,
    selection: { kind: 'unit' as const, relation: measured, unitKind: 'minion' as const },
  };
  const manifest = createGameManifest(input(effectProgram));
  measured.measured = 99;
  cohort.query.excludeSource = false;
  cohort.query.kind = 'avatar';
  effects.push({ op: 'draw-card' });

  const stored = manifest.cards['north-spell-1'];
  assert.equal(stored?.cardType, 'magic');
  if (stored?.cardType !== 'magic' || stored.effectProgram === undefined) throw new Error('program missing');
  assert.deepEqual(stored.effectProgram.selection, {
    kind: 'unit',
    relation: { measured: 2 },
    unitKind: 'minion',
  });
  assert.equal(stored.effectProgram.effects.length, 4);
  assert.deepEqual(stored.effectProgram.effects[1], effects[1]);
  assert.deepEqual(stored.effectProgram.effects[2], {
    op: 'give-stealth',
    recipients: {
      query: {
        area: 'source',
        controller: 'allied',
        excludeSource: true,
        kind: 'minion',
      },
    },
  });
  assert.deepEqual(Object.keys(stored).sort(), ['cardType', 'effectProgram', 'manaCost', 'thresholds']);
});

test('authored programs from the deck manifest boundary are admitted by Rust', async () => {
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

test('effect programs cannot be mixed with legacy Magic effects', () => {
  const candidate = input({ effects: [{ op: 'draw-card' as const }] });
  const card = candidate.cards['north-spell-1'];
  if (card?.cardType !== 'magic') throw new Error('magic fixture missing');
  const mixed = {
    ...candidate,
    cards: {
      ...candidate.cards,
      'north-spell-1': { ...card, damageTargetUnit: 1 as const },
    },
  };
  assert.throws(() => createGameManifest(mixed), /cannot be mixed with legacy Magic effects/);
});

test('effect programs require a supported grant duration', () => {
  assert.throws(
    () => createGameManifest(input({ effects: [{ amount: 1, modifier: 'power', op: 'grant', recipients: 'chosen' }] } as never)),
    /duration must be this-turn or until-your-next-turn/,
  );
  assert.throws(
    () => createGameManifest(input({
      effects: [{ amount: 1, modifier: 'power', op: 'grant-this-turn', recipients: 'chosen' }],
    } as never)),
    /grant-this-turn is obsolete/,
  );
});

test('location choices preserve their own destination independently of unit choices', () => {
  const relation = { measured: 2 };
  const candidate = input({ effects: [
    { op: 'choose-location', relation },
    { op: 'choose-unit', relation: 'anywhere', kind: 'minion' },
    { op: 'summon-token', destination: 'chosen-location', count: 7, token: 'token' },
    { op: 'draw', count: 1, zone: 'spellbook' },
  ] });
  const manifest = createGameManifest({ ...candidate, cards: { ...candidate.cards,
    token: { attack: 1, cardType: 'minion', defense: 1, manaCost: null, token: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
  } });
  relation.measured = 99;
  const stored = manifest.cards['north-spell-1'];
  assert.equal(stored?.cardType, 'magic');
  if (stored?.cardType !== 'magic') throw new Error('magic fixture missing');
  assert.deepEqual(stored.effectProgram?.effects[0], { op: 'choose-location', relation: { measured: 2 } });
  assert.deepEqual(stored.effectProgram?.effects.slice(1),
    (candidate.cards['north-spell-1'] as Extract<GameCardDefinition, { cardType: 'magic' }>).effectProgram?.effects.slice(1));
});

test('location choices require a valid relation and cannot be optional', () => {
  for (const relation of [undefined, null, 'here', 2, {}, { measured: 0 }, { measured: -1 },
    { measured: 1.5 }, { measured: 256 }, { measured: 2, extra: true }]) {
    assert.throws(() => createGameManifest(input({ effects: [
      { op: 'choose-location', relation },
    ] } as never)), /effects\[0\].relation is unsupported/);
  }
  assert.throws(() => createGameManifest(input({ effects: [
    { op: 'choose-location', relation: 'anywhere', optional: true },
  ] } as never)), /optional is unsupported/);
});

test('chosen-location token destinations require an earlier location choice', () => {
  const summon = { op: 'summon-token', token: 'token', count: 7, destination: 'chosen-location' } as const;
  for (const effectProgram of [
    { effects: [summon] },
    { effects: [summon, { op: 'choose-location', relation: 'anywhere' }] },
    { effects: [{ op: 'choose-unit', relation: 'anywhere' }, summon] },
    { selection: { kind: 'location', relation: 'anywhere' }, effects: [summon] },
  ]) {
    assert.throws(() => createGameManifest(input(effectProgram as never)), /requires a preceding choose-location/);
  }
});

test('seven-token location programs from the deck manifest boundary are admitted by Rust', async () => {
  const candidate = input({ effects: [
    { op: 'choose-location', relation: 'nearby' },
    { op: 'summon-token', token: 'token', count: 7, destination: 'chosen-location' },
    { op: 'draw', count: 1, zone: 'spellbook' },
  ] });
  const manifest = createGameManifest({ ...candidate, cards: { ...candidate.cards,
    token: { attack: 1, cardType: 'minion', defense: 1, manaCost: null, token: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
  } });
  const client = await RustSessionClient.start();
  try {
    await client.newSession(canonicalJson(manifest));
    assert.ok((await client.legalActions('north')).length > 0);
    assert.equal(await client.verifyReplay(), true);
  } finally {
    await client.close();
  }
});
