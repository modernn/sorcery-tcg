import assert from 'node:assert/strict';
import test from 'node:test';

import { canonicalJson } from '../../src/authority/canonical-json.ts';
import {
  createGameManifest,
  createGameSession,
  hashGameState,
  legalGameActions,
  observeGame,
  stepGame,
  verifyGameReplay,
  type GameDeckSpec,
  type GameLegalAction,
  type GameManifest,
  type GameSession,
} from '../../src/engine/game.ts';

const SYNTHETIC_AUTHORITY_HASH =
  'sha256:1111111111111111111111111111111111111111111111111111111111111111' as const;

function deck(prefix: string, atlasCount = 30, spellbookCount = 50): GameDeckSpec {
  return {
    atlas: Array.from({ length: atlasCount }, (_, index) => `${prefix}-site-${index + 1}`),
    avatar: `${prefix}-avatar`,
    spellbook: Array.from({ length: spellbookCount }, (_, index) => `${prefix}-spell-${index + 1}`),
  };
}

function manifest(
  seed = 1,
  options: Readonly<{
    north?: GameDeckSpec;
    south?: GameDeckSpec;
  }> = {},
): GameManifest {
  return createGameManifest({
    authority: {
      contentHash: SYNTHETIC_AUTHORITY_HASH,
      mode: 'synthetic',
      revisionId: 'synthetic-setup-fixture-v1',
    },
    decks: {
      north: options.north ?? deck('north'),
      south: options.south ?? deck('south'),
    },
    firstSeat: 'north',
    seed,
  });
}

function action(
  session: GameSession,
  predicate: (candidate: GameLegalAction) => boolean,
): GameLegalAction {
  const found = legalGameActions(session.state, session.state.activeSeat).find(predicate);
  assert.ok(found, 'expected legal action');
  return found;
}

function accept(session: GameSession, candidate: GameLegalAction): GameSession {
  const result = stepGame(session, candidate);
  assert.equal(result.accepted, true);
  return result.session;
}

function keep(session: GameSession): GameSession {
  return accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0));
}

test('RULE-01 setup shuffles two decks, deals split hidden hands, and places Avatars', () => {
  const session = createGameSession(manifest(7));
  const north = session.state.players.north;
  const southView = observeGame(session.state, 'south');

  assert.equal(session.state.phase, 'mulligan');
  assert.equal(session.state.activeSeat, 'north');
  assert.equal(session.state.turnNumber, 0);
  assert.equal(north.hand.atlas.length, 3);
  assert.equal(north.hand.spellbook.length, 3);
  assert.equal(north.atlas.length, 27);
  assert.equal(north.spellbook.length, 47);
  assert.equal(north.avatar.location, 'C4');
  assert.equal(session.state.players.south.avatar.location, 'C1');
  assert.ok(session.initialRandomDraws.length >= 156);
  assert.ok(session.initialRandomDraws.every(({ purpose }) => purpose.startsWith('setup_')));
  assert.equal(southView.players.north.hand.atlas, 3);
  assert.equal(southView.players.north.hand.spellbook, 3);
  assert.doesNotMatch(JSON.stringify(southView), /north-(?:site|spell)-/);
  assert.equal(legalGameActions(session.state, 'north').length, 76);
  assert.deepEqual(legalGameActions(session.state, 'south'), []);
});

test('TEST-03 different opponent hidden cards cannot change an observation or legal actions', () => {
  const first = createGameSession(manifest(9));
  const second = createGameSession(manifest(9, {
    north: {
      ...deck('north'),
      atlas: deck('north').atlas.map((_, index) => `alternate-site-${index + 1}`),
      spellbook: deck('north').spellbook.map((_, index) => `alternate-spell-${index + 1}`),
    },
  }));

  assert.notEqual(first.state.players.north.hand.atlas[0]?.cardId, second.state.players.north.hand.atlas[0]?.cardId);
  assert.equal(canonicalJson(observeGame(first.state, 'south')), canonicalJson(observeGame(second.state, 'south')));
  assert.equal(
    canonicalJson(legalGameActions(first.state, 'south')),
    canonicalJson(legalGameActions(second.state, 'south')),
  );
});

