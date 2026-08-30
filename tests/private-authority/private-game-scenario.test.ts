import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import test from 'node:test';

import { runGameBatch } from '../../src/commands/run-game-batch.ts';
import {
  loadPrivateStarterCatalog,
  type PrivateGameCheck,
  type PrivateStarterPreset,
  runPrivateGameCheck,
} from '../../src/commands/run-private-game-check.ts';
import { createGamePrototypeServer } from '../../src/prototype/game-server.ts';

type JsonObject = Record<string, unknown>;

async function verifyPrivateStarterHttp(catalog: readonly PrivateStarterPreset[]): Promise<void> {
  const server = createGamePrototypeServer(undefined, catalog);
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const origin = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  const json = async (path: string, init?: RequestInit): Promise<JsonObject> => {
    const response = await fetch(`${origin}${path}`, init);
    const body = await response.json() as JsonObject;
    assert.equal(response.status, 200, JSON.stringify(body));
    return body;
  };
  const findAction = (response: JsonObject, predicate: (value: JsonObject) => boolean): JsonObject => {
    const found = (response.actions as JsonObject[])
      .find((candidate) => predicate(candidate.descriptor as JsonObject));
    assert.ok(found, 'expected actual-card browser action');
    return found;
  };
  const submit = (candidate: JsonObject): Promise<JsonObject> => json('/api/action', {
    body: JSON.stringify({
      actionId: candidate.actionId,
      seat: candidate.seat,
      stateVersion: candidate.stateVersion,
    }),
    headers: { 'content-type': 'application/json' },
    method: 'POST',
  });
  const keep = (response: JsonObject): JsonObject => findAction(response, (descriptor) =>
    descriptor.kind === 'mulligan'
      && (descriptor.atlasOrder as unknown[]).length === 0
      && (descriptor.spellbookOrder as unknown[]).length === 0);
  const deterministicAction = (response: JsonObject): JsonObject => {
    const candidates = response.actions as JsonObject[];
    const player = ((((response.view as JsonObject).players as JsonObject).north as JsonObject));
    const drawZone = Number(player.atlasCount) > 3
      || Number(player.spellbookCount) <= Number(player.atlasCount)
      ? 'atlas'
      : 'spellbook';
    const enemyCell = String(((((response.view as JsonObject).players as JsonObject)
      .south as JsonObject).avatar as JsonObject).location);
    const movement = candidates
      .map((candidate) => {
        const descriptor = candidate.descriptor as JsonObject;
        const cell = String((descriptor.to as JsonObject | undefined)?.cell);
        const inPlaceAvatarAttack = descriptor.kind === 'move-and-attack'
          && (descriptor.path as unknown[]).length === 1
          && cell === enemyCell
          && (descriptor.to as JsonObject).region === 'surface';
        return {
          candidate,
          distance: inPlaceAvatarAttack
            ? -1
            : descriptor.kind === 'move-and-attack'
                && (descriptor.path as unknown[]).length > 1
              ? Math.abs(cell.charCodeAt(0) - enemyCell.charCodeAt(0))
                + Math.abs(Number(cell[1]) - Number(enemyCell[1]))
              : Number.POSITIVE_INFINITY,
        };
      })
      .sort((left, right) => left.distance - right.distance)[0];
    const selected = candidates.find((candidate) => {
      const descriptor = candidate.descriptor as JsonObject;
      return descriptor.kind === 'mulligan'
        && (descriptor.atlasOrder as unknown[]).length === 0
        && (descriptor.spellbookOrder as unknown[]).length === 0;
    })
      ?? candidates.find(({ descriptor }) => (descriptor as JsonObject).kind === 'play-site')
      ?? candidates.find(({ descriptor }) => (descriptor as JsonObject).kind === 'summon-minion')
      ?? candidates.find(({ descriptor }) => {
        const value = descriptor as JsonObject;
        return value.kind === 'draw' && value.zone === drawZone;
      })
      ?? (movement && Number.isFinite(movement.distance) ? movement.candidate : undefined)
      ?? candidates.find(({ descriptor }) => (descriptor as JsonObject).kind === 'end-turn')
      ?? candidates[0];
    assert.ok(selected, 'expected deterministic actual-card action');
    return selected;
  };

  try {
    let current = await json('/api/view?seat=north');
    assert.equal(current.presetId, 'air-vs-earth-lesson');
    assert.equal(current.mode, 'private-local');
    assert.ok(Object.keys(current.cardNames as JsonObject).length < Object.keys(catalog[0]!.cardNames).length);
    const defaultNames = current.cardNames as Record<string, string>;
    const defaultPlayers = ((current.view as JsonObject).players as JsonObject);
    assert.equal(defaultNames[(((defaultPlayers.north as JsonObject).avatar as JsonObject).cardId as string)],
      'Sparkmage');
    assert.equal(defaultNames[(((defaultPlayers.south as JsonObject).avatar as JsonObject).cardId as string)],
      'Geomancer');
    const hiddenSouthCardIds = [
      ...catalog[0]!.manifest.decks.south.atlas,
      ...catalog[0]!.manifest.decks.south.spellbook,
    ];
    assert.equal(hiddenSouthCardIds.every((cardId) => defaultNames[cardId] === undefined), true);
    const airSandbox = catalog.find(({ id }) => id === 'air-starter');
    assert.ok(airSandbox);
    current = await json('/api/reset', {
      body: JSON.stringify({
        opponent: 'manual',
        presetId: airSandbox.id,
        seed: airSandbox.manifest.seed,
      }),
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    });
    assert.equal(current.presetId, 'air-starter');
    const north = (((current.view as JsonObject).players as JsonObject).north as JsonObject);
    const hand = north.hand as JsonObject;
    const names = current.cardNames as Record<string, string>;
    const sparkmage = north.avatar as JsonObject;
    assert.equal(names[sparkmage.cardId as string], 'Sparkmage');
    const spire = (hand.atlas as JsonObject[]).find(({ cardId }) => names[cardId as string] === 'Spire');
    const leopard = (hand.spellbook as JsonObject[])
      .find(({ cardId }) => names[cardId as string] === 'Snow Leopard');
    const zap = (hand.spellbook as JsonObject[])
      .find(({ cardId }) => names[cardId as string] === 'Zap!');
    assert.ok(spire && leopard && zap, 'known-good Air seed must expose its teaching cards');
    const facts = current.cardFacts as Record<string, JsonObject>;
    assert.deepEqual(facts[sparkmage.cardId as string], {
      attack: 1,
      cardType: 'avatar',
      defense: 1,
      life: 20,
    });
    assert.deepEqual(facts[spire.cardId as string], {
      cardType: 'site',
      elements: ['air'],
    });
    assert.deepEqual(facts[leopard.cardId as string], {
      attack: 2,
      cardType: 'minion',
      defense: 2,
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    });
    assert.deepEqual(facts[zap.cardId as string], {
      cardType: 'magic',
      manaCost: 1,
      thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
    });
    assert.doesNotMatch(JSON.stringify(facts), /rulesText|Deal 1 damage/);

    current = await submit(keep(current));
    current = await json('/api/view?seat=south');
    current = await submit(keep(current));
    current = await json('/api/view?seat=north');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === spire.instanceId
        && descriptor.cell === 'C4'));
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === leopard.instanceId
        && descriptor.cell === 'C4'));

    const view = current.view as JsonObject;
    const realm = view.realm as JsonObject;
    const site = (realm.sites as JsonObject).C4 as JsonObject;
    const unit = (realm.units as JsonObject[])
      .find(({ instanceId }) => instanceId === leopard.instanceId)!;
    const visibleNames = current.cardNames as Record<string, string>;
    assert.equal(visibleNames[site.cardId as string], 'Spire');
    assert.equal(visibleNames[unit.cardId as string], 'Snow Leopard');
    const afterSummonNorth = (((current.view as JsonObject).players as JsonObject).north as JsonObject);
    assert.equal(afterSummonNorth.airThresholdsCastThisTurn, 1);
    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'end-turn'));
    current = await json('/api/view?seat=south');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'play-site'));
    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'end-turn'));
    current = await json('/api/view?seat=north');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    assert.equal(((((current.view as JsonObject).players as JsonObject).north as JsonObject)
      .airThresholdsCastThisTurn), 0);
    const castZap = findAction(current, (descriptor) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === zap.instanceId
        && (descriptor.target as JsonObject | undefined)?.instanceId === leopard.instanceId);
    assert.match(String(castZap.label), /Cast Zap!.*Snow Leopard.*attempt to deal 1 damage/);
    assert.doesNotMatch(String(castZap.label), /card:|sha256:/);
    current = await submit(castZap);
    assert.match(String(current.playerAction), /Cast Zap!.*Snow Leopard.*attempt to deal 1 damage/);
    assert.doesNotMatch(String(current.playerAction), /card:|sha256:/);
    const afterCastNorth = (((current.view as JsonObject).players as JsonObject).north as JsonObject);
    assert.equal(afterCastNorth.airThresholdsCastThisTurn, 1);
    const activateSparkmage = findAction(current, (descriptor) => {
      const target = descriptor.targetLocation as JsonObject | undefined;
      return descriptor.kind === 'activate-sparkmage'
        && target?.cell === 'C4'
        && target.region === 'surface';
    });
    assert.match(String(activateSparkmage.label), /Sparkmage.*C4/);
    assert.doesNotMatch(String(activateSparkmage.label), /card:|sha256:/);
    current = await submit(activateSparkmage);
    assert.match(String(current.playerAction), /Sparkmage.*C4/);
    const afterSparkmage = (((current.view as JsonObject).players as JsonObject).north as JsonObject);
    assert.equal((afterSparkmage.avatar as JsonObject).tapped, true);
    assert.equal((afterSparkmage.cemetery as JsonObject[])
      .some(({ instanceId }) => instanceId === leopard.instanceId), true);
    assert.equal(afterSparkmage.airThresholdsCastThisTurn, 1);
    const sparkmageEvents = ((current.receipt as JsonObject).events as JsonObject[])
      .map(({ type }) => type);
    assert.equal(sparkmageEvents.includes('sparkmage-activated'), true);
    assert.equal(sparkmageEvents.includes('damage-dealt'), true);
    assert.equal(sparkmageEvents.includes('minion-died'), true);
    const replay = await json('/api/replay', { method: 'POST' });
    assert.equal(replay.acceptedActionCount, 11);
    assert.equal(replay.verified, true);
    assert.equal(replay.finalStateHash, current.stateHash);

    const earthPreset = catalog.find(({ id }) => id === 'earth-starter');
    assert.ok(earthPreset);
    assert.equal(earthPreset.cardNames[earthPreset.manifest.decks.north.avatar], 'Geomancer');
    current = await json('/api/reset', {
      body: JSON.stringify({
        opponent: 'manual',
        presetId: earthPreset.id,
        seed: earthPreset.manifest.seed,
      }),
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    });
    const earthNames = current.cardNames as Record<string, string>;
    let earthNorth = (((current.view as JsonObject).players as JsonObject).north as JsonObject);
    const earthHand = earthNorth.hand as JsonObject;
    const village = (earthHand.atlas as JsonObject[])
      .find(({ cardId }) => earthNames[cardId as string] === 'Humble Village');
    const boars = (earthHand.spellbook as JsonObject[])
      .find(({ cardId }) => earthNames[cardId as string] === 'Wild Boars');
    assert.ok(village && boars, 'known-good Earth seed must expose its teaching cards');
    current = await submit(keep(current));
    current = await json('/api/view?seat=south');
    current = await submit(keep(current));
    current = await json('/api/view?seat=north');
    const villageChoices = (current.actions as JsonObject[]).filter(({ descriptor }) => {
      const value = descriptor as JsonObject;
      return value.kind === 'play-site'
        && value.cardInstanceId === village.instanceId
        && value.cell === 'C4';
    });
    assert.deepEqual(villageChoices.map(({ descriptor }) => {
      const value = descriptor as JsonObject;
      return `${value.createRubbleAt}:${value.genesisTokenChoice}`;
    }).sort(), [
      'B4:decline',
      'B4:pay-one-mana',
      'C3:decline',
      'C3:pay-one-mana',
      'D4:decline',
      'D4:pay-one-mana',
    ]);
    assert.equal(villageChoices.every(({ label }) =>
      String(label).includes('Humble Village') && !/card:|sha256:/.test(String(label))), true);
    assert.equal(new Set(villageChoices.map(({ label }) => label)).size, 6);
    assert.equal(villageChoices.every(({ descriptor, label }) =>
      String(label).includes(`create Rubble at ${(descriptor as JsonObject).createRubbleAt}`)), true);
    const paidVillage = villageChoices.find(({ descriptor }) => {
      const value = descriptor as JsonObject;
      return value.genesisTokenChoice === 'pay-one-mana' && value.createRubbleAt === 'C3';
    });
    const declinedVillage = villageChoices.find(({ descriptor }) =>
      (descriptor as JsonObject).genesisTokenChoice === 'decline');
    assert.ok(paidVillage);
    assert.ok(declinedVillage);
    assert.match(String(paidVillage.label), /spend 1 mana to summon Foot Soldier there/);
    assert.match(String(declinedVillage.label), /keep 1 mana and summon no Foot Soldier/);
    assert.doesNotMatch(String(paidVillage.label), /Genesis|card:|sha256:/);
    current = await submit(paidVillage);
    assert.match(String(current.playerAction), /spend 1 mana to summon Foot Soldier there/);
    assert.doesNotMatch(String(current.playerAction), /Genesis|card:|sha256:/);
    const paidView = current.view as JsonObject;
    const paidRealm = paidView.realm as JsonObject;
    const paidNames = current.cardNames as Record<string, string>;
    const footSoldiers = (paidRealm.units as JsonObject[]).filter(({ cardId }) =>
      paidNames[cardId as string] === 'Foot Soldier');
    assert.equal(footSoldiers.length, 1);
    assert.equal(footSoldiers[0]!.location, 'C4');
    assert.equal(footSoldiers[0]!.controller, 'north');
    assert.deepEqual(((current.receipt as JsonObject).events as JsonObject[])
      .map(({ type }) => type), ['site-played', 'minion-summoned', 'rubble-created']);
    assert.equal(((paidRealm.sites as JsonObject).C3 as JsonObject).cardId, 'rubble');
    assert.equal((((paidView.players as JsonObject).north as JsonObject).mana), 0);

    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'end-turn'));
    current = await json('/api/view?seat=south');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'play-site'));
    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'end-turn'));
    const southBeforeReplacement = JSON.stringify(await json('/api/view?seat=south'));
    current = await json('/api/view?seat=north');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    earthNorth = (((current.view as JsonObject).players as JsonObject).north as JsonObject);
    const atlasBefore = earthNorth.atlasCount as number;
    const handBefore = ((earthNorth.hand as JsonObject).atlas as unknown[]).length;
    const replaceRubble = findAction(current, (descriptor) =>
      descriptor.kind === 'replace-rubble-with-top-atlas-site'
        && descriptor.targetCell === 'C3');
    assert.equal(replaceRubble.label,
      'Replace Rubble at C3 with the top site of your Atlas');
    assert.deepEqual(Object.keys(replaceRubble.descriptor as JsonObject).sort(), [
      'kind',
      'targetCell',
      'targetRubbleInstanceId',
    ]);
    current = await submit(replaceRubble);
    const replacement = current;
    const replacementView = replacement.view as JsonObject;
    const replacementNorth = ((replacementView.players as JsonObject).north as JsonObject);
    const replacementSite = ((replacementView.realm as JsonObject).sites as JsonObject).C3 as JsonObject;
    assert.equal(southBeforeReplacement.includes(String(replacementSite.instanceId)), false);
    assert.equal(replacementNorth.atlasCount, atlasBefore - 1);
    assert.equal(((replacementNorth.hand as JsonObject).atlas as unknown[]).length, handBefore);
    assert.deepEqual(((replacement.receipt as JsonObject).events as JsonObject[])
      .map(({ type }) => type), ['rubble-replaced', 'site-played']);
    const deferredGenesis = replacementView.phase === 'genesis';
    if (deferredGenesis) {
      current = await submit(findAction(current, (descriptor) =>
        descriptor.kind === 'resolve-genesis-token' && descriptor.choice === 'decline'));
    }
    const villageReplay = await json('/api/replay', { method: 'POST' });
    assert.equal(villageReplay.acceptedActionCount, deferredGenesis ? 10 : 9);
    assert.equal(villageReplay.verified, true);
    assert.equal(villageReplay.finalStateHash, current.stateHash);

    const firePreset = catalog.find(({ id }) => id === 'fire-starter');
    assert.ok(firePreset);
    assert.equal(firePreset.label, 'Fire — Wasteland + Raal Dromedary + Charge');
    current = await json('/api/reset', {
      body: JSON.stringify({
        opponent: 'manual',
        presetId: firePreset.id,
        seed: firePreset.manifest.seed,
      }),
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    });
    const fireNames = current.cardNames as Record<string, string>;
    const fireFacts = current.cardFacts as Record<string, JsonObject>;
    const fireHand = ((((current.view as JsonObject).players as JsonObject)
      .north as JsonObject).hand as JsonObject);
    const wasteland = (fireHand.atlas as JsonObject[])
      .find(({ cardId }) => fireNames[cardId as string] === 'Wasteland');
    const secondFireSite = (fireHand.atlas as JsonObject[]).find(({ cardId, instanceId }) =>
      instanceId !== wasteland?.instanceId
        && (fireFacts[cardId as string]?.elements as unknown[] | undefined)?.includes('fire'));
    const raal = (fireHand.spellbook as JsonObject[])
      .find(({ cardId }) => fireNames[cardId as string] === 'Raal Dromedary');
    const charge = (fireHand.spellbook as JsonObject[])
      .find(({ cardId }) => fireNames[cardId as string] === 'Charge');
    assert.ok(wasteland && secondFireSite && raal && charge,
      'known-good Fire seed must expose Wasteland, a second Fire site, Raal, and Charge');
    assert.deepEqual(fireFacts[charge.cardId as string], {
      cardType: 'magic',
      manaCost: 1,
      thresholds: { air: 0, earth: 0, fire: 1, water: 0 },
    });
    current = await submit(keep(current));
    current = await json('/api/view?seat=south');
    current = await submit(keep(current));
    current = await json('/api/view?seat=north');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === wasteland.instanceId
        && descriptor.cell === 'C4'));
    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'end-turn'));
    current = await json('/api/view?seat=south');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'draw' && descriptor.zone === 'atlas'));
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'play-site' && descriptor.cell === 'C1'));
    current = await submit(findAction(current, (descriptor) => descriptor.kind === 'end-turn'));
    current = await json('/api/view?seat=north');
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'draw' && descriptor.zone === 'spellbook'));
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'play-site'
        && descriptor.cardInstanceId === secondFireSite.instanceId
        && descriptor.cell === 'C3'));
    current = await submit(findAction(current, (descriptor) =>
      descriptor.kind === 'summon-minion'
        && descriptor.cardInstanceId === raal.instanceId
        && descriptor.cell === 'C3'));
    assert.equal((current.actions as JsonObject[]).every(({ descriptor }) => {
      const value = descriptor as JsonObject;
      return value.kind !== 'move-and-attack' || value.unitInstanceId !== raal.instanceId;
    }), true);
    const castCharge = findAction(current, (descriptor) =>
      descriptor.kind === 'cast-magic'
        && descriptor.cardInstanceId === charge.instanceId
        && (descriptor.ally as JsonObject | undefined)?.instanceId === raal.instanceId);
    assert.match(String(castCharge.label),
      /Cast Charge.*Raal Dromedary.*ally can move and attack this turn/);
    assert.doesNotMatch(String(castCharge.label), /card:|sha256:/);
    current = await submit(castCharge);
    assert.match(String(current.playerAction),
      /Cast Charge.*Raal Dromedary.*ally can move and attack this turn/);
    assert.deepEqual(((current.receipt as JsonObject).events as JsonObject[])
      .map(({ type }) => type), ['magic-cast', 'charge-granted', 'magic-resolved']);
    const chargedMove = findAction(current, (descriptor) =>
      descriptor.kind === 'move-and-attack'
        && descriptor.unitInstanceId === raal.instanceId
        && (descriptor.path as JsonObject[]).map(({ cell }) => cell).join(',') === 'C3,C4');
    current = await submit(chargedMove);
    const movedRaal = (((current.view as JsonObject).realm as JsonObject).units as JsonObject[])
      .find(({ instanceId }) => instanceId === raal.instanceId);
    assert.equal(movedRaal?.location, 'C4');
    const chargeReplay = await json('/api/replay', { method: 'POST' });
    assert.equal(chargeReplay.acceptedActionCount, 12);
    assert.equal(chargeReplay.verified, true);
    assert.equal(chargeReplay.finalStateHash, current.stateHash);

    const waterPreset = catalog.find(({ id }) => id === 'water-starter');
    assert.ok(waterPreset);
    current = await json('/api/reset', {
      body: JSON.stringify({
        opponent: 'manual',
        presetId: waterPreset.id,
        seed: waterPreset.manifest.seed,
      }),
      headers: { 'content-type': 'application/json' },
      method: 'POST',
    });
    const waterNames = current.cardNames as Record<string, string>;
    const waterHand = ((((current.view as JsonObject).players as JsonObject)
      .north as JsonObject).hand as JsonObject);
    const river = (waterHand.atlas as JsonObject[])
      .find(({ cardId }) => waterNames[cardId as string] === 'Autumn River');
    assert.ok(river, 'known-good Water seed must expose Autumn River');
    current = await submit(keep(current));
    current = await json('/api/view?seat=south');
    current = await submit(keep(current));
    current = await json('/api/view?seat=north');
    const riverPlays = (current.actions as JsonObject[]).filter(({ descriptor }) => {
      const value = descriptor as JsonObject;
      return value.kind === 'play-site'
        && value.cardInstanceId === river.instanceId
        && value.cell === 'C4';
    });
    assert.equal(riverPlays.length, 1);
    assert.match(String(riverPlays[0]!.label), /Play Autumn River at C4.*then inspect the next spell/);
    assert.doesNotMatch(String(riverPlays[0]!.label), /put .* on bottom|keep .* on top|card:|sha256:/i);
    current = await submit(riverPlays[0]!);
    assert.match(String(current.playerAction), /Play Autumn River at C4.*then inspect the next spell/);
    assert.doesNotMatch(String(current.playerAction), /put .* on bottom|keep .* on top|card:|sha256:/i);
    assert.equal(((current.view as JsonObject).phase), 'genesis');
    const southPending = await json('/api/view?seat=south');
    assert.deepEqual(southPending.actions, []);
    current = await json('/api/view?seat=north');
    const riverChoices = (current.actions as JsonObject[]).filter(({ descriptor }) =>
      (descriptor as JsonObject).kind === 'resolve-genesis-spell');
    assert.deepEqual(riverChoices.map(({ descriptor }) =>
      (descriptor as JsonObject).choice).sort(), ['bottom-next', 'keep-next']);
    assert.equal(riverChoices.every(({ label }) =>
      !/card:|sha256:/.test(String(label))), true);
    const bottomRiver = riverChoices.find(({ descriptor }) =>
      (descriptor as JsonObject).choice === 'bottom-next');
    assert.ok(bottomRiver);
    current = await submit(bottomRiver);
    const riverReplay = await json('/api/replay', { method: 'POST' });
    assert.equal(riverReplay.acceptedActionCount, 4);
    assert.equal(riverReplay.verified, true);
    assert.equal(riverReplay.finalStateHash, current.stateHash);

    for (const preset of catalog) {
      current = await json('/api/reset', {
        body: JSON.stringify({
          opponent: 'south',
          presetId: preset.id,
          seed: preset.manifest.seed,
        }),
        headers: { 'content-type': 'application/json' },
        method: 'POST',
      });
      let opponentActionCount = 0;
      let combatObserved = false;
      let opponentPowerAwaitingAttack = false;
      let opponentPowerUsed = false;
      for (let count = 0; count < 500; count += 1) {
        const currentView = current.view as JsonObject;
        const north = ((currentView.players as JsonObject).north as JsonObject);
        combatObserved ||= Number((north.avatar as JsonObject).life) < 20
          || (north.cemetery as unknown[]).length > 0;
        if ((currentView.terminal as JsonObject).status === 'finished') break;
        assert.equal(currentView.decisionSeat, 'north', preset.id);
        assert.ok((current.actions as JsonObject[]).length > 0, preset.id);
        current = await submit(deterministicAction(current));
        opponentActionCount += Number(current.opponentActionCount);
        const opponentActions = current.opponentActions as JsonObject[];
        assert.equal(opponentActions.length, Number(current.opponentActionCount), preset.id);
        opponentActions.forEach(({ events }) => {
          const eventTypes = events as string[];
          if (eventTypes.includes('power-granted')) {
            opponentPowerAwaitingAttack = true;
            opponentPowerUsed = false;
          }
          if (opponentPowerAwaitingAttack && eventTypes.includes('attack-declared')) {
            opponentPowerUsed = true;
          }
          if (opponentPowerAwaitingAttack && eventTypes.includes('power-expired')) {
            assert.equal(opponentPowerUsed, true, 'Earth computer wasted temporary power');
            opponentPowerAwaitingAttack = false;
          }
        });
        assert.doesNotMatch(JSON.stringify(opponentActions), /card:|sha256:/, preset.id);
      }
      const terminal = ((current.view as JsonObject).terminal as JsonObject);
      assert.equal(current.opponent, 'south', preset.id);
      assert.ok(opponentActionCount > 0, preset.id);
      assert.equal(combatObserved, true, preset.id);
      assert.equal(terminal.status, 'finished', preset.id);
      assert.ok(['avatar_defeated', 'simultaneous_avatar_defeat']
        .includes(String(terminal.reason)), preset.id);
      if (terminal.reason === 'avatar_defeated') {
        assert.notEqual(terminal.winner, terminal.loser, preset.id);
      } else {
        assert.equal(terminal.result, 'draw', preset.id);
      }
      assert.deepEqual(current.actions, [], preset.id);
      const fullReplay = await json('/api/replay', { method: 'POST' });
      assert.equal(fullReplay.verified, true, preset.id);
      assert.equal(fullReplay.finalStateHash, current.stateHash, preset.id);
    }
  } finally {
    await new Promise<void>((resolve, reject) => {
      server.close((error) => error ? reject(error) : resolve());
    });
  }
}

