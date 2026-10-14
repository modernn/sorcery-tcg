import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import test from 'node:test';
import { createContext, runInContext } from 'node:vm';

import {
  createSyntheticDemoManifest,
  runGameDemo,
  runGameRecord,
} from '../../src/commands/run-game-demo.ts';
import { identityHash } from '../../src/authority/hash.ts';
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
  assert.equal(body.recovery, undefined, "Recovery invalidates successful game evidence");
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
  assert.match(page, /seat=result\.view\.decisionSeat/);
  assert.doesNotMatch(page, /clearActionResult\(\);seat=result\.view\.decisionSeat/);
  assert.match(page, /guarded\(async function\(\)\{seat=button\.dataset\.seat/);
  assert.match(page, /function guarded/);
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
    assert.equal(current.recovery, undefined, "Recovery invalidates game outcomes");
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
    assert.equal(current.recovery, undefined, "Recovery invalidates game outcomes");
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

function embeddedBrowser(script: string) {
  const context = createContext({ console });
  runInContext(String.raw`
    var elements={},requests=[],keyHandlers={},buttons=[];
    function element(id){if(!elements[id])elements[id]={id:id,value:'',options:[],textContent:'',innerHTML:'',disabled:false,dataset:{},events:{},classList:{remove:function(){},add:function(){},contains:function(){return false}},addEventListener:function(name,fn){this.events[name]=fn},setAttribute:function(){},removeAttribute:function(){},querySelectorAll:function(){return[]},querySelector:function(){return resetButton}};return elements[id]}
    var resetButton=element('reset-button'),northButton=element('north-button'),southButton=element('south-button');northButton.dataset.seat='north';southButton.dataset.seat='south';buttons=[resetButton,northButton,southButton];
    var document={getElementById:element,querySelectorAll:function(selector){if(selector==='[data-seat]')return[northButton,southButton];if(selector==='button')return buttons;return[]},querySelector:function(){return null},addEventListener:function(name,fn){keyHandlers[name]=fn}};
    var localStorage={getItem:function(){return null},removeItem:function(){},setItem:function(){}};
    function fetch(path,options){return new Promise(function(resolve,reject){requests.push({path:path,options:options,resolve:resolve,reject:reject})})}
    function deliver(body,status){var next=requests.shift();next.resolve({ok:!status||status===200,status:status||200,json:async function(){return body}})}
  `, context);
  runInContext(script, context);
  runInContext(`render=function(data){snapshot=data;syncControls()};focusFirstAction=function(){};`, context);
  const evaluate = (code: string) => runInContext(code, context);
  const flush = () => new Promise<void>((resolve) => setImmediate(resolve));
  return { evaluate, flush };
}

function browserReceipt(actionId: string, stateVersion = 0) {
  const body = { actionId, seat: 'north', stateVersion, nextStateVersion: stateVersion + 1,
    receiptSequence: stateVersion + 1, preStateHash: identityHash({ stateVersion }),
    postStateHash: identityHash({ stateVersion: stateVersion + 1 }), events: [], randomDraws: [] };
  return { ...body, receiptId: identityHash(body) };
}

test('embedded browser owns every command guard and retains acceptance/replacement evidence across render, refresh and transport faults', async () => {
  const page = await (await fetch(origin)).text();
  const script = page.match(/<script>([\s\S]*?)<\/script>/)![1]!;
  const { evaluate, flush } = embeddedBrowser(script);
  const fireAll = () => evaluate(`savedPosition={checkpoint:'{}',opponent:'manual'};lastCommand={actionId:'old',seat:'north',stateVersion:0};element('reset-form').events.submit({preventDefault:function(){}});element('save').events.click();element('resume').events.click();element('replay').events.click();element('stale').events.click();southButton.events.click();submit('second');visibleActionButtons=function(){return [{click:function(){submit('shortcut')}}]};keyHandlers.keydown({key:'1',target:{tagName:'DIV'},preventDefault:function(){}});`);
  assert.equal(evaluate('busy'), true);
  fireAll();
  assert.equal(evaluate('requests.length'), 1);
  assert.equal(evaluate('seat'), 'north');
  const ready = { opponent: 'manual', actions: [], presets: [], view: { stateVersion: 0, decisionSeat: 'north' } };
  evaluate(`deliver(${JSON.stringify(ready)})`); await flush();
  assert.equal(evaluate('busy'), false);
  const actionId = identityHash({ fixture: 'browser-keep' });
  const receipt = browserReceipt(actionId);
  const accepted = { ...ready, accepted: true, playerAction: 'Confirmed keep', receipt, opponentActionCount: 0, opponentActions: [], view: { stateVersion: 1, decisionSeat: 'north' } };
  evaluate(`render=function(){throw new Error('render failed')};submit(${JSON.stringify(actionId)})`);
  fireAll();
  assert.equal(evaluate('requests.length'), 1);
  evaluate(`deliver(${JSON.stringify(accepted)})`); await flush();
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(receipt, null, 2));
  assert.match(evaluate("element('notice').textContent") as string, /Action accepted.*render failed/);
  assert.equal(evaluate('snapshotCurrent'), false);
  assert.equal(evaluate('busy'), false);
  // Follow-up seat refresh stays inside the action owner and cannot erase known acceptance.
  evaluate(`render=function(data){snapshot=data};snapshotCurrent=true;sessionReady=true;actionsReady=true;submit(${JSON.stringify(actionId)})`);
  evaluate(`deliver(${JSON.stringify({ ...accepted, view: { stateVersion: 2, decisionSeat: 'south' } })})`); await flush();
  assert.equal(evaluate('busy'), true);
  assert.equal(evaluate('requests[0].path'), '/api/view?seat=south');
  evaluate(`requests.shift().reject(new Error('refresh failed'))`); await flush();
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(receipt, null, 2));
  assert.match(evaluate("element('notice').textContent") as string, /Action accepted.*refresh failed/);
  // A confirmed replacement retires old match evidence before fallible rendering.
  const replacementReceipt = { manifestId: identityHash({ match: 'next' }), stateHash: identityHash({ state: 'next' }), stateVersion: 0 };
  evaluate(`render=function(){throw new Error('replacement render failed')};element('reset-form').events.submit({preventDefault:function(){}})`);
  evaluate(`deliver(${JSON.stringify({ ...ready, replacement: { status: 'confirmed', receipt: replacementReceipt } })})`); await flush();
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(replacementReceipt, null, 2));
  assert.match(evaluate("element('notice').textContent") as string, /replacement confirmed.*replacement render failed/);
  assert.equal(evaluate('snapshot'), undefined);
  evaluate(`renderReplayStep=function(){throw new Error('replay cleanup failed')};element('reset-form').events.submit({preventDefault:function(){}})`);
  evaluate(`deliver(${JSON.stringify({ ...ready, replacement: { status: 'confirmed', receipt: replacementReceipt } })})`); await flush();
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(replacementReceipt, null, 2));
  assert.match(evaluate("element('notice').textContent") as string, /replacement confirmed.*replay cleanup failed/);
  evaluate(`renderReplayStep=function(){}`);
  // An unknown POST is never called a rejection, and stale/action/save/replay remain guarded.
  evaluate(`snapshot=${JSON.stringify(ready)};snapshotCurrent=true;sessionReady=true;actionsReady=true;submit('uncertain')`);
  evaluate(`requests.shift().reject(new Error('lost response'))`); await flush();
  assert.match(evaluate("element('notice').textContent") as string, /Request outcome unknown/);
  assert.doesNotMatch(evaluate("element('notice').textContent") as string, /Rejected:/);
  assert.equal(evaluate('sessionReady'), false);
  evaluate(`applyResponse(${JSON.stringify(ready)})`);
  assert.equal(evaluate('sessionReady'), false, 'A successful GET cannot clear a lost POST outcome');
  assert.equal(evaluate('snapshotCurrent'), false);
  evaluate(`submit('retry');element('stale').events.click();element('save').events.click();element('replay').events.click()`);
  assert.equal(evaluate('requests.length'), 0);
  const unknown = { recovery: { sessionStatus: 'unknown', snapshotStatus: 'untrusted', message: 'Explicit replacement required' } };
  evaluate(`applyResponse(${JSON.stringify(unknown)})`);
  assert.equal(evaluate('sessionReady'), false);
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(replacementReceipt, null, 2));
  // Replacement stays available under recovery, and replay's two requests share one guard.
  evaluate(`render=function(data){snapshot=data};element('reset-form').events.submit({preventDefault:function(){}})`);
  assert.equal(evaluate('requests.length'), 1);
  evaluate(`deliver(${JSON.stringify({ ...ready, replacement: { status: 'confirmed', receipt: replacementReceipt } })})`); await flush();
  evaluate(`element('replay').events.click()`);
  assert.equal(evaluate('busy'), true);
  evaluate(`deliver({verified:true})`); await flush();
  assert.equal(evaluate('requests[0].path'), '/api/replay/steps');
  fireAll(); assert.equal(evaluate('requests.length'), 1);
  evaluate(`deliver({steps:[],chained:true})`); await flush();
  assert.equal(evaluate('busy'), false);
});

