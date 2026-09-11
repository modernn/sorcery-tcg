import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import test from 'node:test';

import {
  createSyntheticDemoManifest,
  runGameDemo,
  runGameRecord,
} from '../../src/commands/run-game-demo.ts';
import { createGameManifest } from '../../src/engine/game.ts';
import { withRustSession } from '../../src/engine/rust-session-helpers.ts';
import { createGamePrototypeServer } from '../../src/prototype/game-server.ts';

type JsonObject = Record<string, unknown>;

const server = createGamePrototypeServer(31);
await new Promise<void>((resolve, reject) => {
  server.once('error', reject);
  server.listen(0, '127.0.0.1', resolve);
});
const address = server.address() as AddressInfo;
const origin = `http://127.0.0.1:${address.port}`;

test.after(async () => {
  await new Promise<void>((resolve, reject) => {
    server.close((error) => error ? reject(error) : resolve());
  });
});

async function json(path: string, init?: RequestInit, base = origin): Promise<JsonObject> {
  const response = await fetch(`${base}${path}`, init);
  const body = await response.json() as JsonObject;
  assert.equal(response.status, 200, JSON.stringify(body));
  return body;
}

async function post(path: string, body?: unknown, base = origin): Promise<JsonObject> {
  return json(path, {
    method: 'POST',
    ...(body === undefined ? {} : {
      body: JSON.stringify(body),
      headers: { 'content-type': 'application/json' },
    }),
  }, base);
}

function actions(response: JsonObject): JsonObject[] {
  return response.actions as JsonObject[];
}

function descriptor(action: JsonObject): JsonObject {
  return action.descriptor as JsonObject;
}

function findAction(response: JsonObject, predicate: (value: JsonObject) => boolean): JsonObject {
  const found = actions(response).find((candidate) => predicate(descriptor(candidate)));
  assert.ok(found, 'expected legal action');
  return found;
}

async function submit(candidate: JsonObject, base = origin): Promise<JsonObject> {
  return post('/api/action', {
    actionId: candidate.actionId,
    seat: candidate.seat,
    stateVersion: candidate.stateVersion,
  }, base);
}

function recordedAction(value: unknown): JsonObject {
  assert.equal(typeof value === 'object' && value !== null && !Array.isArray(value), true);
  const receipt = value as JsonObject;
  assert.equal(typeof receipt.actionId, 'string');
  assert.equal(typeof receipt.postStateHash, 'string');
  assert.equal(receipt.seat === 'north' || receipt.seat === 'south', true);
  assert.equal(Number.isSafeInteger(receipt.stateVersion), true);
  return {
    actionId: receipt.actionId,
    postStateHash: receipt.postStateHash,
    seat: receipt.seat,
    stateVersion: receipt.stateVersion,
  };
}

function keep(response: JsonObject): JsonObject {
  return findAction(response, (value) =>
    value.kind === 'mulligan'
      && (value.atlasOrder as unknown[]).length === 0
      && (value.spellbookOrder as unknown[]).length === 0);
}

function deterministicAction(response: JsonObject): JsonObject {
  const candidates = actions(response);
  const enemyCell = String(((((response.view as JsonObject).players as JsonObject)
    .south as JsonObject).avatar as JsonObject).location);
  const movement = candidates
    .map((candidate) => {
      const value = descriptor(candidate);
      const cell = String((value.to as JsonObject | undefined)?.cell);
      const inPlaceAvatarAttack = value.kind === 'move-and-attack'
        && (value.path as unknown[]).length === 1
        && cell === enemyCell
        && (value.to as JsonObject).region === 'surface';
      return {
        candidate,
        distance: inPlaceAvatarAttack
          ? -1
          : value.kind === 'move-and-attack' && (value.path as unknown[]).length > 1
            ? Math.abs(cell.charCodeAt(0) - enemyCell.charCodeAt(0))
              + Math.abs(Number(cell[1]) - Number(enemyCell[1]))
            : Number.POSITIVE_INFINITY,
      };
    })
    .sort((left, right) => left.distance - right.distance)[0];
  const selected = candidates.find((candidate) => {
    const value = descriptor(candidate);
    return value.kind === 'mulligan'
      && (value.atlasOrder as unknown[]).length === 0
      && (value.spellbookOrder as unknown[]).length === 0;
  })
    ?? candidates.find((candidate) => descriptor(candidate).kind === 'play-site')
    ?? candidates.find((candidate) => descriptor(candidate).kind === 'summon-minion')
    ?? candidates.find((candidate) => {
      const value = descriptor(candidate);
      return value.kind === 'draw' && value.zone === 'atlas';
    })
    ?? (movement && Number.isFinite(movement.distance) ? movement.candidate : undefined)
    ?? candidates.find((candidate) => descriptor(candidate).kind === 'end-turn')
    ?? candidates[0];
  assert.ok(selected, 'expected deterministic legal action');
  return selected;
}

