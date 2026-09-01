import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeShootProjectileActionParityFixture } from '../../scripts/capture-shoot-projectile-action-parity.ts';
import {
  opaqueActionId,
  orderLegalActions,
  type EngineActionDescriptor,
} from '../../src/engine/contract.ts';

type Fixture = Readonly<{
  actions: readonly Readonly<{
    actionId: string;
    descriptor: EngineActionDescriptor;
    label: string;
  }>[];
  canonicalActionIds: readonly string[];
  contract: string;
  schemaVersion: number;
  seat: 'north';
  shooterInstanceId: string;
  source: string;
  stateVersion: number;
  targetInstanceId: string;
}>;

const fixtureUrl = new URL('./fixtures/shoot-projectile-action-v1.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

test('ordinary Ranged Shoot Projectile fixture regenerates byte-identically from TypeScript legality', () => {
  assert.equal(serializeShootProjectileActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'typescript-legality-engine');
  assert.equal(fixture.actions.length, 4);
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.kind),
    Array(4).fill('shoot-projectile'),
  );
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.direction),
    ['east', 'north', 'south', 'west'],
  );
  assert.deepEqual(
    fixture.actions.map(({ label }) => label),
    [
      'Shoot east at nothing',
      'Shoot north at nothing',
      `Shoot south at minion ${fixture.targetInstanceId.slice(0, 15)}…`,
      'Shoot west at nothing',
    ],
  );

  for (const action of fixture.actions) {
    assert.deepEqual(Object.keys(action.descriptor).sort(), [
      'direction',
      'hit',
      'kind',
      'path',
      'shooterInstanceId',
    ]);
    assert.equal(action.descriptor.shooterInstanceId, fixture.shooterInstanceId);
    assert.equal(
      opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, action.descriptor),
      action.actionId,
    );
  }
  const south = fixture.actions[2]!.descriptor;
  assert.deepEqual(south.hit, {
    instanceId: fixture.targetInstanceId,
    kind: 'minion',
    seat: 'south',
  });
  assert.deepEqual(
    (south.path as readonly Readonly<{ cell: string; region: string }>[]),
    [
      { cell: 'C4', region: 'surface' },
      { cell: 'C3', region: 'surface' },
    ],
  );
  assert.equal(fixture.actions.filter(({ descriptor }) => descriptor.hit === null).length, 3);

  const ordered = orderLegalActions(fixture.actions.map((action) => ({
    ...action,
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ actionId }) => actionId), fixture.canonicalActionIds);
});