test('exact embedded script treats missing or malformed mutation outcomes as unknown until an explicit valid confirmed replacement', async () => {
  const script = (await (await fetch(origin)).text()).match(/<script>([\s\S]*?)<\/script>/)![1]!;
  const priorId = identityHash({ browser: 'confirmed-prior' });
  const nextId = identityHash({ browser: 'next-command' });
  const priorReceipt = browserReceipt(priorId);
  const checkpointHash = identityHash({ browser: 'saved-session' });
  const ack = { manifestId: identityHash({ browser: 'explicit-replacement' }), stateHash: identityHash({ browser: 'fresh-state' }), stateVersion: 0 };
  const ready = { opponent: 'manual', actions: [], presets: [], view: { stateVersion: 0, decisionSeat: 'north' } };
  const prior = { ...ready, accepted: true, playerAction: 'Prior confirmed action', receipt: priorReceipt,
    opponentActionCount: 0, opponentActions: [], view: { stateVersion: 1, decisionSeat: 'north' } };
  const recovery = { stage: 'replacement', message: 'Replacement unchanged', snapshotStatus: 'unavailable',
    actionsAvailable: false, sessionStatus: 'ready', metadataStatus: 'known' };
  for (const operation of ['action', 'reset', 'resume'] as const) {
    for (const fault of ['bare-json-503', 'bare-json-400', 'missing-outcome-200', 'malformed-outcome-200', 'malformed-recovery-503', 'contradictory-recovery-503']) {
      const { evaluate, flush } = embeddedBrowser(script);
      evaluate(`deliver(${JSON.stringify(ready)})`); await flush();
      evaluate(`element('preset').value='synthetic';element('opponent').value='manual';element('seed').value='31';savedPosition={checkpoint:JSON.stringify({expectedSessionHash:${JSON.stringify(checkpointHash)}}),opponent:'manual'};submit(${JSON.stringify(priorId)})`);
      evaluate(`deliver(${JSON.stringify(prior)})`); await flush();
      assert.equal(evaluate('outcomeUnknown'), false);
      assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(priorReceipt, null, 2));
      if (operation === 'action') evaluate(`submit(${JSON.stringify(nextId)})`);
      if (operation === 'reset') evaluate(`element('reset-form').events.submit({preventDefault:function(){}})`);
      if (operation === 'resume') evaluate(`element('resume').events.click()`);
      assert.equal(evaluate('requests.length'), 1, `${operation}/${fault} entered exactly once`);
      let body: JsonObject;
      let status = 200;
      if (fault === 'bare-json-503' || fault === 'bare-json-400') {
        body = { error: 'Synthetic response lacks conclusive mutation evidence' };
        status = fault === 'bare-json-503' ? 503 : 400;
      } else if (fault === 'missing-outcome-200') body = ready;
      else if (fault === 'malformed-recovery-503') {
        body = { error: 'Synthetic invalid recovery', recovery: { sessionStatus: 'ready' } }; status = 503;
      } else if (fault === 'contradictory-recovery-503') {
        body = { error: 'Synthetic contradictory recovery', recovery: { ...recovery,
          stage: 'player-presentation', metadataStatus: 'unknown' } }; status = 503;
      } else if (operation === 'action') {
        body = { ...prior, receipt: { ...browserReceipt(nextId, 1), events: null } };
      } else {
        body = { ...ready, replacement: { status: 'confirmed', receipt: operation === 'reset'
          ? { ...ack, stateHash: 'invalid-hash' }
          : { stateHash: ack.stateHash, stateVersion: 0, expectedSessionHash: identityHash({ browser: 'wrong-saved-session' }) } } };
      }
      evaluate(`deliver(${JSON.stringify(body)},${status})`); await flush();
      assert.equal(evaluate('outcomeUnknown'), true, `${operation}/${fault}`);
      assert.equal(evaluate('sessionReady'), false);
      assert.equal(evaluate('snapshotCurrent'), false);
      assert.equal(evaluate('actionsReady'), false);
      assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(priorReceipt, null, 2));
      assert.match(evaluate("element('notice').textContent") as string, /Request outcome unknown/);
      assert.doesNotMatch(evaluate("element('notice').textContent") as string, /Rejected:/);
      evaluate(`northButton.events.click()`);
      assert.equal(evaluate('requests.length'), 1);
      evaluate(`deliver(${JSON.stringify({ ...ready, view: prior.view })})`); await flush();
      assert.equal(evaluate('outcomeUnknown'), true);
      assert.equal(evaluate('sessionReady'), false, 'GET cannot establish a missing POST outcome');
      evaluate(`applyResponse(${JSON.stringify({ ...ready, replacement: { status: 'confirmed', receipt: ack } })})`);
      assert.equal(evaluate('outcomeUnknown'), true, 'Uncorrelated replacement fields cannot clear uncertainty');
      assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(priorReceipt, null, 2));
      evaluate(`submit(${JSON.stringify(nextId)});element('stale').events.click();element('save').events.click();element('replay').events.click()`);
      assert.equal(evaluate('requests.length'), 0, 'No action, stale retry, save or either replay request');
      // A conclusive unchanged replacement retains a previously uncertain binding.
      evaluate(`element('reset-form').events.submit({preventDefault:function(){}})`);
      evaluate(`deliver(${JSON.stringify({ error: 'Replacement unchanged', replacement: { status: 'unchanged' },
        recovery: { ...recovery, sessionStatus: 'unknown', snapshotStatus: 'untrusted', metadataStatus: 'unknown' } })},400)`); await flush();
      assert.equal(evaluate('outcomeUnknown'), true);
      assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(priorReceipt, null, 2));
      // Only this explicitly requested, complete and valid acknowledgement clears it.
      if (operation === 'resume') {
        evaluate(`element('resume').events.click()`);
        evaluate(`deliver(${JSON.stringify({ ...ready, replacement: { status: 'confirmed', receipt: {
          stateHash: ack.stateHash, stateVersion: 0, expectedSessionHash: checkpointHash } } })})`);
      } else {
        evaluate(`element('reset-form').events.submit({preventDefault:function(){}})`);
        evaluate(`deliver(${JSON.stringify({ ...ready, replacement: { status: 'confirmed', receipt: ack } })})`);
      }
      await flush();
      assert.equal(evaluate('outcomeUnknown'), false);
      assert.equal(evaluate('sessionReady'), true);
      assert.equal(evaluate('snapshotCurrent'), true);
      assert.equal(evaluate('lastCommand'), undefined);
      assert.equal(evaluate('busy'), false);
      assert.equal(evaluate('requests.length'), 0);
    }
  }
});

