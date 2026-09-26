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

test('Duel fixture regenerates byte-identically from Rust legality', async () => {
  assert.equal(await serializeDuelActionParityFixture(), readFileSync(fixtureUrl, 'utf8'));
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.source, 'rust-legality-engine');
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
    'sha256:97500225cb1f606d4e25ee82693765706fa08166c99fc00b2c9119ac91adabf1',
  );
  assert.equal(
    fixture.normalTransition.stateHash,
    'sha256:485a86fb59b996e917aa0393e14c78923c464ad1e43339286c5f10bad545d57f',
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
    'sha256:26af52f8822e7706508c90c9fa788691915f93af5e992f25a59bdfdb775e8774',
  );
  assert.equal(
    fixture.wardedTransition.stateHash,
    'sha256:84b0eb83e63d9e3be3bc5fbc7d7d5cf1d02ecd3d0027f840645c92704f8c98b1',
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
  assert.equal(underground.pending.phase, 'trigger-order');
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
    'sha256:a6f909c96d85929ecb7bedba688ef04b41db8a99e71a07e763dc1f87c9ffee32',
  );
  assert.equal(
    underground.pending.stateHash,
    'sha256:75222649ee372363f8866f500afd4d707df15aa3315d4c13d821c5fa79cfe8ba',
  );
  assert.equal(underground.pending.orderActions.length, 2);
  assert.equal(underground.resolved.selectedOrderActionId, underground.pending.orderActions[0]?.actionId);
  assert.deepEqual(underground.resolved.receipt.events.map(({ type }) => type), [
    'trigger-order-committed',
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
    'sha256:bc77307efe4e4b3635bf16eceaa9cd8aadc9c40ea8b049c488c574a7e22adb21',
  );
  assert.equal(
    underground.resolved.stateHash,
    'sha256:d3a68eb7735cff9639b69e3167ad0519391d2c39e5eb1b783adddea6642c2dd8',
  );
  assert.equal(underground.resolved.phase, 'main');
  assert.equal(underground.resolved.decisionSeat, 'north');
  assert.equal(underground.resolved.replayVerified, true);
});