function assertStarter(
  result: PrivateGameCheck['earthStarter'],
  site: string,
  minion: string,
  acceptedActionCount = 4,
): void {
  assert.equal(result.site, site);
  assert.equal(result.minion, minion);
  assert.equal(result.acceptedActionCount, acceptedActionCount);
  assert.equal(result.manaPaid, 1);
  assert.equal(result.siteAndMinionStateVerified, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === site)?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === minion)?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertHamlet(result: PrivateGameCheck['fireHamlet']): void {
  assert.equal(result.hamlet, 'Hamlet');
  assert.equal(result.wasteland, 'Wasteland');
  assert.equal(result.raalDromedary, 'Raal Dromedary');
  assert.equal(result.seed, 135);
  assert.equal(result.acceptedActionCount, 10);
  assert.equal(result.exactDestinationCosts, true);
  assert.equal(result.fireAffinityVerified, true);
  assert.equal(result.manaPaid, 0);
  assert.equal(result.siteAndMinionStateVerified, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Hamlet')?.copies, 4);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Wasteland')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertGranaryRats(result: PrivateGameCheck['fireGranaryRats']): void {
  assert.equal(result.granaryRats, 'Granary Rats');
  assert.equal(result.wasteland, 'Wasteland');
  assert.equal(result.acceptedActionCount, 4);
  assert.equal(result.seed, 102);
  assert.equal(result.fireAffinityBeforeSummon, true);
  assert.equal(result.siteThresholdSuppressed, true);
  assert.equal(result.manaPaid, 1);
  assert.equal(result.siteAndMinionStateVerified, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Wasteland')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Granary Rats')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertSwordAndShield(result: PrivateGameCheck['earthSwordAndShield']): void {
  assert.equal(result.swordAndShield, 'Sword and Shield');
  assert.equal(result.elthamTownsfolk, 'Eltham Townsfolk');
  assert.equal(result.boskTroll, 'Bosk Troll');
  assert.equal(result.acceptedActionCount, 22);
  assert.equal(result.dropAcceptedActionCount, 20);
  assert.equal(result.dropDeathAcceptedActionCount, 42);
  assert.equal(result.seed, 9492);
  assert.equal(result.exactPickupChoice, true);
  assert.equal(result.dropChoiceVerified, true);
  assert.equal(result.dropEventVerified, true);
  assert.equal(result.dropStateVerified, true);
  assert.equal(result.dropSideEffectsAbsent, true);
  assert.equal(result.dropSecondUseUnavailable, true);
  assert.equal(result.dropUnavailableAfterInteraction, true);
  assert.equal(result.dropNoRandomDraws, true);
  assert.equal(result.dropReplayVerified, true);
  assert.equal(result.dropDeathVerified, true);
  assert.equal(result.dropDeathReplayVerified, true);
  assert.equal(result.manaPaid, 3);
  assert.equal(result.artifactCastUncarried, true);
  assert.equal(result.artifactPickedUpAndCarried, true);
  assert.equal(result.pickupSideEffectsAbsent, true);
  assert.equal(result.swordFollowedBearer, true);
  assert.equal(result.combatDamageAndSurvivalVerified, true);
  assert.equal(result.swordRemainedCarried, true);
  assert.equal(result.swordStayedOutOfCemetery, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.unrelatedStatePreserved, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.gameRemainedActive, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Spire')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Sword and Shield')?.copies, 3);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Zap!')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Eltham Townsfolk')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertVoidArtifact(result: PrivateGameCheck['airVoidArtifact']): void {
  assert.equal(result.spectralStalker, 'Spectral Stalker');
  assert.equal(result.swordAndShield, 'Sword and Shield');
  assert.equal(result.seed, 18);
  assert.equal(result.acceptedActionCount, 22);
  assert.equal(result.relocationVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Spire')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Spectral Stalker')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Sword and Shield')?.copies, 3);
  assert.equal(result.replayVerified, true);
}

function assertPoisonousDagger(result: PrivateGameCheck['earthPoisonousDagger']): void {
  assert.equal(result.poisonousDagger, 'Poisonous Dagger');
  assert.equal(result.elthamTownsfolk, 'Eltham Townsfolk');
  assert.equal(result.boskTroll, 'Bosk Troll');
  assert.equal(result.acceptedActionCount, 20);
  assert.equal(result.exactBearerChoice, true);
  assert.equal(result.manaPaid, 2);
  assert.equal(result.artifactCastAndCarried, true);
  assert.equal(result.combatLethalVerified, true);
  assert.equal(result.daggerDroppedUncontrolled, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.stateAndCemeteriesVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.gameRemainedActive, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Poisonous Dagger')?.copies, 3);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Eltham Townsfolk')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertHuntersLodge(result: PrivateGameCheck['earthHuntersLodge']): void {
  assert.equal(result.hunterLodge, "Hunter's Lodge");
  assert.equal(result.slyFox, 'Sly Fox');
  assert.equal(result.acceptedActionCount, 7);
  assert.equal(result.slyFoxGainedStealthFirst, true);
  assert.equal(result.enemyStealthRemoved, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.statePreserved, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === "Hunter's Lodge")?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Sly Fox')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertVikings(result: PrivateGameCheck['fireVikings']): void {
  assert.equal(result.vikings, 'Vikings');
  assert.equal(result.boskTroll, 'Bosk Troll');
  assert.equal(result.poisonousDagger, 'Poisonous Dagger');
  assert.equal(result.acceptedActionCount, 30);
  assert.equal(result.activationUnavailableWhileSickAndTapped, true);
  assert.equal(result.artifactCastAndCarried, true);
  assert.equal(result.abilityLethalVerified, true);
  assert.equal(result.daggerManaPaid, 2);
  assert.equal(result.exactAdjacentTarget, true);
  assert.equal(result.summonManaPaid, 5);
  assert.equal(result.simultaneousDamageVerified, true);
  assert.equal(result.targetsEnteredCemetery, true);
  assert.equal(result.vikingsSurvivedAndTapped, true);
  assert.equal(result.noCombatOrReturnDamage, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Valley')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Vikings')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Poisonous Dagger')?.copies, 3);
  assert.equal(result.replayVerified, true);
}

function assertMesmerism(result: PrivateGameCheck['waterMesmerism']): void {
  assert.equal(result.mesmerism, 'Mesmerism');
  assert.equal(result.seravaTownsfolk, 'Serava Townsfolk');
  assert.equal(result.kettletopLeprechaun, 'Kettletop Leprechaun');
  assert.equal(result.acceptedActionCount, 34);
  assert.equal(result.waterAffinityFour, true);
  assert.equal(result.exactNearbyTarget, true);
  assert.equal(result.farTargetUnavailable, true);
  assert.equal(result.manaPaid, 4);
  assert.equal(result.controlTransferred, true);
  assert.equal(result.deathriteControllerDrewSite, true);
  assert.equal(result.deathriteOwnerKeptCemetery, true);
  assert.equal(result.oldControllerHadAction, true);
  assert.equal(result.newControllerGainedAction, true);
  assert.equal(result.oldControllerLostAction, true);
  assert.equal(result.seed, 4724);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Stream')?.copies, 4);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Valley')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Mesmerism')?.copies, 1);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Serava Townsfolk')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Kettletop Leprechaun')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertMalakhim(result: PrivateGameCheck['earthMalakhim']): void {
  assert.equal(result.malakhim, 'Malakhim');
  assert.equal(result.acceptedActionCount, 32);
  assert.equal(result.airborneAndWard, true);
  assert.equal(result.earthAffinityThree, true);
  assert.equal(result.manaPaid, 6);
  assert.equal(result.normalActionTapped, true);
  assert.equal(result.endPhaseUntapped, true);
  assert.equal(result.opponentTurnReady, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Valley')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Malakhim')?.copies, 2);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Eltham Townsfolk')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertFatality(result: PrivateGameCheck['airFireFatality']): void {
  assert.equal(result.fatality, 'Fatality');
  assert.equal(result.zap, 'Zap!');
  assert.equal(result.snowLeopard, 'Snow Leopard');
  assert.equal(result.acceptedActionCount, 22);
  assert.equal(result.airFireAffinity, true);
  assert.equal(result.healthyTargetUnavailable, true);
  assert.equal(result.zapDamageExactlyOne, true);
  assert.equal(result.zapManaPaid, 1);
  assert.equal(result.exactWoundedTarget, true);
  assert.equal(result.manaPaid, 3);
  assert.equal(result.fatalityDealtNoDamage, true);
  assert.equal(result.targetLeftRealm, true);
  assert.equal(result.targetEnteredOwnerCemetery, true);
  assert.equal(result.zapEnteredCemetery, true);
  assert.equal(result.fatalityEnteredCemetery, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Spire')?.copies, 4);
  assert.equal(result.deck.atlas.find(({ name }) => name === 'Wasteland')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Fatality')?.copies, 3);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Zap!')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertMinorExplosion(result: PrivateGameCheck['fireMinorExplosion']): void {
  assert.equal(result.minorExplosion, 'Minor Explosion');
  assert.equal(result.raalDromedary, 'Raal Dromedary');
  assert.equal(result.acceptedActionCount, 17);
  assert.equal(result.exactLocationTargetAvailable, true);
  assert.equal(result.targetWithinTwoSteps, true);
  assert.equal(result.manaPaid, 3);
  assert.equal(result.avatarTookThreeDamage, true);
  assert.equal(result.simultaneousDamageVerified, true);
  assert.equal(result.twoMinionsDied, true);
  assert.equal(result.twoMinionsEnteredCemetery, true);
  assert.equal(result.spellEnteredCemetery, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.noRandomDraws, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Minor Explosion')?.copies, 4);
  assert.equal(result.deck.spellbook
    .find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

function assertFireResponse(result: PrivateGameCheck['fireResponse']): void {
  assert.equal(result.lumberingGiant, 'Lumbering Giant');
  assert.equal(result.monstrousLion, 'Monstrous Lion');
  assert.equal(result.chargeMoveAndAttack, true);
  assert.equal(result.unitTargetAvailable, true);
  assert.equal(result.siteTargetUnavailable, true);
  assert.equal(result.defendUnavailable, true);
  assert.equal(result.interceptUnavailable, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.replayVerified, true);
}

function assertFireCharge(result: PrivateGameCheck['fireCharge']): void {
  assert.equal(result.charge, 'Charge');
  assert.equal(result.raalDromedary, 'Raal Dromedary');
  assert.equal(result.acceptedActionCount, 12);
  assert.equal(result.moveUnavailableBeforeCharge, true);
  assert.equal(result.exactNonTargetAllyChoice, true);
  assert.equal(result.manaPaid, 1);
  assert.equal(result.temporaryChargeRecorded, true);
  assert.equal(result.moveAvailableAfterCharge, true);
  assert.equal(result.unitStatePreservedOnGrant, true);
  assert.equal(result.expiredAtEndOfTurn, true);
  assert.equal(result.causalEventsVerified, true);
  assert.equal(result.spellEnteredCemetery, true);
  assert.equal(result.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Charge')?.copies, 4);
  assert.equal(result.deck.spellbook.find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.replayVerified, true);
}

test('private actual-card browser presets reach combat, terminal state, and exact replay', async () => {
  await verifyPrivateStarterHttp(await loadPrivateStarterCatalog());
});

test('private actual-card manifests produce identical summary-only worker batches', async () => {
  const lessons = (await loadPrivateStarterCatalog()).filter(({ id }) => id.endsWith('-lesson'));
  assert.deepEqual(lessons.map(({ id }) => id), ['air-vs-earth-lesson', 'earth-vs-air-lesson']);
  assert.equal(lessons[1]?.manifest.seed, 7_382);
  const manifests = lessons.map(({ manifest }) => manifest);
  const oneWorker = await runGameBatch(manifests, 1);
  const twoWorkers = await runGameBatch(manifests, 2);
  assert.deepEqual(oneWorker, twoWorkers);
  assert.equal(oneWorker.every(({ report }) =>
    report.fightCount > 0
      && report.replayVerified
      && report.terminal.status === 'finished'), true);

  const serialized = JSON.stringify(oneWorker);
  assert.doesNotMatch(serialized, /\.local|officialSourceId|rulesText/);
  for (const preset of lessons) {
    assert.equal(serialized.includes(preset.manifest.authority.contentHash), false);
    assert.equal(serialized.includes(preset.manifest.authority.revisionId), false);
    assert.equal(Object.keys(preset.manifest.cards).some((cardId) => serialized.includes(cardId)), false);
    assert.equal(Object.values(preset.cardNames).some((name) => serialized.includes(name)), false);
  }
});

test('private actual-card decks complete deterministic combat, Earth, Air, Fire, and Water scenarios', async () => {
  const [result, starterCatalog] = await Promise.all([
    runPrivateGameCheck(),
    loadPrivateStarterCatalog(),
  ]);
  assert.deepEqual(starterCatalog.map(({ id }) => id), [
    'air-vs-earth-lesson',
    'earth-vs-air-lesson',
    'air-starter',
    'earth-starter',
    'fire-starter',
    'water-starter',
  ]);
  for (const preset of starterCatalog) {
    assert.equal(preset.manifest.authority.mode, 'private-local');
    if (preset.id.endsWith('-lesson')) {
      assert.equal(preset.usesOnlyOrdinaryOrExceptionalCards, false);
      assert.equal(
        preset.manifest.decks.north.atlas.length,
        preset.id === 'air-vs-earth-lesson' ? 13 : 16,
      );
      assert.equal(
        preset.manifest.decks.north.spellbook.length,
        preset.id === 'air-vs-earth-lesson' ? 26 : 35,
      );
      assert.equal(
        preset.manifest.decks.south.atlas.length,
        preset.id === 'air-vs-earth-lesson' ? 16 : 13,
      );
      assert.equal(
        preset.manifest.decks.south.spellbook.length,
        preset.id === 'air-vs-earth-lesson' ? 35 : 26,
      );
      assert.notDeepEqual(preset.manifest.decks.north, preset.manifest.decks.south);
    } else {
      assert.equal(preset.usesOnlyOrdinaryOrExceptionalCards, true);
      assert.equal(preset.manifest.decks.north.atlas.length, 30);
      assert.equal(preset.manifest.decks.north.spellbook.length, 60);
      assert.deepEqual(preset.manifest.decks.north, preset.manifest.decks.south);
    }
    assert.equal(Object.keys(preset.cardNames).length, Object.keys(preset.manifest.cards).length);
  }
  const summarize = (
    preset: PrivateStarterPreset,
    seat: 'north' | 'south',
    zone: 'atlas' | 'spellbook',
  ): Record<string, number> => preset.manifest.decks[seat][zone].reduce<Record<string, number>>(
    (counts, cardId) => {
      const name = preset.cardNames[cardId]!;
      counts[name] = (counts[name] ?? 0) + 1;
      return counts;
    },
    {},
  );
  const airLesson = starterCatalog[0]!;
  assert.deepEqual(summarize(airLesson, 'north', 'atlas'), {
    'Dark Tower': 3,
    'Gothic Tower': 3,
    'Lone Tower': 3,
    'Mountain Pass': 2,
    'Updraft Ridge': 2,
  });
  assert.deepEqual(summarize(airLesson, 'north', 'spellbook'), {
    'Apprentice Wizard': 2,
    Blink: 2,
    'Cloud Spirit': 2,
    'Dead of Night Demon': 2,
    'Gyre Hippogriffs': 1,
    'Grandmaster Wizard': 1,
    'Highland Clansmen': 1,
    'Lightning Bolt': 3,
    'Midnight Rogue': 2,
    'Plumed Pegasus': 2,
    'Roaming Monster': 1,
    'Sling Pixies': 1,
    'Snow Leopard': 2,
    'Spire Lich': 1,
    'Spectral Stalker': 2,
    Teleport: 1,
  });
  const grandmasterWizardId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Grandmaster Wizard')?.[0];
  assert.ok(grandmasterWizardId);
  const grandmasterWizard = airLesson.manifest.cards[grandmasterWizardId];
  assert.equal(grandmasterWizard?.cardType, 'minion');
  if (grandmasterWizard?.cardType === 'minion') {
    assert.equal(grandmasterWizard.attack, 0);
    assert.equal(grandmasterWizard.defense, 0);
    assert.equal(grandmasterWizard.manaCost, 6);
    assert.equal(grandmasterWizard.mortal, true);
    assert.equal(grandmasterWizard.spellcaster, true);
    assert.equal(grandmasterWizard.genesisDrawSpells, 3);
    assert.deepEqual(grandmasterWizard.thresholds, { air: 2, earth: 0, fire: 0, water: 0 });
  }
  const slingPixiesId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Sling Pixies')?.[0];
  assert.ok(slingPixiesId);
  const slingPixies = airLesson.manifest.cards[slingPixiesId];
  assert.equal(slingPixies?.cardType, 'minion');
  if (slingPixies?.cardType === 'minion') {
    assert.equal(slingPixies.attack, 1);
    assert.equal(slingPixies.defense, 1);
    assert.equal(slingPixies.manaCost, 1);
    assert.equal(slingPixies.airborne, true);
    assert.equal(slingPixies.ranged, true);
    assert.equal(slingPixies.preventsDamageFromUnitsWithPowerAtLeast, 4);
    assert.deepEqual(slingPixies.thresholds, { air: 1, earth: 0, fire: 0, water: 0 });
  }
  const spireLichId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Spire Lich')?.[0];
  assert.ok(spireLichId);
  const spireLich = airLesson.manifest.cards[spireLichId];
  assert.equal(spireLich?.cardType, 'minion');
  if (spireLich?.cardType === 'minion') {
    assert.equal(spireLich.attack, 1);
    assert.equal(spireLich.defense, 1);
    assert.equal(spireLich.manaCost, 3);
    assert.equal(spireLich.ranged, undefined);
    assert.equal(spireLich.spellcaster, undefined);
    assert.equal(spireLich.gainsPowerRangedAndSpellcasterAtopTower, 2);
    assert.deepEqual(spireLich.thresholds, { air: 1, earth: 0, fire: 0, water: 0 });
  }
  for (const towerName of ['Dark Tower', 'Gothic Tower', 'Lone Tower']) {
    const towerId = Object.entries(airLesson.cardNames)
      .find(([, name]) => name === towerName)?.[0];
    assert.ok(towerId);
    assert.equal(airLesson.manifest.cards[towerId]?.cardType, 'site');
    assert.equal(airLesson.manifest.cards[towerId]?.cardType === 'site'
      && airLesson.manifest.cards[towerId].isTower, true);
  }
  assert.deepEqual(summarize(airLesson, 'south', 'atlas'), {
    Bedrock: 1,
    'Holy Ground': 1,
    'Humble Village': 3,
    Quagmire: 2,
    'Rustic Village': 3,
    'Simple Village': 3,
    Sinkhole: 1,
    'Vantage Hills': 2,
  });
  assert.deepEqual(summarize(airLesson, 'south', 'spellbook'), {
    'Amazon Warriors': 2,
    'Autumn Unicorn': 2,
    'Belmotte Longbowmen': 3,
    'Border Militia': 1,
    Bury: 2,
    'Cave-In': 1,
    'Cave Trolls': 3,
    'Dalcean Phalanx': 1,
    'Divine Healing': 1,
    'Entangle Terrain': 1,
    'House Arn Bannerman': 2,
    'King of the Realm': 1,
    'Land Surveyor': 2,
    'Mountain Giant': 1,
    Overpower: 2,
    'Payload Trebuchet': 1,
    'Pudge Butcher': 1,
    'Rolling Boulder': 1,
    'Scent Hounds': 2,
    'Siege Ballista': 1,
    'Slumbering Giantess': 1,
    'Wild Boars': 2,
    'Wraetannis Titan': 1,
  });
  const midnightRogueId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Midnight Rogue')?.[0];
  assert.ok(midnightRogueId);
  const midnightRogue = airLesson.manifest.cards[midnightRogueId];
  assert.equal(midnightRogue?.cardType, 'minion');
  if (midnightRogue?.cardType === 'minion') {
    assert.equal(midnightRogue.attack, 2);
    assert.equal(midnightRogue.defense, 2);
    assert.equal(midnightRogue.manaCost, 3);
    assert.equal(midnightRogue.ordinary, true);
    assert.equal(midnightRogue.ranged, true);
    assert.equal(midnightRogue.stealth, true);
    assert.deepEqual(midnightRogue.thresholds, { air: 1, earth: 0, fire: 0, water: 0 });
  }
  const expectedActualMinions = {
    'Amazon Warriors': { airborne: undefined, attack: 5, charge: undefined, defense: 5, manaCost: 5, ordinary: true, stealth: undefined },
    'Autumn Unicorn': { airborne: undefined, attack: 4, charge: undefined, defense: 4, manaCost: 3, ordinary: undefined, stealth: undefined },
    'Dead of Night Demon': { airborne: undefined, attack: 2, charge: undefined, defense: 2, manaCost: 2, ordinary: true, stealth: true },
    'Gyre Hippogriffs': { airborne: true, attack: 3, charge: true, defense: 3, manaCost: 4, ordinary: undefined, stealth: undefined },
    'Highland Clansmen': { airborne: undefined, attack: 5, charge: true, defense: 5, manaCost: 7, ordinary: true, stealth: undefined },
  } as const;
  for (const [name, expected] of Object.entries(expectedActualMinions)) {
    const cardId = Object.entries(airLesson.cardNames)
      .find(([, candidate]) => candidate === name)?.[0];
    assert.ok(cardId);
    const definition = airLesson.manifest.cards[cardId];
    assert.equal(definition?.cardType, 'minion');
    if (definition?.cardType !== 'minion') continue;
    assert.deepEqual({
      airborne: definition.airborne,
      attack: definition.attack,
      charge: definition.charge,
      defense: definition.defense,
      manaCost: definition.manaCost,
      ordinary: definition.ordinary,
      stealth: definition.stealth,
    }, expected);
  }
  const houseArnId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'House Arn Bannerman')?.[0];
  assert.ok(houseArnId);
  const houseArn = airLesson.manifest.cards[houseArnId];
  assert.equal(houseArn?.cardType, 'minion');
  if (houseArn?.cardType === 'minion') {
    assert.deepEqual({
      attack: houseArn.attack,
      defense: houseArn.defense,
      manaCost: houseArn.manaCost,
      mortal: houseArn.mortal,
      otherNearbyAlliesPowerBonus: houseArn.otherNearbyAlliesPowerBonus,
      thresholds: houseArn.thresholds,
    }, {
      attack: 2,
      defense: 2,
      manaCost: 4,
      mortal: true,
      otherNearbyAlliesPowerBonus: 1,
      thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
    });
  }
  const wraetannisTitanId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Wraetannis Titan')?.[0];
  assert.ok(wraetannisTitanId);
  const wraetannisTitan = airLesson.manifest.cards[wraetannisTitanId];
  assert.equal(wraetannisTitan?.cardType, 'minion');
  if (wraetannisTitan?.cardType === 'minion') {
    assert.deepEqual({
      attack: wraetannisTitan.attack,
      defense: wraetannisTitan.defense,
      genesisStrikeEachEnemyHere: wraetannisTitan.genesisStrikeEachEnemyHere,
      manaCost: wraetannisTitan.manaCost,
      thresholds: wraetannisTitan.thresholds,
    }, {
      attack: 6,
      defense: 6,
      genesisStrikeEachEnemyHere: true,
      manaCost: 7,
      thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
    });
  }
  const kingOfRealmId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'King of the Realm')?.[0];
  assert.ok(kingOfRealmId);
  const kingOfRealm = airLesson.manifest.cards[kingOfRealmId];
  assert.equal(kingOfRealm?.cardType, 'minion');
  if (kingOfRealm?.cardType === 'minion') {
    assert.deepEqual({
      attack: kingOfRealm.attack,
      defense: kingOfRealm.defense,
      manaCost: kingOfRealm.manaCost,
      mortal: kingOfRealm.mortal,
      otherControlledMortalsPowerBonus: kingOfRealm.otherControlledMortalsPowerBonus,
      thresholds: kingOfRealm.thresholds,
    }, {
      attack: 3,
      defense: 3,
      manaCost: 7,
      mortal: true,
      otherControlledMortalsPowerBonus: 1,
      thresholds: { air: 0, earth: 3, fire: 0, water: 0 },
    });
  }
  const mountainGiantId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Mountain Giant')?.[0];
  assert.ok(mountainGiantId);
  const mountainGiant = airLesson.manifest.cards[mountainGiantId];
  assert.equal(mountainGiant?.cardType, 'minion');
  if (mountainGiant?.cardType === 'minion') {
    assert.deepEqual({
      attack: mountainGiant.attack,
      defense: mountainGiant.defense,
      manaCost: mountainGiant.manaCost,
      occupiesSquareArea: mountainGiant.occupiesSquareArea,
      thresholds: mountainGiant.thresholds,
    }, {
      attack: 8,
      defense: 8,
      manaCost: 8,
      occupiesSquareArea: 2,
      thresholds: { air: 0, earth: 4, fire: 0, water: 0 },
    });
  }
  const landSurveyorId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Land Surveyor')?.[0];
  assert.ok(landSurveyorId);
  const landSurveyor = airLesson.manifest.cards[landSurveyorId];
  assert.equal(landSurveyor?.cardType, 'minion');
  if (landSurveyor?.cardType === 'minion') assert.equal(landSurveyor.mortal, true);
  const slumberingGiantessId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Slumbering Giantess')?.[0];
  assert.ok(slumberingGiantessId);
  const slumberingGiantess = airLesson.manifest.cards[slumberingGiantessId];
  assert.equal(slumberingGiantess?.cardType, 'minion');
  if (slumberingGiantess?.cardType === 'minion') {
    assert.deepEqual({
      attack: slumberingGiantess.attack,
      defense: slumberingGiantess.defense,
      genesisDisableSelfUntilDamaged: slumberingGiantess.genesisDisableSelfUntilDamaged,
      manaCost: slumberingGiantess.manaCost,
      thresholds: slumberingGiantess.thresholds,
    }, {
      attack: 5,
      defense: 5,
      genesisDisableSelfUntilDamaged: true,
      manaCost: 3,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    });
  }
  const caveInId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Cave-In')?.[0];
  assert.ok(caveInId);
  const caveIn = airLesson.manifest.cards[caveInId];
  assert.equal(caveIn?.cardType, 'magic');
  if (caveIn?.cardType === 'magic') {
    assert.deepEqual({
      burrowAllMinionsAndArtifactsAtTargetLandSite:
        caveIn.burrowAllMinionsAndArtifactsAtTargetLandSite,
      manaCost: caveIn.manaCost,
      thresholds: caveIn.thresholds,
    }, {
      burrowAllMinionsAndArtifactsAtTargetLandSite: true,
      manaCost: 4,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    });
  }
  const siegeBallistaId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Siege Ballista')?.[0];
  assert.ok(siegeBallistaId);
  const siegeBallista = airLesson.manifest.cards[siegeBallistaId];
  assert.equal(siegeBallista?.cardType, 'artifact');
  if (siegeBallista?.cardType === 'artifact') {
    assert.deepEqual({
      manaCost: siegeBallista.manaCost,
      tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps:
        siegeBallista.tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps,
      thresholds: siegeBallista.thresholds,
    }, {
      manaCost: 3,
      tapBearerAndAnotherAllyHereToDamageTargetWithinTwoSteps: 3,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    });
  }
  const payloadTrebuchetId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Payload Trebuchet')?.[0];
  assert.ok(payloadTrebuchetId);
  const payloadTrebuchet = airLesson.manifest.cards[payloadTrebuchetId];
  assert.equal(payloadTrebuchet?.cardType, 'artifact');
  if (payloadTrebuchet?.cardType === 'artifact') {
    assert.deepEqual({
      manaCost: payloadTrebuchet.manaCost,
      tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps:
        payloadTrebuchet
          .tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps,
      thresholds: payloadTrebuchet.thresholds,
    }, {
      manaCost: 5,
      tapBearerAndAnotherAllyHereAndDiscardCardToDamageEachUnitAtLocationWithinThreeSteps: true,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    });
  }
  const rollingBoulderId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Rolling Boulder')?.[0];
  assert.ok(rollingBoulderId);
  const rollingBoulder = airLesson.manifest.cards[rollingBoulderId];
  assert.equal(rollingBoulder?.cardType, 'artifact');
  if (rollingBoulder?.cardType === 'artifact') {
    assert.deepEqual({
      manaCost: rollingBoulder.manaCost,
      tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath:
        rollingBoulder.tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath,
      thresholds: rollingBoulder.thresholds,
    }, {
      manaCost: 4,
      tapUnitHereToRollInCardinalDirectionAndDamageOtherUnitsAlongPath: 4,
      thresholds: { air: 0, earth: 0, fire: 0, water: 0 },
    });
  }
  const scentHoundsId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Scent Hounds')?.[0];
  assert.ok(scentHoundsId);
  const scentHounds = airLesson.manifest.cards[scentHoundsId];
  assert.equal(scentHounds?.cardType, 'minion');
  if (scentHounds?.cardType === 'minion') {
    assert.deepEqual({
      attack: scentHounds.attack,
      defense: scentHounds.defense,
      manaCost: scentHounds.manaCost,
      mortal: scentHounds.mortal,
      nearbyEnemiesPermanentlyLoseStealth:
        scentHounds.nearbyEnemiesPermanentlyLoseStealth,
      ordinary: scentHounds.ordinary,
      thresholds: scentHounds.thresholds,
    }, {
      attack: 2,
      defense: 2,
      manaCost: 2,
      mortal: undefined,
      nearbyEnemiesPermanentlyLoseStealth: true,
      ordinary: true,
      thresholds: { air: 0, earth: 1, fire: 0, water: 0 },
    });
  }
  const blinkId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Blink')?.[0];
  assert.ok(blinkId);
  assert.deepEqual(airLesson.manifest.cards[blinkId], {
    cardType: 'magic',
    manaCost: 2,
    teleportNearbyAllyThenDrawCard: true,
    thresholds: { air: 1, earth: 0, fire: 0, water: 0 },
  });
  const vantageHillsId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Vantage Hills')?.[0];
  assert.ok(vantageHillsId);
  assert.deepEqual(airLesson.manifest.cards[vantageHillsId], {
    cardType: 'site',
    elements: ['earth'],
    rangedUnitsHereRangeBonus: 1,
  });
  const quagmireId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Quagmire')?.[0];
  assert.ok(quagmireId);
  assert.deepEqual(airLesson.manifest.cards[quagmireId], {
    cardType: 'site',
    elements: ['earth'],
    genesisImmobilizeNearbyUntilNextTurn: true,
  });
  const entangleTerrainId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Entangle Terrain')?.[0];
  assert.ok(entangleTerrainId);
  assert.deepEqual(airLesson.manifest.cards[entangleTerrainId], {
    cardType: 'aura',
    immobilizeAndGroundMinionsAtAffectedSitesForThreeControllerTurns: true,
    manaCost: 4,
    thresholds: { air: 0, earth: 2, fire: 0, water: 0 },
  });
  const holyGroundId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Holy Ground')?.[0];
  assert.ok(holyGroundId);
  assert.deepEqual(airLesson.manifest.cards[holyGroundId], {
    cardType: 'site',
    elements: ['earth'],
    genesisHealNearbyAvatars: 3,
  });
  const bedrockId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Bedrock')?.[0];
  assert.ok(bedrockId);
  assert.deepEqual(airLesson.manifest.cards[bedrockId], {
    cannotBeMovedDestroyedOrModified: true,
    cardType: 'site',
    elements: ['earth'],
  });
  const mountainPassId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Mountain Pass')?.[0];
  assert.ok(mountainPassId);
  assert.deepEqual(airLesson.manifest.cards[mountainPassId], {
    blocksGroundMinionEntryWhileMinionAtop: true,
    cardType: 'site',
    elements: ['air'],
  });
  const updraftRidgeId = Object.entries(airLesson.cardNames)
    .find(([, name]) => name === 'Updraft Ridge')?.[0];
  assert.ok(updraftRidgeId);
  assert.deepEqual(airLesson.manifest.cards[updraftRidgeId], {
    airborneMinionsAtopMoveFreelyAway: true,
    cardType: 'site',
    elements: ['air'],
  });
  for (const name of ['Dark Tower', 'Gothic Tower', 'Lone Tower']) {
    const cardId = Object.entries(airLesson.cardNames)
      .find(([, candidate]) => candidate === name)?.[0];
    assert.ok(cardId);
    assert.deepEqual(airLesson.manifest.cards[cardId], {
      cardType: 'site',
      elements: ['air'],
      genesisGainManaIfOnlyControlledCopy: 1,
      isTower: true,
    });
  }
  const earthLesson = starterCatalog[1]!;
  assert.equal(airLesson.manifest.decks.south.spellbook
    .filter((cardId) => cardId === entangleTerrainId).length, 1);
  assert.deepEqual(earthLesson.manifest.decks.north, airLesson.manifest.decks.south);
  assert.deepEqual(earthLesson.manifest.decks.south, airLesson.manifest.decks.north);
  assert.equal(airLesson.cardNames[airLesson.manifest.decks.north.avatar], 'Sparkmage');
  assert.equal(airLesson.cardNames[airLesson.manifest.decks.south.avatar], 'Geomancer');
  assert.equal(earthLesson.cardNames[earthLesson.manifest.decks.north.avatar], 'Geomancer');
  assert.equal(earthLesson.cardNames[earthLesson.manifest.decks.south.avatar], 'Sparkmage');
  [
    ['air-starter', 'Snow Leopard'],
    ['earth-starter', 'Wild Boars'],
    ['fire-starter', 'Raal Dromedary'],
    ['water-starter', 'Serava Townsfolk'],
  ].forEach(([id, name]) => {
    const preset = starterCatalog.find((candidate) => candidate.id === id);
    assert.ok(preset);
    assert.equal(Object.values(preset.cardNames).includes(name!), true);
  });
  assert.equal(
    starterCatalog[0]!.cardNames[starterCatalog[0]!.manifest.decks.north.avatar],
    'Sparkmage',
  );
  assert.match(starterCatalog[0]!.label, /Air Beta vs Earth Beta.*one boxed precon each/);
  assert.equal(Object.values(starterCatalog.find(({ id }) =>
    id === 'water-starter')!.cardNames).includes('Autumn River'), true);
  assert.equal(Object.values(starterCatalog.find(({ id }) =>
    id === 'fire-starter')!.cardNames).includes('Charge'), true);
  assertStarter(result.airStarter, 'Spire', 'Snow Leopard');
  assert.equal(result.airStarter.deck.spellbook
    .find(({ name }) => name === 'Zap!')?.copies, 4);
  assertFatality(result.airFireFatality);
  assertStarter(result.earthStarter, 'Humble Village', 'Wild Boars');
  assertMalakhim(result.earthMalakhim);
  assertStarter(result.fireStarter, 'Wasteland', 'Raal Dromedary');
  assert.equal(result.fireStarter.deck.spellbook
    .find(({ name }) => name === 'Charge')?.copies, 4);
  assert.equal(result.fireVileImp.vileImp, 'Vile Imp');
  assert.equal(result.fireVileImp.wasteland, 'Wasteland');
  assert.equal(result.fireVileImp.acceptedActionCount, 10);
  assert.equal(result.fireVileImp.avatarTookTwoDamage, true);
  assert.equal(result.fireVileImp.causalEventsVerified, true);
  assert.equal(result.fireVileImp.declinePreservedAvatar, true);
  assert.equal(result.fireVileImp.exactChoices, true);
  assert.equal(result.fireVileImp.legalLowRarityDeck, true);
  assert.equal(result.fireVileImp.manaPaid, 2);
  assert.equal(result.fireVileImp.noRandomDraws, true);
  assert.equal(result.fireVileImp.seed, 141);
  assert.equal(result.fireVileImp.summonedAtC3, true);
  assert.equal(result.fireVileImp.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireVileImp.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireVileImp.deck.atlas
    .find(({ name }) => name === 'Wasteland')?.copies, 4);
  assert.equal(result.fireVileImp.deck.spellbook
    .find(({ name }) => name === 'Vile Imp')?.copies, 4);
  assert.equal(result.fireVileImp.replayVerified, true);
  assertGranaryRats(result.fireGranaryRats);
  assertHamlet(result.fireHamlet);
  assertVoidArtifact(result.airVoidArtifact);
  assertStarter(result.waterStarter, 'Autumn River', 'Serava Townsfolk', 5);
  assert.equal(result.waterRiver.river, 'Autumn River');
  assert.equal(result.waterRiver.acceptedActionCount, 4);
  assert.equal(result.waterRiver.exactChoices, true);
  assert.equal(result.waterRiver.keptNextSpell, true);
  assert.equal(result.waterRiver.bottomedNextSpell, true);
  assert.equal(result.waterRiver.hiddenFromOpponent, true);
  assert.equal(result.waterRiver.causalEventsVerified, true);
  assert.equal(result.waterRiver.legalLowRarityDeck, true);
  assert.equal(result.waterRiver.noRandomDraws, true);
  assert.equal(result.waterRiver.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterRiver.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterRiver.deck.atlas
    .find(({ name }) => name === 'Autumn River')?.copies, 4);
  assert.equal(result.waterRiver.deck.atlas
    .find(({ name }) => name === 'Stream')?.copies, 4);
  assert.equal(result.waterRiver.replayVerified, true);
  assertVikings(result.fireVikings);
  assert.equal(result.classification, 'private-local_actual-cards_unranked-partial-rules');
  assert.equal(result.airGenesisSpell.genesisMinion, 'Apprentice Wizard');
  assert.equal(result.airGenesisSpell.acceptedActionCount, 15);
  assert.equal(result.airGenesisSpell.drewSpell, true);
  assert.equal(result.airGenesisSpell.handSizePreserved, true);
  assert.equal(result.airGenesisSpell.hiddenFromOpponent, true);
  assert.equal(result.airGenesisSpell.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airGenesisSpell.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airGenesisSpell.deck.spellbook
    .find(({ name }) => name === 'Apprentice Wizard')?.copies, 4);
  assert.equal(result.airGenesisSpell.replayVerified, true);
  assert.equal(result.airGrandmasterWizard.grandmasterWizard, 'Grandmaster Wizard');
  assert.equal(result.airGrandmasterWizard.acceptedActionCount, 30);
  assert.equal(result.airGrandmasterWizard.manaPaid, 6);
  assert.equal(result.airGrandmasterWizard.spellcasterAndZeroPowerVerified, true);
  assert.equal(result.airGrandmasterWizard.exactlyThreeOrderedDraws, true);
  assert.equal(result.airGrandmasterWizard.hiddenFromOpponent, true);
  assert.equal(result.airGrandmasterWizard.causalEventsVerified, true);
  assert.equal(result.airGrandmasterWizard.legalConstructedDeck, true);
  assert.equal(result.airGrandmasterWizard.noRandomDraws, true);
  assert.equal(result.airGrandmasterWizard.unsupportedMechanicsAbsent, true);
  assert.equal(result.airGrandmasterWizard.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airGrandmasterWizard.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airGrandmasterWizard.deck.spellbook
    .find(({ name }) => name === 'Grandmaster Wizard')?.copies, 1);
  assert.equal(result.airGrandmasterWizard.replayVerified, true);
  assert.equal(result.airSlingPixies.slingPixies, 'Sling Pixies');
  assert.equal(result.airSlingPixies.vikings, 'Vikings');
  assert.equal(result.airSlingPixies.raalDromedary, 'Raal Dromedary');
  assert.equal(result.airSlingPixies.seed, 280);
  assert.equal(result.airSlingPixies.acceptedActionCount, 53);
  assert.equal(result.airSlingPixies.currentPowersVerified, true);
  assert.equal(result.airSlingPixies.firstFightPrevented, true);
  assert.equal(result.airSlingPixies.secondFightKilledSling, true);
  assert.equal(result.airSlingPixies.causalEventsVerified, true);
  assert.equal(result.airSlingPixies.noRandomDraws, true);
  assert.equal(result.airSlingPixies.unsupportedMechanicsAbsent, true);
  assert.equal(result.airSlingPixies.legalConstructedDeck, true);
  assert.equal(result.airSlingPixies.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airSlingPixies.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airSlingPixies.deck.spellbook
    .find(({ name }) => name === 'Sling Pixies')?.copies, 3);
  assert.equal(result.airSlingPixies.replayVerified, true);
  assert.equal(result.airSpireLich.spireLich, 'Spire Lich');
  assert.equal(result.airSpireLich.seed, 220);
  assert.equal(result.airSpireLich.acceptedActionCount, 38);
  assert.equal(result.airSpireLich.towerBonusVerified, true);
  assert.equal(result.airSpireLich.spellcasterActionResolved, true);
  assert.equal(result.airSpireLich.rangedActionResolved, true);
  assert.equal(result.airSpireLich.capabilitiesRemovedOffTower, true);
  assert.equal(result.airSpireLich.causalEventsVerified, true);
  assert.equal(result.airSpireLich.noRandomDraws, true);
  assert.equal(result.airSpireLich.unsupportedMechanicsAbsent, true);
  assert.equal(result.airSpireLich.legalConstructedDeck, true);
  assert.equal(result.airSpireLich.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airSpireLich.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airSpireLich.deck.spellbook
    .find(({ name }) => name === 'Spire Lich')?.copies, 3);
  assert.equal(result.airSpireLich.replayVerified, true);
  assert.equal(result.airSpellcasterFreeze.apprenticeWizard, 'Apprentice Wizard');
  assert.equal(result.airSpellcasterFreeze.freeze, 'Freeze');
  assert.equal(result.airSpellcasterFreeze.seravaTownsfolk, 'Serava Townsfolk');
  assert.equal(result.airSpellcasterFreeze.acceptedActionCount, 17);
  assert.equal(result.airSpellcasterFreeze.exactCasterRelativeAction, true);
  assert.equal(result.airSpellcasterFreeze.wizardCastWhileSummoningSick, true);
  assert.equal(result.airSpellcasterFreeze.wizardStatePreserved, true);
  assert.equal(result.airSpellcasterFreeze.genesisDrewSpell, true);
  assert.equal(result.airSpellcasterFreeze.seravaDisabled, true);
  assert.equal(result.airSpellcasterFreeze.manaPaid, 1);
  assert.equal(result.airSpellcasterFreeze.spellEnteredCemetery, true);
  assert.equal(result.airSpellcasterFreeze.causalEventsVerified, true);
  assert.equal(result.airSpellcasterFreeze.noRandomDraws, true);
  assert.equal(result.airSpellcasterFreeze.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airSpellcasterFreeze.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airSpellcasterFreeze.deck.spellbook
    .find(({ name }) => name === 'Apprentice Wizard')?.copies, 4);
  assert.equal(result.airSpellcasterFreeze.deck.spellbook
    .find(({ name }) => name === 'Freeze')?.copies, 4);
  assert.equal(result.airSpellcasterFreeze.deck.spellbook
    .find(({ name }) => name === 'Serava Townsfolk')?.copies, 4);
  assert.equal(result.airSpellcasterFreeze.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.airSpellcasterFreeze.replayVerified, true);
  assert.equal(result.airborne.airborneMinion, 'Plumed Pegasus');
  assert.equal(result.airborne.groundMinion, 'Ghoul');
  assert.equal(result.airborne.diagonalMove, true);
  assert.equal(result.airborne.airborneCanAttackGround, true);
  assert.equal(result.airborne.groundCannotIntercept, true);
  assert.equal(result.airborne.groundCannotAttackAirborne, true);
  assert.equal(result.airborne.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airborne.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airborne.deck.spellbook
    .find(({ name }) => name === 'Plumed Pegasus')?.copies, 4);
  assert.equal(result.airborne.deck.spellbook
    .find(({ name }) => name === 'Ghoul')?.copies, 4);
  assert.equal(result.airborne.replayVerified, true);
  assert.equal(result.stealth.stealthMinion, 'Band of Thieves');
  assert.equal(result.stealth.groundMinion, 'Snow Leopard');
  assert.equal(result.stealth.enteredStealthed, true);
  assert.equal(result.stealth.groundCouldNotAttack, true);
  assert.equal(result.stealth.attackSkippedDefend, true);
  assert.equal(result.stealth.stealthLostAfterAttack, true);
  assert.equal(result.stealth.groundMinionDied, true);
  assert.equal(result.stealth.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.stealth.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.stealth.deck.spellbook
    .find(({ name }) => name === 'Band of Thieves')?.copies, 4);
  assert.equal(result.stealth.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.deepEqual(result.stealth.deck, result.airborne.deck);
  assert.equal(result.stealth.replayVerified, true);
  assert.equal(result.airMovement.movementMinion, 'Snallygaster');
  assert.equal(result.airMovement.twoStepDefend, true);
  assert.equal(result.airMovement.twoStepMoveAndAttack, true);
  assert.equal(result.airMovement.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airMovement.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airMovement.replayVerified, true);
  assert.equal(result.airMovementTwo.movementMinion, 'Cloud Spirit');
  assert.equal(result.airMovementTwo.threeStepAirbornePath, true);
  assert.equal(result.airMovementTwo.attackAvailableAfterThreeSteps, true);
  assert.equal(result.airMovementTwo.returningPathAvailable, true);
  assert.equal(result.airMovementTwo.repeatedStepUnavailable, true);
  assert.equal(result.airMovementTwo.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airMovementTwo.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airMovementTwo.deck.spellbook
    .find(({ name }) => name === 'Cloud Spirit')?.copies, 4);
  assert.deepEqual(result.airMovementTwo.deck, result.airborne.deck);
  assert.equal(result.airMovementTwo.replayVerified, true);
  assert.equal(result.airSummoning.roamingMinion, 'Roaming Monster');
  assert.equal(result.airSummoning.ordinaryRestricted, true);
  assert.equal(result.airSummoning.summonedAtEnemySite, true);
  assert.equal(result.airSummoning.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airSummoning.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.airSummoning.deck, result.airMovement.deck);
  assert.equal(result.airSummoning.replayVerified, true);
  assert.equal(result.airVoidwalk.voidwalkMinion, 'Spectral Stalker');
  assert.equal(result.airVoidwalk.forsaken, 'Forsaken');
  assert.equal(result.airVoidwalk.acceptedActionCount, 17);
  assert.equal(result.airVoidwalk.forsakenOuterVoidAvailable, true);
  assert.equal(result.airVoidwalk.forsakenInnerVoidUnavailable, true);
  assert.equal(result.airVoidwalk.forsakenInnerSurfaceUnavailable, true);
  assert.equal(result.airVoidwalk.targetWasVoid, true);
  assert.equal(result.airVoidwalk.surfaceSummonAvailable, true);
  assert.equal(result.airVoidwalk.voidSummonAvailable, true);
  assert.equal(result.airVoidwalk.nonVoidSurfaceAvailable, true);
  assert.equal(result.airVoidwalk.nonVoidVoidUnavailable, true);
  assert.equal(result.airVoidwalk.summonedInVoid, true);
  assert.equal(result.airVoidwalk.voidMoveAvailable, true);
  assert.equal(result.airVoidwalk.surfaceExitAvailable, true);
  assert.equal(result.airVoidwalk.subsurfaceExitUnavailable, true);
  assert.equal(result.airVoidwalk.siteTargetAvailableAfterExit, true);
  assert.equal(result.airVoidwalk.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airVoidwalk.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airVoidwalk.deck.spellbook
    .find(({ name }) => name === 'Spectral Stalker')?.copies, 4);
  assert.equal(result.airVoidwalk.deck.spellbook
    .find(({ name }) => name === 'Forsaken')?.copies, 4);
  assert.equal(result.airVoidwalk.replayVerified, true);
  assert.equal(result.airZap.zap, 'Zap!');
  assert.equal(result.airZap.snowLeopard, 'Snow Leopard');
  assert.equal(result.airZap.acceptedActionCount, 10);
  assert.equal(result.airZap.damageDealt, 1);
  assert.equal(result.airZap.manaPaid, 1);
  assert.equal(result.airZap.snowLeopardSurvived, true);
  assert.equal(result.airZap.spellLeftHand, true);
  assert.equal(result.airZap.spellEnteredCemetery, true);
  assert.equal(result.airZap.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airZap.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airZap.deck.spellbook
    .find(({ name }) => name === 'Zap!')?.copies, 4);
  assert.equal(result.airZap.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airZap.replayVerified, true);
  assert.equal(result.airArcLightning.arcLightning, 'Arc Lightning');
  assert.equal(result.airArcLightning.snowLeopard, 'Snow Leopard');
  assert.equal(result.airArcLightning.acceptedActionCount, 26);
  assert.equal(result.airArcLightning.nearbyTargetAvailable, true);
  assert.equal(result.airArcLightning.farSameRegionUnitUnavailable, true);
  assert.equal(result.airArcLightning.manaPaid, 4);
  assert.equal(result.airArcLightning.snowLeopardDied, true);
  assert.equal(result.airArcLightning.spellEnteredCemetery, true);
  assert.equal(result.airArcLightning.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airArcLightning.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airArcLightning.deck.spellbook
    .find(({ name }) => name === 'Arc Lightning')?.copies, 4);
  assert.equal(result.airArcLightning.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airArcLightning.replayVerified, true);
  assert.equal(result.airLightningBolt.lightningBolt, 'Lightning Bolt');
  assert.equal(result.airLightningBolt.snowLeopard, 'Snow Leopard');
  assert.equal(result.airLightningBolt.acceptedActionCount, 11);
  assert.equal(result.airLightningBolt.occupiedLocationTargeted, true);
  assert.equal(result.airLightningBolt.randomSelectionRecorded, true);
  assert.equal(result.airLightningBolt.manaPaid, 2);
  assert.equal(result.airLightningBolt.lethalDamageRecorded, true);
  assert.equal(result.airLightningBolt.snowLeopardDied, true);
  assert.equal(result.airLightningBolt.avatarUnchanged, true);
  assert.equal(result.airLightningBolt.spellEnteredCemetery, true);
  assert.equal(result.airLightningBolt.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airLightningBolt.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airLightningBolt.deck.spellbook
    .find(({ name }) => name === 'Lightning Bolt')?.copies, 4);
  assert.equal(result.airLightningBolt.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airLightningBolt.replayVerified, true);
  assert.equal(result.airBladderblimp.bladderblimp, 'Bladderblimp');
  assert.equal(result.airBladderblimp.lightningBolt, 'Lightning Bolt');
  assert.equal(result.airBladderblimp.acceptedActionCount, 26);
  assert.equal(result.airBladderblimp.airborneAtC3, true);
  assert.equal(result.airBladderblimp.exactNearbySiteCounts, true);
  assert.equal(result.airBladderblimp.summonManaPaid, 5);
  assert.equal(result.airBladderblimp.magicManaPaid, 2);
  assert.equal(result.airBladderblimp.causalEventsVerified, true);
  assert.equal(result.airBladderblimp.lifeLossOnly, true);
  assert.equal(result.airBladderblimp.randomSelectionRecorded, true);
  assert.equal(result.airBladderblimp.minionAndMagicEnteredCemetery, true);
  assert.equal(result.airBladderblimp.gameRemainedActive, true);
  assert.equal(result.airBladderblimp.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airBladderblimp.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airBladderblimp.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.airBladderblimp.deck.spellbook
    .find(({ name }) => name === 'Bladderblimp')?.copies, 3);
  assert.equal(result.airBladderblimp.deck.spellbook
    .find(({ name }) => name === 'Lightning Bolt')?.copies, 4);
  assert.equal(result.airBladderblimp.replayVerified, true);
  assert.equal(result.airRainOfArrows.rainOfArrows, 'Rain of Arrows');
  assert.equal(result.airRainOfArrows.shellycoat, 'Shellycoat');
  assert.equal(result.airRainOfArrows.snowLeopard, 'Snow Leopard');
  assert.equal(result.airRainOfArrows.seed, 389);
  assert.equal(result.airRainOfArrows.acceptedActionCount, 16);
  assert.equal(result.airRainOfArrows.noTargetChoice, true);
  assert.equal(result.airRainOfArrows.surfaceMinionsComparedAndSurvived, true);
  assert.equal(result.airRainOfArrows.damageReductionVerified, true);
  assert.equal(result.airRainOfArrows.manaPaid, 2);
  assert.equal(result.airRainOfArrows.spellEnteredCemetery, true);
  assert.equal(result.airRainOfArrows.causalEventsVerified, true);
  assert.equal(result.airRainOfArrows.noRandomDraws, true);
  assert.equal(result.airRainOfArrows.avatarsPreserved, true);
  assert.equal(result.airRainOfArrows.sitesPreserved, true);
  assert.equal(result.airRainOfArrows.cemeteriesOtherwisePreserved, true);
  assert.equal(result.airRainOfArrows.gameRemainedActive, true);
  assert.equal(result.airRainOfArrows.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airRainOfArrows.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airRainOfArrows.deck.atlas
    .find(({ name }) => name === 'Spire')?.copies, 4);
  assert.equal(result.airRainOfArrows.deck.atlas
    .find(({ name }) => name === 'Stream')?.copies, 4);
  assert.equal(result.airRainOfArrows.deck.spellbook
    .find(({ name }) => name === 'Shellycoat')?.copies, 4);
  assert.equal(result.airRainOfArrows.deck.spellbook
    .find(({ name }) => name === 'Rain of Arrows')?.copies, 4);
  assert.equal(result.airRainOfArrows.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airRainOfArrows.replayVerified, true);
  assert.equal(result.airStaticServant.staticServant, 'Static Servant');
  assert.equal(result.airStaticServant.snowLeopard, 'Snow Leopard');
  assert.equal(result.airStaticServant.acceptedActionCount, 11);
  assert.equal(result.airStaticServant.avatarAndLeopardDamaged, true);
  assert.equal(result.airStaticServant.staticServantExcludedAndUndamaged, true);
  assert.equal(result.airStaticServant.manaPaid, 2);
  assert.equal(result.airStaticServant.causalEventsVerified, true);
  assert.equal(result.airStaticServant.noTargetChoiceOrRandomness, true);
  assert.equal(result.airStaticServant.cemeteriesUnchanged, true);
  assert.equal(result.airStaticServant.otherStatePreserved, true);
  assert.equal(result.airStaticServant.gameRemainedActive, true);
  assert.equal(result.airStaticServant.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airStaticServant.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airStaticServant.deck.spellbook
    .find(({ name }) => name === 'Static Servant')?.copies, 4);
  assert.equal(result.airStaticServant.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airStaticServant.replayVerified, true);
  assert.equal(result.airTeleport.teleport, 'Teleport');
  assert.equal(result.airTeleport.snowLeopard, 'Snow Leopard');
  assert.equal(result.airTeleport.acceptedActionCount, 11);
  assert.equal(result.airTeleport.exactAllySitePair, true);
  assert.equal(result.airTeleport.manaPaid, 2);
  assert.equal(result.airTeleport.noPathTeleport, true);
  assert.equal(result.airTeleport.teleportedToOpponentSiteSurface, true);
  assert.equal(result.airTeleport.unitStatePreserved, true);
  assert.equal(result.airTeleport.siteUnchanged, true);
  assert.equal(result.airTeleport.spellEnteredCemetery, true);
  assert.equal(result.airTeleport.causalEventsVerified, true);
  assert.equal(result.airTeleport.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airTeleport.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airTeleport.deck.spellbook
    .find(({ name }) => name === 'Teleport')?.copies, 4);
  assert.equal(result.airTeleport.deck.spellbook
    .find(({ name }) => name === 'Snow Leopard')?.copies, 4);
  assert.equal(result.airTeleport.replayVerified, true);
  assert.equal(result.airLeyline.henge, 'Leyline Henge');
  assert.equal(result.airLeyline.acceptedActionCount, 9);
  assert.equal(result.airLeyline.firstHengeDrewNothing, true);
  assert.equal(result.airLeyline.genesisDrewOne, true);
  assert.equal(result.airLeyline.hiddenFromOpponent, true);
  assert.equal(result.airLeyline.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.airLeyline.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.airLeyline.deck.atlas
    .find(({ name }) => name === 'Leyline Henge')?.copies, 4);
  assert.equal(result.airLeyline.replayVerified, true);
  assert.equal(result.earthOverpower.overpower, 'Overpower');
  assert.equal(result.earthOverpower.elthamTownsfolk, 'Eltham Townsfolk');
  assert.equal(result.earthOverpower.acceptedActionCount, 12);
  assert.equal(result.earthOverpower.exactOwnAllyChoices, true);
  assert.equal(result.earthOverpower.currentPowerIncreasedByTwo, true);
  assert.equal(result.earthOverpower.unitStatePreservedOnGrant, true);
  assert.equal(result.earthOverpower.manaPaid, 1);
  assert.equal(result.earthOverpower.spellEnteredCemetery, true);
  assert.equal(result.earthOverpower.causalEventsVerified, true);
  assert.equal(result.earthOverpower.expiredBeforeTurnEnded, true);
  assert.equal(result.earthOverpower.printedPowerRestored, true);
  assert.equal(result.earthOverpower.noRandomDraws, true);
  assert.equal(result.earthOverpower.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthOverpower.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthOverpower.deck.spellbook
    .find(({ name }) => name === 'Overpower')?.copies, 4);
  assert.equal(result.earthOverpower.deck.spellbook
    .find(({ name }) => name === 'Eltham Townsfolk')?.copies, 4);
  assert.equal(result.earthOverpower.replayVerified, true);
  assertSwordAndShield(result.earthSwordAndShield);
  assertPoisonousDagger(result.earthPoisonousDagger);
  assertHuntersLodge(result.earthHuntersLodge);
  assert.equal(result.waterEdgeConnection.polarBears, 'Polar Bears');
  assert.equal(result.waterEdgeConnection.acceptedActionCount, 16);
  assert.equal(result.waterEdgeConnection.wrapMoveAvailable, true);
  assert.equal(result.waterEdgeConnection.avatarWrapUnavailable, true);
  assert.equal(result.waterEdgeConnection.siteTargetAvailable, true);
  assert.equal(result.waterEdgeConnection.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterEdgeConnection.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterEdgeConnection.deck.spellbook
    .find(({ name }) => name === 'Polar Bears')?.copies, 4);
  assert.equal(result.waterEdgeConnection.replayVerified, true);
  assert.equal(result.avatarSpellDrawn, true);
  assert.equal(result.charge.activatedOnSummon, true);
  assert.equal(result.genesis.siteDrawn, true);
  assert.equal(result.lethal.tougherMinionKilled, true);
  assert.equal(result.provider.affinityAdded, true);
  assert.equal(result.decks.north.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.decks.south.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.decks.north.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.decks.south.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthBury.bury, 'Bury');
  assert.equal(result.earthBury.boskTroll, 'Bosk Troll');
  assert.equal(result.earthBury.acceptedActionCount, 17);
  assert.equal(result.earthBury.exactlyOneBuryTarget, true);
  assert.equal(result.earthBury.manaPaid, 3);
  assert.equal(result.earthBury.buriedBeforeDeath, true);
  assert.equal(result.earthBury.causalEventsVerified, true);
  assert.equal(result.earthBury.deathNotBanishmentAndGameActive, true);
  assert.equal(result.earthBury.targetLeftRealm, true);
  assert.equal(result.earthBury.targetEnteredCemetery, true);
  assert.equal(result.earthBury.spellEnteredCemetery, true);
  assert.equal(result.earthBury.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthBury.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthBury.deck.spellbook
    .find(({ name }) => name === 'Bury')?.copies, 4);
  assert.equal(result.earthBury.deck.spellbook
    .find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.earthBury.replayVerified, true);
  assert.equal(result.earthQuagmire.quagmire, 'Quagmire');
  assert.equal(result.earthQuagmire.wildBoars, 'Wild Boars');
  assert.equal(result.earthQuagmire.acceptedActionCount, 22);
  assert.equal(result.earthQuagmire.causalEventsVerified, true);
  assert.equal(result.earthQuagmire.unitImmobileThroughOpponentTurn, true);
  assert.equal(result.earthQuagmire.movementRestoredAfterExpiry, true);
  assert.equal(result.earthQuagmire.legalConstructedDeck, true);
  assert.equal(result.earthQuagmire.noRandomDraws, true);
  assert.equal(result.earthQuagmire.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthQuagmire.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthQuagmire.deck.atlas
    .find(({ name }) => name === 'Quagmire')?.copies, 2);
  assert.equal(result.earthQuagmire.deck.spellbook
    .find(({ name }) => name === 'Wild Boars')?.copies, 4);
  assert.equal(result.earthQuagmire.replayVerified, true);
  assert.equal(result.earthEntangleTerrain.entangleTerrain, 'Entangle Terrain');
  assert.equal(result.earthEntangleTerrain.malakhim, 'Malakhim');
  assert.equal(result.earthEntangleTerrain.caveTrolls, 'Cave Trolls');
  assert.equal(result.earthEntangleTerrain.seed, 2);
  assert.equal(result.earthEntangleTerrain.acceptedActionCount, 49);
  assert.equal(result.earthEntangleTerrain.exactCast, true);
  assert.equal(result.earthEntangleTerrain.canonicalCastVerified, true);
  assert.equal(result.earthEntangleTerrain.auraIdentityVerified, true);
  assert.equal(result.earthEntangleTerrain.surfaceAndSubsurfaceMinionsAffected, true);
  assert.equal(result.earthEntangleTerrain.airborneRestoredAfterDispel, true);
  assert.equal(result.earthEntangleTerrain.countersVerified, true);
  assert.equal(result.earthEntangleTerrain.dispelledToOwnerCemetery, true);
  assert.equal(result.earthEntangleTerrain.causalEventsVerified, true);
  assert.equal(result.earthEntangleTerrain.legalConstructedDeck, true);
  assert.equal(result.earthEntangleTerrain.noRandomDraws, true);
  assert.equal(result.earthEntangleTerrain.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthEntangleTerrain.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthEntangleTerrain.deck.spellbook
    .find(({ name }) => name === 'Entangle Terrain')?.copies, 1);
  assert.equal(result.earthEntangleTerrain.deck.spellbook
    .find(({ name }) => name === 'Cave Trolls')?.copies, 4);
  assert.equal(result.earthEntangleTerrain.replayVerified, true);
  assert.equal(result.earthMountainGiant.mountainGiant, 'Mountain Giant');
  assert.equal(result.earthMountainGiant.wildBoars, 'Wild Boars');
  assert.equal(result.earthMountainGiant.seed, 25);
  assert.equal(result.earthMountainGiant.acceptedActionCount, 54);
  assert.equal(result.earthMountainGiant.canonicalSummonVerified, true);
  assert.equal(result.earthMountainGiant.initialFootprintVerified, true);
  assert.equal(result.earthMountainGiant.movedFootprintVerified, true);
  assert.equal(result.earthMountainGiant.footprintInteractionVerified, true);
  assert.equal(result.earthMountainGiant.causalEventsVerified, true);
  assert.equal(result.earthMountainGiant.legalConstructedDeck, true);
  assert.equal(result.earthMountainGiant.noRandomDraws, true);
  assert.equal(result.earthMountainGiant.unsupportedMechanicsAbsent, true);
  assert.equal(result.earthMountainGiant.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthMountainGiant.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthMountainGiant.deck.spellbook
    .find(({ name }) => name === 'Mountain Giant')?.copies, 1);
  assert.equal(result.earthMountainGiant.deck.spellbook
    .find(({ name }) => name === 'Wild Boars')?.copies, 4);
  assert.equal(result.earthMountainGiant.replayVerified, true);
  assert.equal(result.earthHolyGround.holyGround, 'Holy Ground');
  assert.equal(result.earthHolyGround.lesserBloodDemon, 'Lesser Blood Demon');
  assert.equal(result.earthHolyGround.seed, 7398);
  assert.equal(result.earthHolyGround.acceptedActionCount, 16);
  assert.equal(result.earthHolyGround.lifeWasReducedByFour, true);
  assert.equal(result.earthHolyGround.healed, 3);
  assert.equal(result.earthHolyGround.farAvatarUnchanged, true);
  assert.equal(result.earthHolyGround.causalEventsVerified, true);
  assert.equal(result.earthHolyGround.siteEstablished, true);
  assert.equal(result.earthHolyGround.legalConstructedDeck, true);
  assert.equal(result.earthHolyGround.noRandomDraws, true);
  assert.equal(result.earthHolyGround.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthHolyGround.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthHolyGround.deck.atlas
    .find(({ name }) => name === 'Holy Ground')?.copies, 1);
  assert.equal(result.earthHolyGround.deck.spellbook
    .find(({ name }) => name === 'Lesser Blood Demon')?.copies, 4);
  assert.equal(result.earthHolyGround.replayVerified, true);
  assert.equal(result.earthBedrock.bedrock, 'Bedrock');
  assert.equal(result.earthBedrock.sinkhole, 'Sinkhole');
  assert.equal(result.earthBedrock.granaryRats, 'Granary Rats');
  assert.equal(result.earthBedrock.seed, 7398);
  assert.equal(result.earthBedrock.acceptedActionCount, 16);
  assert.equal(result.earthBedrock.exactActivationAvailable, true);
  assert.equal(result.earthBedrock.sourceCostResolved, true);
  assert.equal(result.earthBedrock.bedrockStayedInRealm, true);
  assert.equal(result.earthBedrock.noFalseDestruction, true);
  assert.equal(result.earthBedrock.thresholdUnsuppressed, true);
  assert.equal(result.earthBedrock.causalEventsVerified, true);
  assert.equal(result.earthBedrock.legalConstructedDeck, true);
  assert.equal(result.earthBedrock.noRandomDraws, true);
  assert.equal(result.earthBedrock.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthBedrock.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthBedrock.deck.atlas
    .find(({ name }) => name === 'Bedrock')?.copies, 1);
  assert.equal(result.earthBedrock.deck.atlas
    .find(({ name }) => name === 'Sinkhole')?.copies, 2);
  assert.equal(result.earthBedrock.deck.spellbook
    .find(({ name }) => name === 'Granary Rats')?.copies, 4);
  assert.equal(result.earthBedrock.replayVerified, true);
  assert.equal(result.earthWraetannisTitan.wraetannisTitan, 'Wraetannis Titan');
  assert.equal(result.earthWraetannisTitan.houseArnBannerman, 'House Arn Bannerman');
  assert.equal(result.earthWraetannisTitan.wildBoars, 'Wild Boars');
  assert.equal(result.earthWraetannisTitan.seed, 9036);
  assert.equal(result.earthWraetannisTitan.acceptedActionCount, 68);
  assert.equal(result.earthWraetannisTitan.derivedPower, 7);
  assert.equal(result.earthWraetannisTitan.enemiesStruckSimultaneously, true);
  assert.equal(result.earthWraetannisTitan.allyAndSourceExcluded, true);
  assert.equal(result.earthWraetannisTitan.enemyDeathsVerified, true);
  assert.equal(result.earthWraetannisTitan.causalEventsVerified, true);
  assert.equal(result.earthWraetannisTitan.legalConstructedDeck, true);
  assert.equal(result.earthWraetannisTitan.noRandomDraws, true);
  assert.equal(result.earthWraetannisTitan.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthWraetannisTitan.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthWraetannisTitan.deck.spellbook
    .find(({ name }) => name === 'Wraetannis Titan')?.copies, 1);
  assert.equal(result.earthWraetannisTitan.deck.spellbook
    .find(({ name }) => name === 'House Arn Bannerman')?.copies, 3);
  assert.equal(result.earthWraetannisTitan.replayVerified, true);
  assert.equal(result.earthKingOfRealm.kingOfRealm, 'King of the Realm');
  assert.equal(result.earthKingOfRealm.landSurveyor, 'Land Surveyor');
  assert.equal(result.earthKingOfRealm.scentHounds, 'Scent Hounds');
  assert.equal(result.earthKingOfRealm.seed, 2);
  assert.equal(result.earthKingOfRealm.acceptedActionCount, 37);
  assert.equal(result.earthKingOfRealm.exactCast, true);
  assert.equal(result.earthKingOfRealm.mortalBoosted, true);
  assert.equal(result.earthKingOfRealm.kingSelfExcluded, true);
  assert.equal(result.earthKingOfRealm.nonMortalUnaffected, true);
  assert.equal(result.earthKingOfRealm.causalEventsVerified, true);
  assert.equal(result.earthKingOfRealm.legalConstructedDeck, true);
  assert.equal(result.earthKingOfRealm.noRandomDraws, true);
  assert.equal(result.earthKingOfRealm.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthKingOfRealm.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthKingOfRealm.deck.spellbook
    .find(({ name }) => name === 'King of the Realm')?.copies, 1);
  assert.equal(result.earthKingOfRealm.deck.spellbook
    .find(({ name }) => name === 'Land Surveyor')?.copies, 4);
  assert.equal(result.earthKingOfRealm.deck.spellbook
    .find(({ name }) => name === 'Scent Hounds')?.copies, 4);
  assert.equal(result.earthKingOfRealm.replayVerified, true);
  assert.equal(result.earthSlumberingGiantess.slumberingGiantess, 'Slumbering Giantess');
  assert.equal(result.earthSlumberingGiantess.albespinePikemen, 'Albespine Pikemen');
  assert.equal(result.earthSlumberingGiantess.seed, 8883);
  assert.equal(result.earthSlumberingGiantess.acceptedActionCount, 27);
  assert.equal(result.earthSlumberingGiantess.disabledOnSummon, true);
  assert.equal(result.earthSlumberingGiantess.firstStrikeDamage, 3);
  assert.equal(result.earthSlumberingGiantess.giantessReturnedStrike, true);
  assert.equal(result.earthSlumberingGiantess.giantessSurvivedAwake, true);
  assert.equal(result.earthSlumberingGiantess.causalEventsVerified, true);
  assert.equal(result.earthSlumberingGiantess.legalConstructedDeck, true);
  assert.equal(result.earthSlumberingGiantess.noRandomDraws, true);
  assert.equal(result.earthSlumberingGiantess.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthSlumberingGiantess.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthSlumberingGiantess.deck.spellbook
    .find(({ name }) => name === 'Slumbering Giantess')?.copies, 1);
  assert.equal(result.earthSlumberingGiantess.deck.spellbook
    .find(({ name }) => name === 'Albespine Pikemen')?.copies, 3);
  assert.equal(result.earthSlumberingGiantess.replayVerified, true);
  assert.equal(result.earthCaveIn.caveIn, 'Cave-In');
  assert.equal(result.earthCaveIn.caveTrolls, 'Cave Trolls');
  assert.equal(result.earthCaveIn.boskTroll, 'Bosk Troll');
  assert.equal(result.earthCaveIn.scentHounds, 'Scent Hounds');
  assert.equal(result.earthCaveIn.swordAndShield, 'Sword and Shield');
  assert.equal(result.earthCaveIn.seed, 9852);
  assert.equal(result.earthCaveIn.acceptedActionCount, 67);
  assert.equal(result.earthCaveIn.exactTargetAndCost, true);
  assert.equal(result.earthCaveIn.simultaneousBurrowOrderVerified, true);
  assert.equal(result.earthCaveIn.causalEventsVerified, true);
  assert.equal(result.earthCaveIn.survivorAndArtifactUndergroundCarried, true);
  assert.equal(result.earthCaveIn.nonBurrowerDied, true);
  assert.equal(result.earthCaveIn.controlUntouched, true);
  assert.equal(result.earthCaveIn.legalConstructedDeck, true);
  assert.equal(result.earthCaveIn.noRandomDraws, true);
  assert.equal(result.earthCaveIn.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthCaveIn.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthCaveIn.deck.spellbook
    .find(({ name }) => name === 'Cave-In')?.copies, 1);
  assert.equal(result.earthCaveIn.replayVerified, true);
  assert.equal(result.earthSiegeBallista.siegeBallista, 'Siege Ballista');
  assert.equal(result.earthSiegeBallista.scentHounds, 'Scent Hounds');
  assert.equal(result.earthSiegeBallista.snowLeopard, 'Snow Leopard');
  assert.equal(result.earthSiegeBallista.seed, 3828);
  assert.equal(result.earthSiegeBallista.acceptedActionCount, 28);
  assert.equal(result.earthSiegeBallista.twoStepRangeVerified, true);
  assert.equal(result.earthSiegeBallista.exactCastAndActivation, true);
  assert.equal(result.earthSiegeBallista.fixedDamageKilledTarget, true);
  assert.equal(result.earthSiegeBallista.noReturnStrike, true);
  assert.equal(result.earthSiegeBallista.ballistaRemainedCarried, true);
  assert.equal(result.earthSiegeBallista.causalEventsVerified, true);
  assert.equal(result.earthSiegeBallista.legalConstructedDeck, true);
  assert.equal(result.earthSiegeBallista.noRandomDraws, true);
  assert.equal(result.earthSiegeBallista.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthSiegeBallista.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthSiegeBallista.deck.spellbook
    .find(({ name }) => name === 'Siege Ballista')?.copies, 1);
  assert.equal(result.earthSiegeBallista.replayVerified, true);
  assert.equal(result.earthPayloadTrebuchet.payloadTrebuchet, 'Payload Trebuchet');
  assert.equal(result.earthPayloadTrebuchet.scentHounds, 'Scent Hounds');
  assert.equal(result.earthPayloadTrebuchet.caveTrolls, 'Cave Trolls');
  assert.equal(result.earthPayloadTrebuchet.seed, 188);
  assert.equal(result.earthPayloadTrebuchet.acceptedActionCount, 45);
  assert.equal(result.earthPayloadTrebuchet.threeStepRangeVerified, true);
  assert.equal(result.earthPayloadTrebuchet.exactCastAndActivation, true);
  assert.equal(result.earthPayloadTrebuchet.bothCostsTapped, true);
  assert.equal(result.earthPayloadTrebuchet.discardedToCemetery, true);
  assert.equal(result.earthPayloadTrebuchet.targetsKilledByArtifact, true);
  assert.equal(result.earthPayloadTrebuchet.noStrikeOrLethal, true);
  assert.equal(result.earthPayloadTrebuchet.trebuchetRemainedCarried, true);
  assert.equal(result.earthPayloadTrebuchet.causalEventsVerified, true);
  assert.equal(result.earthPayloadTrebuchet.legalConstructedDeck, true);
  assert.equal(result.earthPayloadTrebuchet.noRandomDraws, true);
  assert.equal(result.earthPayloadTrebuchet.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthPayloadTrebuchet.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthPayloadTrebuchet.deck.spellbook
    .find(({ name }) => name === 'Payload Trebuchet')?.copies, 1);
  assert.equal(result.earthPayloadTrebuchet.replayVerified, true);
  assert.equal(result.earthRollingBoulder.rollingBoulder, 'Rolling Boulder');
  assert.equal(result.earthRollingBoulder.scentHounds, 'Scent Hounds');
  assert.equal(result.earthRollingBoulder.wildBoars, 'Wild Boars');
  assert.equal(result.earthRollingBoulder.seed, 378);
  assert.equal(result.earthRollingBoulder.acceptedActionCount, 28);
  assert.equal(result.earthRollingBoulder.exactCastAndRoll, true);
  assert.equal(result.earthRollingBoulder.pusherExcludedAndTapped, true);
  assert.equal(result.earthRollingBoulder.originAndPathTargetsDamaged, true);
  assert.equal(result.earthRollingBoulder.targetDeathsVerified, true);
  assert.equal(result.earthRollingBoulder.boulderLooseAtDestination, true);
  assert.equal(result.earthRollingBoulder.causalEventsVerified, true);
  assert.equal(result.earthRollingBoulder.noStrikeOrLethal, true);
  assert.equal(result.earthRollingBoulder.legalConstructedDeck, true);
  assert.equal(result.earthRollingBoulder.noRandomDraws, true);
  assert.equal(result.earthRollingBoulder.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthRollingBoulder.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthRollingBoulder.deck.spellbook
    .find(({ name }) => name === 'Rolling Boulder')?.copies, 1);
  assert.equal(result.earthRollingBoulder.replayVerified, true);
  assert.equal(result.earthBorderMilitia.borderMilitia, 'Border Militia');
  assert.equal(result.earthBorderMilitia.footSoldier, 'Foot Soldier');
  assert.equal(result.earthBorderMilitia.seed, 7688);
  assert.equal(result.earthBorderMilitia.acceptedActionCount, 21);
  assert.equal(result.earthBorderMilitia.manaPaid, 3);
  assert.equal(result.earthBorderMilitia.tokensVerified, true);
  assert.equal(result.earthBorderMilitia.spellEnteredCemetery, true);
  assert.equal(result.earthBorderMilitia.noRandomDraws, true);
  assert.equal(result.earthBorderMilitia.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthBorderMilitia.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthBorderMilitia.deck.spellbook
    .find(({ name }) => name === 'Border Militia')?.copies, 4);
  assert.equal(result.earthBorderMilitia.deck.spellbook
    .some(({ name }) => name === 'Foot Soldier'), false);
  assert.equal(result.earthBorderMilitia.replayVerified, true);
  assert.equal(result.earthHumbleVillage.humbleVillage, 'Humble Village');
  assert.equal(result.earthHumbleVillage.footSoldier, 'Foot Soldier');
  assert.equal(result.earthHumbleVillage.seed, 7383);
  assert.equal(result.earthHumbleVillage.acceptedActionCount, 3);
  assert.equal(result.earthHumbleVillage.counterfactualRootCoverage, true);
  assert.equal(result.earthHumbleVillage.exactChoices, true);
  assert.equal(result.earthHumbleVillage.declinedKeptManaAndSummonedNothing, true);
  assert.equal(result.earthHumbleVillage.paidSpentManaAndSummonedToken, true);
  assert.equal(result.earthHumbleVillage.tokenDefinitionVerified, true);
  assert.equal(result.earthHumbleVillage.gameRemainedActive, true);
  assert.equal(result.earthHumbleVillage.noRandomDraws, true);
  assert.equal(result.earthHumbleVillage.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthHumbleVillage.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthHumbleVillage.deck.atlas
    .find(({ name }) => name === 'Humble Village')?.copies, 4);
  assert.equal(result.earthHumbleVillage.deck.spellbook
    .some(({ name }) => name === 'Foot Soldier'), false);
  assert.equal(result.earthHumbleVillage.replayVerified, true);
  assert.equal(result.earthDuel.duel, 'Duel');
  assert.equal(result.earthDuel.boskTroll, 'Bosk Troll');
  assert.equal(result.earthDuel.elthamTownsfolk, 'Eltham Townsfolk');
  assert.equal(result.earthDuel.acceptedActionCount, 18);
  assert.equal(result.earthDuel.exactFightPair, true);
  assert.equal(result.earthDuel.manaPaid, 3);
  assert.equal(result.earthDuel.allySurvivedWithTwoDamage, true);
  assert.equal(result.earthDuel.targetDiedAndEnteredCemetery, true);
  assert.equal(result.earthDuel.unitsDidNotMoveOrTap, true);
  assert.equal(result.earthDuel.spellEnteredCemetery, true);
  assert.equal(result.earthDuel.causalEventsVerified, true);
  assert.equal(result.earthDuel.noRandomDraws, true);
  assert.equal(result.earthDuel.sitesAndAvatarsPreserved, true);
  assert.equal(result.earthDuel.gameRemainedActive, true);
  assert.equal(result.earthDuel.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthDuel.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthDuel.deck.spellbook
    .find(({ name }) => name === 'Duel')?.copies, 4);
  assert.equal(result.earthDuel.deck.spellbook
    .find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.earthDuel.deck.spellbook
    .find(({ name }) => name === 'Eltham Townsfolk')?.copies, 4);
  assert.equal(result.earthDuel.replayVerified, true);
  assert.equal(result.earthRescue.rescue, 'Rescue');
  assert.equal(result.earthRescue.boskTroll, 'Bosk Troll');
  assert.equal(result.earthRescue.acceptedActionCount, 21);
  assert.equal(result.earthRescue.onlyOwnCemeteryMinionChoice, true);
  assert.equal(result.earthRescue.manaPaid, 3);
  assert.equal(result.earthRescue.returnedToSouthHand, true);
  assert.equal(result.earthRescue.hiddenFromNorthAfterReturn, true);
  assert.equal(result.earthRescue.rescueEnteredSouthCemetery, true);
  assert.equal(result.earthRescue.buryStayedNorthCemetery, true);
  assert.equal(result.earthRescue.causalEventsVerified, true);
  assert.equal(result.earthRescue.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthRescue.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthRescue.deck.spellbook
    .find(({ name }) => name === 'Rescue')?.copies, 4);
  assert.equal(result.earthRescue.deck.spellbook
    .find(({ name }) => name === 'Bury')?.copies, 4);
  assert.equal(result.earthRescue.deck.spellbook
    .find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.earthRescue.replayVerified, true);
  assert.equal(result.earthShallowGrave.shallowGrave, 'Shallow Grave');
  assert.equal(result.earthShallowGrave.acceptedActionCount, 3);
  assert.equal(result.earthShallowGrave.hiddenBeforeDiscard, true);
  assert.equal(result.earthShallowGrave.publicAfterDiscard, true);
  assert.equal(result.earthShallowGrave.discardedInDeckOrder, true);
  assert.equal(result.earthShallowGrave.spellHandUnchanged, true);
  assert.equal(result.earthShallowGrave.spellbookReducedByTwo, true);
  assert.equal(result.earthShallowGrave.siteEstablished, true);
  assert.equal(result.earthShallowGrave.avatarTapped, true);
  assert.equal(result.earthShallowGrave.manaProvided, true);
  assert.equal(result.earthShallowGrave.affinityProvided, true);
  assert.equal(result.earthShallowGrave.causalEventsVerified, true);
  assert.equal(result.earthShallowGrave.gameRemainedActive, true);
  assert.equal(result.earthShallowGrave.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthShallowGrave.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthShallowGrave.deck.atlas
    .find(({ name }) => name === 'Shallow Grave')?.copies, 3);
  assert.equal(result.earthShallowGrave.replayVerified, true);
  assert.equal(result.earthSinkhole.sinkhole, 'Sinkhole');
  assert.equal(result.earthSinkhole.valley, 'Valley');
  assert.equal(result.earthSinkhole.seed, 7545);
  assert.equal(result.earthSinkhole.acceptedActionCount, 15);
  assert.equal(result.earthSinkhole.destructionAcceptedActionCount, 10);
  assert.equal(result.earthSinkhole.exactActivationAvailable, true);
  assert.equal(result.earthSinkhole.sourceAndTargetEnteredCemetery, true);
  assert.equal(result.earthSinkhole.twoNeutralRubbleSites, true);
  assert.equal(result.earthSinkhole.noAffinityOrControlContribution, true);
  assert.equal(result.earthSinkhole.avatarRemainedOnSurface, true);
  assert.equal(result.earthSinkhole.causalEventsVerified, true);
  assert.equal(result.earthSinkhole.recoveryVerified, true);
  assert.equal(result.earthSinkhole.noRandomDraws, true);
  assert.equal(result.earthSinkhole.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthSinkhole.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthSinkhole.deck.atlas
    .find(({ name }) => name === 'Sinkhole')?.copies, 2);
  assert.equal(result.earthSinkhole.deck.atlas
    .find(({ name }) => name === 'Valley')?.copies, 4);
  assert.equal(result.earthSinkhole.replayVerified, true);
  assert.equal(result.earthDivineHealing.divineHealing, 'Divine Healing');
  assert.equal(result.earthDivineHealing.acceptedActionCount, 24);
  assert.equal(result.earthDivineHealing.lifeWasDamagedAboveDeathsDoor, true);
  assert.equal(result.earthDivineHealing.exactlyOneTargetlessCast, true);
  assert.equal(result.earthDivineHealing.actualLifeGained, 3);
  assert.equal(result.earthDivineHealing.lifeCappedAtMaximum, true);
  assert.equal(result.earthDivineHealing.manaPaid, 1);
  assert.equal(result.earthDivineHealing.spellEnteredCemetery, true);
  assert.equal(result.earthDivineHealing.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthDivineHealing.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthDivineHealing.deck.spellbook
    .find(({ name }) => name === 'Divine Healing')?.copies, 3);
  assert.equal(result.earthDivineHealing.replayVerified, true);
  assert.equal(result.earthGrainSparrow.grainSparrow, 'Grain Sparrow');
  assert.equal(result.earthGrainSparrow.lesserBloodDemon, 'Lesser Blood Demon');
  assert.equal(result.earthGrainSparrow.steppe, 'Steppe');
  assert.equal(result.earthGrainSparrow.acceptedActionCount, 11);
  assert.equal(result.earthGrainSparrow.summonedAtC3, true);
  assert.equal(result.earthGrainSparrow.lifeLostBeforeSummon, true);
  assert.equal(result.earthGrainSparrow.actualLifeGained, 2);
  assert.equal(result.earthGrainSparrow.lifeCappedAtMaximum, true);
  assert.equal(result.earthGrainSparrow.causalEventsVerified, true);
  assert.equal(result.earthGrainSparrow.otherStatePreserved, true);
  assert.equal(result.earthGrainSparrow.noDamageDeathTerminalOrRandomEffects, true);
  assert.equal(result.earthGrainSparrow.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthGrainSparrow.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthGrainSparrow.deck.spellbook
    .find(({ name }) => name === 'Grain Sparrow')?.copies, 4);
  assert.equal(result.earthGrainSparrow.deck.spellbook
    .find(({ name }) => name === 'Lesser Blood Demon')?.copies, 4);
  assert.equal(result.earthGrainSparrow.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.earthGrainSparrow.deck.atlas
    .find(({ name }) => name === 'Steppe')?.copies, 3);
  assert.equal(result.earthGrainSparrow.replayVerified, true);
  assert.equal(result.earthBurrowing.burrowingMinion, 'Cave Trolls');
  assert.equal(result.earthBurrowing.acceptedActionCount, 28);
  assert.equal(result.earthBurrowing.targetIsLandSite, true);
  assert.equal(result.earthBurrowing.surfaceSummonAvailable, true);
  assert.equal(result.earthBurrowing.undergroundSummonAvailable, true);
  assert.equal(result.earthBurrowing.nonBurrowingSurfaceAvailable, true);
  assert.equal(result.earthBurrowing.nonBurrowingUndergroundUnavailable, true);
  assert.equal(result.earthBurrowing.movedUnderground, true);
  assert.equal(result.earthBurrowing.siteTargetUnavailableUnderground, true);
  assert.equal(result.earthBurrowing.surfaced, true);
  assert.equal(result.earthBurrowing.siteTargetAvailableAfterSurfacing, true);
  assert.equal(result.earthBurrowing.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthBurrowing.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthBurrowing.deck.spellbook
    .find(({ name }) => name === 'Cave Trolls')?.copies, 4);
  assert.equal(result.earthBurrowing.replayVerified, true);
  assert.equal(result.earthEntombed.entombed, 'Entombed');
  assert.equal(result.earthEntombed.boskTroll, 'Bosk Troll');
  assert.equal(result.earthEntombed.acceptedActionCount, 10);
  assert.equal(result.earthEntombed.entombedSurfaceUnavailable, true);
  assert.equal(result.earthEntombed.entombedUndergroundAvailable, true);
  assert.equal(result.earthEntombed.boskTrollSurfaceAvailable, true);
  assert.equal(result.earthEntombed.boskTrollUndergroundUnavailable, true);
  assert.equal(result.earthEntombed.summonedUnderground, true);
  assert.equal(result.earthEntombed.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthEntombed.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthEntombed.deck.spellbook
    .find(({ name }) => name === 'Entombed')?.copies, 4);
  assert.equal(result.earthEntombed.replayVerified, true);
  assert.equal(result.earthForwardMovement.phalanx, 'Dalcean Phalanx');
  assert.equal(result.earthForwardMovement.acceptedActionCount, 22);
  assert.equal(result.earthForwardMovement.forwardPathAvailable, true);
  assert.equal(result.earthForwardMovement.backwardPathUnavailable, true);
  assert.equal(result.earthForwardMovement.sidewaysPathUnavailable, true);
  assert.equal(result.earthForwardMovement.siteTargetAvailable, true);
  assert.equal(result.earthForwardMovement.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthForwardMovement.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthForwardMovement.deck.spellbook
    .find(({ name }) => name === 'Dalcean Phalanx')?.copies, 3);
  assert.equal(result.earthForwardMovement.replayVerified, true);
  assert.equal(result.earthImmobile.pudgeButcher, 'Pudge Butcher');
  assert.equal(result.earthImmobile.comparatorMinion, 'Bosk Troll');
  assert.equal(result.earthImmobile.acceptedActionCount, 30);
  assert.equal(result.earthImmobile.nearbySitePresent, true);
  assert.equal(result.earthImmobile.positiveStepMoveUnavailable, true);
  assert.equal(result.earthImmobile.sameLocationAttackAvailable, true);
  assert.equal(result.earthImmobile.localDefendAvailable, true);
  assert.equal(result.earthImmobile.dragChoicePairAvailable, true);
  assert.equal(result.earthImmobile.dragOnlyAcceptedActionCount, 22);
  assert.equal(result.earthImmobile.dragOnlyEventsVerified, true);
  assert.equal(result.earthImmobile.dragOnlyStateVerified, true);
  assert.equal(result.earthImmobile.dragOnlyReplayVerified, true);
  assert.equal(result.earthImmobile.fightAcceptedActionCount, 22);
  assert.equal(result.earthImmobile.fightEventsVerified, true);
  assert.equal(result.earthImmobile.fightStateVerified, true);
  assert.equal(result.earthImmobile.fightReplayVerified, true);
  assert.equal(result.earthImmobile.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthImmobile.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthImmobile.deck.spellbook
    .find(({ name }) => name === 'Pudge Butcher')?.copies, 3);
  assert.equal(result.earthImmobile.deck.spellbook
    .find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.equal(result.earthImmobile.replayVerified, true);
  assert.equal(result.earthSecretTunnel.secretTunnel, 'Secret Tunnel');
  assert.equal(result.earthSecretTunnel.caveTrolls, 'Cave Trolls');
  assert.equal(result.earthSecretTunnel.acceptedActionCount, 21);
  assert.equal(result.earthSecretTunnel.physicalMoveAvailable, true);
  assert.equal(result.earthSecretTunnel.directTunnelMoveAvailable, true);
  assert.equal(result.earthSecretTunnel.directOpponentUnavailable, true);
  assert.equal(result.earthSecretTunnel.avatarPhysicalAvailable, true);
  assert.equal(result.earthSecretTunnel.avatarDirectUnavailable, true);
  assert.equal(result.earthSecretTunnel.movedUnderground, true);
  assert.equal(result.earthSecretTunnel.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthSecretTunnel.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthSecretTunnel.deck.atlas
    .find(({ name }) => name === 'Secret Tunnel')?.copies, 3);
  assert.equal(result.earthSecretTunnel.deck.spellbook
    .find(({ name }) => name === 'Cave Trolls')?.copies, 4);
  assert.equal(result.earthSecretTunnel.replayVerified, true);
  assert.equal(result.earthRamp.affinityAdded, true);
  assert.equal(result.earthRamp.manaUnavailableWhileSick, true);
  assert.equal(result.earthRamp.manaGained, 2);
  assert.equal(result.earthRamp.rampPaidFive, true);
  assert.equal(result.earthRamp.payoffCanMoveAndAttack, true);
  assert.equal(result.earthRamp.movingDefendUnavailable, true);
  assert.equal(result.earthRamp.genesisSiteDrawn, true);
  assert.equal(result.earthRamp.ghostTown, 'Ghost Town');
  assert.equal(result.earthRamp.ghostTownBonusMana, 1);
  assert.equal(result.earthRamp.ghostTownUnusedManaExpired, true);
  assert.equal(result.earthRamp.deathriteSiteDrawnBeforeCemetery, true);
  assert.equal(result.earthRamp.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthRamp.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthRamp.replayVerified, true);
  assert.equal(result.earthFirstStrike.firstStrikeMinion, 'Albespine Pikemen');
  assert.equal(result.earthFirstStrike.targetMinion, 'Bosk Troll');
  assert.equal(result.earthFirstStrike.attackerSurvivedUndamaged, true);
  assert.equal(result.earthFirstStrike.targetDiedBeforeReturn, true);
  assert.equal(result.earthFirstStrike.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthFirstStrike.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.earthFirstStrike.deck.spellbook
    .find(({ name }) => name === 'Albespine Pikemen')?.copies, 3);
  assert.equal(result.earthFirstStrike.deck.spellbook
    .find(({ name }) => name === 'Bosk Troll')?.copies, 4);
  assert.deepEqual(result.earthFirstStrike.deck, result.earthRamp.deck);
  assert.equal(result.earthFirstStrike.replayVerified, true);
  assert.equal(result.earthRanged.rangedMinion, 'Belmotte Longbowmen');
  assert.equal(result.earthRanged.rangedOneStep, true);
  assert.equal(result.earthRanged.rangedShooterStayedSafe, true);
  assert.equal(result.earthRanged.rangedTargetDied, true);
  assert.equal(result.earthRanged.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthRanged.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.earthRanged.deck, result.earthRamp.deck);
  assert.equal(result.earthRanged.replayVerified, true);
  assert.equal(result.earthWard.rangedMinion, 'Belmotte Longbowmen');
  assert.equal(result.earthWard.wardMinion, 'Holy Warrior');
  assert.equal(result.earthWard.wardBroke, true);
  assert.equal(result.earthWard.wardPreventedDamage, true);
  assert.equal(result.earthWard.wardTargetSurvived, true);
  assert.equal(result.earthWard.wardTargetDiedAfterSecondShot, true);
  assert.equal(result.earthWard.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.earthWard.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.earthWard.deck, result.earthRamp.deck);
  assert.equal(result.earthWard.replayVerified, true);
  assertFireResponse(result.fireResponse);
  assertMinorExplosion(result.fireMinorExplosion);
  assertFireCharge(result.fireCharge);
  assert.equal(result.fireAramos.aramosMercenaries, 'Aramos Mercenaries');
  assert.equal(result.fireAramos.raalDromedary, 'Raal Dromedary');
  assert.equal(result.fireAramos.acceptedActionCount, 10);
  assert.equal(result.fireAramos.normalManaSummonUnavailable, true);
  assert.equal(result.fireAramos.paymentModeVerified, true);
  assert.equal(result.fireAramos.manaPaid, 0);
  assert.equal(result.fireAramos.discardedNonCastingCard, true);
  assert.equal(result.fireAramos.randomDiscardVerified, true);
  assert.equal(result.fireAramos.summonedAtC3, true);
  assert.equal(result.fireAramos.causalEventsVerified, true);
  assert.equal(result.fireAramos.hiddenInformationVerified, true);
  assert.equal(result.fireAramos.unrelatedStatePreserved, true);
  assert.equal(result.fireAramos.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireAramos.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireAramos.deck.spellbook
    .find(({ name }) => name === 'Aramos Mercenaries')?.copies, 4);
  assert.equal(result.fireAramos.deck.spellbook
    .find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.fireAramos.replayVerified, true);
  assert.equal(result.fireLash.lash, 'Lash');
  assert.equal(result.fireLash.raalDromedary, 'Raal Dromedary');
  assert.equal(result.fireLash.acceptedActionCount, 13);
  assert.equal(result.fireLash.exactNearbyTarget, true);
  assert.equal(result.fireLash.manaPaid, 3);
  assert.equal(result.fireLash.tappedThenUntapped, true);
  assert.equal(result.fireLash.survivedWithOneDamage, true);
  assert.equal(result.fireLash.damageBeforeUntap, true);
  assert.equal(result.fireLash.causalEventsVerified, true);
  assert.equal(result.fireLash.otherStatePreserved, true);
  assert.equal(result.fireLash.noDeathTerminalOrRandomEffects, true);
  assert.equal(result.fireLash.spellEnteredCemetery, true);
  assert.equal(result.fireLash.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireLash.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireLash.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.fireLash.deck.spellbook
    .find(({ name }) => name === 'Lash')?.copies, 4);
  assert.equal(result.fireLash.deck.spellbook
    .find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.fireLash.replayVerified, true);
  assert.equal(result.fireLeapAttack.leapAttack, 'Leap Attack');
  assert.equal(result.fireLeapAttack.raalDromedary, 'Raal Dromedary');
  assert.equal(result.fireLeapAttack.acceptedActionCount, 19);
  assert.equal(result.fireLeapAttack.exactOptionalStepChoices, true);
  assert.equal(result.fireLeapAttack.manaPaid, 4);
  assert.equal(result.fireLeapAttack.allySteppedWithoutTapOrDamage, true);
  assert.equal(result.fireLeapAttack.struckAndKilledEveryEnemy, true);
  assert.equal(result.fireLeapAttack.causalEventsVerified, true);
  assert.equal(result.fireLeapAttack.noAttackResponseOrRandomness, true);
  assert.equal(result.fireLeapAttack.sitesAndAvatarsPreserved, true);
  assert.equal(result.fireLeapAttack.spellEnteredCemetery, true);
  assert.equal(result.fireLeapAttack.gameRemainedActive, true);
  assert.equal(result.fireLeapAttack.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireLeapAttack.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireLeapAttack.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.fireLeapAttack.deck.spellbook
    .find(({ name }) => name === 'Leap Attack')?.copies, 3);
  assert.equal(result.fireLeapAttack.deck.spellbook
    .find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.fireLeapAttack.replayVerified, true);
  assert.equal(result.fireRecklessSquire.recklessSquire, 'Reckless Squire');
  assert.equal(result.fireRecklessSquire.raalDromedary, 'Raal Dromedary');
  assert.equal(result.fireRecklessSquire.acceptedActionCount, 27);
  assert.equal(result.fireRecklessSquire.lanceCreatedAndCarried, true);
  assert.equal(result.fireRecklessSquire.firstStrikeLanceDamage, true);
  assert.equal(result.fireRecklessSquire.lanceUsedAndRemoved, true);
  assert.equal(result.fireRecklessSquire.secondStrikeNormal, true);
  assert.equal(result.fireRecklessSquire.causalEventsVerified, true);
  assert.equal(result.fireRecklessSquire.stateAndCemeteriesVerified, true);
  assert.equal(result.fireRecklessSquire.noRandomDraws, true);
  assert.equal(result.fireRecklessSquire.gameRemainedActive, true);
  assert.equal(result.fireRecklessSquire.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireRecklessSquire.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireRecklessSquire.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.fireRecklessSquire.deck.spellbook
    .find(({ name }) => name === 'Reckless Squire')?.copies, 4);
  assert.equal(result.fireRecklessSquire.deck.spellbook
    .find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.fireRecklessSquire.replayVerified, true);
  assert.equal(result.fireGenesisLifeLoss.lesserBloodDemon, 'Lesser Blood Demon');
  assert.equal(result.fireGenesisLifeLoss.acceptedActionCount, 10);
  assert.equal(result.fireGenesisLifeLoss.summonedAtC3, true);
  assert.equal(result.fireGenesisLifeLoss.lifeLost, 2);
  assert.equal(result.fireGenesisLifeLoss.lifeAfter, 18);
  assert.equal(result.fireGenesisLifeLoss.causalEventsVerified, true);
  assert.equal(result.fireGenesisLifeLoss.noDamageDeathOrTerminalEvents, true);
  assert.equal(result.fireGenesisLifeLoss.noRandomDraws, true);
  assert.equal(result.fireGenesisLifeLoss.otherStatePreserved, true);
  assert.equal(result.fireGenesisLifeLoss.cemeteriesUnchanged, true);
  assert.equal(result.fireGenesisLifeLoss.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireGenesisLifeLoss.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireGenesisLifeLoss.deck.spellbook
    .find(({ name }) => name === 'Lesser Blood Demon')?.copies, 4);
  assert.equal(result.fireGenesisLifeLoss.replayVerified, true);
  assert.equal(result.fireIgnited.ignited, 'Ignited');
  assert.equal(result.fireIgnited.acceptedActionCount, 11);
  assert.equal(result.fireIgnited.manaPaid, 2);
  assert.equal(result.fireIgnited.summonedStateVerified, true);
  assert.equal(result.fireIgnited.chargeActionAvailableImmediately, true);
  assert.equal(result.fireIgnited.mandatoryDeathAndCemetery, true);
  assert.equal(result.fireIgnited.causalEventsVerified, true);
  assert.equal(result.fireIgnited.noDeathriteDamageTerminalOrRandomEffects, true);
  assert.equal(result.fireIgnited.otherStatePreserved, true);
  assert.equal(result.fireIgnited.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireIgnited.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireIgnited.deck.spellbook
    .find(({ name }) => name === 'Ignited')?.copies, 4);
  assert.equal(result.fireIgnited.replayVerified, true);
  assert.equal(result.fireSacredScarabs.sacredScarabs, 'Sacred Scarabs');
  assert.equal(result.fireSacredScarabs.raalDromedary, 'Raal Dromedary');
  assert.equal(result.fireSacredScarabs.acceptedActionCount, 23);
  assert.equal(result.fireSacredScarabs.seed, 135);
  assert.equal(result.fireSacredScarabs.normalFightKilledScarab, true);
  assert.equal(result.fireSacredScarabs.normalStrikeWoundedRaal, true);
  assert.equal(result.fireSacredScarabs.deathriteDamagedAvatar, true);
  assert.equal(result.fireSacredScarabs.deathriteFinishedRaal, true);
  assert.equal(result.fireSacredScarabs.exactCausalReceipts, true);
  assert.equal(result.fireSacredScarabs.noRandomDraws, true);
  assert.equal(result.fireSacredScarabs.unsupportedMechanicsAbsent, true);
  assert.equal(result.fireSacredScarabs.legalConstructedDeck, true);
  assert.equal(result.fireSacredScarabs.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.fireSacredScarabs.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.fireSacredScarabs.deck.spellbook
    .find(({ name }) => name === 'Sacred Scarabs')?.copies, 4);
  assert.equal(result.fireSacredScarabs.deck.spellbook
    .find(({ name }) => name === 'Raal Dromedary')?.copies, 4);
  assert.equal(result.fireSacredScarabs.replayVerified, true);
  assert.equal(result.combat.northMinionDied, true);
  assert.equal(result.combat.southMinionDied, true);
  assert.equal(result.waterDrown.drown, 'Drown');
  assert.equal(result.waterDrown.seravaTownsfolk, 'Serava Townsfolk');
  assert.equal(result.waterDrown.acceptedActionCount, 11);
  assert.equal(result.waterDrown.exactTargetAvailable, true);
  assert.equal(result.waterDrown.manaPaid, 3);
  assert.equal(result.waterDrown.ghostTownManaConsumed, true);
  assert.equal(result.waterDrown.transitionBeforeDeath, true);
  assert.equal(result.waterDrown.targetLeftRealm, true);
  assert.equal(result.waterDrown.targetEnteredCemetery, true);
  assert.equal(result.waterDrown.spellEnteredCemetery, true);
  assert.equal(result.waterDrown.causalEventsVerified, true);
  assert.equal(result.waterDrown.deathNotBanishmentAndGameActive, true);
  assert.equal(result.waterDrown.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterDrown.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterDrown.deck.spellbook
    .find(({ name }) => name === 'Drown')?.copies, 4);
  assert.equal(result.waterDrown.deck.spellbook
    .find(({ name }) => name === 'Serava Townsfolk')?.copies, 4);
  assert.equal(result.waterDrown.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.waterDrown.replayVerified, true);
  assert.equal(result.waterDrowned.drowned, 'Drowned');
  assert.equal(result.waterDrowned.slyFox, 'Sly Fox');
  assert.equal(result.waterDrowned.acceptedActionCount, 10);
  assert.equal(result.waterDrowned.drownedSurfaceUnavailable, true);
  assert.equal(result.waterDrowned.drownedUnderwaterAvailable, true);
  assert.equal(result.waterDrowned.slyFoxSurfaceAvailable, true);
  assert.equal(result.waterDrowned.slyFoxUnderwaterUnavailable, true);
  assert.equal(result.waterDrowned.summonedUnderwater, true);
  assert.equal(result.waterDrowned.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterDrowned.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterDrowned.deck.spellbook
    .find(({ name }) => name === 'Drowned')?.copies, 4);
  assert.equal(result.waterDrowned.replayVerified, true);
  assert.equal(result.waterLugbog.lugbogCat, 'Lugbog Cat');
  assert.equal(result.waterLugbog.slyFox, 'Sly Fox');
  assert.equal(result.waterLugbog.acceptedActionCount, 16);
  assert.equal(result.waterLugbog.enemyWaterAvailable, true);
  assert.equal(result.waterLugbog.enemyLandUnavailable, true);
  assert.equal(result.waterLugbog.slyFoxControlledWaterAvailable, true);
  assert.equal(result.waterLugbog.slyFoxEnemyWaterUnavailable, true);
  assert.equal(result.waterLugbog.summonedToEnemyWater, true);
  assert.equal(result.waterLugbog.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterLugbog.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterLugbog.deck.spellbook
    .find(({ name }) => name === 'Lugbog Cat')?.copies, 3);
  assert.equal(result.waterLugbog.deck.spellbook
    .find(({ name }) => name === 'Sly Fox')?.copies, 4);
  assert.equal(result.waterLugbog.replayVerified, true);
  assert.equal(result.waterEndTurnStealth.slyFox, 'Sly Fox');
  assert.equal(result.waterEndTurnStealth.acceptedActionCount, 23);
  assert.equal(result.waterEndTurnStealth.summonedUnstealthed, true);
  assert.equal(result.waterEndTurnStealth.gainedStealthAtEndOfTurn, true);
  assert.equal(result.waterEndTurnStealth.coLocatedReadyAttacker, true);
  assert.equal(result.waterEndTurnStealth.attackSiteAvailable, true);
  assert.equal(result.waterEndTurnStealth.slyFoxAttackUnavailable, true);
  assert.equal(result.waterEndTurnStealth.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterEndTurnStealth.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterEndTurnStealth.deck.spellbook
    .find(({ name }) => name === 'Sly Fox')?.copies, 4);
  assert.equal(result.waterEndTurnStealth.replayVerified, true);
  assert.equal(result.waterSidewaysMovement.sedgeCrabs, 'Sedge Crabs');
  assert.equal(result.waterSidewaysMovement.acceptedActionCount, 22);
  assert.equal(result.waterSidewaysMovement.sidewaysPathAvailable, true);
  assert.equal(result.waterSidewaysMovement.forwardPathUnavailable, true);
  assert.equal(result.waterSidewaysMovement.backwardPathUnavailable, true);
  assert.equal(result.waterSidewaysMovement.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterSidewaysMovement.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterSidewaysMovement.deck.spellbook
    .find(({ name }) => name === 'Sedge Crabs')?.copies, 4);
  assert.deepEqual(result.waterSidewaysMovement.deck, result.waterEndTurnStealth.deck);
  assert.equal(result.waterSidewaysMovement.replayVerified, true);
  assert.equal(result.waterSubmerge.submergeMinion, 'Coral-Reef Kelpie');
  assert.equal(result.waterSubmerge.seaWitch, 'Sea Witch');
  assert.equal(result.waterSubmerge.freeze, 'Freeze');
  assert.equal(result.waterSubmerge.acceptedActionCount, 22);
  assert.equal(result.waterSubmerge.targetIsWaterSite, true);
  assert.equal(result.waterSubmerge.surfaceSummonAvailable, true);
  assert.equal(result.waterSubmerge.underwaterSummonAvailable, true);
  assert.equal(result.waterSubmerge.nonSubmergeSurfaceAvailable, true);
  assert.equal(result.waterSubmerge.nonSubmergeUnderwaterUnavailable, true);
  assert.equal(result.waterSubmerge.summonedUnderwater, true);
  assert.equal(result.waterSubmerge.underwaterFreezeSettlementVerified, true);
  assert.equal(result.waterSubmerge.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterSubmerge.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterSubmerge.deck.spellbook
    .find(({ name }) => name === 'Coral-Reef Kelpie')?.copies, 4);
  assert.equal(result.waterSubmerge.deck.spellbook
    .find(({ name }) => name === 'Sea Witch')?.copies, 4);
  assert.equal(result.waterSubmerge.deck.spellbook
    .find(({ name }) => name === 'Freeze')?.copies, 4);
  assert.equal(result.waterSubmerge.replayVerified, true);
  assert.equal(result.waterFreeze.freeze, 'Freeze');
  assert.equal(result.waterFreeze.seravaTownsfolk, 'Serava Townsfolk');
  assert.equal(result.waterFreeze.acceptedActionCount, 15);
  assert.equal(result.waterFreeze.actionAvailableBefore, true);
  assert.equal(result.waterFreeze.disabledStateRecorded, true);
  assert.equal(result.waterFreeze.actionUnavailableWhileDisabled, true);
  assert.equal(result.waterFreeze.disabledThroughOpponentTurn, true);
  assert.equal(result.waterFreeze.expiredAtCasterStart, true);
  assert.equal(result.waterFreeze.actionReturnedOnNextTurn, true);
  assert.equal(result.waterFreeze.manaPaid, 1);
  assert.equal(result.waterFreeze.spellEnteredCemetery, true);
  assert.equal(result.waterFreeze.unitStatePreserved, true);
  assert.equal(result.waterFreeze.causalEventsVerified, true);
  assert.equal(result.waterFreeze.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterFreeze.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterFreeze.deck.spellbook
    .find(({ name }) => name === 'Freeze')?.copies, 4);
  assert.equal(result.waterFreeze.deck.spellbook
    .find(({ name }) => name === 'Serava Townsfolk')?.copies, 4);
  assert.equal(result.waterFreeze.replayVerified, true);
  assert.equal(result.waterGnarledWendigo.gnarledWendigo, 'Gnarled Wendigo');
  assert.equal(result.waterGnarledWendigo.seravaTownsfolk, 'Serava Townsfolk');
  assert.equal(result.waterGnarledWendigo.acceptedActionCount, 17);
  assert.equal(result.waterGnarledWendigo.noNormalManaSummon, true);
  assert.equal(result.waterGnarledWendigo.exactDiscountedSummonAvailable, true);
  assert.equal(result.waterGnarledWendigo.canonicalSacrificeChoice, true);
  assert.equal(result.waterGnarledWendigo.causalEventsVerified, true);
  assert.equal(result.waterGnarledWendigo.manaPaid, 4);
  assert.equal(result.waterGnarledWendigo.ghostTownManaConsumed, true);
  assert.equal(result.waterGnarledWendigo.handRealmCemeteryVerified, true);
  assert.equal(result.waterGnarledWendigo.summonedAtC4, true);
  assert.equal(result.waterGnarledWendigo.stateVersionAdvancedOnce, true);
  assert.equal(result.waterGnarledWendigo.noRandomOrUnrelatedEffects, true);
  assert.equal(result.waterGnarledWendigo.gameRemainedActive, true);
  assert.equal(result.waterGnarledWendigo.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterGnarledWendigo.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterGnarledWendigo.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.waterGnarledWendigo.deck.spellbook
    .find(({ name }) => name === 'Gnarled Wendigo')?.copies, 3);
  assert.equal(result.waterGnarledWendigo.deck.spellbook
    .find(({ name }) => name === 'Serava Townsfolk')?.copies, 4);
  assert.equal(result.waterGnarledWendigo.replayVerified, true);
  assert.equal(result.waterLure.lure, 'Lure');
  assert.equal(result.waterLure.seravaTownsfolk, 'Serava Townsfolk');
  assert.equal(result.waterLure.acceptedActionCount, 17);
  assert.equal(result.waterLure.exactNonTargetChoices, true);
  assert.equal(result.waterLure.uniqueStepResolved, true);
  assert.equal(result.waterLure.manaPaid, 1);
  assert.equal(result.waterLure.allyUnchanged, true);
  assert.equal(result.waterLure.noCombatDamageOrTap, true);
  assert.equal(result.waterLure.targetCemeteriesUnchanged, true);
  assert.equal(result.waterLure.spellEnteredCemetery, true);
  assert.equal(result.waterLure.noRandomDraws, true);
  assert.equal(result.waterLure.causalEventsVerified, true);
  assert.equal(result.waterLure.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterLure.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterLure.deck.spellbook
    .find(({ name }) => name === 'Lure')?.copies, 4);
  assert.equal(result.waterLure.deck.spellbook
    .find(({ name }) => name === 'Serava Townsfolk')?.copies, 4);
  assert.equal(result.waterLure.replayVerified, true);
  assertMesmerism(result.waterMesmerism);
  assert.equal(result.waterPirateShip.pirateShip, 'Pirate Ship');
  assert.equal(result.waterPirateShip.ghostTown, 'Ghost Town');
  assert.equal(result.waterPirateShip.acceptedActionCount, 25);
  assert.equal(result.waterPirateShip.enabledAtWater, true);
  assert.equal(result.waterPirateShip.exactMoveAvailable, true);
  assert.equal(result.waterPirateShip.disabledAtLand, true);
  assert.equal(result.waterPirateShip.noSubsequentUnitActions, true);
  assert.equal(result.waterPirateShip.ghostTownManaUsed, true);
  assert.equal(result.waterPirateShip.movementEventVerified, true);
  assert.equal(result.waterPirateShip.unitStatePreserved, true);
  assert.equal(result.waterPirateShip.sitesUnchanged, true);
  assert.equal(result.waterPirateShip.noCombatDamageDeathOrRandomness, true);
  assert.equal(result.waterPirateShip.deck.atlas
    .reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterPirateShip.deck.spellbook
    .reduce((total, card) => total + card.copies, 0), 60);
  assert.equal(result.waterPirateShip.deck.atlas
    .find(({ name }) => name === 'Ghost Town')?.copies, 3);
  assert.equal(result.waterPirateShip.deck.spellbook
    .find(({ name }) => name === 'Pirate Ship')?.copies, 4);
  assert.equal(result.waterPirateShip.replayVerified, true);
  assert.equal(result.waterHealing.healingMinion, 'Muddy Pigs');
  assert.equal(result.waterHealing.healed, 3);
  assert.equal(result.waterHealing.healedBeforeCemetery, true);
  assert.equal(result.waterHealing.healingMinionDied, true);
  assert.equal(result.waterHealing.opponentMinionDied, true);
  assert.equal(result.waterHealing.deck.atlas.reduce((total, card) => total + card.copies, 0), 30);
  assert.equal(result.waterHealing.deck.spellbook.reduce((total, card) => total + card.copies, 0), 60);
  assert.deepEqual(result.waterEndTurnStealth.deck, result.waterHealing.deck);
  assert.equal(result.waterHealing.replayVerified, true);
  assert.equal(result.replayVerified, true);
});