test('playable-core page renders the authoritative 5x4 checkpoint without artwork', async () => {
  const response = await fetch(origin);
  const page = await response.text();
  assert.equal(response.status, 200);
  assert.match(page, /Sorcery Playable Core/);
  assert.match(page, /Unranked · partial rules/);
  assert.match(page, /<select id="preset"/);
  assert.match(page, /<select id="opponent"/);
  assert.match(page, /id="save">Save position/);
  assert.match(page, /id="resume" disabled>Resume position/);
  assert.match(page, /localStorage\.setItem\(saveKey/);
  assert.match(page, /South computer/);
  assert.equal(page.match(/class="cell"/g)?.length, 20);
  assert.doesNotMatch(page, /<img\b/i);
  assert.match(response.headers.get('content-security-policy') ?? '', /img-src 'none'/);
  assert.match(page, /role="status" aria-live="polite"><strong>Game over<\/strong>/);
  assert.match(page, /Winner:.*Loser:.*Reason:/);
  assert.match(page, /South actions/);
  assert.match(page, /Technical receipt/);
  assert.match(page, /details:not\(\[open\]\)>:not\(summary\)\{display:none\}/);
  assert.match(page, /function actionGroup/);
  assert.match(page, /items\.length<=3/);
  assert.match(page, /items\.length\+' choices'/);
  assert.match(page, /function cardFactText/);
  assert.match(page, /view\.realm\.artifacts/);
  assert.match(page, /view\.realm\.auras/);
  assert.match(page, /unit\.occupiedCells/);
  assert.match(page, /view\.realm\.immobileAreas/);
  assert.match(page, /kind==='cast-aura'/);
  assert.match(page, /grid-template-rows:repeat\(4,minmax\(0,1fr\)\)/);
  assert.match(page, /align-content:safe center[^{]*overflow:auto/);
  assert.match(page, /site minions immobile/);
  assert.match(page, /grounds airborne/);
  assert.doesNotMatch(page, /class=\"card\" title=/);
  assert.doesNotMatch(page, /button\.title=JSON\.stringify/);
  assert.match(page, /clearActionResult\(\);seat=result\.view\.decisionSeat/);
  assert.match(page, /clearActionResult\(\);seat=button\.dataset\.seat/);
  assert.match(page, /\/api\/replay[\s\S]*clearActionResult\(\)/);
  assert.match(page, /id="replay-prev" disabled>Previous step/);
  assert.match(page, /id="replay-next" disabled>Next step/);
  assert.match(page, /id="replay-step" role="status" aria-live="polite"/);
  assert.match(page, /id="replay-detail"/);
  assert.match(page, /\/api\/replay\/steps/);
  assert.match(page, /ArrowLeft/);
  assert.match(page, /ArrowRight/);
  assert.match(page, /Chain status is written in text/);
  assert.match(page, /function shiftReplay/);
  assert.match(page, /id="notice" role="status" aria-live="polite"/);
  assert.match(page, /function visibleActionButtons/);
  assert.match(page, /function focusAction/);
  assert.match(page, /function focusFirstAction/);
  assert.match(page, /\.action:focus,\.action\.is-focused/);
  assert.match(page, /Number keys 1-9 play the first nine visible actions/);
  assert.match(page, /Up and Down move among those actions/);
  assert.match(page, /aria-keyshortcuts/);
  assert.match(page, /\/\^\[1-9\]\$\//);
});

test('browser API keeps cast artifacts and auras visible without exposing the opponent hand', async () => {
  const base = createSyntheticDemoManifest(53);
  const openingSpells = await withRustSession(
    base,
    async (handle) => handle.snapshot.state.players.north.hand.spellbook,
  );
  const artifactCardId = openingSpells[0]?.cardId;
  const auraCardId = openingSpells[1]?.cardId;
  assert.ok(artifactCardId && auraCardId);
  const realmManifest = createGameManifest({
    authority: base.authority,
    cards: {
      ...base.cards,
      [artifactCardId]: {
        cardType: 'artifact',
        grantsBearerPower: 2,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
      [auraCardId]: {
        cardType: 'aura',
        immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: true,
        manaCost: 0,
        thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
      },
    },
    decks: base.decks,
    firstSeat: base.firstSeat,
    seed: base.seed,
  });
  const realmServer = createGamePrototypeServer(undefined, [{
    cardNames: {
      [artifactCardId]: 'Test Relic',
      [auraCardId]: 'Test Aura',
      'south-spell-1': 'South Secret',
    },
    id: 'realm-pieces',
    label: 'Realm pieces',
    manifest: realmManifest,
  }]);
  await new Promise<void>((resolve, reject) => {
    realmServer.once('error', reject);
    realmServer.listen(0, '127.0.0.1', resolve);
  });
  const realmOrigin = `http://127.0.0.1:${(realmServer.address() as AddressInfo).port}`;
  try {
    let current = await post('/api/reset', { presetId: 'realm-pieces', seed: 53 }, realmOrigin);
    current = await submit(keep(current), realmOrigin);
    current = await json('/api/view?seat=south', undefined, realmOrigin);
    current = await submit(keep(current), realmOrigin);
    current = await json('/api/view?seat=north', undefined, realmOrigin);
    current = await submit(findAction(current, ({ kind }) => kind === 'play-site'), realmOrigin);
    current = await submit(findAction(current, ({ bearer, cardId, kind }) =>
      kind === 'cast-artifact' && cardId === artifactCardId
        && (bearer as JsonObject | undefined)?.kind === 'avatar'), realmOrigin);
    current = await submit(findAction(current, ({ cardId, kind }) =>
      kind === 'cast-aura' && cardId === auraCardId), realmOrigin);

    const realm = (current.view as JsonObject).realm as JsonObject;
    const artifact = (realm.artifacts as JsonObject[])[0];
    const aura = (realm.auras as JsonObject[])[0];
    assert.ok(artifact && aura);
    assert.equal(artifact.cardId, artifactCardId);
    assert.equal(artifact.controller, 'north');
    assert.deepEqual(artifact.bearer, {
      instanceId: ((((current.view as JsonObject).players as JsonObject)
        .north as JsonObject).avatar as JsonObject).instanceId,
      kind: 'avatar',
      seat: 'north',
    });
    assert.equal(aura.cardId, auraCardId);
    assert.equal((aura.cells as unknown[]).length, 4);
    assert.equal(aura.turnCounters, 0);
    assert.equal((realm.immobileAreas as JsonObject[])[0]?.sourceInstanceId, aura.instanceId);
    assert.equal((current.cardNames as JsonObject)[artifactCardId], 'Test Relic');
    assert.equal((current.cardNames as JsonObject)[auraCardId], 'Test Aura');
    assert.equal(((current.cardFacts as JsonObject)[artifactCardId] as JsonObject).cardType, 'artifact');
    assert.equal(((current.cardFacts as JsonObject)[auraCardId] as JsonObject).cardType, 'aura');
    assert.doesNotMatch(JSON.stringify(current), /South Secret|south-(?:site|spell)-/);
  } finally {
    await new Promise<void>((resolve, reject) => {
      realmServer.close((error) => error ? reject(error) : resolve());
    });
  }
});

test('browser API switches injected starter presets and replays the selected match', async () => {
  const airBase = createSyntheticDemoManifest(11);
  const airManifest = createGameManifest({
    authority: airBase.authority,
    cards: Object.fromEntries(Object.entries(airBase.cards).map(([cardId, card]) => [
      cardId,
      cardId.startsWith('north-spell-')
        ? {
          cardType: 'magic' as const,
          manaCost: 0,
          teleportNearbyAllyThenDrawCard: true as const,
          thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
        }
        : card,
    ])),
    decks: airBase.decks,
    firstSeat: airBase.firstSeat,
    seed: airBase.seed,
  });
  const earthBase = createSyntheticDemoManifest(19);
  const earthManifest = createGameManifest({
    authority: earthBase.authority,
    cards: Object.fromEntries(Object.entries(earthBase.cards).map(([cardId, card]) => [
      cardId,
      cardId === 'north-avatar' && card.cardType === 'avatar'
        ? {
          ...card,
          earthSitePlayCreatesAdjacentRubble: true as const,
          replaceAdjacentRubbleWithTopAtlasSite: true as const,
        }
        : cardId.startsWith('north-site-') && card.cardType === 'site'
        ? {
          ...card,
          blocksGroundMinionEntryWhileMinionAtop: true as const,
          genesisMayBottomNextSpell: true as const,
        }
        : card,
    ])),
    decks: earthBase.decks,
    firstSeat: earthBase.firstSeat,
    seed: earthBase.seed,
  });
  const earthNames = Object.fromEntries(Object.keys(earthManifest.cards)
    .map((cardId, index) => [cardId, `Earth card ${index + 1}`]));
  earthNames['north-avatar'] = 'Earth Avatar';
  earthNames['south-spell-1'] = 'South Secret';
  const catalogServer = createGamePrototypeServer(undefined, [
    {
      cardNames: {
        'north-avatar': 'Air Avatar',
        ...Object.fromEntries(Object.keys(airManifest.cards)
          .filter((cardId) => cardId.startsWith('north-spell-'))
          .map((cardId) => [cardId, 'Blink'])),
      },
      id: 'air-starter',
      label: 'Air — Spire + Snow Leopard',
      manifest: airManifest,
    },
    {
      cardNames: earthNames,
      id: 'earth-starter',
      label: 'Earth — Valley + Wild Boars',
      manifest: earthManifest,
    },
  ]);
  await new Promise<void>((resolve, reject) => {
    catalogServer.once('error', reject);
    catalogServer.listen(0, '127.0.0.1', resolve);
  });
  const catalogOrigin = `http://127.0.0.1:${(catalogServer.address() as AddressInfo).port}`;
  try {
    let current = await post('/api/reset', { presetId: 'air-starter', seed: 11 }, catalogOrigin);
    current = await submit(keep(current), catalogOrigin);
    current = await json('/api/view?seat=south', undefined, catalogOrigin);
    current = await submit(keep(current), catalogOrigin);
    current = await json('/api/view?seat=north', undefined, catalogOrigin);
    current = await submit(findAction(current, ({ kind }) => kind === 'play-site'), catalogOrigin);
    const blinkActions = actions(current).filter((candidate) =>
      descriptor(candidate).kind === 'cast-magic' && descriptor(candidate).drawZone);
    assert.equal(blinkActions.length > 0, true);
    assert.equal(blinkActions.every((candidate) => String(candidate.label).includes(
      descriptor(candidate).drawZone === 'atlas' ? 'draw from Atlas' : 'draw from Spellbook')), true);

    current = await post('/api/reset', { presetId: 'earth-starter', seed: 23 }, catalogOrigin);
    assert.equal(current.presetId, 'earth-starter');
    assert.equal(current.seed, 23);
    assert.equal(current.mode, 'synthetic');
    assert.equal((current.cardNames as JsonObject)['north-avatar'], 'Earth Avatar');
    assert.doesNotMatch(JSON.stringify(current), /South Secret/);
    const visibleFacts = current.cardFacts as Record<string, JsonObject>;
    assert.deepEqual(visibleFacts['north-avatar'], {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      life: 20,
    });
    const northHand = ((((current.view as JsonObject).players as JsonObject)
      .north as JsonObject).hand as JsonObject);
    const firstMinion = (northHand.spellbook as JsonObject[])[0]!;
    assert.deepEqual(visibleFacts[firstMinion.cardId as string], {
      attack: 1,
      cardType: 'minion',
      defense: 1,
      manaCost: 1,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    });
    const firstSite = (northHand.atlas as JsonObject[])[0]!;
    assert.deepEqual(visibleFacts[firstSite.cardId as string], {
      cardType: 'site',
      elements: ['earth'],
      keywords: ['Occupied: blocks ground minion entry'],
    });
    assert.equal(visibleFacts['south-spell-1'], undefined);
    assert.deepEqual((current.presets as JsonObject[]).map(({ id, seed }) => ({ id, seed })), [
      { id: 'air-starter', seed: 11 },
      { id: 'earth-starter', seed: 19 },
    ]);
    const mulliganActions = actions(current)
      .filter((candidate) => descriptor(candidate).kind === 'mulligan');
    assert.equal(new Set(mulliganActions.map(({ label }) => label)).size, mulliganActions.length);
    assert.equal(mulliganActions.some(({ label }) => String(label).includes('Earth card')), true);

    current = await submit(keep(current), catalogOrigin);
    current = await json('/api/view?seat=south', undefined, catalogOrigin);
    current = await submit(keep(current), catalogOrigin);
    current = await json('/api/view?seat=north', undefined, catalogOrigin);
    const riverPlay = findAction(current, ({ kind }) => kind === 'play-site');
    assert.match(String(riverPlay.label), /then inspect the next spell/);
    assert.doesNotMatch(String(riverPlay.label), /put .* on bottom|keep .* on top/i);
    current = await submit(riverPlay, catalogOrigin);
    assert.match(String(current.playerAction), /then inspect the next spell/);
    assert.doesNotMatch(String(current.playerAction), /north-spell-/);
    const bottomNext = findAction(current, ({ choice, kind }) =>
      kind === 'resolve-genesis-spell' && choice === 'bottom-next');
    assert.match(String(bottomNext.label), /Put Earth card \d+ on bottom/);
    assert.doesNotMatch(String(bottomNext.label), /north-spell-/);
    current = await submit(bottomNext, catalogOrigin);
    current = await submit(findAction(current, ({ kind }) => kind === 'summon-minion'), catalogOrigin);
    current = await submit(findAction(current, ({ kind }) => kind === 'end-turn'), catalogOrigin);
    current = await json('/api/view?seat=south', undefined, catalogOrigin);
    current = await submit(findAction(current, ({ kind, zone }) =>
      kind === 'draw' && zone === 'atlas'), catalogOrigin);
    current = await submit(findAction(current, ({ kind }) => kind === 'play-site'), catalogOrigin);
    current = await submit(findAction(current, ({ kind }) => kind === 'end-turn'), catalogOrigin);
    current = await json('/api/view?seat=north', undefined, catalogOrigin);
    current = await submit(findAction(current, ({ kind, zone }) =>
      kind === 'draw' && zone === 'atlas'), catalogOrigin);
    const northUnit = ((((current.view as JsonObject).realm as JsonObject).units as JsonObject[])
      .find(({ controller }) => controller === 'north'))!;
    const unitAction = findAction(current, ({ kind, unitInstanceId }) =>
      kind === 'move-and-attack' && unitInstanceId === northUnit.instanceId);
    assert.match(String(unitAction.label), /Earth card \d+ · north · C4 surface/);
    assert.doesNotMatch(String(unitAction.label), new RegExp(String(northUnit.instanceId).slice(0, 15)));
    const replay = await post('/api/replay', undefined, catalogOrigin);
    assert.equal(replay.verified, true);
    assert.equal(replay.finalStateHash, current.stateHash);
  } finally {
    await new Promise<void>((resolve, reject) => {
      catalogServer.close((error) => error ? reject(error) : resolve());
    });
  }
});

test('browser API plays setup through the second-seat draw choice and verifies replay', async () => {
  const northStart = await post('/api/reset', { seed: 31 });
  assert.equal(actions(northStart).length, 76);
  assert.doesNotMatch(JSON.stringify(northStart), /south-(?:site|spell)-/);

  const northKept = await submit(keep(northStart));
  assert.equal(northKept.accepted, true);
  assert.equal(((northKept.view as JsonObject).activeSeat), 'south');
  assert.deepEqual(actions(northKept), []);

  const southStart = await json('/api/view?seat=south');
  const southKept = await submit(keep(southStart));
  assert.equal(southKept.accepted, true);

  const northMain = await json('/api/view?seat=north');
  const played = await submit(findAction(northMain, ({ kind }) => kind === 'play-site'));
  assert.equal(played.accepted, true);
  const summoned = await submit(findAction(played, ({ kind }) => kind === 'summon-minion'));
  assert.equal(summoned.accepted, true);
  assert.equal((((summoned.view as JsonObject).realm as JsonObject).units as unknown[]).length, 1);
  const ended = await submit(findAction(summoned, ({ kind }) => kind === 'end-turn'));
  assert.equal(ended.accepted, true);

  const southDraw = await json('/api/view?seat=south');
  assert.deepEqual(actions(southDraw).map((candidate) => descriptor(candidate).zone), ['atlas', 'spellbook']);
  const drawn = await submit(findAction(southDraw, ({ kind, zone }) => kind === 'draw' && zone === 'atlas'));
  assert.equal(drawn.accepted, true);
  assert.doesNotMatch(JSON.stringify(drawn.receipt), /south-site-/);

  const replay = await post('/api/replay');
  assert.equal(replay.verified, true);
  assert.equal(replay.acceptedActionCount, 6);
  assert.equal(replay.finalStateHash, drawn.stateHash);
  const steps = await json('/api/replay/steps');
  assert.equal(steps.schemaVersion, 1);
  assert.equal(steps.chained, true);
  assert.equal(steps.stepCount, 6);
  assert.equal(steps.finalStateHash, drawn.stateHash);
  const stepList = steps.steps as JsonObject[];
  assert.equal(stepList.length, 6);
  assert.equal(stepList[0]?.index, 0);
  assert.equal(stepList[5]?.index, 5);
  assert.equal(stepList[0]?.postStateHash, stepList[1]?.preStateHash);
});

test('browser API rejects a stale action without exposing or mutating authority', async () => {
  const start = await post('/api/reset', { seed: 37 });
  const command = keep(start);
  const accepted = await submit(command);
  const stale = await submit(command);
  assert.equal(accepted.accepted, true);
  assert.equal(stale.accepted, false);
  assert.equal((stale.reason as JsonObject).code, 'stale_version');
  assert.equal(stale.stateHash, accepted.stateHash);
  assert.equal('session' in stale, false);
});

test('browser API restores a canonical checkpoint across server restart', async () => {
  const start = await post('/api/reset', { opponent: 'manual', seed: 41 });
  const saved = await post('/api/checkpoint');
  assert.deepEqual(Object.keys(saved).sort(), [
    'checkpoint',
    'checkpointId',
    'opponent',
    'stateHash',
    'turnNumber',
  ]);
  assert.equal(saved.stateHash, start.stateHash);
  assert.equal(saved.opponent, 'manual');
  assert.equal(typeof saved.checkpoint, 'string');

  const advanced = await submit(keep(start));
  assert.notEqual(advanced.stateHash, saved.stateHash);
  const restarted = createGamePrototypeServer(99);
  await new Promise<void>((resolve, reject) => {
    restarted.once('error', reject);
    restarted.listen(0, '127.0.0.1', resolve);
  });
  const restartedAddress = restarted.address() as AddressInfo;
  const restartedOrigin = `http://127.0.0.1:${restartedAddress.port}`;
  try {
    const restored = await json('/api/resume?seat=north&opponent=manual', {
      body: saved.checkpoint as string,
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    }, restartedOrigin);
    assert.equal(restored.stateHash, start.stateHash);
    assert.deepEqual(actions(restored), actions(start));
    assert.doesNotMatch(JSON.stringify(restored), /south-(?:site|spell)-/);

    const continued = await submit(keep(restored), restartedOrigin);
    assert.equal(continued.accepted, true);
    const replay = await post('/api/replay', undefined, restartedOrigin);
    assert.equal(replay.verified, true);
    assert.equal(replay.finalStateHash, continued.stateHash);

    const invalid = await fetch(`${restartedOrigin}/api/resume?seat=north&opponent=manual`, {
      body: '{}',
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    });
    assert.equal(invalid.status, 400);
    assert.match(String((await invalid.json() as JsonObject).error), /checkpoint/);
    assert.equal((await json('/api/view?seat=north', undefined, restartedOrigin)).stateHash,
      continued.stateHash);
  } finally {
    await new Promise<void>((resolve, reject) => {
      restarted.close((error) => error ? reject(error) : resolve());
    });
  }
});

test('browser API lets North play a deterministic South opponent through terminal replay', async () => {
  const invalidOpponent = await fetch(`${origin}/api/reset`, {
    body: JSON.stringify({ opponent: 'north', seed: 31 }),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });
  assert.equal(invalidOpponent.status, 400);
  assert.deepEqual(await invalidOpponent.json(), { error: 'opponent must be manual or south' });
  let current = await post('/api/reset', { opponent: 'south', seed: 31 });
  assert.equal(current.opponent, 'south');
  const hiddenOpponent = await fetch(`${origin}/api/view?seat=south`);
  assert.equal(hiddenOpponent.status, 403);
  assert.deepEqual(await hiddenOpponent.json(), {
    error: 'south is hidden while controlled by the deterministic opponent',
  });
  const northKeep = keep(current);
  const forbidden = await fetch(`${origin}/api/action`, {
    body: JSON.stringify({
      actionId: northKeep.actionId,
      seat: 'south',
      stateVersion: northKeep.stateVersion,
    }),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });
  assert.equal(forbidden.status, 400);
  assert.deepEqual(await forbidden.json(), {
    error: 'south is controlled by the deterministic opponent',
  });
  assert.equal((await json('/api/view?seat=north')).stateHash, current.stateHash);
  let opponentActionCount = 0;
  let opponentSummaryCount = 0;
  let combatObserved = false;
  for (let count = 0; count < 500; count += 1) {
    const view = current.view as JsonObject;
    const north = ((view.players as JsonObject).north as JsonObject);
    combatObserved ||= Number((north.avatar as JsonObject).life) < 20
      || (north.cemetery as unknown[]).length > 0;
    if ((view.terminal as JsonObject).status === 'finished') break;
    assert.equal(view.decisionSeat, 'north');
    assert.ok(actions(current).length > 0);
    current = await submit(deterministicAction(current));
    opponentActionCount += Number(current.opponentActionCount);
    const opponentActions = current.opponentActions as JsonObject[];
    assert.equal(opponentActions.length, Number(current.opponentActionCount));
    opponentSummaryCount += opponentActions.length;
    for (const summary of opponentActions) {
      assert.deepEqual(Object.keys(summary).sort(), ['events', 'kind']);
      assert.equal(typeof summary.kind, 'string');
      assert.equal((summary.events as unknown[]).every((event) => typeof event === 'string'), true);
    }
    assert.doesNotMatch(JSON.stringify(opponentActions), /card:|sha256:|south-(?:site|spell)-/);
  }

  const terminal = (current.view as JsonObject).terminal as JsonObject;
  assert.equal(terminal.status, 'finished');
  assert.equal(terminal.reason, 'avatar_defeated');
  assert.notEqual(terminal.winner, terminal.loser);
  assert.deepEqual(actions(current), []);
  assert.ok(opponentActionCount > 0);
  assert.equal(opponentSummaryCount, opponentActionCount);
  assert.equal(combatObserved, true);
  const replay = await post('/api/replay');
  assert.equal(replay.verified, true);
  assert.equal(replay.finalStateHash, current.stateHash);
});

test('WEB-06 seed-31 hashes match through headless and browser adapters', async () => {
  const demo = runGameDemo(31);
  const record = runGameRecord(31);
  assert.equal(record.finalStateHash, demo.finalStateHash);
  assert.equal(record.transcriptHash, demo.transcriptHash);
  assert.equal(record.transcript.length, 230);

  await withRustSession(createSyntheticDemoManifest(31), async (handle) => {
    for (const receipt of record.transcript) {
      const action = recordedAction(receipt);
      const result = await handle.step({
        actionId: String(action.actionId),
        seat: action.seat as 'north' | 'south',
        stateVersion: action.stateVersion as number,
      });
      assert.equal(result.accepted, true);
      assert.equal(await handle.stateHash(), action.postStateHash);
    }
    assert.equal(await handle.stateHash(), record.finalStateHash);
    assert.equal(await handle.verifyReplay(), true);
  });

  let current = await post('/api/reset', { opponent: 'manual', seed: 31 });
  for (const receipt of record.transcript) {
    const action = recordedAction(receipt);
    current = await submit(action);
    assert.equal(current.accepted, true);
    assert.equal(current.stateHash, action.postStateHash);
  }
  assert.equal(current.stateHash, demo.finalStateHash);
  assert.equal(((current.view as JsonObject).terminal as JsonObject).status, 'finished');
  const replay = await post('/api/replay');
  assert.equal(replay.verified, true);
  assert.equal(replay.finalStateHash, record.finalStateHash);
  assert.equal(replay.acceptedActionCount, 230);
  const steps = await json('/api/replay/steps');
  assert.equal(steps.chained, true);
  assert.equal(steps.stepCount, 230);
  assert.equal(steps.finalStateHash, record.finalStateHash);
});