test('embedded script distinguishes conclusive preflight, native rejection, read-only presentation failure and unchanged replacement', async () => {
  const script = (await (await fetch(origin)).text()).match(/<script>([\s\S]*?)<\/script>/)![1]!;
  const actionId = identityHash({ browser: 'preflight-command' });
  const ready = { opponent: 'manual', presetId: 'synthetic', actions: [], presets: [{ id: 'synthetic', label: 'Synthetic' }], view: { stateVersion: 0, decisionSeat: 'north' } };
  const validRecovery = { stage: 'player-presentation', message: 'Action presentation unavailable', snapshotStatus: 'unavailable', actionsAvailable: false, sessionStatus: 'ready', metadataStatus: 'known' };
  for (const refused of [
    { operation: 'action', status: 400, body: { error: 'south is controlled by the deterministic opponent' } },
    { operation: 'reset', status: 400, body: { error: 'seed must be an unsigned 32-bit integer' } },
    { operation: 'reset', status: 400, body: { error: 'presetId must name an available preset' } },
    { operation: 'resume', status: 403, body: { error: 'south is hidden while controlled by the deterministic opponent' } },
    { operation: 'action', status: 200, body: { ...ready, accepted: false, opponentActionCount: 0, opponentActions: [], reason: { code: 'unknown_action', message: 'Action is not available', currentStateHash: identityHash({ state: 'unchanged' }), currentStateVersion: 0 } } },
    { operation: 'action', status: 503, body: { error: 'Action presentation unavailable', recovery: validRecovery } },
    { operation: 'reset', status: 400, body: { error: 'Replacement unchanged', replacement: { status: 'unchanged' }, recovery: { ...validRecovery, stage: 'replacement' } } },
    { operation: 'resume', status: 400, body: { error: 'Replacement unchanged', replacement: { status: 'unchanged' }, recovery: { ...validRecovery, stage: 'replacement' } } },
  ]) {
    const { evaluate, flush } = embeddedBrowser(script);
    const initial = refused.operation === 'action' && refused.body.error === 'south is controlled by the deterministic opponent'
      ? { ...ready, opponent: 'south' } : ready;
    evaluate(`deliver(${JSON.stringify(initial)})`); await flush();
    evaluate(`savedPosition={checkpoint:JSON.stringify({expectedSessionHash:${JSON.stringify(identityHash({ saved: true }))}}),opponent:'manual'};element('receipt').textContent='Historical confirmed receipt'`);
    if (refused.operation === 'action') evaluate(`seat=${JSON.stringify(initial.opponent === 'south' ? 'south' : 'north')};submit(${JSON.stringify(actionId)})`);
    if (refused.operation === 'reset') {
      if (refused.body.error === 'seed must be an unsigned 32-bit integer') evaluate(`element('seed').value='not-a-number'`);
      if (refused.body.error === 'presetId must name an available preset') evaluate(`element('preset').value='unavailable-preset'`);
      evaluate(`element('reset-form').events.submit({preventDefault:function(){}})`);
    }
    if (refused.operation === 'resume') evaluate(`seat='south';savedPosition.opponent='south';element('resume').events.click()`);
    const body = refused.body.replacement ? { error: refused.body.error, replacement: refused.body.replacement,
      recovery: refused.body.recovery } : refused.body;
    evaluate(`deliver(${JSON.stringify(body)},${refused.status})`); await flush();
    assert.equal(evaluate('outcomeUnknown'), false, `${refused.operation}/${refused.status}/${refused.body.error}`);
    assert.equal(evaluate("element('receipt').textContent"), 'Historical confirmed receipt');
    evaluate(`northButton.events.click()`);
    evaluate(`deliver(${JSON.stringify(ready)})`); await flush();
    assert.equal(evaluate('sessionReady'), true);
    assert.equal(evaluate('snapshotCurrent'), true);
  }
});