test('RULE-01 one mulligan returns at most three chosen cards to their deck bottoms and redraws', () => {
  const initial = createGameSession(manifest(11));
  const returned = initial.state.players.north.hand.atlas[0];
  assert.ok(returned);
  const mulligan = action(initial, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 1
      && descriptor.atlasOrder[0] === returned.instanceId
      && descriptor.spellbookOrder.length === 0);
  const result = stepGame(initial, mulligan);
  assert.equal(result.accepted, true);
  const north = result.session.state.players.north;

  assert.equal(north.mulliganComplete, true);
  assert.equal(north.hand.atlas.length, 3);
  assert.equal(north.atlas.length, 27);
  assert.equal(north.atlas.at(-1)?.instanceId, returned.instanceId);
  assert.equal(north.hand.atlas.some(({ instanceId }) => instanceId === returned.instanceId), false);
  assert.deepEqual(result.receipt.events[0]?.payload, {
    atlasCount: 1,
    seat: 'north',
    spellbookCount: 0,
  });
  assert.equal(result.session.state.activeSeat, 'south');
});

test('RULE-01 first player skips its draw, establishes a domain, then second player chooses a deck', () => {
  let session = keep(createGameSession(manifest(13)));
  session = keep(session);

  assert.equal(session.state.turnNumber, 1);
  assert.equal(session.state.activeSeat, 'north');
  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.players.north.hand.atlas.length, 3);
  assert.equal(session.transcript.at(-1)?.events.at(-1)?.type, 'turn-started');
  assert.deepEqual(session.transcript.at(-1)?.events.at(-1)?.payload, {
    drawSkipped: true,
    seat: 'north',
    turnNumber: 1,
  });

  const site = action(session, ({ descriptor }) => descriptor.kind === 'play-site');
  const siteId = site.descriptor.kind === 'play-site' ? site.descriptor.cardInstanceId : '';
  session = accept(session, site);
  assert.equal(session.state.realm.sites.C4?.instanceId, siteId);
  assert.equal(session.state.realm.sites.C4?.controller, 'north');
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.players.north.domainEstablished, true);
  assert.equal(session.state.players.north.mana, 1);
  assert.deepEqual(legalGameActions(session.state, 'north').map(({ descriptor }) => descriptor.kind), ['end-turn']);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  assert.equal(session.state.turnNumber, 2);
  assert.equal(session.state.activeSeat, 'south');
  assert.equal(session.state.phase, 'draw');
  assert.deepEqual(
    legalGameActions(session.state, 'south').map(({ descriptor }) => descriptor.kind === 'draw' && descriptor.zone),
    ['atlas', 'spellbook'],
  );

  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
  assert.equal(session.state.phase, 'main');
  assert.equal(session.state.players.south.hand.atlas.length, 4);
  assert.deepEqual(session.transcript.at(-1)?.events[0]?.payload, { seat: 'south', zone: 'atlas' });
  assert.doesNotMatch(canonicalJson(session.transcript.at(-1)?.events[0]?.payload ?? null), /south-site-/);
  assert.equal(verifyGameReplay(session), true);
});

function northSecondMain(seed = 23, shortDecks = false): GameSession {
  const options = shortDecks
    ? { north: deck('north', 3, 4), south: deck('south', 3, 4) }
    : {};
  let session = keep(createGameSession(manifest(seed, options)));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  return accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
}

test('RULE-02 sites expand through unoccupied orthogonal cells controlled by their player', () => {
  let session = northSecondMain();
  const northCard = session.state.players.north.hand.atlas[0];
  assert.ok(northCard);
  const northCells = legalGameActions(session.state, 'north')
    .filter(({ descriptor }) =>
      descriptor.kind === 'play-site' && descriptor.cardInstanceId === northCard.instanceId)
    .map(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell);
  assert.deepEqual(northCells, ['B4', 'C3', 'D4']);

  const playC3 = action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site'
      && descriptor.cardInstanceId === northCard.instanceId
      && descriptor.cell === 'C3');
  const beforeHand = session.state.players.north.hand.atlas.length;
  const result = stepGame(session, playC3);
  assert.equal(result.accepted, true);
  session = result.session;
  assert.equal(session.state.realm.sites.C3?.instanceId, northCard.instanceId);
  assert.equal(session.state.realm.sites.C3?.controller, 'north');
  assert.equal(session.state.players.north.hand.atlas.length, beforeHand - 1);
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.equal(session.state.players.north.mana, 2);
  assert.equal(result.receipt.events[0]?.type, 'site-played');
  assert.deepEqual(legalGameActions(session.state, 'north').map(({ descriptor }) => descriptor.kind), ['end-turn']);
  assert.equal(verifyGameReplay(session), true);

  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'play-site' && descriptor.cell === 'C2'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));

  const expansion = [...new Set(legalGameActions(session.state, 'north')
    .filter(({ descriptor }) => descriptor.kind === 'play-site')
    .map(({ descriptor }) => descriptor.kind === 'play-site' && descriptor.cell))];
  assert.deepEqual(expansion, ['B3', 'B4', 'D3', 'D4']);
  assert.equal(session.state.players.north.avatar.tapped, false);
  assert.equal(session.state.players.north.mana, 2);
});

