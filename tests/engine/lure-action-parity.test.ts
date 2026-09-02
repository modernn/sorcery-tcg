import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeLureActionParityFixture } from '../../scripts/capture-lure-action-parity.ts';
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
  canonicalActionIds: readonly string[];
  contract: string;
  firstLureTransition: Transition;
  mobileEnemyInstanceId: string;
  noOpTransition: Transition;
  schemaVersion: number;
  seat: 'north';
  secondLureAction: FixtureAction;
  source: string;
  stateVersion: number;
}>;

const fixtureUrl = new URL('./fixtures/lure-action-v1.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

type LureDescriptor = EngineActionDescriptor & Readonly<{
  kind: 'cast-magic';
  ally?: Readonly<{ instanceId: string }>;
  temptedDestination?: Readonly<{ cell: string }>;
  temptedEnemy?: Readonly<{ instanceId: string }>;
  target?: unknown;
}>;

function lureDescriptor(descriptor: EngineActionDescriptor): LureDescriptor | undefined {
  return descriptor.kind === 'cast-magic' ? descriptor as LureDescriptor : undefined;
}

test('Lure fixture regenerates byte-identically from TypeScript legality', () => {
  assert.equal(serializeLureActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'typescript-legality-engine');
  assert.equal(fixture.actions.length, 2);

  for (const action of fixture.actions) {
    const lure = lureDescriptor(action.descriptor);
    assert.ok(lure);
    assert.equal(lure.ally?.instanceId, fixture.allyInstanceId);
    assert.equal(lure.temptedEnemy?.instanceId, fixture.mobileEnemyInstanceId);
    assert.equal(lure.target, undefined);
    assert.match(action.label, /tempts minion/);
    assert.equal(
      opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, action.descriptor),
      action.actionId,
    );
  }

  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => lureDescriptor(descriptor)?.temptedDestination?.cell).sort(),
    ['C3', 'D4'],
  );

  const ordered = orderLegalActions(fixture.actions.map((action) => ({
    ...action,
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ actionId }) => actionId), fixture.canonicalActionIds);

  assert.deepEqual(
    fixture.firstLureTransition.receipt.events.map(({ type }) => type),
    ['magic-cast', 'unit-lured', 'magic-resolved'],
  );
  assert.equal(fixture.firstLureTransition.summary.mobileLocation, 'C3');
  assert.equal(fixture.firstLureTransition.replayVerified, true);

  assert.deepEqual(
    fixture.noOpTransition.receipt.events.map(({ type }) => type),
    ['magic-cast', 'magic-resolved'],
  );
  assert.equal(fixture.noOpTransition.summary.mobileLocation, 'C4');
  assert.equal(fixture.noOpTransition.replayVerified, true);

  const second = lureDescriptor(fixture.secondLureAction.descriptor);
  assert.ok(second);
  assert.equal(second.temptedDestination?.cell, 'C4');
});