test('preset refusal is inconclusive without its catalog after confirmed replacement retires the snapshot', async () => {
  const script = (await (await fetch(origin)).text()).match(/<script>([\s\S]*?)<\/script>/)![1]!;
  const presetId = 'synthetic';
  const actionId = identityHash({ browser: 'preset-catalog-unavailable' });
  const ack = { manifestId: identityHash({ browser: 'replacement-ack' }), stateHash: identityHash({ browser: 'replacement-state' }), stateVersion: 0 };
  const ready = { opponent: 'manual', presetId, presets: [{ id: presetId, label: 'Synthetic' }], actions: [], view: { stateVersion: 0, decisionSeat: 'north' } };
  const { evaluate, flush } = embeddedBrowser(script);
  evaluate(`deliver(${JSON.stringify(ready)})`); await flush();
  evaluate(`element('preset').value=${JSON.stringify(presetId)};element('opponent').value='manual';element('seed').value='31';element('receipt').textContent='Previous evidence'`);
  evaluate(`element('reset-form').events.submit({preventDefault:function(){}})`);
  assert.equal(evaluate('requests.length'), 1);
  evaluate(`deliver({replacement:{status:'confirmed',receipt:${JSON.stringify(ack)}},recovery:{stage:'snapshot',message:'Presentation unavailable',snapshotStatus:'unavailable',actionsAvailable:false,sessionStatus:'ready',metadataStatus:'known'}})`); await flush();
  assert.equal(evaluate('outcomeUnknown'), false);
  assert.equal(evaluate('snapshot'), undefined, 'The confirmed replacement retired the old presentation cache');
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(ack, null, 2));

  evaluate(`element('preset').value=${JSON.stringify(presetId)};element('reset-form').events.submit({preventDefault:function(){}})`);
  assert.equal(evaluate('requests.length'), 1, 'The valid preset can be submitted without a current snapshot');
  assert.deepEqual(JSON.parse(evaluate('requests[0].options.body') as string), { opponent: 'manual', presetId, seed: 31 });
  evaluate(`deliver({error:'presetId must name an available preset'},400)`); await flush();
  assert.equal(evaluate('outcomeUnknown'), true);
  assert.equal(evaluate('sessionReady'), false);
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(ack, null, 2), 'The last acknowledged replacement remains visible');

  evaluate(`northButton.events.click()`);
  assert.equal(evaluate('requests.length'), 1);
  evaluate(`deliver(${JSON.stringify(ready)})`); await flush();
  assert.equal(evaluate('outcomeUnknown'), true, 'An unrelated GET cannot resolve the second reset');
  assert.equal(evaluate('sessionReady'), false);
  evaluate(`submit(${JSON.stringify(actionId)});element('save').events.click();element('replay').events.click();element('stale').events.click()`);
  assert.equal(evaluate('requests.length'), 0, 'Action, stale retry, save and replay remain blocked');
  assert.equal(evaluate("element('receipt').textContent"), JSON.stringify(ack, null, 2));
});

