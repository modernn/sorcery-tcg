import assert from 'node:assert/strict';
import test from 'node:test';

import { createGameManifest, type GameCardDefinition, type GameManifestInput } from '../../src/engine/game.ts';

type Effect = NonNullable<Extract<GameCardDefinition, { cardType: 'magic' }>['effectProgram']>['effects'][number];

const authority = {
  contentHash: 'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const,
  mode: 'synthetic' as const,
  revisionId: 'synthetic-artifact-token-test',
};

function input(effect: Effect, token: GameCardDefinition): GameManifestInput {
  const cards: Record<string, GameCardDefinition> = {
    avatar: { attack: 1, cardType: 'avatar', defense: 1, drawSpell: false, life: 20 },
    site: { cardType: 'site', elements: ['earth'] },
    'north-spell': { cardType: 'magic', effectProgram: { effects: [effect] }, manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
    'south-spell': { cardType: 'magic', effectProgram: { effects: [{ op: 'draw-card' }] }, manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
    token,
  };
  return {
    authority,
    cards,
    decks: {
      north: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['north-spell', 'north-spell', 'north-spell'] },
      south: { atlas: ['site', 'site', 'site'], avatar: 'avatar', spellbook: ['south-spell', 'south-spell', 'south-spell'] },
    },
    firstSeat: 'north',
    seed: 1,
  };
}

test('conjure-token admits and canonicalizes token artifacts with null mana cost', () => {
  const manifest = createGameManifest(input(
    { count: 1, destination: 'source', op: 'conjure-token', token: 'token' },
    { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true },
  ));
  assert.deepEqual(manifest.cards.token, {
    bearerControllerChoosesExtraRandomOutcome: true,
    cardType: 'artifact',
    manaCost: null,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    token: true,
  });
});

test('carried conjure-token preserves explicit placement', () => {
  const manifest = createGameManifest(input(
    { count: 1, destination: 'target', op: 'conjure-token', placement: 'carried', token: 'token' },
    { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true },
  ));
  assert.deepEqual((manifest.cards['north-spell'] as Extract<GameCardDefinition, { cardType: 'magic' }>).effectProgram, {
    effects: [{ count: 1, destination: 'target', op: 'conjure-token', placement: 'carried', token: 'token' }],
  });
});

test('loose and default conjure-token retain location destinations', () => {
  for (const placement of [undefined, 'loose'] as const) {
    const manifest = createGameManifest(input(
      { count: 1, destination: 'location', op: 'conjure-token', ...(placement ? { placement } : {}), token: 'token' },
      { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true },
    ));
    assert.equal((manifest.cards['north-spell'] as Extract<GameCardDefinition, { cardType: 'magic' }>).effectProgram
      ?.effects[0]?.op, 'conjure-token');
  }
});

test('carried conjure-token rejects location destinations and non-carriable artifacts', () => {
  const token = { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact' as const,
    manaCost: null, thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true as const } as const;
  assert.throws(() => createGameManifest(input(
    { count: 1, destination: 'location', op: 'conjure-token', placement: 'carried', token: 'token' } as never, token,
  )), /destination is unsupported/);
  assert.throws(() => createGameManifest(input(
    { count: 1, destination: 'chosen-location', op: 'conjure-token', placement: 'carried', token: 'token' } as never, token,
  )), /destination is unsupported/);
  assert.throws(() => createGameManifest(input(
    { count: 1, destination: 'target', op: 'conjure-token', placement: 'carried', token: 'token' },
    { ...token, bearerControllerChoosesExtraRandomOutcome: true as const, cannotBeCarried: true },
  )), /token artifact/);
});

test('summon-token rejects placement', () => {
  assert.throws(() => createGameManifest(input(
    { count: 1, destination: 'source', op: 'summon-token', placement: 'carried', token: 'token' } as never,
    { attack: 1, cardType: 'minion', defense: 1, manaCost: null, token: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } },
  )), /placement is unsupported/);
});

test('legacy summon-token remains minion-only', () => {
  assert.throws(() => createGameManifest(input(
    { count: 1, destination: 'source', op: 'summon-token', token: 'token' },
    { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true },
  )), /token minion/);
});

test('conjure-token rejects minions and non-token artifacts', () => {
  const conjure = { count: 1, destination: 'source', op: 'conjure-token', token: 'token' } as const;
  assert.throws(() => createGameManifest(input(conjure, {
    attack: 1, cardType: 'minion', defense: 1, manaCost: null, token: true,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  })), /token artifact/);
  assert.throws(() => createGameManifest(input(conjure, {
    bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: 1,
    thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
  })), /token artifact/);
});

test('artifact token metadata requires null mana cost only for token artifacts', () => {
  assert.throws(() => createGameManifest(input(
    { count: 1, destination: 'source', op: 'conjure-token', token: 'token' },
    { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 } } as never,
  )), /null only for token artifacts/);
});

test('token artifacts cannot enter a spellbook', () => {
  const candidate = input(
    { count: 1, destination: 'source', op: 'conjure-token', token: 'token' },
    { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true },
  );
  assert.throws(() => createGameManifest({ ...candidate, decks: {
    ...candidate.decks,
    north: { ...candidate.decks.north, spellbook: ['token', 'token', 'token'] },
  } }), /references an unsupported spell|exactly the deck-referenced definitions/);
});

test('mixed summon and conjure operations cannot share one token ID', () => {
  const candidate = input(
    { count: 1, destination: 'source', op: 'summon-token', token: 'token' },
    { bearerControllerChoosesExtraRandomOutcome: true, cardType: 'artifact', manaCost: null,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 }, token: true },
  );
  assert.throws(() => createGameManifest({ ...candidate, cards: {
    ...candidate.cards,
    'north-spell': ({ ...candidate.cards['north-spell'], effectProgram: { effects: [
      { count: 1, destination: 'source', op: 'summon-token', token: 'token' },
      { count: 1, destination: 'source', op: 'conjure-token', token: 'token' },
    ] } } as never),
  } }), /token minion/);
});
