import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeTeleportActionParityFixture } from '../../scripts/capture-teleport-action-parity.ts';
import {
  opaqueActionId,
  orderLegalActions,
  type EngineActionDescriptor,
} from '../../src/engine/contract.ts';

type FixtureAction = Readonly<{
  actionId: string;
  descriptor: EngineActionDescriptor;
  label: string;
}>;

type Transition = Readonly<{
  receipt: Readonly<{ events: readonly Readonly<{ type: string }>[] }>;
  replayVerified: boolean;
  summary: Readonly<Record<string, unknown>>;
}>;

type Fixture = Readonly<{
  actions: readonly FixtureAction[];
  allyInstanceId: string;
  allyTeleportTransition: Transition;
  avatarNoMoveTransition: Transition;
  canonicalActionIds: readonly string[];
  contract: string;
  schemaVersion: number;
  seat: 'north';
  source: string;
  stateVersion: number;
  targetSiteInstanceId: string;
}>;

const fixtureUrl = new URL('./fixtures/teleport-action-v1.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

type CastMagicDescriptor = EngineActionDescriptor & Readonly<{
  kind: 'cast-magic';
  ally?: Readonly<{ instanceId: string; kind: string }>;
  target?: unknown;
  targetLocation?: Readonly<{ cell: string; region: string }>;
  targetSiteInstanceId?: string;
}>;

function castMagic(descriptor: EngineActionDescriptor): CastMagicDescriptor | undefined {
  return descriptor.kind === 'cast-magic' ? descriptor as CastMagicDescriptor : undefined;
}

test('Teleport fixture regenerates byte-identically from Rust legality', async () => {
  assert.equal(await serializeTeleportActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'rust-legality-engine');
  assert.equal(fixture.actions.length, 6);

  for (const action of fixture.actions) {
    const cast = castMagic(action.descriptor);
    assert.ok(cast);
    assert.equal(cast.target, undefined);
    assert.equal(cast.targetLocation?.region, 'surface');
    assert.ok(cast.targetSiteInstanceId);
    assert.equal(
      opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, action.descriptor),
      action.actionId,
    );
  }

  const ordered = orderLegalActions(fixture.actions.map((action) => ({
    ...action,
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ actionId }) => actionId), fixture.canonicalActionIds);

  assert.deepEqual(
    fixture.avatarNoMoveTransition.receipt.events.map(({ type }) => type),
    ['magic-cast', 'magic-resolved'],
  );
  assert.equal(fixture.avatarNoMoveTransition.replayVerified, true);

  assert.deepEqual(
    fixture.allyTeleportTransition.receipt.events.map(({ type }) => type),
    ['magic-cast', 'unit-teleported', 'magic-resolved'],
  );
  assert.equal(fixture.allyTeleportTransition.summary.targetSiteInstanceId, fixture.targetSiteInstanceId);
  assert.deepEqual(fixture.allyTeleportTransition.summary.unit, {
    damage: 0,
    location: 'C1',
    region: 'surface',
    stealthed: true,
    tapped: false,
    warded: true,
  });
  assert.equal(fixture.allyTeleportTransition.replayVerified, true);
});
