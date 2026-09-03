import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  serializeRandomCardDiscardSummonActionParityFixture,
} from '../../scripts/capture-random-card-discard-summon-action-parity.ts';
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
  source: string;
  stateVersion: number;
  transition: Readonly<{
    eligibleCandidates: readonly Readonly<{ instanceId: string; zone: string }>[];
    receipt: Readonly<{
      actionId: string;
      events: readonly Readonly<{ eventId: string; payload: unknown; type: string }>[];
      postStateHash: string;
      randomDraws: readonly unknown[];
      receiptId: string;
    }>;
    selectedActionId: string;
  }>;
}>;

const fixtureUrl = new URL(
  './fixtures/random-card-discard-summon-action-v1.json',
  import.meta.url,
);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

test('random-card-discard summon fixture regenerates byte-identically from Rust legality', async () => {
  assert.equal(
    await serializeRandomCardDiscardSummonActionParityFixture(),
    readFileSync(fixtureUrl, 'utf8'),
  );
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'rust-legality-engine');
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.kind),
    Array(4).fill('summon-minion'),
  );
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => ({
      cell: descriptor.cell,
      manaCost: descriptor.manaCost,
      paymentMode: descriptor.paymentMode,
    })),
    [
      { cell: 'C3', manaCost: 0, paymentMode: 'random-card-discard' },
      { cell: 'C3', manaCost: 2, paymentMode: undefined },
      { cell: 'C4', manaCost: 0, paymentMode: 'random-card-discard' },
      { cell: 'C4', manaCost: 2, paymentMode: undefined },
    ],
  );
  assert.deepEqual(
    fixture.actions.map(({ label }) => label),
    [
      'Summon synthetic-random-discard-minion at C3 (discard random card)',
      'Summon synthetic-random-discard-minion at C3 (2 mana)',
      'Summon synthetic-random-discard-minion at C4 (discard random card)',
      'Summon synthetic-random-discard-minion at C4 (2 mana)',
    ],
  );
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
  assert.ok(fixture.transition.eligibleCandidates.length > 1);
  assert.equal(fixture.transition.selectedActionId, fixture.actions[0]!.actionId);
  assert.equal(fixture.transition.receipt.actionId, fixture.transition.selectedActionId);
  assert.deepEqual(
    fixture.transition.receipt.events.map(({ type }) => type),
    ['card-discarded', 'minion-summoned'],
  );
  assert.equal(fixture.transition.receipt.randomDraws.length, 1);
  assert.match(fixture.transition.receipt.postStateHash, /^sha256:[0-9a-f]{64}$/);
  assert.match(fixture.transition.receipt.receiptId, /^sha256:[0-9a-f]{64}$/);
});

test('canonical action ordering keeps a shorter numeric prefix before its extension', () => {
  const base = fixture.actions[1]!.descriptor;
  const descriptors = [1, 10].map((manaCost) => ({ ...base, manaCost }));
  const ordered = orderLegalActions(descriptors.map((descriptor) => ({
    actionId: opaqueActionId(fixture.contract, fixture.seat, fixture.stateVersion, descriptor),
    descriptor,
    label: '',
    seat: fixture.seat,
    stateVersion: fixture.stateVersion,
  })));
  assert.deepEqual(ordered.map(({ descriptor }) => descriptor.manaCost), [1, 10]);
});
