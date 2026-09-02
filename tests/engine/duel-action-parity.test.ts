import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { serializeDuelActionParityFixture } from '../../scripts/capture-duel-action-parity.ts';
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
  summary: Readonly<{
    allyDamage: number;
    mana: number;
    targetPresent: boolean;
    targetWarded: boolean | null;
  }>;
}>;

type Fixture = Readonly<{
  actions: readonly FixtureAction[];
  allyInstanceId: string;
  canonicalActionIds: readonly string[];
  contract: string;
  manifestId: string;
  normalTargetInstanceId: string;
  normalTransition: Transition;
  schemaVersion: number;
  seat: 'north';
  source: string;
  stateVersion: number;
  undergroundDeathrite: Readonly<{
    duelAction: FixtureAction;
    manifestId: string;
    pending: Readonly<{
      checkpointId: string;
      checkpointRoundTrip: boolean;
      continuation: Readonly<{
        attackerStrikesFirst: boolean;
        firstCombatantInstanceIds: readonly string[];
        kind: 'first-strike';
        pending: Readonly<{ region?: string }>;
      }>;
      decisionSeat: string;
      orderActions: readonly FixtureAction[];
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
      selectedOrderActionId: string;
      serializedCheckpointHash: string;
      stateHash: string;
      stateVersion: number;
    }>;
  }>;
  wardedTargetInstanceId: string;
  wardedTransition: Transition;
}>;

const fixtureUrl = new URL('./fixtures/duel-action-v1.json', import.meta.url);
const fixture = JSON.parse(readFileSync(fixtureUrl, 'utf8')) as Fixture;

function instanceId(value: unknown): unknown {
  return value !== null && typeof value === 'object' && 'instanceId' in value
    ? value.instanceId
    : undefined;
}

