import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeLeapAttackActionParityFixture } from '../../scripts/capture-leap-attack-action-parity.ts';
import {
  opaqueActionId,
  orderLegalActions,
  type EngineActionDescriptor,
} from '../../src/engine/contract.ts';

type Receipt = Readonly<{
  events: readonly Readonly<{ type: string }>[];
  randomDraws: readonly unknown[];
  receiptId: string;
}>;

type FixtureAction = Readonly<{
  actionId: string;
  descriptor: EngineActionDescriptor;
  label: string;
  seat?: 'north' | 'south';
  stateVersion?: number;
}>;

type Transition = Readonly<{
  checkpointId: string;
  expectedSessionHash: string;
  receipt: Receipt;
  replayVerified: boolean;
  selectedActionId: string;
  serializedCheckpointHash: string;
  stateHash: string;
  summary?: Readonly<Record<string, unknown>>;
}>;

type MovementDeathrite = Readonly<{
  leapAction?: FixtureAction;
  manifestId: string;
  pending: Readonly<{
    checkpointId: string;
    checkpointRoundTrip: boolean;
    continuation: Readonly<{
      kind: string;
      pending?: Readonly<Record<string, unknown>>;
    }>;
    decisionSeat: string;
    orderActions?: readonly FixtureAction[];
    phase: string;
    receipt: Receipt;
    serializedCheckpointHash: string;
    stateHash: string;
    stateVersion: number;
  }>;
  resolved: Readonly<{
    checkpointId: string;
    decisionSeat: string;
    phase: string;
    receipt: Receipt;
    replayVerified: boolean;
    selectedOrderActionId?: string;
    serializedCheckpointHash: string;
    stateHash: string;
    stateVersion: number;
  }>;
}>;

type Fixture = Readonly<{
  actions: readonly FixtureAction[];
  allyInstanceId: string;
  canonicalActionIds: readonly string[];
  contract: string;
  manifestId: string;
  movementDeathrite: MovementDeathrite;
  noStepTransition: Transition;
  schemaVersion: number;
  seat: 'north';
  source: string;
  stateVersion: number;
  stepTransition: Transition;
}>;

const fixtureUrl = new URL('./fixtures/leap-attack-action-v1.json', import.meta.url);
const fixtureExists = existsSync(fixtureUrl);
const fixture = fixtureExists
  ? JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture
  : undefined;

type CastMagicDescriptor = EngineActionDescriptor & Readonly<{
  kind: 'cast-magic';
  allyDestination?: Readonly<{
    cell?: string;
    region?: string;
  }> | null;
}>;

function castMagicDescriptor(
  descriptor: EngineActionDescriptor,
): CastMagicDescriptor | undefined {
  return descriptor.kind === 'cast-magic'
    ? descriptor as CastMagicDescriptor
    : undefined;
}

function eventTypes(receipt: Receipt): readonly string[] {
  return receipt.events.map(({ type }) => type);
}

test('Leap Attack fixture regenerates byte-identically from Rust legality', async () => {
  assert.ok(fixture, 'fixture missing: run capture-leap-attack-action-parity.ts --write');
  assert.equal(await serializeLeapAttackActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'rust-legality-engine');

  // hashes filled after capture --write

  const stayAction = fixture.actions.find(({ descriptor }) => {
    const cast = castMagicDescriptor(descriptor);
    return cast?.allyDestination?.cell === 'C4';
  });
  const stepAction = fixture.actions.find(({ descriptor }) => {
    const cast = castMagicDescriptor(descriptor);
    return cast?.allyDestination?.cell === 'C3';
  });
  assert.ok(stayAction, 'expected stay-at-C4 Leap Attack action');
  assert.ok(stepAction, 'expected step-to-C3 Leap Attack action');
  assert.match(stayAction.label, /stays/);
  assert.match(stepAction.label, /steps to C3/);

  for (const action of fixture.actions) {
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

  assert.equal(fixture.noStepTransition.selectedActionId, stayAction.actionId);
  const noStepEvents = eventTypes(fixture.noStepTransition.receipt);
  assert.equal(noStepEvents.includes('unit-stepped'), false);
  assert.ok(noStepEvents.includes('strike-damage-allocated'));
  assert.ok(
    noStepEvents.at(-1) === 'magic-resolved' || noStepEvents.includes('magic-resolved'),
  );
  assert.equal(fixture.noStepTransition.replayVerified, true);

  assert.equal(fixture.stepTransition.selectedActionId, stepAction.actionId);
  const stepEvents = eventTypes(fixture.stepTransition.receipt);
  assert.ok(stepEvents.includes('unit-stepped'));
  assert.ok(stepEvents.includes('magic-resolved'));
  assert.equal(stepEvents.includes('fight-started'), false);
  assert.ok(
    stepEvents.filter((type) => type === 'strike-damage-allocated').length
      > noStepEvents.filter((type) => type === 'strike-damage-allocated').length,
  );
  assert.equal(fixture.stepTransition.replayVerified, true);

  const movement = fixture.movementDeathrite;
  assert.equal(movement.pending.phase, 'deathrite-order');
  assert.equal(movement.pending.checkpointRoundTrip, true);
  assert.equal(movement.pending.continuation.kind, 'leap-attack');
  assert.deepEqual(eventTypes(movement.pending.receipt), ['magic-cast', 'unit-stepped']);
  assert.equal(eventTypes(movement.resolved.receipt).at(-1), 'magic-resolved');
  assert.equal(movement.resolved.replayVerified, true);
});