test('browser only treats request-matched route refusals as conclusive and keeps post-ack resume fallback uncertain', async () => {
  const script = (await (await fetch(origin)).text()).match(/<script>([\s\S]*?)<\/script>/)![1]!;
  const ready = { opponent: 'manual', actions: [], presets: [{ id: 'synthetic' }], presetId: 'synthetic', view: { stateVersion: 0, decisionSeat: 'north' } };
  const savedCheckpoint = (await post('/api/checkpoint')).checkpoint as string;
  const cases = [
    { operation: 'reset', status: 403, error: 'seed must be an unsigned 32-bit integer' },
    { operation: 'action', status: 403, error: 'valid seat, stateVersion, and actionId are required' },
    { operation: 'reset', status: 400, error: 'seed must be an unsigned 32-bit integer' },
    { operation: 'action', status: 400, error: 'south is controlled by the deterministic opponent' },
    { operation: 'resume', status: 400, error: 'Invalid checkpoint request' },
  ] as const;
  for (const fault of cases) {
    const { evaluate, flush } = embeddedBrowser(script);
    evaluate(`deliver(${JSON.stringify(ready)})`); await flush();
    evaluate(`element('preset').value='synthetic';element('opponent').value='manual';element('seed').value='31';savedPosition={checkpoint:${JSON.stringify(savedCheckpoint)},opponent:'manual'};element('receipt').textContent='Historical confirmed receipt'`);
    if (fault.operation === 'action') evaluate(`submit(${JSON.stringify(identityHash({ browser: 'request-correlation' }))})`);
    if (fault.operation === 'reset') evaluate(`element('reset-form').events.submit({preventDefault:function(){}})`);
    if (fault.operation === 'resume') evaluate(`element('resume').events.click()`);
    if (fault.operation === 'resume') assert.equal(evaluate('requests[0].options.body') as string, savedCheckpoint, 'The ambiguous fallback follows a real checkpoint request');
    assert.equal(evaluate('requests.length'), 1);
    evaluate(`deliver({error:${JSON.stringify(fault.error)}},${fault.status})`); await flush();
    assert.equal(evaluate('outcomeUnknown'), true, `${fault.operation}/${fault.status}/${fault.error}`);
    assert.equal(evaluate('sessionReady'), false);
    assert.equal(evaluate("element('receipt').textContent"), 'Historical confirmed receipt');
    evaluate(`northButton.events.click()`);
    evaluate(`deliver(${JSON.stringify({ ...ready, view: { stateVersion: 0, decisionSeat: 'north' } })})`); await flush();
    assert.equal(evaluate('outcomeUnknown'), true, 'A later checked GET cannot clear the uncertain response');
    assert.equal(evaluate('sessionReady'), false);
    assert.equal(evaluate('requests.length'), 0);
  }
});