test('Duel fixture regenerates byte-identically from TypeScript legality', () => {
  assert.equal(serializeDuelActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'typescript-legality-engine');
  assert.equal(fixture.actions.length, 2);
  assert.deepEqual(
    fixture.actions.map(({ descriptor }) => descriptor.kind),
    ['cast-magic', 'cast-magic'],
  );
  assert.equal(
    fixture.manifestId,
    'sha256:569fa17a0058f8338ab3110fc0936efa59d344e6d65c08d8474b776e704de3ce',
  );
  assert.deepEqual(fixture.actions.map(({ actionId }) => actionId), [
    'sha256:3112b1da8b90fd5ce6fcdfdef7166aefc4dcba47530ce41d5488a2295d57277b',
    'sha256:bba867cef8f17effe2c0529ea6e6c768fc534efa4bd8b58a5b526e2caba421ab',
  ]);
  assert.deepEqual(fixture.actions.map(({ label }) => label), [
    'Cast synthetic-forced-duel: minion sha256:4f8d832b… fights minion sha256:bad42974…',
    'Cast synthetic-forced-duel: minion sha256:4f8d832b… fights minion sha256:c379d65b…',
  ]);
  assert.deepEqual(fixture.actions.map(({ descriptor }) => descriptor.kind === 'cast-magic'
    ? {
      ally: instanceId(descriptor.ally),
      target: instanceId(descriptor.target),
    }
    : undefined), [
    { ally: fixture.allyInstanceId, target: fixture.wardedTargetInstanceId },
    { ally: fixture.allyInstanceId, target: fixture.normalTargetInstanceId },
  ]);
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
  assert.equal(fixture.normalTransition.selectedActionId, fixture.actions[1]?.actionId);
  assert.deepEqual(fixture.normalTransition.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'fight-started',
    'strike-damage-allocated',
    'damage-dealt',
    'damage-dealt',
    'minion-died',
    'magic-resolved',
  ]);
  assert.deepEqual(fixture.normalTransition.receipt.randomDraws, []);
  assert.deepEqual(fixture.normalTransition.summary, {
    allyDamage: 2,
    mana: 0,
    targetPresent: false,
    targetWarded: null,
  });
  assert.equal(
    fixture.normalTransition.receipt.receiptId,
    'sha256:ee971fb48fd345615bd54b650c3d2c92d0f6977e35c5c070be8049d1753b93ab',
  );
  assert.equal(
    fixture.normalTransition.stateHash,
    'sha256:0e86b939246e778b4031bed82bb8de227be72728fc31c41ec3c1cd076c5bf76e',
  );
  assert.equal(fixture.normalTransition.replayVerified, true);

  assert.equal(fixture.wardedTransition.selectedActionId, fixture.actions[0]?.actionId);
  assert.deepEqual(fixture.wardedTransition.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'ward-broken',
    'magic-resolved',
  ]);
  assert.deepEqual(fixture.wardedTransition.receipt.randomDraws, []);
  assert.deepEqual(fixture.wardedTransition.summary, {
    allyDamage: 0,
    mana: 0,
    targetPresent: true,
    targetWarded: false,
  });
  assert.equal(
    fixture.wardedTransition.receipt.receiptId,
    'sha256:898d8c6803c8e9e71ca0aac04b37ac2656c2a063e9b2bf79436e9e38d2ac1b38',
  );
  assert.equal(
    fixture.wardedTransition.stateHash,
    'sha256:2fcb419fe7becd715616acf8a8ba59b24fe5861ea6d408c65fd481fea6443edb',
  );
  assert.equal(fixture.wardedTransition.replayVerified, true);

  const underground = fixture.undergroundDeathrite;
  assert.equal(
    underground.manifestId,
    'sha256:8f857338748d12ac172ff982ca5e9029b3eac53ee84f284e47dad9c9e6eb0338',
  );
  assert.equal(
    underground.duelAction.actionId,
    'sha256:fc0f2936ee40e40b397c23d8692e71fecd950777c09c8936925bd6b87124114d',
  );
  assert.equal(underground.pending.phase, 'deathrite-order');
  assert.equal(underground.pending.decisionSeat, 'south');
  assert.equal(underground.pending.checkpointRoundTrip, true);
  assert.equal(underground.pending.continuation.kind, 'first-strike');
  assert.equal(underground.pending.continuation.attackerStrikesFirst, true);
  assert.deepEqual(underground.pending.continuation.firstCombatantInstanceIds, []);
  assert.equal(underground.pending.continuation.pending.region, 'underground');
  assert.deepEqual(underground.pending.receipt.events.map(({ type }) => type), [
    'magic-cast',
    'fight-started',
    'strike-damage-allocated',
    'damage-dealt',
    'deathrite-damage-allocated',
    'deathrite-damage-allocated',
    'deathrite-damage-allocated',
    'damage-dealt',
    'damage-dealt',
    'damage-dealt',
  ]);
  assert.deepEqual(underground.pending.receipt.randomDraws, []);
  assert.equal(
    underground.pending.receipt.receiptId,
    'sha256:4aa31626e11c88edb403e68482d1ab58362809e76e8ed0f6c66037a572b4a98e',
  );
  assert.equal(
    underground.pending.stateHash,
    'sha256:18593a0971bc7f00d80fdbb5c87b7755d9509f772f8b65fb7ca2eb870ce6cc25',
  );
  assert.equal(underground.pending.orderActions.length, 2);
  assert.equal(underground.resolved.selectedOrderActionId, underground.pending.orderActions[0]?.actionId);
  assert.deepEqual(underground.resolved.receipt.events.map(({ type }) => type), [
    'deathrite-order-committed',
    'site-drawn',
    'site-drawn',
    'minion-died',
    'minion-died',
    'minion-died',
    'magic-resolved',
  ]);
  assert.deepEqual(underground.resolved.receipt.randomDraws, []);
  assert.equal(
    underground.resolved.receipt.receiptId,
    'sha256:32cbdd58a5dc427db5f9217a587a1c88bbc473a67f52dfcc85a9ab6b3de14eb2',
  );
  assert.equal(
    underground.resolved.stateHash,
    'sha256:e5a3370614a777e90a8a9d1fdfdb555c5d35a0bd6ee7f92bc97f786ae2627c3c',
  );
  assert.equal(underground.resolved.phase, 'main');
  assert.equal(underground.resolved.decisionSeat, 'north');
  assert.equal(underground.resolved.replayVerified, true);
});