test('RULE-02 the Avatar may draw a private site instead of playing one', () => {
  let session = northSecondMain(29);
  const before = session.state.players.north;
  const drawn = before.atlas[0];
  assert.ok(drawn);
  const draw = action(session, ({ descriptor }) => descriptor.kind === 'draw-site');
  const result = stepGame(session, draw);
  assert.equal(result.accepted, true);
  session = result.session;

  assert.equal(session.state.players.north.atlas.length, before.atlas.length - 1);
  assert.equal(session.state.players.north.hand.atlas.length, before.hand.atlas.length + 1);
  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.deepEqual(result.receipt.events[0]?.payload, { seat: 'north' });
  assert.equal(result.receipt.events[0]?.type, 'site-drawn');
  assert.equal(canonicalJson(result.receipt.events[0]?.payload ?? null).includes(drawn.cardId), false);
  assert.equal(canonicalJson(observeGame(session.state, 'south')).includes(drawn.cardId), false);
  assert.deepEqual(legalGameActions(session.state, 'north').map(({ descriptor }) => descriptor.kind), ['end-turn']);
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-02 drawing a site from an empty Atlas pays the tap cost and loses', () => {
  let session = northSecondMain(31, true);
  assert.equal(session.state.players.north.atlas.length, 0);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'draw-site'));

  assert.equal(session.state.players.north.avatar.tapped, true);
  assert.deepEqual(session.state.terminal, {
    loser: 'north',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'south',
  });
  assert.equal(verifyGameReplay(session), true);
});

test('RULE-02 forged spatial actions cannot mutate the game', () => {
  const session = northSecondMain(37);
  const beforeState = canonicalJson(session.state);
  const result = stepGame(session, {
    actionId: 'sha256:2222222222222222222222222222222222222222222222222222222222222222',
    seat: 'north',
    stateVersion: session.state.stateVersion,
  });

  assert.equal(result.accepted, false);
  assert.equal(result.reason.code, 'unknown_action');
  assert.equal(canonicalJson(result.session.state), beforeState);
  assert.equal(result.session.transcript.length, session.transcript.length);
});

test('shared stale rejection leaves game state, PRNG, and accepted transcript unchanged', () => {
  const initial = createGameSession(manifest(17));
  const command = action(initial, ({ descriptor }) =>
    descriptor.kind === 'mulligan'
      && descriptor.atlasOrder.length === 0
      && descriptor.spellbookOrder.length === 0);
  const accepted = stepGame(initial, command);
  assert.equal(accepted.accepted, true);
  const before = canonicalJson(accepted.session.state);
  const beforeHash = hashGameState(accepted.session.state);
  const stale = stepGame(accepted.session, command);

  assert.equal(stale.accepted, false);
  assert.equal(stale.reason.code, 'stale_version');
  assert.equal(stale.reason.currentStateHash, beforeHash);
  assert.equal(canonicalJson(stale.session.state), before);
  assert.equal(stale.session.transcript.length, 1);
  assert.equal(stale.session.attempts.length, 2);
});

test('RULE-01 attempting to draw from an empty deck immediately loses', () => {
  const short = deck('short', 3, 3);
  let session = keep(createGameSession(manifest(19, { north: short, south: short })));
  session = keep(session);
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'play-site'));
  session = accept(session, action(session, ({ descriptor }) => descriptor.kind === 'end-turn'));
  session = accept(session, action(session, ({ descriptor }) =>
    descriptor.kind === 'draw' && descriptor.zone === 'atlas'));

  assert.deepEqual(session.state.terminal, {
    loser: 'south',
    reason: 'deck_empty',
    status: 'finished',
    winner: 'north',
  });
  assert.equal(session.state.phase, 'terminal');
  assert.deepEqual(legalGameActions(session.state, 'south'), []);
  assert.equal(session.transcript.at(-1)?.events[0]?.type, 'game-ended');
  assert.equal(verifyGameReplay(session), true);
});
