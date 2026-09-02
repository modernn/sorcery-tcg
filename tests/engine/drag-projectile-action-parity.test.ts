import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeDragProjectileActionParityFixture } from '../../scripts/capture-drag-projectile-action-parity.ts';
import {
  opaqueActionId,
  orderLegalActions,
  type EngineActionDescriptor,
} from '../../src/engine/contract.ts';

type Receipt = Readonly<{
  events: readonly Readonly<{ type: string }>[];
}>;

type FixtureAction = Readonly<{
  actionId: string;
  descriptor: EngineActionDescriptor;
  label: string;
}>;

type Transition = Readonly<{
  receipt: Receipt;
  replayVerified: boolean;
  selectedActionId: string;
  summary: Readonly<{
    eventTypes: readonly string[];
    pudge: Readonly<Record<string, unknown>>;
    target: Readonly<Record<string, unknown>>;
  }>;
}>;

type MovementDeathrite = Readonly<{
  pending: Readonly<{
    checkpointRoundTrip: boolean;
    continuation: Readonly<{ kind: string }>;
    phase: string;
    targetLocationAfterFirstDrag: string | null;
  }>;
  resolved: Readonly<{
    replayVerified: boolean;
    targetLocationAfterResume: string | null;
  }>;
}>;

type Fixture = Readonly<{
  actions: readonly FixtureAction[];
  canonicalActionIds: readonly string[];
  contract: string;
  fightTransition: Transition;
  movementDeathrite: MovementDeathrite;
  noFightTransition: Transition;
  schemaVersion: number;
  seat: 'north';
  shooterInstanceId: string;
  source: string;
  stateVersion: number;
  targetInstanceId: string;
}>;

const fixtureUrl = new URL('./fixtures/drag-projectile-action-v1.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

type DragDescriptor = EngineActionDescriptor & Readonly<{
  kind: 'shoot-drag-projectile';
  fightOnArrival: boolean;
  direction: string;
  hit: Readonly<{ instanceId: string }> | null;
  path: readonly Readonly<{ cell: string; region: string }>[];
}>;

function dragDescriptor(descriptor: EngineActionDescriptor): DragDescriptor | undefined {
  return descriptor.kind === 'shoot-drag-projectile'
    ? descriptor as DragDescriptor
    : undefined;
}

test('Pudge drag projectile fixture regenerates byte-identically from TypeScript legality', () => {
  assert.equal(serializeDragProjectileActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'typescript-legality-engine');
  assert.equal(fixture.actions.length, 2);

  for (const action of fixture.actions) {
    const drag = dragDescriptor(action.descriptor);
    assert.ok(drag);
    assert.equal(drag.direction, 'south');
    assert.equal(drag.hit?.instanceId, fixture.targetInstanceId);
    assert.equal(drag.shooterInstanceId, fixture.shooterInstanceId);
    assert.equal(
      opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, action.descriptor),
      action.actionId,
    );
  }

  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => dragDescriptor(descriptor)?.fightOnArrival),
    [false, true],
  );
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => dragDescriptor(descriptor)?.path.map(({ cell }) => cell).join(',')),
    ['C4,C3,C2', 'C4,C3,C2'],
  );

  const ordered = orderLegalActions(fixture.actions.map((action) => ({
    ...action,
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ actionId }) => actionId), fixture.canonicalActionIds);

  assert.deepEqual(fixture.noFightTransition.summary.eventTypes, [
    'projectile-shot',
    'unit-dragged',
  ]);
  assert.deepEqual(fixture.noFightTransition.summary.target, {
    damage: 0,
    location: 'C4',
    tapped: false,
    warded: true,
  });
  assert.equal(fixture.noFightTransition.replayVerified, true);

  assert.ok(fixture.fightTransition.summary.eventTypes.includes('fight-started'));
  assert.deepEqual(fixture.fightTransition.summary.pudge, {
    damage: 3,
    location: 'C4',
    tapped: true,
    warded: false,
  });
  assert.equal(fixture.fightTransition.replayVerified, true);

  assert.equal(fixture.movementDeathrite.pending.continuation.kind, 'drag-projectile');
  assert.equal(fixture.movementDeathrite.pending.phase, 'deathrite-order');
  assert.equal(fixture.movementDeathrite.pending.checkpointRoundTrip, true);
  assert.equal(fixture.movementDeathrite.pending.targetLocationAfterFirstDrag, 'C3');
  assert.equal(fixture.movementDeathrite.resolved.targetLocationAfterResume, 'C4');
  assert.equal(fixture.movementDeathrite.resolved.replayVerified, true);
});
